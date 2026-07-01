use crate::*;

/// A general text-buffer implementation that handles
/// version, undo, redo, etc for you
#[derive(Default)]
pub struct Buffer {
    pub(crate) rope: ropey::Rope,

    pub(crate) version: u128,

    change: OpSet,

    undo_stack: Vec<OpSet>,
    redo_stack: Vec<OpSet>,
}

impl Buffer {
    fn apply_op(&mut self, op: Operation) -> Result<()> {
        let inverse = op.inverse(self)?;

        op.apply(self)?;

        self.redo_stack.clear();
        self.change.0.push(inverse);

        Ok(())
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

            op.apply(self).expect("Undo SHOULD be appliable");

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

            op.apply(self).expect("Redo SHOULD be appliable");

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
}
