use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;
use kwark_buffer as buffer;
use ratatui::layout::{self, Rect};

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// A single pane in the editor
#[derive(Default)]
pub struct Window {
    bufs: buffer::List,
}

impl Deref for Window {
    type Target = buffer::List;

    fn deref(&self) -> &Self::Target {
        &self.bufs
    }
}

impl DerefMut for Window {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bufs
    }
}

impl Window {
    fn new() -> Self {
        Self::default()
    }

    /// Renders the window's active buffer to the given rect
    pub fn render(
        &self,
        storage: &buffer::Storage,
        frame: &mut Frame<'_>,
        rect: Rect,

        pipeline: &mut Pipeline,
        state: &mut State,

        is_active: bool,
    ) {
        let border_style = match is_active {
            true => Style::new().on_green(),
            false => Style::new(),
        };

        let edge_span = Span::styled(" - ", border_style);
        let mut spans = vec![edge_span.clone()];

        for (primary, buf) in self.bufs.iter() {
            if primary {
                spans.push(Span::raw(buf.to_string()).magenta());
            } else {
                spans.push(Span::raw(buf.to_string()));
            }

            spans.push(Span::styled(" - ", border_style));
        }

        if spans.len() > 1 {
            spans.pop();
        }

        spans.push(edge_span);

        let block_text = Paragraph::new(Line::from_iter(spans)).centered();

        let mut header_rect = rect;
        header_rect.height = 1;
        frame.render_widget(block_text, header_rect);

        let mut text_rect = rect;
        text_rect.height -= 1;
        text_rect.y += 1;

        let Some(buf) = self.bufs.selected().and_then(|id| storage.get(&id)) else {
            return;
        };

        buf.render(pipeline, state, frame, text_rect);
    }
}

/// Identifies a window within a [`WindowLayout`]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LeafId(usize);

/// Identifies a split within a [`WindowLayout`]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct NodeId(usize);

/// Marks possible optioons for a split to go to.
#[derive(Clone, Copy, PartialEq)]
enum Child {
    Leaf(LeafId),
    Split(NodeId),
}

/// A window plus the split that contains it
struct LeafEntry {
    window: Window,
    parent: Option<NodeId>,
}

/// A two-way split of the layout
struct Split {
    direction: layout::Direction,

    ratio: f32,

    children: [Child; 2],
    parent: Option<NodeId>,
}

/// The layout of the editor's windows
pub struct WindowLayout {
    leaves: HashMap<LeafId, LeafEntry>,
    splits: HashMap<NodeId, Split>,

    root: Child,
    active: LeafId,

    next_leaf: usize,
    next_node: usize,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowLayout {
    /// Creates a new window layout with a single window taking the whole layout
    pub fn new() -> Self {
        let root = LeafId(0);

        let mut leaves = HashMap::new();
        leaves.insert(
            root,
            LeafEntry {
                window: Window::new(),
                parent: None,
            },
        );

        Self {
            leaves,
            splits: HashMap::new(),

            root: Child::Leaf(root),
            active: root,

            next_leaf: 1,
            next_node: 0,
        }
    }

    /// Returns the id of the active window
    pub fn active_id(&self) -> LeafId {
        self.active
    }

    /// Returns the active window
    pub fn active(&self) -> &Window {
        &self.leaves[&self.active].window
    }

    /// Returns the active window, mutably
    pub fn active_mut(&mut self) -> &mut Window {
        &mut self.leaves.get_mut(&self.active).unwrap().window
    }

    /// Focuses the given window, if it exists
    pub fn focus(&mut self, id: LeafId) {
        if self.leaves.contains_key(&id) {
            self.active = id;
        }
    }

    /// Moves focus to the window adjacent to the active one along `direction`
    pub fn focus_direction(&mut self, direction: layout::Direction, forward: bool) {
        let mut child = Child::Leaf(self.active);
        let mut parent = self.leaves[&self.active].parent;

        while let Some(parent_id) = parent {
            let split = &self.splits[&parent_id];

            if split.direction == direction {
                let is_first = split.children[0] == child;

                if is_first == forward {
                    let target = if forward {
                        split.children[1]
                    } else {
                        split.children[0]
                    };
                    self.active = self.first_leaf(target);
                    return;
                }
            }

            child = Child::Split(parent_id);
            parent = split.parent;
        }
    }

    /// Closes the given window, unless it is the only window left
    pub fn close(&mut self, id: LeafId) {
        let Some(entry) = self.leaves.get(&id) else {
            return;
        };

        let Some(parent_id) = entry.parent else {
            return;
        };

        let parent = self
            .splits
            .remove(&parent_id)
            .expect("leaf's parent must exist");
        let sibling = match parent.children {
            [Child::Leaf(l), other] if l == id => other,
            [other, Child::Leaf(l)] if l == id => other,
            _ => unreachable!("a leaf's parent always has that leaf as a direct child"),
        };

        self.leaves.remove(&id);
        self.reparent(sibling, parent.parent);

        match parent.parent {
            Some(grandparent_id) => {
                let grandparent = self
                    .splits
                    .get_mut(&grandparent_id)
                    .expect("split's parent must exist");

                let slot = grandparent
                    .children
                    .iter_mut()
                    .find(|c| matches!(c, Child::Split(n) if *n == parent_id))
                    .expect("split must be a direct child of its recorded parent");

                *slot = sibling;
            }
            None => self.root = sibling,
        }

        if self.active == id {
            self.active = self.first_leaf(sibling);
        }
    }

    /// Splits the active window in the given direction, focusing the new half
    pub fn split(&mut self, direction: layout::Direction) -> LeafId {
        let old_id = self.active;
        let old_parent = self.leaves[&old_id].parent;

        let new_id = LeafId(self.next_leaf);
        self.next_leaf += 1;

        let node_id = NodeId(self.next_node);
        self.next_node += 1;

        self.leaves.insert(
            new_id,
            LeafEntry {
                window: Window::new(),
                parent: Some(node_id),
            },
        );
        self.leaves.get_mut(&old_id).unwrap().parent = Some(node_id);

        self.splits.insert(
            node_id,
            Split {
                direction,
                ratio: 0.5,
                children: [Child::Leaf(old_id), Child::Leaf(new_id)],
                parent: old_parent,
            },
        );

        match old_parent {
            Some(parent_id) => {
                let parent = self.splits.get_mut(&parent_id).unwrap();
                let slot = parent
                    .children
                    .iter_mut()
                    .find(|c| matches!(c, Child::Leaf(l) if *l == old_id))
                    .expect("old window must be a direct child of its recorded parent");

                *slot = Child::Split(node_id);
            }
            None => self.root = Child::Split(node_id),
        }

        self.active = new_id;
        new_id
    }

    /// Moves the border of the active window's nearest `direction` split by `amount`
    /// (between 0 and 1)
    pub fn resize(&mut self, direction: layout::Direction, amount: f32) {
        let mut parent = self.leaves[&self.active].parent;

        while let Some(parent_id) = parent {
            let split = self.splits.get_mut(&parent_id).unwrap();

            if split.direction == direction {
                split.ratio = (split.ratio + amount).clamp(0.1, 0.9);
                return;
            }

            parent = split.parent;
        }
    }

    /// Updates every window in the layout based on the buffers in storage
    pub fn update_all(&mut self, bufs: &buffer::Storage) {
        for entry in self.leaves.values_mut() {
            entry.window.update(bufs);
        }
    }

    /// Renders the layout to the frame in the given rect
    pub fn render(
        &self,
        storage: &buffer::Storage,
        frame: &mut Frame<'_>,
        rect: Rect,

        pipeline: &mut Pipeline,
        state: &mut State,
    ) {
        self.render_child(self.root, storage, frame, rect, pipeline, state);
    }

    fn render_child(
        &self,
        child: Child,
        storage: &buffer::Storage,
        frame: &mut Frame<'_>,
        rect: Rect,

        pipeline: &mut Pipeline,
        state: &mut State,
    ) {
        match child {
            Child::Leaf(id) => {
                let entry = &self.leaves[&id];
                entry
                    .window
                    .render(storage, frame, rect, pipeline, state, id == self.active);
            }
            Child::Split(id) => {
                let split = &self.splits[&id];

                let percentage = (split.ratio * 100.0).round() as u16;
                let rects = Layout::default()
                    .direction(split.direction)
                    .constraints([
                        layout::Constraint::Percentage(percentage),
                        layout::Constraint::Percentage(100 - percentage),
                    ])
                    .split(rect);

                self.render_child(split.children[0], storage, frame, rects[0], pipeline, state);
                self.render_child(split.children[1], storage, frame, rects[1], pipeline, state);
            }
        }
    }

    fn reparent(&mut self, child: Child, parent: Option<NodeId>) {
        match child {
            Child::Leaf(id) => self.leaves.get_mut(&id).unwrap().parent = parent,
            Child::Split(id) => self.splits.get_mut(&id).unwrap().parent = parent,
        }
    }

    fn first_leaf(&self, child: Child) -> LeafId {
        match child {
            Child::Leaf(id) => id,
            Child::Split(id) => self.first_leaf(self.splits[&id].children[0]),
        }
    }
}
