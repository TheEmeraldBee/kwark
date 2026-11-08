mod cache;
mod layout;

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

use parley::{
    FontContext, FontFamily, FontFamilyName, FontStyle, FontWeight, GenericFamily, LayoutContext,
    StyleProperty,
};

pub use layout::TextLayout;

use crate::Color;
use cache::LineCache;

/// A font family to draw text with
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Family {
    #[default]
    Monospace,
    SansSerif,
    Serif,
    /// A family by its installed name
    Named(Arc<str>),
}

impl Family {
    fn to_parley(&self) -> FontFamily<'_> {
        let name = match self {
            Self::Monospace => FontFamilyName::Generic(GenericFamily::Monospace),
            Self::SansSerif => FontFamilyName::Generic(GenericFamily::SansSerif),
            Self::Serif => FontFamilyName::Generic(GenericFamily::Serif),
            Self::Named(name) => FontFamilyName::Named(name.as_ref().into()),
        };

        FontFamily::Single(name)
    }
}

/// The look of a run of text
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub family: Family,
    /// Font size in logical pixels
    pub size: f32,
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            family: Family::default(),
            size: 14.0,
            color: Color::WHITE,
            bold: false,
            italic: false,
            underline: false,
            strike: false,
        }
    }
}

impl TextStyle {
    pub fn family(mut self, family: Family) -> Self {
        self.family = family;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn strike(mut self) -> Self {
        self.strike = true;
        self
    }
}

impl Eq for TextStyle {}

impl Hash for TextStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.family.hash(state);
        self.size.to_bits().hash(state);
        self.color.hash(state);
        self.bold.hash(state);
        self.italic.hash(state);
        self.underline.hash(state);
        self.strike.hash(state);
    }
}

/// A run of text with one style
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span<'a> {
    pub text: &'a str,
    pub style: TextStyle,
}

impl<'a> Span<'a> {
    pub fn new(text: &'a str, style: TextStyle) -> Self {
        Self { text, style }
    }
}

/// How glyphs turn into pixels
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextRendering {
    /// Whether glyph outlines snap to the pixel grid
    pub hinting: bool,
    /// Extra stroke weight in logical pixels, for fonts that render too thin
    pub embolden: f32,
}

impl Default for TextRendering {
    fn default() -> Self {
        Self {
            hinting: true,
            embolden: 0.0,
        }
    }
}

/// The font database and the cache of shaped lines, shared by every clone
#[derive(Clone, Default)]
pub struct Fonts(Rc<RefCell<FontsInner>>);

impl Fonts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the shaped line for a set of spans
    pub fn line(&self, spans: &[Span]) -> Arc<TextLayout> {
        let mut inner = self.0.borrow_mut();
        if let Some(layout) = inner.cache.get(spans) {
            return layout;
        }

        let layout = Arc::new(inner.shape(spans));
        inner.cache.insert(spans, layout.clone());
        layout
    }

    /// Returns the advance of one character in a monospace style
    pub fn char_width(&self, style: &TextStyle) -> f32 {
        self.line(&[Span::new("0", style.clone())]).width()
    }

    /// Returns the height of one line in a style
    pub fn line_height(&self, style: &TextStyle) -> f32 {
        self.line(&[Span::new(" ", style.clone())]).height()
    }

    /// Returns how glyphs turn into pixels
    pub fn rendering(&self) -> TextRendering {
        self.0.borrow().rendering
    }

    pub fn set_rendering(&self, rendering: TextRendering) {
        self.0.borrow_mut().rendering = rendering;
    }

    pub fn set_hinting(&self, hinting: bool) {
        self.0.borrow_mut().rendering.hinting = hinting;
    }

    /// Sets the extra stroke weight in logical pixels, where 0.2 to 0.4 suits thin fonts
    pub fn set_embolden(&self, amount: f32) {
        self.0.borrow_mut().rendering.embolden = amount.max(0.0);
    }

    /// Drops the lines that recent frames did not use
    pub(crate) fn end_frame(&self) {
        self.0.borrow_mut().cache.end_frame();
    }
}

/// The parts of `Fonts` behind its shared handle
struct FontsInner {
    fonts: FontContext,
    layouts: LayoutContext<Color>,
    cache: LineCache,
    rendering: TextRendering,
    /// Joined span text, kept to reuse its allocation
    text: String,
}

impl Default for FontsInner {
    fn default() -> Self {
        Self {
            fonts: FontContext::new(),
            layouts: LayoutContext::new(),
            cache: LineCache::default(),
            rendering: TextRendering::default(),
            text: String::new(),
        }
    }
}

impl FontsInner {
    fn shape(&mut self, spans: &[Span]) -> TextLayout {
        self.text.clear();
        for span in spans {
            self.text.push_str(span.text);
        }

        let mut builder = self
            .layouts
            .ranged_builder(&mut self.fonts, &self.text, 1.0, true);

        let mut start = 0;
        for span in spans {
            let range = start..start + span.text.len();
            start = range.end;

            let style = &span.style;
            let weight = if style.bold {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            };
            let slant = if style.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            };

            builder.push(
                StyleProperty::FontFamily(style.family.to_parley()),
                range.clone(),
            );
            builder.push(StyleProperty::FontSize(style.size), range.clone());
            builder.push(StyleProperty::Brush(style.color), range.clone());
            builder.push(StyleProperty::FontWeight(weight), range.clone());
            builder.push(StyleProperty::FontStyle(slant), range.clone());
            builder.push(StyleProperty::Underline(style.underline), range.clone());
            builder.push(StyleProperty::Strikethrough(style.strike), range);
        }

        let mut layout = builder.build(&self.text);
        layout.break_all_lines(None);

        TextLayout::new(layout)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hit_tests_map_columns_both_ways() {
        let fonts = Fonts::new();
        let style = TextStyle::default();
        let width = fonts.char_width(&style);
        let line = fonts.line(&[
            Span::new("let", style.clone().bold()),
            Span::new(" x = 1;", style),
        ]);

        assert!(width > 0.0);
        assert_eq!(line.len(), 10);
        assert_eq!(line.x_for_index(0), 0.0);

        for column in 0..=line.len() {
            let x = line.x_for_index(column);
            assert_eq!(line.index_for_x(x + width * 0.25), column);
        }

        assert_eq!(line.index_for_x(-50.0), 0);
        assert_eq!(line.index_for_x(line.width() + 50.0), line.len());
    }

    #[test]
    fn same_spans_share_one_layout() {
        let fonts = Fonts::new();
        let spans = [Span::new("fn main() {}", TextStyle::default())];

        let a = fonts.line(&spans);
        let b = fonts.line(&spans);

        assert!(Arc::ptr_eq(&a, &b));
    }
}
