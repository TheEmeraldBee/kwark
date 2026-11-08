use super::layout::{Content, Frame};
use crate::{Color, Painter, Point, Rect, Span};

impl<S> Frame<S> {
    /// Draws the whole frame, with hover looks for the nodes above `hit`
    pub fn paint(&mut self, painter: &mut Painter, state: &mut S, hit: Option<usize>) {
        if !self.nodes.is_empty() {
            self.paint_node(0, painter, state, hit, Color::WHITE);
        }
    }

    fn paint_node(
        &mut self,
        index: usize,
        p: &mut Painter,
        state: &mut S,
        hit: Option<usize>,
        color: Color,
    ) {
        let node = &self.nodes[index];
        let visual = match &node.hover {
            Some(hover) if self.is_hovered(index, hit) => hover(node.visual),
            _ => node.visual,
        };

        if visual.opacity <= 0.0 {
            return;
        }

        if visual.opacity < 1.0 {
            p.opacity(visual.opacity, |p| {
                self.paint_body(index, p, state, hit, color, visual);
            });
        } else {
            self.paint_body(index, p, state, hit, color, visual);
        }
    }

    fn paint_body(
        &mut self,
        index: usize,
        p: &mut Painter,
        state: &mut S,
        hit: Option<usize>,
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
            p.clip(rect, |p| self.paint_inside(index, p, state, hit, color));
        } else {
            self.paint_inside(index, p, state, hit, color);
        }
    }

    /// Draws a node's content and children
    fn paint_inside(
        &mut self,
        index: usize,
        p: &mut Painter,
        state: &mut S,
        hit: Option<usize>,
        color: Color,
    ) {
        let node = &mut self.nodes[index];
        let inner = node.inner;
        let origin = Point::new(inner.left(), inner.top());

        match &mut node.content {
            Content::None => {}
            Content::Text(spans) => {
                let spans: Vec<Span> = spans
                    .iter()
                    .map(|(t, s)| Span::new(t, s.clone().color(color)))
                    .collect();
                let line = p.fonts().line(&spans);
                p.text(&line, origin);
            }
            Content::Rich(spans) => {
                let spans: Vec<Span> = spans.iter().map(|(t, s)| Span::new(t, s.clone())).collect();
                let line = p.fonts().line(&spans);
                p.text(&line, origin);
            }
            Content::Canvas(paint) => paint(state, inner, p),
        }

        let children: Vec<usize> = self.children(index).collect();
        for child in children {
            self.paint_node(child, p, state, hit, color);
        }
    }
}
