use kui::Node;

/// A node that the editor's tree sits inside
pub trait Wrapper<S> {
    /// Returns the editor's tree placed inside the wrapper
    fn wrap(&mut self, state: &mut S, editor: Node<S>) -> Node<S>;
}

impl<S, F: FnMut(&mut S, Node<S>) -> Node<S>> Wrapper<S> for F {
    fn wrap(&mut self, state: &mut S, editor: Node<S>) -> Node<S> {
        self(state, editor)
    }
}
