use crate::{
    error::Error,
    injection::{self, Candidate},
    position::{Edit, Source, parse_callback},
    store::LanguageStore,
};

/// A layer of parsing in a tree
pub struct Layer {
    tree: tree_sitter::Tree,
    language: String,
    included_ranges: Vec<tree_sitter::Range>,
    children: Vec<Layer>,
}

/// The maximum depth of injected layers allowed
const MAX_INJECTION_DEPTH: usize = 8;

impl Layer {
    fn parse(
        store: &mut LanguageStore,
        lang: &str,
        included_ranges: Vec<tree_sitter::Range>,
        source: &impl Source,
        depth: usize,
    ) -> Result<Layer, Error> {
        let parser = store
            .get_parser(lang)
            .ok_or_else(|| Error::NoLanguage(lang.to_string()))?;

        if !included_ranges.is_empty() {
            parser.set_included_ranges(&included_ranges)?;
        }

        let tree = parser
            .parse_with_options(&mut parse_callback(source), None, None)
            .ok_or(Error::ParseFailed)?;

        let mut layer = Layer {
            tree,
            language: lang.to_string(),
            included_ranges,
            children: Vec::new(),
        };

        if depth < MAX_INJECTION_DEPTH {
            layer.resync_children(store, source, depth)?;
        }

        Ok(layer)
    }

    /// Applies edits to the tree, then reparses
    fn reparse(
        &mut self,
        store: &mut LanguageStore,
        edits: &[Edit],
        source: &impl Source,
        is_root: bool,
        depth: usize,
    ) -> Result<(), Error> {
        for edit in edits {
            self.tree.edit(&edit.as_input_edit());
        }

        let touched = is_root
            || edits.iter().any(|edit| {
                self.included_ranges
                    .iter()
                    .any(|r| edit.touches(&(r.start_byte..r.end_byte)))
            });

        if touched {
            let parser = store
                .get_parser(&self.language)
                .ok_or_else(|| Error::NoLanguage(self.language.clone()))?;

            if !self.included_ranges.is_empty() {
                parser.set_included_ranges(&self.included_ranges)?;
            }

            self.tree = parser
                .parse_with_options(&mut parse_callback(source), Some(&self.tree), None)
                .ok_or(Error::ParseFailed)?;

            if depth < MAX_INJECTION_DEPTH {
                self.resync_children(store, source, depth)?;
            } else {
                self.children.clear();
            }
        } else {
            for child in &mut self.children {
                child.reparse(store, edits, source, false, depth + 1)?;
            }
        }

        Ok(())
    }

    /// Syncs the child layers with the current injections
    fn resync_children(
        &mut self,
        store: &mut LanguageStore,
        source: &impl Source,
        depth: usize,
    ) -> Result<(), Error> {
        let candidates = injection::find_candidates(store, &self.language, &self.tree, source)?;

        let mut next = Vec::with_capacity(candidates.len());

        for Candidate { language, ranges } in candidates {
            let language = match language {
                injection::LanguageRef::Named(name) => name,
                injection::LanguageRef::SelfRef => self.language.clone(),
            };

            if store.get_language(&language).is_none() {
                continue;
            }

            let ranges: Vec<tree_sitter::Range> = ranges
                .into_iter()
                .map(|r| crate::position::ts_range(source, r))
                .collect();

            if let Some(pos) = self
                .children
                .iter()
                .position(|c| c.language == language && ranges_eq(&c.included_ranges, &ranges))
            {
                next.push(self.children.remove(pos));
                continue;
            }

            next.push(Layer::parse(store, &language, ranges, source, depth + 1)?);
        }

        self.children = next;

        Ok(())
    }

    /// Returns the root node of the tree
    pub fn root_node(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }

    /// Returns the language used to parse this layer
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Returns the tree-sitter tree for this layer
    pub fn tree(&self) -> &tree_sitter::Tree {
        &self.tree
    }

    /// Returns this layer's child layers
    pub fn children(&self) -> &[Layer] {
        &self.children
    }

    /// Walks this layer and its children, depth first
    pub fn walk(&self) -> impl Iterator<Item = &Layer> {
        LayerWalk { stack: vec![self] }
    }
}

struct LayerWalk<'a> {
    stack: Vec<&'a Layer>,
}

impl<'a> Iterator for LayerWalk<'a> {
    type Item = &'a Layer;

    fn next(&mut self) -> Option<&'a Layer> {
        let layer = self.stack.pop()?;
        self.stack.extend(layer.children.iter());
        Some(layer)
    }
}

fn ranges_eq(a: &[tree_sitter::Range], b: &[tree_sitter::Range]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.start_byte == b.start_byte && a.end_byte == b.end_byte)
}

/// A full SyntaxTree of a source
pub struct SyntaxTree {
    root: Layer,
}

impl SyntaxTree {
    /// Parses `source` as `lang`
    pub fn parse(
        store: &mut LanguageStore,
        lang: &str,
        source: &impl Source,
    ) -> Result<SyntaxTree, Error> {
        let root = Layer::parse(store, lang, Vec::new(), source, 0)?;
        Ok(SyntaxTree { root })
    }

    /// Updates the tree to match the source and its edits
    pub fn reparse(
        &mut self,
        store: &mut LanguageStore,
        edits: &[Edit],
        source: &impl Source,
    ) -> Result<(), Error> {
        self.root.reparse(store, edits, source, true, 0)
    }

    pub fn root(&self) -> &Layer {
        &self.root
    }

    /// Walks every layer, depth first
    pub fn layers(&self) -> impl Iterator<Item = &Layer> {
        self.root.walk()
    }
}
