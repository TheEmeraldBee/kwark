use kui::{Node, text};
use kwark_render::Element;

use kwark_core::{Editor, Has};

use super::Plugin;

/// The command line text, or the latest log message
pub struct StatusLine;

impl<S: Has<Plugin> + 'static, C> Element<Editor<S>, C> for StatusLine {
    fn view(&mut self, state: &mut Editor<S>, _ctx: &C) -> Option<Node<Editor<S>>> {
        let (core, command_line) = state.split::<Plugin>();
        let theme = &core.theme;

        let (style, line) = if core.input.mode() == "command" {
            core.log.clear();
            (
                theme.get("statusline.command"),
                format!(":{}", command_line.text()),
            )
        } else if let Some((message, severity)) = core.log.get().cloned() {
            (theme.get(&format!("statusline.log.{}", severity)), message)
        } else {
            (theme.get("statusline.empty"), String::new())
        };

        Some(style.apply(text(line).truncate()))
    }
}
