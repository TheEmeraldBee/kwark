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

    /// Sets whether, when the cursor's column goes past the end of line, to move it to the beginning of the next line or not
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Sets whether or not to apply this to all cursors in the set
    pub fn all(mut self, all: bool) -> Self {
        self.all = all;
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
    fn start(&self) -> usize {
        self.anchor.min(self.caret)
    }

    fn end(&self) -> usize {
        self.anchor.max(self.caret)
    }
}

/// A full set of multiple cursors that we can interact with using simple options
pub struct CursorSet {
    cursors: Vec<Cursor>,
    primary: usize,
}

impl Default for CursorSet {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorSet {
    pub fn new() -> Self {
        CursorSet {
            cursors: vec![Cursor {
                anchor: 0,
                caret: 0,

                desired_col: 0,
            }],
            primary: 0,
        }
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
            let line_len = buf.rope().line(target_line).len_chars().max(1);

            let mut desired_col = cursor.desired_col;
            let display_col = desired_col.min(line_len - 1);

            let caret = buf.line_col_to_char(target_line, display_col);

            let caret = if columns != 0 {
                if options.wrap {
                    let last = buf.rope().len_chars().saturating_sub(1);
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

        primary.desired_col = col;

        self.merge();
    }

    /// Deletes all cursors other than the primary
    pub fn remove_other(&mut self) {
        let primary = self.cursors.remove(self.primary);
        self.cursors.clear();
        self.cursors.push(primary);
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
            cursor.caret += len;

            if !options.extend {
                cursor.anchor = cursor.caret;
            }

            cursor.desired_col = col + len;
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
            if let Some(last) = merged.last_mut() {
                if cursor.start() <= last.end() {
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
            }

            merged.push(cursor);
        }

        self.primary = merged
            .iter()
            .position(|c| c.caret == primary_caret)
            .unwrap_or(0);

        self.cursors = merged;
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

pub struct BoundCursorSet<'a> {
    set: &'a mut CursorSet,
    buf: &'a mut Buffer,
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
}
