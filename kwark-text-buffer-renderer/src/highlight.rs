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
    pub exclusive: bool,
}

impl Highlight {
    /// Creates a new Highlight with no tag
    pub fn plain(spans: Vec<StyleSpan>) -> Self {
        Self {
            spans,
            tag: None,
            exclusive: false,
        }
    }

    /// Creates a new Highlight given a tag
    pub fn tagged(spans: Vec<StyleSpan>, tag: impl Any) -> Self {
        Self {
            spans,
            tag: Some(Box::new(tag)),
            exclusive: false,
        }
    }

    /// Creates a new Highlight no later highlighter joins onto
    pub fn exclusive(spans: Vec<StyleSpan>, tag: impl Any) -> Self {
        Self {
            spans,
            tag: Some(Box::new(tag)),
            exclusive: true,
        }
    }

    /// Folds another Highlight into this one, keeping the first tag set
    pub fn merge(&mut self, other: Highlight) {
        self.spans.extend(other.spans);

        if self.tag.is_none() {
            self.tag = other.tag;
        }

        self.exclusive = self.exclusive || other.exclusive;
    }

    /// Returns the style of the character at col
    pub fn style_at(&self, col: usize) -> Style {
        combine(
            self.spans
                .iter()
                .filter(|span| span.range.contains(&col))
                .map(|span| span.style),
        )
    }

    /// Returns the tag downcast to T, or None if there is no tag or it's type is wrong
    pub fn tag_as<T: 'static>(&self) -> Option<&T> {
        self.tag.as_deref()?.downcast_ref()
    }
}

/// Combines the given styles into one, later styles winning
pub fn combine(styles: impl IntoIterator<Item = Style>) -> Style {
    styles.into_iter().fold(Style::default(), Style::patch)
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

    /// Resolves a line through the highlighter chain, joining every result in order
    pub fn resolve(&mut self, state: &mut S, ctx: &LineCtx) -> Highlight {
        let mut joined = Highlight::default();

        for h in &mut self.0 {
            let Some(hl) = h.highlight(state, ctx) else {
                continue;
            };

            let exclusive = hl.exclusive;
            joined.merge(hl);

            if exclusive {
                break;
            }
        }

        joined
    }
}
