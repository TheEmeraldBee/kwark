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
            primary: 0,
        }
    }

    use ropey::Rope;

    #[test]
    fn all_results_join() {
        let rope = Rope::from_str("hello");
        let mut chain = Highlighters::default();

        chain.push(Fixed(Some(Highlight::plain(vec![span(0..1)]))));
        chain.push(Fixed(Some(Highlight::plain(vec![span(1..2)]))));

        let resolved = chain.resolve(&mut (), &ctx(&rope));
        assert_eq!(resolved.spans.len(), 2);
        assert!(resolved.spans.iter().any(|s| s.range == (0..1usize)));
        assert!(resolved.spans.iter().any(|s| s.range == (1..2usize)));
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

    #[test]
    fn first_tag_wins() {
        let rope = Rope::from_str("hello");
        let mut chain = Highlighters::default();

        chain.push(Fixed(Some(Highlight::tagged(vec![], "first"))));
        chain.push(Fixed(Some(Highlight::tagged(vec![], "second"))));

        let resolved = chain.resolve(&mut (), &ctx(&rope));
        assert_eq!(resolved.tag_as::<&str>(), Some(&"first"));
    }

    #[test]
    fn exclusive_stops_the_chain() {
        let rope = Rope::from_str("hello");
        let mut chain = Highlighters::default();

        chain.push(Fixed(None));
        chain.push(Fixed(Some(Highlight::exclusive(vec![span(0..1)], "stop"))));
        chain.push(Fixed(Some(Highlight::plain(vec![span(1..2)]))));

        let resolved = chain.resolve(&mut (), &ctx(&rope));
        assert_eq!(resolved.tag_as::<&str>(), Some(&"stop"));
        assert!(resolved.spans.iter().all(|s| s.range == (0..1usize)));
    }

    #[test]
    fn combine_merges_styles_in_order() {
        let base = Style::default().fg(ratatui::style::Color::Red);
        let overlay = Style::default().fg(ratatui::style::Color::Blue);

        let combined = combine([base, overlay]);

        assert_eq!(combined.fg, Some(ratatui::style::Color::Blue));
    }

    #[test]
    fn style_at_patches_overlapping_spans() {
        let mut base = Style::default();
        base.fg = Some(ratatui::style::Color::Red);

        let overlay = Style::default().add_modifier(ratatui::style::Modifier::REVERSED);

        let hl = Highlight::plain(vec![
            StyleSpan {
                range: 0..5,
                style: base,
            },
            StyleSpan {
                range: 2..3,
                style: overlay,
            },
        ]);

        let at2 = hl.style_at(2);
        assert_eq!(at2.fg, Some(ratatui::style::Color::Red));
        assert!(at2.add_modifier.contains(ratatui::style::Modifier::REVERSED));

        let at4 = hl.style_at(4);
        assert_eq!(at4.fg, Some(ratatui::style::Color::Red));
        assert!(!at4.add_modifier.contains(ratatui::style::Modifier::REVERSED));
    }

    fn span(range: std::ops::Range<usize>) -> StyleSpan {
        StyleSpan {
            range,
            style: Style::default(),
        }
    }
}
