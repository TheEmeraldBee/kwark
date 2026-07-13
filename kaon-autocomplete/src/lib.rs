pub mod check;
pub mod complete;
pub mod highlight;

use kaon::{engine::Engine, scope::Scope, spanned::Spanned};

pub use check::{Diagnostic, Severity};
pub use complete::{Completion, CompletionKind};
pub use highlight::HighlightKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn empty_at(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
}

pub trait Autocomplete<Cx> {
    fn check_line(&self, source: &str, scope: &Scope) -> Vec<Diagnostic>;
    fn complete_at(&self, source: &str, cursor: usize, scope: &Scope) -> Vec<Completion>;
    fn highlight(&self, source: &str) -> Vec<Spanned<HighlightKind>>;
}

impl<Cx> Autocomplete<Cx> for Engine<Cx> {
    fn check_line(&self, source: &str, scope: &Scope) -> Vec<Diagnostic> {
        check::check_line(self, source, scope)
    }

    fn complete_at(&self, source: &str, cursor: usize, scope: &Scope) -> Vec<Completion> {
        complete::complete_at(self, source, cursor, scope)
    }

    fn highlight(&self, source: &str) -> Vec<Spanned<HighlightKind>> {
        highlight::classify_tokens(source, self.ops())
    }
}

#[cfg(test)]
mod test {
    use kaon::engine::Engine;

    use super::*;

    #[test]
    fn test_autocomplete_trait_is_usable_through_engine() {
        let engine: Engine<()> = Engine::default_std();
        let scope = Scope::default();

        assert!(engine.check_line("1 + 2", &scope).is_empty());
        assert!(!engine.complete_at("tr", 2, &scope).is_empty());
        assert!(!engine.highlight("1 + 2").is_empty());
    }
}
