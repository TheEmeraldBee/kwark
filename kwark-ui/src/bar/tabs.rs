use kui::{Node, div, text};
use kwark_render::Element;

use kwark_core::Editor;
use kwark_window::WindowCtx;

/// Lists a window's open buffers, highlighting the primary one
pub struct Tabs;

impl<S: 'static> Element<Editor<S>, WindowCtx> for Tabs {
    fn view(&mut self, state: &mut Editor<S>, ctx: &WindowCtx) -> Option<Node<Editor<S>>> {
        let theme = &state.core.theme;

        let border = match ctx.active {
            true => theme.get("bar.window.selected"),
            false => theme.get("bar.window"),
        };
        let primary = theme.get("bar.tab.primary");
        let tab = theme.get("bar.tab");

        let hover = theme.get("bar.tab.hovered");

        let mut row = div()
            .flex_row()
            .gap(12.0)
            .top(5.0)
            .child(border.apply(div().h(3.0)));

        for t in ctx.tabs.iter() {
            let style = if t.primary { primary } else { tab };
            let id = ctx.id.clone();
            let tab_id = t.id.clone();

            row = row.child(
                style.apply(
                    div()
                        .px(4.0)
                        .py(1.0)
                        .rounded(3.0)
                        .child(text(t.id.to_string()))
                        .hover(move |n| hover.apply(n))
                        .on_click(move |s: &mut Editor<S>, _| {
                            s.core.windows.focus(id);

                            s.core.windows.active_mut().select(tab_id.clone());
                        }),
                ),
            );
        }

        Some(row)
    }
}
