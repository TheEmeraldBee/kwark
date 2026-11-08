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

        let mut row = div().flex_row().child(border.apply(text(" --- ")));

        for (i, t) in ctx.tabs.iter().enumerate() {
            if i > 0 {
                row = row.child(border.apply(text(" - ")));
            }

            let style = if t.primary { primary } else { tab };
            row = row.child(style.apply(text(t.name.clone())));
        }

        Some(row.child(border.apply(text(" --- "))))
    }
}
