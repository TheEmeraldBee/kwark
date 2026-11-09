use crate::{Color, Painter, Point, Rect, Span, TextLayout, TextStyle};

/// How a caret looks, numbers in the structs are pixel widths of the lines
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaretShape {
    Block,
    Bar(f32),
    Underline(f32),
    Hollow(f32),
}

/// The style of a caret
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretStyle {
    pub shape: CaretShape,
    pub color: Color,
    /// The color of the text under a block
    pub text: Color,
}

impl CaretStyle {
    pub fn new(shape: CaretShape, color: Color) -> Self {
        Self {
            shape,
            color,
            text: color,
        }
    }

    pub fn text(mut self, color: Color) -> Self {
        self.text = color;
        self
    }

    /// Draws the caret over a cell
    pub fn paint(&self, cell: Rect, under: Option<(char, &TextStyle)>, p: &mut Painter) {
        match self.shape {
            CaretShape::Block => {
                p.fill(cell, self.color);

                // The block covers the character, so it draws again on top in the caret's text color
                let Some((c, style)) = under.filter(|(c, _)| !c.is_whitespace()) else {
                    return;
                };

                let mut buf = [0; 4];
                let span = Span::new(c.encode_utf8(&mut buf), style.clone().color(self.text));
                let layout = p.fonts().line(&[span]);
                p.text(&layout, Point::new(cell.left(), cell.top()));
            }
            CaretShape::Bar(width) => {
                let bar = Rect::new(cell.left(), cell.top(), width, cell.size.height);
                p.fill(bar, self.color);
            }
            CaretShape::Underline(height) => {
                let top = cell.bottom() - height;
                p.fill(
                    Rect::new(cell.left(), top, cell.size.width, height),
                    self.color,
                );
            }
            CaretShape::Hollow(width) => p.stroke(cell, 0.0, width, self.color),
        }
    }
}

/// A caret placed at a char index of a text node
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Caret {
    pub index: usize,
    pub style: CaretStyle,
}

impl Caret {
    /// Returns the cell the caret covers, relative to the start of the line, `end_width` wide past the end
    pub fn cell(&self, spans: &[Span], line: &TextLayout, end_width: f32) -> Rect {
        let text: String = spans.iter().map(|s| s.text).collect();
        let mut chars = text.char_indices().skip(self.index).map(|(i, _)| i);
        let start = chars.next().unwrap_or(text.len());
        let end = chars.next().unwrap_or(text.len());

        let x = line.x_for_index(start);
        let width = match start < end {
            true => line.x_for_index(end) - x,
            false => end_width,
        };

        Rect::new(x, 0.0, width, line.height())
    }

    pub fn paint(&self, spans: &[Span], cell: Rect, p: &mut Painter) {
        self.style.paint(cell, char_at(spans, self.index), p);
    }
}

/// Returns the character at a char index across spans, with its style
fn char_at<'a>(spans: &'a [Span], index: usize) -> Option<(char, &'a TextStyle)> {
    let mut rest = index;
    for span in spans {
        let count = span.text.chars().count();
        if rest < count {
            return span.text.chars().nth(rest).map(|c| (c, &span.style));
        }
        rest -= count;
    }
    None
}
