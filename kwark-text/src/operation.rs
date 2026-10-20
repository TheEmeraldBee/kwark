use kwark_tree_sitter::position::{Edit, char_position};

use crate::*;

/// A reversable operation that can be applied to a [`Buffer`]
pub(crate) enum Operation {
    /// Inserts the given text at the given position
    Insert {
        line: usize,
        col: usize,
        text: String,
    },
    /// Deletes the given count of characters at the given position
    Delete { line: usize, col: usize, len: usize },
}

impl Operation {
    /// Applies the given operation to the buffer, returning the [`Edit`] it performed
    pub(crate) fn apply(&self, buf: &mut Buffer) -> Result<Edit> {
        let edit = match self {
            Self::Insert { line, col, text } => {
                let idx = buf.rope.try_line_to_char(*line)? + col;

                // can't panic, line index checked before
                let line_len = buf.rope.line(*line).len_chars();

                if line_len <= *col && idx != buf.rope.len_chars() {
                    return Err(ropey::Error::CharIndexOutOfBounds(*col, line_len).into());
                }

                let (start_byte, start_point) = char_position(&buf.rope, idx);

                buf.rope.try_insert(idx, text)?;

                let (new_end_byte, new_end_point) =
                    char_position(&buf.rope, idx + text.chars().count());

                Edit {
                    start_byte,
                    old_end_byte: start_byte,
                    new_end_byte,
                    start_point,
                    old_end_point: start_point,
                    new_end_point,
                }
            }
            Self::Delete { line, col, len } => {
                let idx = buf.rope.try_line_to_char(*line)? + col;

                // can't panic, line index checked before
                let line_len = buf.rope.line(*line).len_chars();

                if line_len <= *col && idx != buf.rope.len_chars() {
                    return Err(ropey::Error::CharIndexOutOfBounds(*col, line_len).into());
                }

                let (start_byte, start_point) = char_position(&buf.rope, idx);
                let (old_end_byte, old_end_point) = char_position(&buf.rope, idx + len);

                buf.rope.try_remove(idx..(idx + len))?;

                Edit {
                    start_byte,
                    old_end_byte,
                    new_end_byte: start_byte,
                    start_point,
                    old_end_point,
                    new_end_point: start_point,
                }
            }
        };

        buf.version += 1;
        Ok(edit)
    }

    /// Given the buffer, reverses an operation
    pub fn inverse(&self, buf: &Buffer) -> Result<Self> {
        match self {
            Self::Insert { line, col, text } => Ok(Self::Delete {
                line: *line,
                col: *col,
                len: text.chars().count(),
            }),

            Self::Delete { line, col, len } => {
                let idx = buf.rope.try_line_to_char(*line)? + *col;

                let text = buf
                    .rope
                    .get_slice(idx..(idx + *len))
                    .ok_or(ropey::Error::CharIndexOutOfBounds(
                        idx,
                        buf.rope.len_chars(),
                    ))?
                    .to_string();

                Ok(Self::Insert {
                    line: *line,
                    col: *col,
                    text,
                })
            }
        }
    }
}

/// A set of changes that can be applied to a buffer to invert a change
#[derive(Default)]
pub(crate) struct OpSet(pub Vec<Operation>);
