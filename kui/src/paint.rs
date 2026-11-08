use vello::kurbo::{
    Affine, Diagonal2, Line, Rect as KurboRect, RoundedRect, RoundedRectRadii, Stroke,
};
use vello::peniko::{Fill, Mix};
use vello::{FontEmbolden, Glyph, Scene};

use parley::PositionedLayoutItem;

use crate::{Color, Corners, Fonts, Point, Rect, Size, TextLayout};

/// Draws shapes and text into a frame, in logical pixels
pub struct Painter<'a> {
    scene: &'a mut Scene,
    fonts: &'a mut Fonts,
    /// Logical pixels to physical pixels
    transform: Affine,
    scale: f64,
    /// The window in logical pixels
    bounds: Rect,
    pointer: Option<Point>,
    /// Normalized font coords for the current run, kept to reuse the allocation
    coords: Vec<i16>,
}

impl<'a> Painter<'a> {
    pub(crate) fn new(
        scene: &'a mut Scene,
        fonts: &'a mut Fonts,
        scale: f64,
        size: Size,
        pointer: Option<Point>,
    ) -> Self {
        Self {
            scene,
            fonts,
            transform: Affine::scale(scale),
            scale,
            bounds: Rect::from_size(size),
            pointer,
            coords: Vec::new(),
        }
    }

    pub fn fonts(&mut self) -> &mut Fonts {
        self.fonts
    }

    /// Returns the pointer position, if the pointer is over the window
    pub fn pointer(&self) -> Option<Point> {
        self.pointer
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

    /// Fills a rect with rounded corners
    pub fn fill_rounded(&mut self, rect: Rect, radius: impl Into<Corners>, color: Color) {
        self.scene.fill(
            Fill::NonZero,
            self.transform,
            vello::peniko::Color::from(color),
            None,
            &rounded(rect, radius.into()),
        );
    }

    /// Draws a border that lies inside the rect
    pub fn stroke(&mut self, rect: Rect, radius: impl Into<Corners>, width: f32, color: Color) {
        if width <= 0.0 {
            return;
        }

        let half = width / 2.0;
        let shape = rounded(rect.inset(half), radius.into().shrink(half));

        self.scene.stroke(
            &Stroke::new(f64::from(width)),
            self.transform,
            vello::peniko::Color::from(color),
            None,
            &shape,
        );
    }

    /// Draws a soft shadow of a rounded rect
    pub fn shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        self.scene.draw_blurred_rounded_rect(
            self.transform,
            kurbo_rect(rect),
            color.into(),
            f64::from(radius),
            f64::from(blur / 2.0),
        );
    }

    /// Draws a straight line
    pub fn line(&mut self, from: Point, to: Point, width: f32, color: Color) {
        let line = Line::new(
            (f64::from(from.x), f64::from(from.y)),
            (f64::from(to.x), f64::from(to.y)),
        );

        self.scene.stroke(
            &Stroke::new(f64::from(width)),
            self.transform,
            vello::peniko::Color::from(color),
            None,
            &line,
        );
    }

    /// Runs the closure with drawing limited to the rect
    pub fn clip<R>(&mut self, rect: Rect, f: impl FnOnce(&mut Self) -> R) -> R {
        self.scene
            .push_clip_layer(Fill::NonZero, self.transform, &kurbo_rect(rect));
        let result = f(self);
        self.scene.pop_layer();

        result
    }

    /// Runs the closure with everything it draws faded to an opacity
    pub fn opacity<R>(&mut self, alpha: f32, f: impl FnOnce(&mut Self) -> R) -> R {
        self.scene.push_layer(
            Fill::NonZero,
            Mix::Normal,
            alpha,
            self.transform,
            &kurbo_rect(self.bounds),
        );
        let result = f(self);
        self.scene.pop_layer();

        result
    }

    /// Draws a text layout with its top left at the origin
    pub fn text(&mut self, layout: &TextLayout, origin: Point) {
        let transform =
            self.transform * Affine::translate((f64::from(origin.x), f64::from(origin.y)));

        let scale = self.scale;
        let rendering = self.fonts.rendering();
        let embolden = f64::from(rendering.embolden) * scale;
        let embolden = FontEmbolden::new(Diagonal2::new(embolden, embolden));

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

                // Glyphs go down in physical pixels with the baseline on a whole pixel, so hinting sees the real size
                let baseline = f64::from(origin.y + glyphs.baseline()) * scale;
                let shift = baseline.round() - baseline;
                let device = |x: f32, y: f32| {
                    let x = f64::from(origin.x + x) * scale;
                    let y = f64::from(origin.y + y) * scale + shift;
                    (x as f32, y as f32)
                };

                self.scene
                    .draw_glyphs(run.font())
                    .font_size(run.font_size() * scale as f32)
                    .glyph_transform(skew)
                    .normalized_coords(&self.coords)
                    .brush(vello::peniko::Color::from(style.brush))
                    .hint(rendering.hinting)
                    .font_embolden(embolden)
                    .draw(
                        Fill::NonZero,
                        glyphs.positioned_glyphs().map(|g| {
                            let (x, y) = device(g.x, g.y);
                            Glyph { id: g.id, x, y }
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

fn rounded(rect: Rect, radius: Corners) -> RoundedRect {
    let radii = RoundedRectRadii::new(
        f64::from(radius.top_left),
        f64::from(radius.top_right),
        f64::from(radius.bottom_right),
        f64::from(radius.bottom_left),
    );

    RoundedRect::from_rect(kurbo_rect(rect), radii)
}

fn kurbo_rect(rect: Rect) -> KurboRect {
    KurboRect::new(
        f64::from(rect.left()),
        f64::from(rect.top()),
        f64::from(rect.right()),
        f64::from(rect.bottom()),
    )
}
