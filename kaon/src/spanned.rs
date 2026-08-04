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
        let len = chars.len();

        let start = self.start.min(len);
        let end = self.end.min(len).max(start);

        let line_start = chars[..start]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |i| i + 1);

        // Only ever show (and underline) the line the span starts on, even
        // if the span itself continues onto later lines
        let line_end = chars[start..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(len, |i| start + i);

        let line: String = chars[line_start..line_end].iter().collect();

        let col = start - line_start;
        let underline_end = end.min(line_end);
        let span = (underline_end - start + 1).max(1);

        let spaces = " ".repeat(col);
        let carets = "^".repeat(span);
        let truncated = if end > line_end { " ..." } else { "" };

        format!("{message}\n\n{line}\n{spaces}{carets}{truncated}")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_point_at_single_line_span() {
        let span = Spanned::new(2, 4, "msg");
        assert_eq!(span.point_at("oops", "ab cdef"), "oops\n\nab cdef\n  ^^^");
    }

    #[test]
    fn test_point_at_only_shows_the_starting_line() {
        // A span that runs past the end of its line (e.g. an unclosed
        // block comment spanning to EOF) must not pull later lines into
        // the rendered snippet or produce a caret run sized to the whole
        // multi-line span.
        let text = "1 /* never closed\nmore\nlines";
        let span = Spanned::new(2, text.len() - 1, "msg");
        assert_eq!(
            span.point_at("oops", text),
            "oops\n\n1 /* never closed\n  ^^^^^^^^^^^^^^^^ ..."
        );
    }

    #[test]
    fn test_point_at_span_confined_to_its_line_has_no_ellipsis() {
        let text = "a\nbcd\ne";
        let span = Spanned::new(2, 4, "msg");
        assert_eq!(span.point_at("oops", text), "oops\n\nbcd\n^^^");
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
