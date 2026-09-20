use std::any::Any;
use std::ops::Range;

use ratatui::style::Style;

use crate::LineCtx;

/// A styled area within a line
pub struct StyleSpan {
    pub range: Range<usize>,
    pub style: Style,
}

/// Data from a highlighter on how to render a line
#[derive(Default)]
pub struct Highlight {
    pub spans: Vec<StyleSpan>,
    pub tag: Option<Box<dyn Any>>,
}

impl Highlight {
    /// Creates a new Highlight with no tag
    pub fn plain(spans: Vec<StyleSpan>) -> Self {
        Self { spans, tag: None }
    }

    /// Creates a new Highlight given a tag
    pub fn tagged(spans: Vec<StyleSpan>, tag: impl Any) -> Self {
        Self {
            spans,
            tag: Some(Box::new(tag)),
        }
    }

    /// Returns the tag downcast to T, or None if there is no tag or it's type is wrong
    pub fn tag_as<T: 'static>(&self) -> Option<&T> {
        self.tag.as_deref()?.downcast_ref()
    }
}

/// A highlighter that adds style to a line of text
pub trait Highlighter<S> {
    /// Styles the given line
    ///
    /// Returns None if this line shouldn't be styled by the highlighter
    fn highlight(&mut self, state: &mut S, ctx: &LineCtx) -> Option<Highlight>;
}

/// A set of Highlighters
pub struct Highlighters<S>(Vec<Box<dyn Highlighter<S>>>);

impl<S> Default for Highlighters<S> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<S> Highlighters<S> {
    /// Appends a Highlighter to the end
    pub fn push(&mut self, h: impl Highlighter<S> + 'static) -> &mut Self {
        self.0.push(Box::new(h));
        self
    }

    /// Resolves a line through the hightlighter chain, taking the first in the chain that succeeds
    pub fn resolve(&mut self, state: &mut S, ctx: &LineCtx) -> Highlight {
        self.0
            .iter_mut()
            .find_map(|h| h.highlight(state, ctx))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Fixed(Option<Highlight>);

    impl<S> Highlighter<S> for Fixed {
        fn highlight(&mut self, _state: &mut S, _ctx: &LineCtx) -> Option<Highlight> {
            self.0.take()
        }
    }

    fn ctx(rope: &Rope) -> LineCtx<'_> {
        LineCtx {
            rope,
            line: 0,
            text: rope.line(0),
            width: 80,
            cursors: &[],
        }
    }

    use ropey::Rope;

    #[test]
    fn first_claim_wins() {
        let rope = Rope::from_str("hello");
        let mut chain = Highlighters::default();

        chain.push(Fixed(None));
        chain.push(Fixed(Some(Highlight::tagged(vec![], "second"))));
        chain.push(Fixed(Some(Highlight::tagged(vec![], "third"))));

        let resolved = chain.resolve(&mut (), &ctx(&rope));
        assert_eq!(resolved.tag_as::<&str>(), Some(&"second"));
    }

    #[test]
    fn no_claim_falls_back_to_default() {
        let rope = Rope::from_str("hello");
        let mut chain = Highlighters::default();

        chain.push(Fixed(None));

        let resolved = chain.resolve(&mut (), &ctx(&rope));
        assert!(resolved.spans.is_empty());
        assert!(resolved.tag.is_none());
    }
}
