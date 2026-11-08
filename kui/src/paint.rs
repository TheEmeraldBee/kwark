use vello::kurbo::{Affine, Rect as KurboRect};
use vello::peniko::Fill;
use vello::{Glyph, Scene};

use parley::PositionedLayoutItem;

use crate::{Color, Fonts, Point, Rect, TextLayout};

/// Draws shapes and text into a frame, in logical pixels
pub struct Painter<'a> {
    scene: &'a mut Scene,
    fonts: &'a mut Fonts,
    /// Logical pixels to physical pixels
    transform: Affine,
    /// Normalized font coords for the current run, kept to reuse the allocation
    coords: Vec<i16>,
}

impl<'a> Painter<'a> {
    pub(crate) fn new(scene: &'a mut Scene, fonts: &'a mut Fonts, scale: f64) -> Self {
        Self {
            scene,
            fonts,
            transform: Affine::scale(scale),
            coords: Vec::new(),
        }
    }

    pub fn fonts(&mut self) -> &mut Fonts {
        self.fonts
    }

    /// Fills a rect with a color
    pub fn fill(&mut self, rect: Rect, color: Color) {
        self.scene.fill(
            Fill::NonZero,
            self.transform,
            vello::peniko::Color::from(color),
            None,
            &kurbo_rect(rect),
        );
    }

    /// Draws a text layout with its top left at the origin
    pub fn text(&mut self, layout: &TextLayout, origin: Point) {
        let transform =
            self.transform * Affine::translate((f64::from(origin.x), f64::from(origin.y)));

        for line in layout.parley().lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyphs) = item else {
                    continue;
                };

                let run = glyphs.run();
                let style = glyphs.style();
                let synthesis = run.synthesis();
                let skew = synthesis
                    .skew()
                    .map(|angle| Affine::skew(f64::from(angle).to_radians().tan(), 0.0));

                self.coords.clear();
                self.coords
                    .extend(run.normalized_coords().iter().map(|c| c.to_bits()));

                self.scene
                    .draw_glyphs(run.font())
                    .font_size(run.font_size())
                    .transform(transform)
                    .glyph_transform(skew)
                    .normalized_coords(&self.coords)
                    .brush(vello::peniko::Color::from(style.brush))
                    .draw(
                        Fill::NonZero,
                        glyphs.positioned_glyphs().map(|g| Glyph {
                            id: g.id,
                            x: g.x,
                            y: g.y,
                        }),
                    );

                let metrics = run.font_metrics();
                let start = glyphs.offset();
                let end = start + glyphs.advance();
                let baseline = glyphs.baseline();

                let decorations = [
                    (
                        &style.underline,
                        metrics.underline_offset,
                        metrics.underline_size,
                    ),
                    (
                        &style.strikethrough,
                        metrics.strikethrough_offset,
                        metrics.strikethrough_size,
                    ),
                ];

                for (decoration, offset, size) in decorations {
                    let Some(decoration) = decoration else {
                        continue;
                    };

                    let offset = decoration.offset.unwrap_or(offset);
                    let size = decoration.size.unwrap_or(size).max(1.0);
                    let top = baseline - offset;
                    let rect = KurboRect::new(
                        f64::from(start),
                        f64::from(top),
                        f64::from(end),
                        f64::from(top + size),
                    );

                    self.scene.fill(
                        Fill::NonZero,
                        transform,
                        vello::peniko::Color::from(decoration.brush),
                        None,
                        &rect,
                    );
                }
            }
        }
    }
}

fn kurbo_rect(rect: Rect) -> KurboRect {
    KurboRect::new(
        f64::from(rect.left()),
        f64::from(rect.top()),
        f64::from(rect.right()),
        f64::from(rect.bottom()),
    )
}
