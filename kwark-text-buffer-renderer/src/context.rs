use kwark_text_buffer::Cursor;
use kwark_tree_sitter::tree::SyntaxTree;
use ropey::{Rope, RopeSlice};

/// The context of any given buffer line
pub struct LineCtx<'a> {
    pub rope: &'a Rope,
    pub line: usize,
    pub text: RopeSlice<'a>,
    pub width: u16,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
}
