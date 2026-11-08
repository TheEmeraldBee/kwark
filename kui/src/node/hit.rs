use super::CursorIcon;
use super::layout::{Frame, Segment};
use crate::Point;

impl<S> Frame<S> {
    /// Returns the topmost node under the point
    pub fn hit(&self, point: Point) -> Option<usize> {
        (0..self.nodes.len()).rev().find(|&i| {
            let node = &self.nodes[i];
            node.rect.intersect(node.clip).contains(point)
        })
    }

    /// Returns the node and its ancestors, nearest first
    pub fn ancestors(&self, index: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(Some(index), |&i| self.nodes[i].parent)
    }

    /// Returns the nearest node at or above `hit` that takes clicks
    pub fn clickable(&self, hit: usize) -> Option<usize> {
        self.ancestors(hit)
            .find(|&i| self.nodes[i].on_click.is_some())
    }

    /// Returns the pointer icon that the hovered nodes set, if any
    pub fn cursor(&self, hit: Option<usize>) -> Option<CursorIcon> {
        hit.and_then(|h| self.ancestors(h).find_map(|i| self.nodes[i].cursor))
    }

    /// Returns whether hover changes the look of any node above `hit`
    pub fn hover_styled(&self, hit: Option<usize>) -> bool {
        hit.is_some_and(|h| self.ancestors(h).any(|i| self.nodes[i].hover))
    }

    /// Returns the steps from the root to a node, stable across frames
    pub fn path(&self, index: usize) -> Vec<Segment> {
        let mut path: Vec<Segment> = self
            .ancestors(index)
            .map(|i| self.nodes[i].segment)
            .collect();
        path.reverse();
        path
    }
}

#[cfg(test)]
mod test {
    use taffy::TaffyTree;

    use crate::node::{Frame, div, text};
    use crate::{Fonts, Point, Size};

    struct State {
        clicks: u32,
    }

    fn frame() -> Frame<State> {
        let root = div()
            .flex_col()
            .child(div().h(20.0).w(100.0))
            .child(
                div()
                    .id("tab")
                    .h(30.0)
                    .w(100.0)
                    .on_click(|s: &mut State, _| s.clicks += 1)
                    .child(text("label")),
            )
            .child(
                div()
                    .h(10.0)
                    .w(100.0)
                    .overflow_hidden()
                    .child(div().h(50.0).w(100.0)),
            );

        let mut fonts = Fonts::new();
        let mut taffy = TaffyTree::new();
        Frame::build(root, None, Size::new(200.0, 200.0), &mut fonts, &mut taffy).unwrap()
    }

    #[test]
    fn hit_finds_the_deepest_node_and_its_clickable_ancestor() {
        let frame = frame();
        let label = frame.hit(Point::new(5.0, 25.0)).unwrap();
        let tab = frame.clickable(label).unwrap();

        assert!(matches!(
            frame.nodes[label].content,
            super::super::layout::Content::Text(_)
        ));
        assert_eq!(frame.path(tab).len(), 2);
    }

    #[test]
    fn hover_replaces_the_nodes_on_the_hovered_path() {
        let tree = || {
            div::<State>()
                .flex_col()
                .child(div().h(20.0).w(100.0).hover(|n| n.h(40.0)))
                .child(
                    div()
                        .id("tab")
                        .h(30.0)
                        .w(100.0)
                        .hover(|n| n.h(60.0).child(text("close")))
                        .child(text("label")),
                )
        };

        let mut taffy = TaffyTree::new();
        let mut fonts = Fonts::new();
        let size = Size::new(200.0, 200.0);

        let plain = Frame::build(tree(), None, size, &mut fonts, &mut taffy).unwrap();
        let label = plain.hit(Point::new(5.0, 25.0)).unwrap();
        let path = plain.path(label);

        let hovered = Frame::build(tree(), Some(&path), size, &mut fonts, &mut taffy).unwrap();

        assert_eq!(plain.nodes.len(), 4);
        assert_eq!(hovered.nodes.len(), 5);
        assert_eq!(hovered.nodes[1].rect.size.height, 20.0);
        assert_eq!(hovered.nodes[2].rect.size.height, 60.0);
        assert!(hovered.hover_styled(Some(label)));
    }

    #[test]
    fn absolute_nodes_fill_their_parent() {
        let root = div::<State>()
            .flex_col()
            .child(div().h(100.0))
            .child(div().w(50.0).h(40.0).child(div().absolute().inset(0.0)));

        let mut taffy = TaffyTree::new();
        let frame = Frame::build(
            root,
            None,
            Size::new(200.0, 200.0),
            &mut Fonts::new(),
            &mut taffy,
        )
        .unwrap();

        assert_eq!(
            frame.nodes[3].rect,
            crate::Rect::new(0.0, 100.0, 50.0, 40.0)
        );
    }

    #[test]
    fn clipped_children_do_not_take_hits() {
        let frame = frame();

        // Inside the clipped child's rect but below its clipping parent
        let hit = frame.hit(Point::new(5.0, 70.0));
        assert_eq!(hit, Some(0));
    }
}
