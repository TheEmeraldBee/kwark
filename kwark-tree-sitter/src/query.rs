use std::{ops::Range, rc::Rc};

use tree_sitter::{Query, QueryCursor, StreamingIterator};

use crate::{
    error::Error,
    position::{Source, SourceTextProvider},
    predicates::Directives,
    store::LanguageStore,
    tree::{Layer, SyntaxTree},
};

/// A type to retrieve a byte-range
pub trait Ranged {
    fn range(&self) -> Range<usize>;
}

/// One query match, with its source layer and `#set!` directives attached
pub struct Match<'a> {
    inner: &'a tree_sitter::QueryMatch<'a, 'a>,
    query: &'a Query,
    layer_language: &'a str,
}

impl<'a> Match<'a> {
    /// Iterates through all captures in the match
    pub fn iter_captures(&self) -> impl Iterator<Item = (&'a str, tree_sitter::Node<'a>)> {
        self.inner
            .captures()
            .iter()
            .map(|x| (self.query.capture_names()[x.index as usize], x.node))
    }

    /// The first node captured under `name`, if any
    pub fn capture(&self, name: &str) -> Option<tree_sitter::Node<'a>> {
        let ix = self.query.capture_index_for_name(name)?;
        self.inner.nodes_for_capture_index(ix).next()
    }

    /// Every node captured under `name`, in match order
    pub fn captures(&self, name: &str) -> Vec<tree_sitter::Node<'a>> {
        let Some(ix) = self.query.capture_index_for_name(name) else {
            return Vec::new();
        };
        self.inner.nodes_for_capture_index(ix).collect()
    }

    /// The text of the first node captured under `name`
    pub fn text(&self, name: &str, source: &impl Source) -> Option<String> {
        let node = self.capture(name)?;
        let bytes = source.slice(node.byte_range());
        std::str::from_utf8(&bytes).ok().map(str::to_string)
    }

    /// The language of the layer this match was found in
    pub fn layer_language(&self) -> &str {
        self.layer_language
    }

    /// The value of a `#set! key "value"` directive on this match's pattern
    pub fn set(&self, key: &str) -> Option<&'a str> {
        self.directives().set(key)
    }

    /// Whether a flag-only `#set! key` directive is present on this match's pattern
    pub fn is(&self, key: &str) -> bool {
        self.directives().is(key)
    }

    fn directives(&self) -> Directives<'a> {
        Directives::for_pattern(self.query, self.inner.pattern_index)
    }
}

fn run_layer<S: Source, T>(
    layer: &Layer,
    query: &Query,
    source: &S,
    byte_range: Option<Range<usize>>,
    f: &impl Fn(Match) -> T,
    out: &mut Vec<T>,
) {
    let mut cursor = QueryCursor::new();

    if let Some(range) = byte_range {
        cursor.set_byte_range(range);
    }

    let mut matches = cursor.matches(query, layer.root_node(), SourceTextProvider(source));

    while let Some(m) = matches.next() {
        let wrapped = Match {
            inner: m,
            query,
            layer_language: layer.language(),
        };
        out.push(f(wrapped));
    }
}

/// The result of `.map`
pub struct Mapped<T>(Vec<T>);

impl<T: Ranged> Mapped<T> {
    /// Sorts by range, then folds `f` across every overlapping pair, left to right
    pub fn combine(mut self, f: impl Fn(T, T) -> T) -> Combined<T> {
        self.0.sort_by_key(|t| t.range().start);

        let mut out: Vec<T> = Vec::with_capacity(self.0.len());

        for item in self.0 {
            match out.pop() {
                Some(last) if ranges_overlap(&last.range(), &item.range()) => {
                    out.push(f(last, item));
                }
                Some(last) => {
                    out.push(last);
                    out.push(item);
                }
                None => out.push(item),
            }
        }

        Combined(out)
    }
}

fn ranges_overlap(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// The result of `.combine`
pub struct Combined<T>(Vec<T>);

impl<T> Combined<T> {
    pub fn finish(self) -> Vec<T> {
        self.0
    }
}

impl SyntaxTree {
    /// Begins a query against every layer whose language defines `name`
    ///
    /// # Errors
    /// Returns an error if a language's query file exists but fails to load
    pub fn query<'t, S: Source>(
        &'t self,
        store: &mut LanguageStore,
        source: &'t S,
        name: &str,
    ) -> Result<MultiQuery<'t, S>, Error> {
        let mut per_language = Vec::new();

        for lang in self.languages() {
            if !store.has_query(&lang, name) {
                continue;
            }
            let query = store.get_query(&lang, name)?;
            per_language.push((lang, query));
        }

        Ok(MultiQuery {
            tree: self,
            source,
            queries: per_language,
            byte_range: None,
        })
    }

    fn languages(&self) -> Vec<String> {
        let mut seen = Vec::new();
        for layer in self.layers() {
            let lang = layer.language().to_string();
            if !seen.contains(&lang) {
                seen.push(lang);
            }
        }
        seen
    }
}

/// A query resolved against every language in a [`SyntaxTree`]
pub struct MultiQuery<'t, S: Source> {
    tree: &'t SyntaxTree,
    source: &'t S,
    queries: Vec<(String, Rc<Query>)>,
    byte_range: Option<Range<usize>>,
}

impl<'t, S: Source> MultiQuery<'t, S> {
    /// Restricts matching to the nodes intersecting `range`
    pub fn in_range(mut self, range: Range<usize>) -> Self {
        self.byte_range = Some(range);
        self
    }

    /// Runs each resolved query over its matching layers, applying the passed function to every [`Match`]
    pub fn map<T>(self, f: impl Fn(Match) -> T) -> Mapped<T> {
        let mut out = Vec::new();

        for layer in self.tree.layers() {
            let Some((_, query)) = self
                .queries
                .iter()
                .find(|(lang, _)| lang == layer.language())
            else {
                continue;
            };

            run_layer(
                layer,
                query,
                self.source,
                self.byte_range.clone(),
                &f,
                &mut out,
            );
        }

        Mapped(out)
    }

    /// Like [`Self::map`], but each [`Match`] contributes zero or more items
    pub fn flat_map<T>(self, f: impl Fn(Match) -> Vec<T>) -> Mapped<T> {
        let mut grouped = Vec::new();

        for layer in self.tree.layers() {
            let Some((_, query)) = self
                .queries
                .iter()
                .find(|(lang, _)| lang == layer.language())
            else {
                continue;
            };

            run_layer(
                layer,
                query,
                self.source,
                self.byte_range.clone(),
                &f,
                &mut grouped,
            );
        }

        Mapped(grouped.into_iter().flatten().collect())
    }
}
