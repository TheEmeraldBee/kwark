use std::rc::Rc;

use kui::{Fonts, TextStyle};
use kwark_text::Cursor;
use kwark_tree_sitter::tree::SyntaxTree;
use ropey::{Rope, RopeSlice};

/// The font and the sizes that buffer text lays out with
#[derive(Clone)]
pub struct Metrics {
    pub fonts: Fonts,
    /// The style under every highlight
    pub text: TextStyle,
    pub line_height: f32,
    /// The advance of one character, the unit for gutters and virtual columns
    pub char_width: f32,
}

impl Metrics {
    pub fn new(fonts: Fonts, text: TextStyle) -> Self {
        let line_height = fonts.line_height(&text);
        let char_width = fonts.char_width(&text);

        Self {
            fonts,
            text,
            line_height,
            char_width,
        }
    }

    /// Returns metrics for the same style measured with other fonts
    pub fn with_fonts(&self, fonts: Fonts) -> Self {
        Self::new(fonts, self.text.clone())
    }

    /// Returns metrics for a new style with the same fonts
    pub fn with_text(&self, text: TextStyle) -> Self {
        Self::new(self.fonts.clone(), text)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(Fonts::new(), TextStyle::default())
    }
}

/// The context of any given buffer line
pub struct LineCtx<'a> {
    pub rope: &'a Rope,
    pub line: usize,
    pub tab: Rc<str>,
    pub text: RopeSlice<'a>,
    /// Width of the text area in logical pixels
    pub width: f32,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub metrics: &'a Metrics,
}
