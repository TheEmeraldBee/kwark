use kwark_core::{Editor, INPUT_TREE_FLAG};
use kwark_render::Overlay;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    macros::{horizontal, vertical},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// A popup that lists the keys available at the current input step
pub struct InputTreePopup;

impl<S: 'static> Overlay<Editor<S>> for InputTreePopup {
    fn render(&mut self, state: &mut Editor<S>, area: Rect, buf: &mut Buffer) {
        let (input, flags) = (&state.core.input, &state.core.flags);
        if !flags.get(INPUT_TREE_FLAG) {
            return;
        }

        let lines = input
            .get_layer()
            .iter()
            .map(|(chord, desc)| {
                Line::raw(format!(
                    "{} : {desc}",
                    chord.map(|x| x.to_string()).unwrap_or("any".to_string())
                ))
            })
            .collect::<Vec<_>>();

        let vertical_layer = vertical![==80%, ==20%].split(area)[1];
        let rect = horizontal![==60%, ==40%].split(vertical_layer)[1];

        Clear.render(rect, buf);
        Paragraph::new(lines)
            .block(Block::new().borders(Borders::ALL))
            .render(rect, buf);
    }
}
