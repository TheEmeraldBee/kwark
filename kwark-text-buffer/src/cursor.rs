use std::ops::{Deref, DerefMut};

use crate::Buffer;

fn shift_clamped(buffer: &Buffer, pivot: usize, pos: usize, distance: isize) -> usize {
    let shifted = (pos as isize + distance).max(pivot as isize) as usize;
    buffer.clamp(shifted)
}

/// A set of options that, when interacting with a cursor, should activate
#[derive(Default, Copy, Clone)]
pub struct CursorOptions {
    /// Whether, when the cursor moves, to extend the selection
    pub extend: bool,

    /// Whether, when the cursor's column goes past the end of line, to move it to the beginning of the next line
    pub wrap: bool,

    /// Whether or not to apply this to all cursors in the set
    pub all: bool,
}

impl CursorOptions {
    /// Creates a new set of cursor move options
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether, when the cursor moves, to extend the selection or not
    pub fn extend(mut self, extend: bool) -> Self {
        self.extend = extend;
        self
    }

    /// Returns whether the cursor will extend selections
    pub fn is_extend(&self) -> bool {
        self.extend
    }

    /// Sets whether, when the cursor's column goes past the end of line, to move it to the beginning of the next line or not
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Returns whether the cursor will wrap between lines
    pub fn is_wrap(&self) -> bool {
        self.wrap
    }

    /// Sets whether or not to apply this to all cursors in the set
    pub fn all(mut self, all: bool) -> Self {
        self.all = all;
        self
    }

    /// Returns whether the action will apply to all cursors in the set
    pub fn is_all(&self) -> bool {
        self.all
    }

    /// Joins two cursor sets together
    pub fn join(mut self, other: CursorOptions) -> Self {
        if other.all {
            self.all = true;
        }
        if other.extend {
            self.extend = true
        }
        if other.wrap {
            self.wrap = true
        }

        self
    }
}

/// A basic Cursor type that holds data needed to handle cursors
#[derive(Clone, Debug)]
pub struct Cursor {
    anchor: usize,
    caret: usize,

    desired_col: usize,
}

impl Cursor {
    pub fn start(&self) -> usize {
        self.anchor.min(self.caret)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.caret)
    }

    pub fn caret(&self) -> usize {
        self.caret
    }
}

/// A full set of multiple cursors that we can interact with using simple options
pub struct CursorSet {
    cursors: Vec<Cursor>,
    primary: usize,

    /// Cursor state as it was at the last commit boundary, before the in-progress edit
    pre_edit: (Vec<Cursor>, usize),

    undo_stack: Vec<(Vec<Cursor>, usize)>,
    redo_stack: Vec<(Vec<Cursor>, usize)>,
}

impl Default for CursorSet {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorSet {
    pub fn new() -> Self {
        let cursors = vec![Cursor {
            anchor: 0,
            caret: 0,

            desired_col: 0,
        }];

        CursorSet {
            cursors: cursors.clone(),
            primary: 0,

            pre_edit: (cursors, 0),

            undo_stack: vec![],
            redo_stack: vec![],
        }
    }

    pub fn reset_pre_edit(&mut self) {
        let cursors = self.cursors.clone();
        let primary = self.primary;

        self.pre_edit = (cursors, primary);
    }

    /// Commits the cursor state from before the just-finished edit into the undo/redo stack
    pub fn commit(&mut self) {
        self.undo_stack.push(std::mem::replace(
            &mut self.pre_edit,
            (self.cursors.clone(), self.primary),
        ));

        self.redo_stack.clear();
    }

    /// Changes the primary cursor, wrapping at each end
    pub fn change_primary(&mut self, dist: isize) {
        self.primary =
            (self.primary as isize + dist).rem_euclid(self.cursors.len() as isize) as usize;
    }

    /// Pops from the undo-stack and restores the cursors to their state before that edit
    ///
    /// Does nothing if stack is empty
    pub fn undo(&mut self) {
        let Some((cursors, primary)) = self.undo_stack.pop() else {
            return;
        };

        let cursors_redo = self.cursors.clone();
        let primary_redo = self.primary;
        self.redo_stack.push((cursors_redo, primary_redo));

        self.cursors = cursors;
        self.primary = primary;

        self.pre_edit = (self.cursors.clone(), self.primary);
    }

    pub fn redo(&mut self) {
        let Some((cursors, primary)) = self.redo_stack.pop() else {
            return;
        };

        self.undo_stack.push((self.cursors.clone(), self.primary));

        self.cursors = cursors;
        self.primary = primary;

        self.pre_edit = (self.cursors.clone(), self.primary);
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Returns the index of the primary cursor
    pub fn primary(&self) -> usize {
        self.primary
    }

    /// Applies the function depending on the cursor options
    ///
    /// If `all` is true, applies action to each cursor
    fn apply(&mut self, all: bool, buf: &Buffer, mut action: impl FnMut(&mut Self, &Buffer)) {
        if all {
            let primary = self.primary;

            for i in 0..self.cursors.len() {
                self.primary = i;

                action(self, buf);
            }

            self.primary = primary;
        } else {
            action(self, buf)
        }

        self.merge();
    }

    /// Applies the function depending on the cursor options
    ///
    /// If `all` is true, applies action to each cursor
    fn apply_mut(
        &mut self,
        all: bool,
        buf: &mut Buffer,
        mut action: impl FnMut(&mut Self, &mut Buffer),
    ) {
        if all {
            let primary = self.primary;

            for i in 0..self.cursors.len() {
                self.primary = i;

                action(self, buf);
            }

            self.primary = primary;
        } else {
            action(self, buf)
        }

        self.merge();
    }

    /// Moves the cursor by lines, then columns, if wrap is true, columns will allow you to move to other lines
    pub fn move_(&mut self, buf: &Buffer, lines: isize, columns: isize, options: &CursorOptions) {
        self.apply(options.all, buf, |set, buf| {
            let idx = set.primary;
            let cursor = set.cursors[idx].clone();

            let (line, _) = buf.char_to_line_col(cursor.caret);
            let len_lines = buf.rope().len_lines();

            let target_line = (line as isize + lines).clamp(0, len_lines as isize - 1) as usize;
            let line_chars = buf.rope().line(target_line).len_chars();
            let line_len = line_chars.max(1);

            // A 0-length line has no columns, so rest at the end of the file with col 0
            let mut desired_col = if line_chars == 0 {
                0
            } else {
                cursor.desired_col
            };
            let display_col = desired_col.min(line_len - 1);

            let caret = buf.line_col_to_char(target_line, display_col);

            let caret = if columns != 0 {
                if options.wrap {
                    let last = buf.rope().len_chars();
                    let shifted = (caret as isize + columns).clamp(0, last as isize) as usize;
                    desired_col = buf.char_to_line_col(shifted).1;
                    shifted
                } else {
                    let col =
                        (display_col as isize + columns).clamp(0, (line_len - 1) as isize) as usize;
                    desired_col = col;
                    buf.line_col_to_char(target_line, col)
                }
            } else {
                caret
            };

            let cursor = &mut set.cursors[idx];
            cursor.caret = caret;

            if !options.extend {
                cursor.anchor = caret;
            }

            cursor.desired_col = desired_col;
        })
    }

    /// Sets **only** the primary cursor's position to the given line/col, clamping line then column.
    /// Ignores the value set in [`CursorOptions::all`]
    pub fn set(&mut self, buf: &Buffer, line: usize, col: usize, options: &CursorOptions) {
        if options.all {
            self.remove_other();
        }

        let primary = &mut self.cursors[self.primary];

        primary.caret = buf.line_col_to_char(line, col);

        if options.extend {
            primary.anchor = buf.line_col_to_char(line, col);
        }

        primary.desired_col = buf.char_to_line_col(primary.caret).1;

        self.merge();
    }

    /// Deletes all cursors other than the primary
    pub fn remove_other(&mut self) {
        let primary = self.cursors.remove(self.primary);
        self.cursors.clear();
        self.cursors.push(primary);
        self.primary = 0;
    }

    /// Deletes the primary cursor
    ///
    /// Does nothing if it's the last cursor
    pub fn remove(&mut self) {
        if self.cursors.len() == 1 {
            // No-op when moving down to 0 cursors
            return;
        }

        self.cursors.remove(self.primary);
    }

    /// Inserts text into the buffer at the cursor's current position
    pub fn insert(&mut self, buffer: &mut Buffer, text: &str, options: &CursorOptions) {
        self.apply_mut(options.all, buffer, |set, buf| {
            let idx = set.primary;
            let caret = set.cursors[idx].caret;
            let (line, col) = buf.char_to_line_col(caret);

            buf.insert(line, col, text)
                .expect("insert should be appliable");

            let len = text.chars().count();

            set.move_after(buf, idx, len as isize);

            let cursor = &mut set.cursors[idx];
            cursor.caret = buf.clamp(cursor.caret + len);

            if !options.extend {
                cursor.anchor = cursor.caret;
            }

            cursor.desired_col = buf.char_to_line_col(cursor.caret).1;
        });
    }

    /// Deletes the currently selected text for the cursor
    pub fn delete(&mut self, buffer: &mut Buffer, options: &CursorOptions) {
        self.apply_mut(options.all, buffer, |set, buf| {
            let idx = set.primary;
            let cursor = &set.cursors[idx];
            let start = cursor.start();
            let end = cursor.end();

            if start == end {
                return;
            }

            let (line, col) = buf.char_to_line_col(start);
            let len = end - start;

            buf.delete(line, col, len)
                .expect("delete should be appliable");

            set.cursors[idx].caret = start;
            set.cursors[idx].anchor = start;
            set.cursors[idx].desired_col = col;

            set.move_after(buf, idx, -(len as isize));
        });
    }

    /// Swaps the head and tail (anchor and caret) of the cursor
    pub fn swap(&mut self, options: &CursorOptions) {
        if options.all {
            for cursor in &mut self.cursors {
                std::mem::swap(&mut cursor.anchor, &mut cursor.caret);
            }
        } else {
            let cursor = &mut self.cursors[self.primary];
            std::mem::swap(&mut cursor.anchor, &mut cursor.caret);
        }
    }

    /// Merges all cursors together that are overlapping
    fn merge(&mut self) {
        if self.cursors.is_empty() {
            return;
        }

        let primary_caret = self.cursors[self.primary].caret;

        let mut cursors = std::mem::take(&mut self.cursors);
        cursors.sort_by_key(Cursor::start);

        let mut merged: Vec<Cursor> = Vec::with_capacity(cursors.len());

        for cursor in cursors {
            if let Some(last) = merged.last_mut()
                && cursor.start() <= last.end()
            {
                let forward = last.caret >= last.anchor;
                let start = last.start().min(cursor.start());
                let end = last.end().max(cursor.end());

                if forward {
                    last.anchor = start;
                    last.caret = end;
                } else {
                    last.anchor = end;
                    last.caret = start;
                }

                continue;
            }

            merged.push(cursor);
        }

        self.primary = merged
            .iter()
            .position(|c| c.caret == primary_caret)
            .unwrap_or(0);

        self.cursors = merged;
    }

    /// Duplicates the primary selection, setting the primary to the new cursor
    pub fn duplicate(&mut self) {
        let cloned = self.cursors[self.primary].clone();
        self.cursors.push(cloned);
        self.primary = self.cursors.len() - 1;
    }

    /// Moves all cursors a distance (negative or positive) that exist after the cursor. This allows for things like auto-moving on insert/delete
    fn move_after(&mut self, buffer: &Buffer, cursor: usize, distance: isize) {
        let pivot = self.cursors[cursor].caret;

        for (i, c) in self.cursors.iter_mut().enumerate() {
            if i == cursor {
                continue;
            }

            if c.caret > pivot {
                c.caret = shift_clamped(buffer, pivot, c.caret, distance);
            }

            if c.anchor > pivot {
                c.anchor = shift_clamped(buffer, pivot, c.anchor, distance);
            }
        }
    }

    pub fn bind<'a>(&'a mut self, buf: &'a mut Buffer) -> BoundCursorSet<'a> {
        BoundCursorSet { set: self, buf }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn setup(text: &str) -> (Buffer, CursorSet) {
        let mut buf = Buffer::default();

        buf.insert(0, 0, text).unwrap();
        buf.commit_change();

        (buf, CursorSet::default())
    }

    #[test]
    fn move_to_bottom_rests_on_empty_line() {
        let (buf, mut set) = setup("aaaa\nbbbb\ncccc\n");
        let opts = CursorOptions::default().wrap(true);

        // Move out to col 2, then down onto the empty final line
        set.move_(&buf, 0, 2, &opts);
        set.move_(&buf, 1, 0, &opts);
        set.move_(&buf, 1, 0, &opts);
        set.move_(&buf, 1, 0, &opts);

        // The caret rests at the end of the buffer, on the empty final line
        assert_eq!(set.cursors()[0].caret(), 15);
        assert_eq!(buf.char_to_line_col(set.cursors()[0].caret()), (3, 0));

        // Moving down again stays at the end of the buffer
        set.move_(&buf, 1, 0, &opts);
        assert_eq!(set.cursors()[0].caret(), 15);

        // The desired col reset to 0, so moving back up lands at col 0
        set.move_(&buf, -1, 0, &opts);
        assert_eq!(buf.char_to_line_col(set.cursors()[0].caret()), (2, 0));
    }

    #[test]
    fn insert_at_bottom_types_onto_empty_line() {
        let (mut buf, mut set) = setup("aaaa\nbbbb\ncccc\n");
        let opts = CursorOptions::default().wrap(true);

        set.move_(&buf, 3, 0, &opts);

        assert_eq!(set.cursors()[0].caret(), 15);

        set.insert(&mut buf, "ab", &opts);

        assert_eq!(buf.rope().to_string(), "aaaa\nbbbb\ncccc\nab");
        assert_eq!(set.cursors()[0].caret(), 16);
        assert_eq!(buf.char_to_line_col(set.cursors()[0].caret()), (3, 1));
    }
}

pub struct BoundCursorSet<'a> {
    set: &'a mut CursorSet,
    buf: &'a mut Buffer,
}

impl<'a> Deref for BoundCursorSet<'a> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        self.buf
    }
}

impl<'a> DerefMut for BoundCursorSet<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buf
    }
}

impl<'a> BoundCursorSet<'a> {
    /// Moves the cursor by lines, then columns, if wrap is true, columns will allow you to move to other lines
    pub fn move_(&mut self, lines: isize, columns: isize, options: &CursorOptions) {
        self.set.move_(self.buf, lines, columns, options);
    }

    /// Sets **only** the primary cursor's position to the given line/col, clamping line then column.
    /// Ignores the value set in [`CursorOptions::all`]
    pub fn set(&mut self, line: usize, col: usize, options: &CursorOptions) {
        self.set.set(self.buf, line, col, options);
    }

    /// Changes the primary cursor, wrapping at each end
    pub fn change_primary(&mut self, dist: isize) {
        self.set.change_primary(dist);
    }

    /// Deletes all cursors other than the primary
    pub fn remove_other(&mut self) {
        self.set.remove_other();
    }

    /// Deletes the primary cursor
    ///
    /// Does nothing if it's the last cursor
    pub fn remove(&mut self) {
        self.set.remove();
    }

    /// Inserts text into the buffer at the cursor's current position
    pub fn insert(&mut self, text: &str, options: &CursorOptions) {
        self.set.insert(self.buf, text, options);
    }

    /// Deletes the currently selected text for the cursor
    pub fn delete(&mut self, options: &CursorOptions) {
        self.set.delete(self.buf, options);
    }

    /// Swaps the head and tail (anchor and caret) of the cursor
    pub fn swap(&mut self, options: &CursorOptions) {
        self.set.swap(options);
    }

    /// Duplicates the primary cursor, and sets the primary cursor to the new one
    pub fn duplicate(&mut self) {
        self.set.duplicate();
    }

    /// Commits current change on buffer, as well as storing the cursors in the undo stack
    pub fn commit_change(&mut self) {
        if self.buf.commit_change() {
            self.set.commit();
        } else {
            self.set.reset_pre_edit();
        }
    }

    /// Undoes a change on the buffer, and restores the cursors
    pub fn undo(&mut self) {
        self.buf.undo();
        self.set.undo();

        // Force-update the cursors **just** in case
        self.move_(0, 0, &CursorOptions::default());
    }

    /// Redoes a change on the buffer, and restores the cursors
    pub fn redo(&mut self) {
        self.buf.redo();
        self.set.redo();

        // Force-update the cursors **just** in case
        self.move_(0, 0, &CursorOptions::default());
    }
}
