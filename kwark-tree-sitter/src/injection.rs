use std::ops::Range;

use tree_sitter::{QueryCursor, StreamingIterator};

use crate::{
    error::Error,
    position::{Source, SourceTextProvider},
    predicates::Directives,
    store::LanguageStore,
};

/// How a candidate injection's language was resolved
pub enum LanguageRef {
    /// By name
    Named(String),

    /// The same language as self
    SelfRef,
}

/// One or more regions injected by a language's injection query
pub struct Candidate {
    pub language: LanguageRef,
    pub ranges: Vec<Range<usize>>,
}

/// Resolves `lang`'s injection query matches against `tree` into [`Candidate`]s
pub fn find_candidates<S: Source>(
    store: &mut LanguageStore,
    lang: &str,
    tree: &tree_sitter::Tree,
    source: &S,
) -> Result<Vec<Candidate>, Error> {
    if !store.has_query(lang, "injections") {
        return Ok(Vec::new());
    }

    let query = store.get_query(lang, "injections")?;

    let content_ix = query.capture_index_for_name("injection.content");
    let language_ix = query.capture_index_for_name("injection.language");

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), SourceTextProvider(source));

    // injection.combined merges ranges sharing a pattern into one Candidate
    let mut combined: Vec<(usize, Candidate)> = Vec::new();
    let mut separate = Vec::new();

    while let Some(m) = matches.next() {
        let directives = Directives::for_pattern(&query, m.pattern_index);

        let language = resolve_language(&directives, language_ix, m, source);
        let Some(language) = language else { continue };

        let ranges: Vec<Range<usize>> = content_ix
            .map(|ix| {
                m.nodes_for_capture_index(ix)
                    .map(|n| n.byte_range())
                    .collect()
            })
            .unwrap_or_default();

        if ranges.is_empty() {
            continue;
        }

        if directives.is("injection.combined") {
            if let Some((_, existing)) = combined.iter_mut().find(|(p, _)| *p == m.pattern_index) {
                existing.ranges.extend(ranges);
            } else {
                combined.push((m.pattern_index, Candidate { language, ranges }));
            }
        } else {
            separate.push(Candidate { language, ranges });
        }
    }

    let mut out: Vec<Candidate> = combined.into_iter().map(|(_, c)| c).collect();
    out.extend(separate);

    for candidate in &mut out {
        candidate.ranges.sort_by_key(|r| r.start);
    }

    Ok(out)
}

fn resolve_language<S: Source>(
    directives: &Directives,
    language_ix: Option<u32>,
    m: &tree_sitter::QueryMatch<'_, '_>,
    source: &S,
) -> Option<LanguageRef> {
    if let Some(name) = directives.set("injection.language") {
        return Some(LanguageRef::Named(name.to_string()));
    }

    if directives.is("injection.self") {
        return Some(LanguageRef::SelfRef);
    }

    if let Some(ix) = language_ix
        && let Some(node) = m.nodes_for_capture_index(ix).next()
    {
        let bytes = source.slice(node.byte_range());
        let name = std::str::from_utf8(&bytes).ok()?;
        return Some(LanguageRef::Named(name.to_string()));
    }

    None
}
