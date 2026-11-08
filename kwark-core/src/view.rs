use kui::{Edges, Node, Painter, Rect, Size, canvas, div};
use kwark_render::Boundaries;
use kwark_window::LeafId;

use crate::{Editor, View};

impl<S: 'static> Editor<S> {
    /// Returns the tree for one frame in a window of the given size
    pub fn view(&mut self, size: Size) -> Node<Editor<S>> {
        self.with_view(|v, e| {
            let (top, bottom) = v.editor_pipeline.view(e, &());

            let View {
                windows,
                window_pipeline,
                ..
            } = v;

            let grid = windows.view(|leaf, window, active, boundaries| {
                let ctx = window.ctx(active, boundaries);
                let insets = match &mut window_pipeline.border {
                    Some(border) => border.insets(e, &ctx),
                    None => Edges::ZERO,
                };
                let top = window_pipeline.top.view(e, &ctx);
                let bottom = window_pipeline.bottom.view(e, &ctx);

                let frame = canvas(move |e: &mut Editor<S>, rect, p| {
                    e.paint_frame(leaf, active, boundaries, rect, p)
                });
                let content =
                    canvas(move |e: &mut Editor<S>, rect, p| e.paint_window(leaf, rect, p));

                div()
                    .flex_col()
                    .min_w(0.0)
                    .min_h(0.0)
                    .pt(insets.top)
                    .pr(insets.right)
                    .pb(insets.bottom)
                    .pl(insets.left)
                    .child(frame.absolute().inset(0.0))
                    .children(top)
                    .child(content.flex_1().min_h(0.0))
                    .children(bottom)
            });

            let overlays = v.overlays.view(e, Rect::from_size(size));

            div()
                .flex_col()
                .size_full()
                .children(top)
                .child(div().flex_col().flex_1().min_h(0.0).child(grid.flex_1()))
                .children(bottom)
                .children(overlays)
        })
    }

    /// Records a window's rect and draws its border
    fn paint_frame(
        &mut self,
        leaf: LeafId,
        active: bool,
        boundaries: Boundaries,
        rect: Rect,
        p: &mut Painter,
    ) {
        self.with_view(|v, e| {
            let Some(window) = v.windows.window_mut(leaf) else {
                return;
            };

            window.set_rect(rect);

            if let Some(border) = &mut v.window_pipeline.border {
                let ctx = window.ctx(active, boundaries);
                border.render(e, &ctx, rect, p);
            }
        });
    }

    /// Draws a window's selected buffer
    fn paint_window(&mut self, leaf: LeafId, rect: Rect, p: &mut Painter) {
        self.with_view(|v, e| {
            if let Some(window) = v.windows.window_mut(leaf) {
                window.paint(&v.buffers, &mut v.pipeline, e, rect, p);
            }
        });
    }
}
