pub use kwark_tree_sitter::position::Edit;

use crate::*;

/// A general text-buffer implementation that handles
/// version, undo, redo, etc for you
#[derive(Default)]
pub struct Buffer {
    pub(crate) rope: ropey::Rope,

    pub(crate) version: u128,

    /// Every Edit applied so far
    edit_log: Vec<Edit>,

    change: OpSet,

    undo_stack: Vec<OpSet>,
    redo_stack: Vec<OpSet>,
}

impl Buffer {
    /// Applies `op`, logging the Edit it produced
    fn log_apply(&mut self, op: &Operation) -> Result<()> {
        let edit = op.apply(self)?;
        self.edit_log.push(edit);
        Ok(())
    }

    fn apply_op(&mut self, op: Operation) -> Result<()> {
        let inverse = op.inverse(self)?;

        self.log_apply(&op)?;

        self.redo_stack.clear();
        self.change.0.push(inverse);

        Ok(())
    }

    /// Every Edit applied since `version`, oldest first
    ///
    /// # Errors
    /// Returns [`Error::OutdatedVersion`] if `version` is newer than the buffer's current version
    pub fn edits_since(&self, version: u128) -> Result<&[Edit]> {
        if version > self.version {
            return Err(Error::OutdatedVersion);
        }

        // edit_log always holds one entry per committed version
        let start = usize::try_from(version).map_err(|_| Error::OutdatedVersion)?;

        Ok(&self.edit_log[start..])
    }

    /// Inserts text at the given line and col (0-indexed char indices)
    pub fn insert(&mut self, line: usize, col: usize, text: impl Into<String>) -> Result<()> {
        self.apply_op(Operation::Insert {
            line,
            col,
            text: text.into(),
        })
    }

    /// Deletes count characters starting at the given line and col (0-indexed char indices)
    pub fn delete(&mut self, line: usize, col: usize, count: usize) -> Result<()> {
        self.apply_op(Operation::Delete {
            line,
            col,
            len: count,
        })
    }

    /// Commits the current change on the buffer to the undo stack
    ///
    /// Returns whether a change was actually committed
    pub fn commit_change(&mut self) -> bool {
        if self.change.0.is_empty() {
            return false;
        }

        let ops = OpSet(std::mem::take(&mut self.change.0));

        self.redo_stack.clear();

        self.undo_stack.push(ops);

        true
    }

    /// Commits changes and undoes
    pub fn undo(&mut self) {
        self.commit_change();

        let Some(ops) = self.undo_stack.pop() else {
            return;
        };

        let mut redo = OpSet::default();

        for op in ops.0.into_iter().rev() {
            let inverse = op.inverse(self).expect("Undo SHOULD be appliable");

            self.log_apply(&op).expect("Undo SHOULD be appliable");

            redo.0.push(inverse);
        }

        self.redo_stack.push(redo);
    }

    /// Redoes a change off of the redo stack
    pub fn redo(&mut self) {
        self.commit_change();

        let Some(ops) = self.redo_stack.pop() else {
            return;
        };

        let mut undo = OpSet::default();

        for op in ops.0.into_iter().rev() {
            let inverse = op.inverse(self).expect("Redo SHOULD be appliable");

            self.log_apply(&op).expect("Redo SHOULD be appliable");

            undo.0.push(inverse);
        }

        self.undo_stack.push(undo);
    }

    /// Creates the buffer from a reader
    pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Buffer> {
        Ok(Buffer {
            rope: ropey::Rope::from_reader(reader)?,

            ..Default::default()
        })
    }

    /// Returns a reference to the inner buffer for retrieving text
    pub fn rope(&self) -> &ropey::Rope {
        &self.rope
    }

    /// Returns the current version of the buffer
    pub fn version(&self) -> u128 {
        self.version
    }

    /// Checks that the version matches the buffer's version
    ///
    /// Returns [`Error::OutdatedVersion`] if versions aren't the same
    pub fn validate_version(&self, version: u128) -> Result<()> {
        if self.version != version {
            return Err(Error::OutdatedVersion);
        }

        Ok(())
    }

    /// Returns a list of [`Span`]s based on a start and end (line, col) pair
    pub fn viewport(&self, start: (usize, usize), end: (usize, usize)) -> Vec<Span> {
        if start.0 > end.0 || start.1 > end.1 {
            return vec![];
        }

        let mut lines = vec![];

        for line in start.0..end.0 {
            if line >= self.rope.len_lines() {
                break;
            }

            let line_slice = self.rope.line(line);

            let span_start = self.rope.line_to_char(line) + start.1.min(line_slice.len_chars());
            let span_end = self.rope.line_to_char(line) + end.1.min(line_slice.len_chars());
            let slice = self.rope.slice(span_start..span_end);

            lines.push(Span {
                start_char: span_start,
                end_char: span_end,
                text: slice.to_string(),
            });
        }

        lines
    }

    /// Clamp a character position into the buffer
    ///
    /// The end of the buffer is valid when the final line is empty
    pub fn clamp(&self, mut chr: usize) -> usize {
        let len = self.rope.len_chars();

        if self.rope.line(self.rope.len_lines() - 1).len_chars() == 0 {
            chr = chr.min(len);
        } else {
            chr = chr.min(len.saturating_sub(1));
        }

        chr
    }

    pub fn char_to_line_col(&self, mut chr: usize) -> (usize, usize) {
        chr = self.clamp(chr);

        let line = self.rope.char_to_line(chr);
        let start_pos = self.rope.line_to_char(line);

        (line, chr - start_pos)
    }

    /// Converts a line, column pair into a resulting character index
    ///
    /// Clamps line and col to be valid within the text
    pub fn line_col_to_char(&self, mut line: usize, mut col: usize) -> usize {
        if line >= self.rope.len_lines() {
            line = self.rope.len_lines() - 1;
        }

        let line_char = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();

        // A 0-length line has no characters, so rest at its start
        if line_len == 0 {
            return line_char;
        }

        if col >= line_len {
            col = line_len - 1;
        }

        line_char + col
    }
}

pub struct Span {
    pub start_char: usize,
    pub end_char: usize,

    pub text: String,
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    pub fn test_basic_operations_at_start() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "hello, world").unwrap();
        assert_eq!(buf.rope().to_string(), "hello, world".to_string());

        buf.delete(0, 0, 5).unwrap();
        assert_eq!(buf.rope().to_string(), ", world".to_string());

        assert_eq!(buf.version, 2);
    }

    #[test]
    pub fn test_basic_operations_in_text() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "hello\nworld").unwrap();
        assert_eq!(buf.rope().to_string(), "hello\nworld".to_string());

        buf.delete(1, 1, 4).unwrap();
        assert_eq!(buf.rope().to_string(), "hello\nw".to_string());

        buf.insert(1, 1, "hi").unwrap();
        assert_eq!(buf.rope().to_string(), "hello\nwhi".to_string());

        assert_eq!(buf.version, 3);
    }

    #[test]
    pub fn test_undo_redo() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "hello, world").unwrap();
        assert_eq!(buf.rope().to_string(), "hello, world".to_string());

        buf.commit_change();

        buf.undo();
        assert_eq!(buf.rope().to_string(), "".to_string());

        buf.redo();
        assert_eq!(buf.rope().to_string(), "hello, world".to_string());

        assert_eq!(buf.version, 3);
    }

    #[test]
    pub fn test_multi_byte() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "héllo").unwrap();
        assert_eq!(buf.rope().to_string(), "héllo".to_string());

        buf.undo();
        assert_eq!(buf.rope().to_string(), "".to_string());
    }

    #[test]
    pub fn test_change_clears_redo() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "hello").unwrap();
        assert_eq!(buf.rope().to_string(), "hello".to_string());

        buf.undo();
        assert_eq!(buf.rope().to_string(), "".to_string());

        buf.redo();
        assert_eq!(buf.rope().to_string(), "hello".to_string());

        buf.insert(0, 0, "hello").unwrap();
        assert_eq!(buf.rope().to_string(), "hellohello".to_string());

        buf.redo();
        assert_eq!(buf.rope().to_string(), "hellohello".to_string());
    }

    #[test]
    pub fn test_zero_length_lines() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "abc\n").unwrap();

        // The trailing line after the newline is 0-length
        assert_eq!(buf.rope().len_lines(), 2);
        assert_eq!(buf.rope().line(1).len_chars(), 0);

        // The end of the buffer rests on the empty final line
        assert_eq!(buf.line_col_to_char(1, 0), 4);
        assert_eq!(buf.line_col_to_char(1, 5), 4);
        assert_eq!(buf.char_to_line_col(4), (1, 0));
        assert_eq!(buf.clamp(10), 4);
    }

    #[test]
    pub fn test_empty_buffer() {
        let mut buf = Buffer::default();

        assert_eq!(buf.clamp(0), 0);
        assert_eq!(buf.clamp(10), 0);
        assert_eq!(buf.char_to_line_col(0), (0, 0));
        assert_eq!(buf.line_col_to_char(0, 0), 0);
        assert_eq!(buf.line_col_to_char(5, 5), 0);

        buf.insert(0, 0, "hi").unwrap();
        assert_eq!(buf.rope().to_string(), "hi");
    }

    #[test]
    pub fn test_multi_change_undo_redo() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "hello").unwrap();
        assert_eq!(buf.rope().to_string(), "hello".to_string());

        buf.insert(0, 5, ", world").unwrap();
        assert_eq!(buf.rope().to_string(), "hello, world".to_string());

        buf.undo();
        assert_eq!(buf.rope().to_string(), "");

        buf.redo();
        assert_eq!(buf.rope().to_string(), "hello, world");
    }

    #[test]
    pub fn edits_since_returns_edits_in_order() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "hello").unwrap();
        buf.insert(0, 5, ", world").unwrap();
        buf.delete(0, 0, 1).unwrap();

        let edits = buf.edits_since(0).unwrap();
        assert_eq!(edits.len(), 3);

        // Replaying the logged edits should match a fresh parse
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_json::LANGUAGE.into())
            .unwrap();

        let mut tree = parser.parse("", None).unwrap();
        for edit in edits {
            tree.edit(&edit.as_input_edit());
        }

        let final_text = buf.rope().to_string();
        let reparsed = parser.parse(&final_text, Some(&tree)).unwrap();
        let fresh = parser.parse(&final_text, None).unwrap();

        assert_eq!(reparsed.root_node().to_sexp(), fresh.root_node().to_sexp());
    }

    #[test]
    pub fn undo_is_logged_as_its_own_edit() {
        let mut buf = Buffer::default();

        buf.insert(0, 0, "hello").unwrap();
        buf.commit_change();
        assert_eq!(buf.version(), 1);

        buf.undo();
        assert_eq!(buf.version(), 2);
        assert_eq!(buf.rope().to_string(), "");

        let edits = buf.edits_since(0).unwrap();
        assert_eq!(edits.len(), 2, "the insert and its undo are both logged");
        assert_eq!(
            edits[1].new_end_byte, edits[1].start_byte,
            "undo deletes back to the start"
        );
    }

    #[test]
    pub fn edits_since_a_future_version_is_an_error() {
        let mut buf = Buffer::default();
        buf.insert(0, 0, "hi").unwrap();

        assert!(matches!(
            buf.edits_since(buf.version() + 1),
            Err(Error::OutdatedVersion)
        ));
    }
}
