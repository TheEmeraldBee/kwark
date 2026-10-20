use crossterm::event::{KeyCode, KeyModifiers};
use kwark_input::Code;

use crate::{InputState, State, commands::CommandRegistry};

/// The state that handles how a command is inputted
#[derive(Default)]
pub struct CommandLineState {
    text: String,
    history: Vec<String>,
    selected: Option<usize>,
    cursor: usize,

    return_mode: String,
}

impl CommandLineState {
    /// Convert a char index into a byte index into `text`.
    fn byte_index(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    /// Change the selected history item
    pub fn change_history(&mut self, delta: isize) {
        let current = match self.selected {
            Some(i) => {
                if i + 1 == self.history.len() {
                    self.history[i] = self.text.clone();
                }
                i
            }
            None => {
                if self.history.is_empty() || delta >= 0 {
                    return;
                }
                self.history.push(self.text.clone());
                self.history.len() - 1
            }
        };

        let next = current
            .saturating_add_signed(delta)
            .min(self.history.len() - 1);

        self.selected = Some(next);
        self.text = self.history[next].clone();
        self.cursor = self.text.chars().count();
    }

    /// Move the cursor
    pub fn move_cursor(&mut self, dist: isize) {
        self.cursor = self
            .cursor
            .saturating_add_signed(dist)
            .min(self.text.chars().count());
    }

    /// Insert text at the cursor
    pub fn insert(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref();

        let at = self.byte_index(self.cursor);
        self.text.insert_str(at, text);

        self.cursor += text.chars().count();
    }

    /// Delete `count` characters before the cursor
    pub fn delete(&mut self, count: usize) {
        let count = count.min(self.cursor);
        if count == 0 {
            return;
        }

        let start = self.byte_index(self.cursor - count);
        let end = self.byte_index(self.cursor);
        self.text.replace_range(start..end, "");

        self.cursor -= count;
    }

    /// Delete `count` characters after the cursor
    pub fn delete_forward(&mut self, count: usize) {
        let total = self.text.chars().count();
        let count = count.min(total - self.cursor);
        if count == 0 {
            return;
        }

        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + count);
        self.text.replace_range(start..end, "");
    }

    /// Submit the text, clearing it and pushing it to history
    pub fn submit(&mut self) -> String {
        if self.selected.take().is_some() {
            self.history.pop();
        }

        let text = std::mem::take(&mut self.text);
        self.cursor = 0;

        if !text.is_empty() && self.history.last() != Some(&text) {
            self.history.push(text.clone());
        }

        text
    }

    /// Returns the return mode that occurs after `submit`
    pub fn return_mode(&self) -> &str {
        &self.return_mode
    }

    /// Sets the return mode that occurs after `submit`
    pub fn set_return_mode(&mut self, mode: impl Into<String>) {
        self.return_mode = mode.into();
    }
}

/// Takes and holds a `command` mode and inserts a state with preset-keybindings that you should utilize.
///
/// When switching into `command` mode, make sure to run [`CommandLineState::set_return_mode`] in order to set the return mode.
/// Or run `command_mode` directly to auto-return to the current mode
pub struct Plugin;

pub fn command_mode(state: &mut State) {
    let (input, command_line) = state.get::<(&mut InputState, &mut CommandLineState)>();
    let cur_mode = input.mode();
    command_line.set_return_mode(cur_mode);

    input.set_mode("command");
}

impl crate::Plugin for Plugin {
    fn register(&self, editor: &mut crate::prelude::Editor) -> anyhow::Result<()> {
        editor.insert(CommandLineState::default());

        let input = editor.get::<&mut InputState>();
        let command = input.tree("command");

        command.bind(&["escape"], "Exit command mode", |s| {
            let (input, command) = s.get::<(&mut InputState, &CommandLineState)>();
            input.set_mode(command.return_mode());

            Ok(())
        })?;

        command.bind(&["space"], "Insert Space", |s| {
            s.get::<&mut CommandLineState>().insert(" ");
            Ok(())
        })?;

        command.bind(&["backspace"], "Delete text", |s| {
            s.get::<&mut CommandLineState>().delete(1);
            Ok(())
        })?;

        command.bind(&["left"], "Move cursor left", |s| {
            s.get::<&mut CommandLineState>().move_cursor(-1);
            Ok(())
        })?;

        command.bind(&["right"], "Move cursor right", |s| {
            s.get::<&mut CommandLineState>().move_cursor(1);
            Ok(())
        })?;

        command.bind(&["up"], "History Older", |s| {
            s.get::<&mut CommandLineState>().change_history(1);
            Ok(())
        })?;

        command.bind(&["down"], "History Newer", |s| {
            s.get::<&mut CommandLineState>().change_history(-1);
            Ok(())
        })?;

        command.bind(&["enter"], "Submit", |s| {
            let (command_line, input) = s.get::<(&mut CommandLineState, &mut InputState)>();

            let command = command_line.submit();

            input.set_mode(command_line.return_mode());

            let mut scoped = s.scope::<(CommandRegistry,)>();
            let ((registry,), s) = scoped.split();
            registry.execute(s, &command)
        })?;

        command.set_backup(
            |s, chord| {
                let command_line = s.get::<&mut CommandLineState>();

                // Don't handle anything other than shift
                if chord.mods != KeyModifiers::SHIFT && chord.mods != KeyModifiers::empty() {
                    return Ok(());
                }

                let key_string = match chord.code {
                    Code::Key(KeyCode::Char(c)) => match chord.mods.contains(KeyModifiers::SHIFT) {
                        false => c.to_string(),
                        true => c.to_ascii_uppercase().to_string(),
                    },
                    _ => return Ok(()),
                };

                command_line.insert(key_string.as_str());

                Ok(())
            },
            "Insert Pressed Key",
        );

        Ok(())
    }
}
