use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

/// An item that spans some portion of the input text
pub struct Spanned<T> {
    pub start: usize,
    pub end: usize,
    pub value: T,
}

impl<T: Clone> Clone for Spanned<T> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
            value: self.value.clone(),
        }
    }
}

impl<T> Spanned<T> {
    /// Creates a new element that spans some text
    pub fn new(start: usize, end: usize, value: T) -> Self {
        Self { start, end, value }
    }

    pub fn into_inner(self) -> T {
        self.value
    }

    /// Renders the given message followed by the source line and a caret span
    pub fn point_at(&self, message: impl Display, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();

        let line_start = chars[..self.start]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |i| i + 1);

        let line_end = chars[self.end..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(chars.len(), |i| self.end + i);

        let line: String = chars[line_start..line_end].iter().collect();

        let col = self.start - line_start;
        let span = self.end - self.start + 1;

        let spaces = " ".repeat(col);
        let carets = "^".repeat(span);

        format!("{message}\n\n{line}\n{spaces}{carets}")
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Debug> Debug for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}: {:?}", self.start, self.end, self.value)
    }
}

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        if !self.start.eq(&other.start) || !self.end.eq(&other.end) {
            return false;
        }

        self.value.eq(&other.value)
    }
}
