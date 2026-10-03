use tree_sitter::Query;

/// Access to a pattern's `#set!` directives
pub struct Directives<'a> {
    settings: &'a [tree_sitter::QueryProperty],
}

impl<'a> Directives<'a> {
    /// Builds the directive table for the given pattern
    pub fn for_pattern(query: &'a Query, pattern_index: usize) -> Self {
        Self {
            settings: query.property_settings(pattern_index),
        }
    }

    /// The value of a `#set! key "value"` directive
    pub fn set(&self, key: &str) -> Option<&'a str> {
        self.settings
            .iter()
            .find(|p| &*p.key == key)
            .and_then(|p| p.value.as_deref())
    }

    /// Whether a flag `#set! key` directive is present
    pub fn is(&self, key: &str) -> bool {
        self.settings.iter().any(|p| &*p.key == key)
    }
}
