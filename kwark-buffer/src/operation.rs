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
    /// Applies the given operation to the buffer, returning the edit it performed
    pub(crate) fn apply(&self, buf: &mut Buffer) -> Result<()> {
        match self {
            Self::Insert { line, col, text } => {
                let idx = buf.rope.try_line_to_char(*line)? + col;

                // can't panic, line index checked before
                let line_len = buf.rope.line(*line).len_chars();

                if line_len <= *col && idx != buf.rope.len_chars() {
                    return Err(ropey::Error::CharIndexOutOfBounds(*col, line_len).into());
                }

                buf.rope.try_insert(idx, text)?;
            }
            Self::Delete { line, col, len } => {
                let idx = buf.rope.try_line_to_char(*line)? + col;

                // can't panic, line index checked before
                let line_len = buf.rope.line(*line).len_chars();

                if line_len <= *col && idx != buf.rope.len_chars() {
                    return Err(ropey::Error::CharIndexOutOfBounds(*col, line_len).into());
                }

                buf.rope.try_remove(idx..(idx + len))?;
            }
        }
        buf.version += 1;
        Ok(())
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
