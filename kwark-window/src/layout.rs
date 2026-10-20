use std::collections::HashMap;

use kwark_buffer as buffer;
use kwark_core::{Pipeline, State};
use kwark_render::Boundaries;

use ratatui::layout::{self, Position, Rect};
use ratatui::prelude::*;

use crate::{Window, WindowPipeline};

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

impl Split {
    /// Divides `rect` into this split's two children's rects
    fn rects(&self, rect: Rect) -> [Rect; 2] {
        let percentage = (self.ratio * 100.0).round() as u16;
        let rects = Layout::default()
            .direction(self.direction)
            .constraints([
                layout::Constraint::Percentage(percentage),
                layout::Constraint::Percentage(100 - percentage),
            ])
            .split(rect);

        [rects[0], rects[1]]
    }
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
    /// Creates a new window layout with a single window
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

    /// Returns the window with the given id, mutably
    ///
    /// Returns `None` if the id doesn't exist
    pub fn window_mut(&mut self, id: LeafId) -> Option<&mut Window> {
        self.leaves.get_mut(&id).map(|entry| &mut entry.window)
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
        &mut self,
        storage: &buffer::Storage,
        frame: &mut Frame<'_>,
        rect: Rect,

        pipeline: &mut Pipeline,
        window_pipeline: &mut WindowPipeline,
        state: &mut State,
    ) {
        self.render_child(
            self.root,
            storage,
            frame,
            rect,
            pipeline,
            window_pipeline,
            state,
            Boundaries::default(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_child(
        &mut self,
        child: Child,
        storage: &buffer::Storage,
        frame: &mut Frame<'_>,
        rect: Rect,

        pipeline: &mut Pipeline,
        window_pipeline: &mut WindowPipeline,
        state: &mut State,

        boundaries: Boundaries,
    ) {
        match child {
            Child::Leaf(id) => {
                let is_active = id == self.active;
                let entry = self.leaves.get_mut(&id).unwrap();
                entry.window.render(
                    storage,
                    frame,
                    rect,
                    pipeline,
                    window_pipeline,
                    state,
                    is_active,
                    boundaries,
                );
            }
            Child::Split(id) => {
                let split = &self.splits[&id];
                let rects = split.rects(rect);
                let children = split.children;

                let (first, second) = match split.direction {
                    layout::Direction::Horizontal => (
                        Boundaries {
                            right: true,
                            ..boundaries
                        },
                        Boundaries {
                            left: true,
                            ..boundaries
                        },
                    ),
                    layout::Direction::Vertical => (
                        Boundaries {
                            bottom: true,
                            ..boundaries
                        },
                        Boundaries {
                            top: true,
                            ..boundaries
                        },
                    ),
                };

                self.render_child(
                    children[0],
                    storage,
                    frame,
                    rects[0],
                    pipeline,
                    window_pipeline,
                    state,
                    first,
                );
                self.render_child(
                    children[1],
                    storage,
                    frame,
                    rects[1],
                    pipeline,
                    window_pipeline,
                    state,
                    second,
                );
            }
        }
    }

    /// Finds the leaf window whose last-rendered rect contains `point`
    pub fn locate(&self, point: Position) -> Option<(LeafId, Rect)> {
        self.leaves.iter().find_map(|(&id, entry)| {
            let rect = entry.window.rect()?;
            rect.contains(point).then_some((id, rect))
        })
    }

    /// Finds the window, buffer, and text position under a screen point
    pub fn hit_test(
        &self,
        storage: &buffer::Storage,
        pipeline: &mut Pipeline,
        state: &mut State,
        point: Position,
    ) -> Option<(LeafId, buffer::ID, buffer::HitPosition)> {
        let (leaf, _) = self.locate(point)?;
        let window = &self.leaves[&leaf].window;
        let text_rect = window.content_rect()?;

        let id = window.selected()?;
        let scroll = window.state().map(|v| v.scroll).unwrap_or(0);

        let pos = storage
            .get(&id)?
            .hit_test(pipeline, state, text_rect, scroll, point)?;
        Some((leaf, id, pos))
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
