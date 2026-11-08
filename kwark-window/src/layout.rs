use std::collections::HashMap;

use kui::{Node, Point, Rect, div};
use kwark_buffer as buffer;
use kwark_bus::EventSender;
use kwark_render::{Boundaries, Pipeline};

use crate::{FocusChanged, Window};

/// The axis that a split lays its two halves along
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    /// Side by side
    Horizontal,
    /// One above the other
    Vertical,
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
    direction: Direction,

    /// The share of the space that the first child takes
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

    bus: EventSender,

    next_leaf: usize,
    next_node: usize,
}

impl WindowLayout {
    /// Creates a new window layout with a single window
    pub fn new(bus: EventSender) -> Self {
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

            bus,

            next_leaf: 1,
            next_node: 0,
        }
    }

    fn set_active(&mut self, id: LeafId) {
        if id != self.active {
            let from = std::mem::replace(&mut self.active, id);
            self.bus.emit(FocusChanged { from, to: id });
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
            self.set_active(id);
        }
    }

    /// Moves focus to the window adjacent to the active one along `direction`
    pub fn focus_direction(&mut self, direction: Direction, forward: bool) {
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
                    self.set_active(self.first_leaf(target));
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
            self.set_active(self.first_leaf(sibling));
        }
    }

    /// Splits the active window in the given direction, focusing the new half
    pub fn split(&mut self, direction: Direction) -> LeafId {
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

        self.set_active(new_id);
        new_id
    }

    /// Moves the border of the active window's nearest `direction` split by `amount`
    /// (between 0 and 1)
    pub fn resize(&mut self, direction: Direction, amount: f32) {
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

    /// Returns every window in the layout
    pub fn windows_mut(&mut self) -> impl Iterator<Item = &mut Window> {
        self.leaves.values_mut().map(|entry| &mut entry.window)
    }

    /// Returns every window in the layout with its id
    pub fn leaves_mut(&mut self) -> impl Iterator<Item = (LeafId, &mut Window)> {
        self.leaves
            .iter_mut()
            .map(|(id, entry)| (*id, &mut entry.window))
    }

    /// Updates every window in the layout based on the buffers in storage
    pub fn update_all(&mut self, bufs: &buffer::Storage) {
        for entry in self.leaves.values_mut() {
            entry.window.update(bufs);
        }
    }

    /// Returns the tree of every window, with the node for each window from `leaf`
    pub fn view<S>(
        &mut self,
        mut leaf: impl FnMut(LeafId, &mut Window, bool, Boundaries) -> Node<S>,
    ) -> Node<S> {
        self.view_child(self.root, &mut leaf, Boundaries::default())
    }

    fn view_child<S>(
        &mut self,
        child: Child,
        leaf: &mut impl FnMut(LeafId, &mut Window, bool, Boundaries) -> Node<S>,
        boundaries: Boundaries,
    ) -> Node<S> {
        match child {
            Child::Leaf(id) => {
                let active = id == self.active;
                let entry = self.leaves.get_mut(&id).unwrap();
                leaf(id, &mut entry.window, active, boundaries)
            }
            Child::Split(id) => {
                let split = &self.splits[&id];
                let (direction, ratio, children) = (split.direction, split.ratio, split.children);

                let (first, second) = match direction {
                    Direction::Horizontal => (
                        Boundaries {
                            right: true,
                            ..boundaries
                        },
                        Boundaries {
                            left: true,
                            ..boundaries
                        },
                    ),
                    Direction::Vertical => (
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

                // Each half grows by its share from nothing, so the ratio holds whatever the content
                let half = |node: Node<S>, share: f32| {
                    div()
                        .basis(0.0)
                        .grow(share)
                        .min_w(0.0)
                        .min_h(0.0)
                        .flex_col()
                        .child(node.flex_1())
                };

                let first = self.view_child(children[0], leaf, first);
                let second = self.view_child(children[1], leaf, second);
                let split = div().flex_1().min_w(0.0).min_h(0.0);
                let split = match direction {
                    Direction::Horizontal => split.flex_row(),
                    Direction::Vertical => split.flex_col(),
                };

                split
                    .child(half(first, ratio))
                    .child(half(second, 1.0 - ratio))
            }
        }
    }

    /// Finds the leaf window whose last-rendered rect contains `point`
    pub fn locate(&self, point: Point) -> Option<(LeafId, Rect)> {
        self.leaves.iter().find_map(|(&id, entry)| {
            let rect = entry.window.rect()?;
            rect.contains(point).then_some((id, rect))
        })
    }

    /// Finds the window, buffer, and text position under a screen point
    pub fn hit_test<S>(
        &self,
        storage: &buffer::Storage,
        pipeline: &mut Pipeline<S>,
        state: &mut S,
        point: Point,
    ) -> Option<(LeafId, buffer::ID, buffer::HitPosition)> {
        let (leaf, _) = self.locate(point)?;
        let window = &self.leaves[&leaf].window;
        let text_rect = window.content_rect()?;

        let id = window.selected()?;
        let pos =
            storage
                .get(&id)?
                .hit_test(pipeline, state, text_rect, window.scroll_shown(), point)?;
        Some((leaf, id, pos))
    }

    /// Top left of a line and column in a window's selected buffer
    pub fn cell_of<S>(
        &self,
        leaf: LeafId,
        storage: &buffer::Storage,
        pipeline: &mut Pipeline<S>,
        state: &mut S,
        pos: (usize, usize),
    ) -> Option<Point> {
        self.leaves
            .get(&leaf)?
            .window
            .cell_of(storage, pipeline, state, pos)
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

#[cfg(test)]
mod test {
    use super::*;

    fn focus_events(rx: &mut kwark_bus::EventReceiver) -> Vec<(LeafId, LeafId)> {
        let mut out = vec![];

        while let Some((_, event)) = rx.try_recv() {
            if let Some(e) = event.downcast_ref::<FocusChanged>() {
                out.push((e.from, e.to));
            }
        }

        out
    }

    #[test]
    pub fn focus_changes_emit_once_each() {
        let (bus, mut rx) = kwark_bus::channel();
        let mut layout = WindowLayout::new(bus);
        let first = layout.active_id();

        let second = layout.split(Direction::Horizontal);
        assert_eq!(focus_events(&mut rx), [(first, second)]);

        layout.focus(second);
        assert!(focus_events(&mut rx).is_empty());

        layout.close(second);
        assert_eq!(focus_events(&mut rx), [(second, first)]);
    }

    #[test]
    pub fn undrawn_window_has_no_cell() {
        let (bus, _rx) = kwark_bus::channel();
        let mut storage = buffer::Storage::new(bus.clone());
        let mut layout = WindowLayout::new(bus);

        let id = storage.insert(kwark_text::BufferEntry::new_scratch("a"));
        layout.active_mut().select(id);
        layout.update_all(&storage);

        let leaf = layout.active_id();
        let cell = layout.cell_of(
            leaf,
            &storage,
            &mut Pipeline::<()>::default(),
            &mut (),
            (0, 0),
        );

        assert_eq!(cell, None);
    }

    #[test]
    pub fn scroll_is_kept_per_window() {
        let (bus, _rx) = kwark_bus::channel();
        let mut storage = buffer::Storage::new(bus.clone());
        let mut layout = WindowLayout::new(bus);

        let id = storage.insert(kwark_text::BufferEntry::new_scratch("a"));
        let first = layout.active_id();
        layout.active_mut().select(id.clone());

        let second = layout.split(Direction::Horizontal);
        layout.active_mut().select(id);

        layout.window_mut(first).unwrap().scroll_by(5, 10);
        layout.window_mut(second).unwrap().scroll_by(20, 10);

        assert_eq!(layout.window_mut(first).unwrap().scroll_shown(), 5);
        assert_eq!(layout.window_mut(second).unwrap().scroll_shown(), 9);
    }

    #[test]
    pub fn shown_scroll_overrides_the_target() {
        let mut window = Window::new();
        window.select(buffer::ID::scratch("missing"));

        window.scroll_by(3, 10);
        window.view_mut().unwrap().scroll.shown = Some(1);

        assert_eq!(window.scroll_shown(), 1);
    }
}
