use kui::{CaretStyle, Node, text};
use kwark_render::Element;

use kwark_core::{Editor, Has};

use super::Plugin;

/// The command line text, or the latest log message
pub struct StatusLine;

impl<S: Has<Plugin> + 'static, C> Element<Editor<S>, C> for StatusLine {
    fn view(&mut self, state: &mut Editor<S>, _ctx: &C) -> Option<Node<Editor<S>>> {
        let caret = state
            .core
            .theme
            .get("statusline.cursor")
            .fg
            .map(|x| CaretStyle::new(kui::CaretShape::Underline(1.0), x));

        let (core, command_line) = state.split::<Plugin>();
        let theme = &core.theme;

        let (style, line, cursor) = if core.input.mode() == "command" {
            core.log.clear();
            (
                theme.get("statusline.command"),
                format!(":{}", command_line.text()),
                // The prompt takes the first column
                Some(command_line.cursor + 1),
            )
        } else if let Some((message, severity)) = core.log.get().cloned() {
            (
                theme.get(&format!("statusline.log.{}", severity)),
                message,
                None,
            )
        } else {
            (theme.get("statusline.empty"), String::new(), None)
        };

        let mut node = style.apply(text(line).truncate());

        if let (Some(cursor), Some(caret)) = (cursor, caret) {
            node = node.caret(cursor, caret);
        }

        Some(node)
    }
}
