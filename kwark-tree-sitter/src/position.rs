use std::ops::Range;

use ropey::Rope;

/// A row-index and column-byte in a text document
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub row: usize,
    pub column: usize,
}

impl From<Point> for tree_sitter::Point {
    fn from(p: Point) -> Self {
        tree_sitter::Point {
            row: p.row,
            column: p.column,
        }
    }
}

/// A description of a text change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_point: Point,
    pub old_end_point: Point,
    pub new_end_point: Point,
}

impl Edit {
    /// Converts this edit a [`tree_sitter::InputEdit`]
    pub fn as_input_edit(&self) -> tree_sitter::InputEdit {
        tree_sitter::InputEdit {
            start_byte: self.start_byte,
            old_end_byte: self.old_end_byte,
            new_end_byte: self.new_end_byte,
            start_position: self.start_point.into(),
            old_end_position: self.old_end_point.into(),
            new_end_position: self.new_end_point.into(),
        }
    }

    /// Whether this edit touches a byte range
    pub fn touches(&self, range: &Range<usize>) -> bool {
        self.start_byte < range.end && range.start < self.old_end_byte.max(self.new_end_byte)
    }
}

/// The byte offset and Point of a char position in a Rope
pub fn char_position(rope: &Rope, char_idx: usize) -> (usize, Point) {
    let byte = rope.char_to_byte(char_idx);
    let row = rope.char_to_line(char_idx);
    let column = byte - rope.line_to_byte(row);
    (byte, Point { row, column })
}

/// Something that can hand tree-sitter the bytes of a document
pub trait Source {
    /// A byte chunk starting at `byte_offset`
    fn chunk(&self, byte_offset: usize) -> &[u8];

    /// The length of the document in bytes
    fn len_bytes(&self) -> usize;

    /// The row and column Point at a byte offset
    fn point_at(&self, byte_offset: usize) -> Point {
        let (mut row, mut col, mut pos) = (0usize, 0usize, 0usize);

        while pos < byte_offset {
            let chunk = self.chunk(pos);
            if chunk.is_empty() {
                break;
            }

            for &b in chunk {
                if pos >= byte_offset {
                    break;
                }
                if b == b'\n' {
                    row += 1;
                    col = 0;
                } else {
                    col += 1;
                }
                pos += 1;
            }
        }

        Point { row, column: col }
    }

    /// The bytes covered by a byte range
    fn slice(&self, range: Range<usize>) -> std::borrow::Cow<'_, [u8]> {
        let first = self.chunk(range.start);
        let want = range.end - range.start;

        if first.len() >= want {
            return std::borrow::Cow::Borrowed(&first[..want]);
        }

        let mut buf = Vec::with_capacity(want);
        buf.extend_from_slice(first);

        let mut offset = range.start + first.len();
        while offset < range.end {
            let chunk = self.chunk(offset);
            if chunk.is_empty() {
                break;
            }
            let take = chunk.len().min(range.end - offset);
            buf.extend_from_slice(&chunk[..take]);
            offset += take;
        }

        std::borrow::Cow::Owned(buf)
    }
}

impl Source for Rope {
    fn chunk(&self, byte_offset: usize) -> &[u8] {
        if byte_offset >= self.len_bytes() {
            return &[];
        }

        let (chunk, chunk_byte_start, _, _) = self.chunk_at_byte(byte_offset);
        chunk.as_bytes()[byte_offset - chunk_byte_start..].as_ref()
    }

    fn len_bytes(&self) -> usize {
        Rope::len_bytes(self)
    }

    fn point_at(&self, byte_offset: usize) -> Point {
        let row = self.byte_to_line(byte_offset);
        let column = byte_offset - self.line_to_byte(row);
        Point { row, column }
    }
}

impl Source for str {
    fn chunk(&self, byte_offset: usize) -> &[u8] {
        self.as_bytes().get(byte_offset..).unwrap_or(&[])
    }

    fn len_bytes(&self) -> usize {
        self.len()
    }
}

impl<S: Source + ?Sized> Source for &S {
    fn chunk(&self, byte_offset: usize) -> &[u8] {
        (**self).chunk(byte_offset)
    }

    fn len_bytes(&self) -> usize {
        (**self).len_bytes()
    }

    fn point_at(&self, byte_offset: usize) -> Point {
        (**self).point_at(byte_offset)
    }
}

/// Builds a tree-sitter Range from a plain byte range
pub fn ts_range(source: &impl Source, range: Range<usize>) -> tree_sitter::Range {
    tree_sitter::Range {
        start_byte: range.start,
        end_byte: range.end,
        start_point: source.point_at(range.start).into(),
        end_point: source.point_at(range.end).into(),
    }
}

/// Builds a valid callback for tree sitter parsing from the source
pub fn parse_callback<'s, S: Source>(
    source: &'s S,
) -> impl FnMut(usize, tree_sitter::Point) -> &'s [u8] {
    move |byte_offset, _point| source.chunk(byte_offset)
}

/// Allows a Source to be used as a [`tree_sitter::TextProvider`]
pub struct SourceTextProvider<'s, S: Source>(pub &'s S);

impl<'s, S: Source> tree_sitter::TextProvider<std::borrow::Cow<'s, [u8]>>
    for SourceTextProvider<'s, S>
{
    type I = std::iter::Once<std::borrow::Cow<'s, [u8]>>;

    fn text(&mut self, node: tree_sitter::Node) -> Self::I {
        std::iter::once(self.0.slice(node.byte_range()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn point_at_start() {
        let rope = Rope::from_str("hello\nworld");
        let (byte, point) = char_position(&rope, 0);
        assert_eq!(byte, 0);
        assert_eq!(point, Point { row: 0, column: 0 });
    }

    #[test]
    fn point_after_newline() {
        let rope = Rope::from_str("hello\nworld");
        let (byte, point) = char_position(&rope, 6);
        assert_eq!(byte, 6);
        assert_eq!(point, Point { row: 1, column: 0 });
    }

    #[test]
    fn point_multibyte() {
        let rope = Rope::from_str("héllo");
        // h=1 byte, é=2 bytes, so char index 2 ('l') sits at byte 3
        let (byte, point) = char_position(&rope, 2);
        assert_eq!(byte, 3);
        assert_eq!(point, Point { row: 0, column: 3 });
    }

    #[test]
    fn rope_and_str_chunks_agree() {
        let text = "the quick brown fox";
        let rope = Rope::from_str(text);

        assert_eq!(Source::chunk(&rope, 4), Source::chunk(text, 4));
        assert_eq!(Source::len_bytes(&rope), Source::len_bytes(text));
    }

    #[test]
    fn slice_spans_chunk_boundary() {
        // Force a rope with multiple small chunks by building it from many inserts
        let mut rope = Rope::new();
        for word in ["abc", "def", "ghi", "jkl"] {
            let at = rope.len_chars();
            rope.insert(at, word);
        }

        let got = Source::slice(&rope, 2..9);
        assert_eq!(&*got, b"cdefghi");
    }

    #[test]
    fn edit_touches_overlapping_range() {
        let edit = Edit {
            start_byte: 5,
            old_end_byte: 8,
            new_end_byte: 10,
            start_point: Point { row: 0, column: 5 },
            old_end_point: Point { row: 0, column: 8 },
            new_end_point: Point { row: 0, column: 10 },
        };

        assert!(edit.touches(&(0..6)));
        assert!(edit.touches(&(9..20)));
        assert!(!edit.touches(&(20..30)));
    }
}
