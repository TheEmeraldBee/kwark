use parley::{Affinity, Cursor, Layout};

use crate::Color;

/// A shaped line of text
pub struct TextLayout {
    layout: Layout<Color>,
}

impl TextLayout {
    pub(super) fn new(layout: Layout<Color>) -> Self {
        Self { layout }
    }

    pub(crate) fn parley(&self) -> &Layout<Color> {
        &self.layout
    }

    /// Returns the width, counting trailing whitespace
    pub fn width(&self) -> f32 {
        self.layout.full_width()
    }

    pub fn height(&self) -> f32 {
        self.layout.height()
    }

    /// Returns the distance from the top to the baseline
    pub fn baseline(&self) -> f32 {
        self.layout.first_baseline().unwrap_or(0.0)
    }

    /// Returns the length of the text in bytes
    pub fn len(&self) -> usize {
        self.layout.text_len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the x offset of a byte index
    pub fn x_for_index(&self, index: usize) -> f32 {
        let cursor = Cursor::from_byte_index(&self.layout, index, Affinity::Downstream);
        cursor.geometry(&self.layout, 0.0).x0 as f32
    }

    /// Returns the byte index closest to an x offset
    pub fn index_for_x(&self, x: f32) -> usize {
        let y = self.height() / 2.0;
        Cursor::from_point(&self.layout, x, y).index()
    }
}

impl std::fmt::Debug for TextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextLayout")
            .field("len", &self.len())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}
