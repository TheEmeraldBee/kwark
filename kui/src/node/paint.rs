use super::Caret;
use super::layout::{Content, Frame};
use crate::{Color, Painter, Point, Rect, Span};

impl<S> Frame<S> {
    /// Draws the whole frame
    pub fn paint(&mut self, painter: &mut Painter, state: &mut S) {
        if !self.nodes.is_empty() {
            self.paint_node(0, painter, state, Color::WHITE);
        }
    }

    fn paint_node(&mut self, index: usize, p: &mut Painter, state: &mut S, color: Color) {
        let visual = self.nodes[index].visual;

        if visual.opacity <= 0.0 {
            return;
        }

        if visual.opacity < 1.0 {
            p.opacity(visual.opacity, |p| {
                self.paint_body(index, p, state, color, visual);
            });
        } else {
            self.paint_body(index, p, state, color, visual);
        }
    }

    fn paint_body(
        &mut self,
        index: usize,
        p: &mut Painter,
        state: &mut S,
        color: Color,
        visual: super::Visual,
    ) {
        let node = &self.nodes[index];
        let rect = node.rect;
        let color = visual.color.unwrap_or(color);

        if let Some(shadow) = visual.shadow {
            let at = Rect::new(
                rect.left() + shadow.offset.x,
                rect.top() + shadow.offset.y,
                rect.size.width,
                rect.size.height,
            );
            p.shadow(at, visual.radius.top_left, shadow.blur, shadow.color);
        }

        if let Some(bg) = visual.bg {
            p.fill_rounded(rect, visual.radius, bg);
        }

        let border = node.border;
        if border.is_uniform() {
            p.stroke(rect, visual.radius, border.top, visual.border_color);
        } else {
            let c = visual.border_color;
            let (w, h) = (rect.size.width, rect.size.height);
            p.fill(Rect::new(rect.left(), rect.top(), w, border.top), c);
            p.fill(
                Rect::new(rect.left(), rect.bottom() - border.bottom, w, border.bottom),
                c,
            );
            p.fill(Rect::new(rect.left(), rect.top(), border.left, h), c);
            p.fill(
                Rect::new(rect.right() - border.right, rect.top(), border.right, h),
                c,
            );
        }

        if node.clips {
            p.clip(rect, |p| self.paint_inside(index, p, state, color));
        } else {
            self.paint_inside(index, p, state, color);
        }
    }

    /// Draws a node's content and children
    fn paint_inside(&mut self, index: usize, p: &mut Painter, state: &mut S, color: Color) {
        let node = &mut self.nodes[index];
        let inner = node.inner;
        let caret = node.caret;

        match &mut node.content {
            Content::None => {}
            Content::Text(spans) => {
                let spans: Vec<Span> = spans
                    .iter()
                    .map(|(t, s)| Span::new(t, s.clone().color(color)))
                    .collect();
                paint_line(&spans, caret, inner, p);
            }
            Content::Rich(spans) => {
                let spans: Vec<Span> = spans.iter().map(|(t, s)| Span::new(t, s.clone())).collect();
                paint_line(&spans, caret, inner, p);
            }
            Content::Canvas(paint) => paint(state, inner, p),
        }

        let children: Vec<usize> = self.children(index).collect();
        for child in children {
            self.paint_node(child, p, state, color);
        }
    }
}

/// Draws a line of spans with its caret
fn paint_line(spans: &[Span], caret: Option<Caret>, inner: Rect, p: &mut Painter) {
    let line = p.fonts().line(spans);

    let Some(caret) = caret else {
        p.text(&line, Point::new(inner.left(), inner.top()));
        return;
    };

    let end_style = spans.last().map(|s| s.style.clone()).unwrap_or_default();
    let end_width = p.fonts().char_width(&end_style);
    let cell = caret.cell(spans, &line, end_width);
    let shift = (cell.right() - inner.size.width).max(0.0);

    let origin = Point::new(inner.left() - shift, inner.top());
    p.text(&line, origin);

    let at = Rect::new(
        origin.x + cell.left(),
        origin.y,
        cell.size.width,
        cell.size.height,
    );
    caret.paint(spans, at, p);
}

#[cfg(test)]
mod test {
    use crate::{
        CaretShape, CaretStyle, Color, Fonts, Node, Painter, Size, div, node::Frame, text,
    };

    fn draws(node: Node<()>) -> usize {
        let mut fonts = Fonts::new();
        let mut taffy = taffy::TaffyTree::new();
        let size = Size::new(400.0, 40.0);
        let root = div().size_full().child(node);
        let mut frame = Frame::build(root, None, size, &mut fonts, &mut taffy).unwrap();

        let mut scene = vello::Scene::new();
        let mut p = Painter::new(&mut scene, &mut fonts, 1.0, size, None);
        frame.paint(&mut p, &mut ());
        scene.encoding().draw_tags.len()
    }

    fn width(node: Node<()>) -> f32 {
        let mut fonts = Fonts::new();
        let mut taffy = taffy::TaffyTree::new();
        let root = div().size_full().child(node);
        let size = Size::new(400.0, 40.0);
        let frame = Frame::build(root, None, size, &mut fonts, &mut taffy).unwrap();
        frame.nodes[1].rect.size.width
    }

    #[test]
    fn text_with_a_caret_leaves_room_for_the_end_cell() {
        let style = CaretStyle::new(CaretShape::Bar(2.0), Color::WHITE);
        let char_width = Fonts::new().char_width(&Default::default());

        let plain = width(text(":abc").truncate());
        let with_caret = width(text(":abc").truncate().caret(4, style));

        // Layout rounds to whole pixels
        assert!((with_caret - plain - char_width).abs() <= 1.0);
    }

    #[test]
    fn text_carets_draw_within_and_past_the_text() {
        let style = CaretStyle::new(CaretShape::Underline(2.0), Color::WHITE);
        let plain = draws(text(":abc").truncate());

        assert_eq!(draws(text(":abc").truncate().caret(1, style)), plain + 1);
        assert_eq!(draws(text(":abc").truncate().caret(4, style)), plain + 1);
    }
}
