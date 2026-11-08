use kui::{Color, Node, Rect, Shadow, div, text};
use kwark_core::{Editor, INPUT_TREE_FLAG};
use kwark_render::Overlay;

/// Fallback colors for a theme without a popup style
const FALLBACK_BG: Color = Color::rgb(0x181825);
const FALLBACK_BORDER: Color = Color::rgb(0x45475A);

/// A popup that lists the keys available at the current input step
pub struct InputTreePopup;

impl<S: 'static> Overlay<Editor<S>> for InputTreePopup {
    fn view(&mut self, state: &mut Editor<S>, _area: Rect) -> Option<Node<Editor<S>>> {
        let (input, flags, theme) = (&state.core.input, &state.core.flags, &state.core.theme);
        if !flags.get(INPUT_TREE_FLAG) {
            return None;
        }

        let lines: Vec<Node<Editor<S>>> = input
            .get_layer()
            .into_iter()
            .map(|(chord, desc)| {
                let chord = chord.map_or("any".to_string(), |x| x.to_string());
                text(format!("{chord} : {desc}"))
            })
            .collect();

        let style = theme.get("popup");
        let border = theme.get("popup.border").fg.unwrap_or(FALLBACK_BORDER);

        let popup = div()
            .absolute()
            .right(32.0)
            .bottom(32.0)
            .max_h(320.0)
            .flex_col()
            .p(8.0)
            .rounded(6.0)
            .bg(style.bg.unwrap_or(FALLBACK_BG))
            .border(1.0, border)
            .overflow_hidden()
            .children(lines);

        Some(style.apply(popup))
    }
}
