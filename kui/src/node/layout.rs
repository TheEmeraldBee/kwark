use taffy::{AvailableSpace, Dimension, NodeId, Overflow, TaffyTree, compute_leaf_layout};

use super::{Caret, ClickHandler, CursorIcon, Kind, Node, Visual};
use crate::{Edges, Error, Fonts, Point, Rect, Size, Span, TextStyle};

/// A clip that lets everything through
const EVERYTHING: Rect = Rect::new(-1.0e9, -1.0e9, 2.0e9, 2.0e9);

/// What a laid out node draws
pub(crate) enum Content<S> {
    None,
    Text(Vec<(String, TextStyle)>),
    /// Spans that keep their own colors
    Rich(Vec<(String, TextStyle)>),
    Canvas(super::PaintFn<S>),
}

/// One node of a frame, in tree order
pub(crate) struct Flat<S> {
    pub parent: Option<usize>,
    /// One past the last node in this subtree
    pub end: usize,
    /// The id or child index that names this node within its parent
    pub segment: Segment,
    pub content: Content<S>,
    pub caret: Option<Caret>,
    pub visual: Visual,
    /// Whether the node changes while the pointer is over it
    pub hover: bool,
    pub cursor: Option<CursorIcon>,
    pub on_click: Option<ClickHandler<S>>,
    pub clips: bool,
    /// The border box
    pub rect: Rect,
    /// The box inside the border and padding
    pub inner: Rect,
    pub border: Edges,
    /// The part of the window that ancestors let this node draw in
    pub clip: Rect,
    style: Option<taffy::Style>,
}

/// One step of a path from the root to a node
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Segment {
    Index(usize),
    Id(u64),
}

/// A laid out tree, kept between frames for paint and input
pub(crate) struct Frame<S> {
    pub nodes: Vec<Flat<S>>,
}

impl<S> Frame<S> {
    /// Lays out a tree to fill a window of the given size, with hover applied along `hovered`
    pub fn build(
        root: Node<S>,
        hovered: Option<&[Segment]>,
        size: Size,
        fonts: &mut Fonts,
        taffy: &mut TaffyTree<usize>,
    ) -> Result<Self, Error> {
        let mut nodes = Vec::new();
        flatten(
            root,
            None,
            Segment::Index(0),
            hovered,
            &TextStyle::default(),
            &mut nodes,
        );

        if let Some(style) = nodes[0].style.as_mut() {
            style.size = taffy::Size {
                width: Dimension::length(size.width),
                height: Dimension::length(size.height),
            };
        }

        taffy.clear();
        let mut ids = Vec::with_capacity(nodes.len());
        let root = build_taffy(&mut nodes, 0, taffy, &mut ids)?;

        let available = taffy::Size {
            width: AvailableSpace::Definite(size.width),
            height: AvailableSpace::Definite(size.height),
        };

        taffy.compute_layout_with_measure(root, available, |inputs, _, index, style| {
            let node = index.map(|&mut i| &nodes[i]);
            let content = node.map(|n| &n.content);
            let caret = node.is_some_and(|n| n.caret.is_some());

            compute_leaf_layout(
                inputs,
                style,
                |_, _| 0.0,
                |known, _| {
                    let measured = match content {
                        Some(Content::Text(spans) | Content::Rich(spans)) => {
                            let spans: Vec<Span> =
                                spans.iter().map(|(t, s)| Span::new(t, s.clone())).collect();
                            let line = fonts.line(&spans);

                            // A caret past the last character needs a cell of its own
                            let end = match (caret, spans.last()) {
                                (true, Some(last)) => fonts.char_width(&last.style),
                                (true, None) => fonts.char_width(&TextStyle::default()),
                                (false, _) => 0.0,
                            };

                            taffy::Size {
                                width: line.width() + end,
                                height: line.height(),
                            }
                        }
                        _ => taffy::Size::ZERO,
                    };

                    taffy::Size {
                        width: known.width.unwrap_or(measured.width),
                        height: known.height.unwrap_or(measured.height),
                    }
                },
            )
        })?;

        for i in 0..nodes.len() {
            let layout = taffy.layout(ids[i])?;
            let (origin, clip) = match nodes[i].parent {
                Some(p) => {
                    let parent = &nodes[p];
                    let clip = if parent.clips {
                        parent.clip.intersect(parent.rect)
                    } else {
                        parent.clip
                    };
                    (parent.rect.origin, clip)
                }
                None => (Point::default(), EVERYTHING),
            };

            let node = &mut nodes[i];
            node.rect = Rect::new(
                origin.x + layout.location.x,
                origin.y + layout.location.y,
                layout.size.width,
                layout.size.height,
            );
            node.border = Edges {
                top: layout.border.top,
                right: layout.border.right,
                bottom: layout.border.bottom,
                left: layout.border.left,
            };

            let left = layout.border.left + layout.padding.left;
            let top = layout.border.top + layout.padding.top;
            let right = layout.border.right + layout.padding.right;
            let bottom = layout.border.bottom + layout.padding.bottom;

            node.inner = Rect::new(
                node.rect.left() + left,
                node.rect.top() + top,
                (node.rect.size.width - left - right).max(0.0),
                (node.rect.size.height - top - bottom).max(0.0),
            );
            node.clip = clip;
        }

        Ok(Self { nodes })
    }

    /// Returns the indices of the children of a node
    pub fn children(&self, index: usize) -> impl Iterator<Item = usize> + '_ {
        let end = self.nodes[index].end;
        let mut next = index + 1;

        std::iter::from_fn(move || {
            if next >= end {
                return None;
            }

            let child = next;
            next = self.nodes[child].end;
            Some(child)
        })
    }
}

/// Appends a node and its subtree in tree order, replacing the nodes on the hovered path
fn flatten<S>(
    mut node: Node<S>,
    parent: Option<usize>,
    segment: Segment,
    hovered: Option<&[Segment]>,
    inherited: &TextStyle,
    out: &mut Vec<Flat<S>>,
) {
    // The segment comes before hover so that the path still matches next frame
    let segment = node.id.map_or(segment, Segment::Id);
    let hovered = hovered
        .and_then(|h| h.split_first())
        .filter(|(first, _)| **first == segment)
        .map(|(_, rest)| rest);

    let hover = node.hover.is_some();
    if hovered.is_some()
        && let Some(f) = node.hover.take()
    {
        node = f(node);
    }

    let index = out.len();
    let text = node.text.apply(inherited);
    let clips = node.style.overflow.x != Overflow::Visible;

    let content = match node.kind {
        Kind::Div => Content::None,
        Kind::Text(string) => Content::Text(vec![(string, text.clone())]),
        Kind::Rich(spans) => Content::Rich(spans),
        Kind::Canvas(paint) => Content::Canvas(paint),
    };

    out.push(Flat {
        parent,
        end: index + 1,
        segment,
        content,
        caret: node.caret,
        visual: node.visual,
        hover,
        cursor: node.cursor,
        on_click: node.on_click,
        clips,
        rect: Rect::default(),
        inner: Rect::default(),
        border: Edges::ZERO,
        clip: EVERYTHING,
        style: Some(node.style),
    });

    for (i, child) in node.children.into_iter().enumerate() {
        flatten(child, Some(index), Segment::Index(i), hovered, &text, out);
    }

    out[index].end = out.len();
}

/// Adds a node and its subtree to the Taffy tree, recording each Taffy id by index
fn build_taffy<S>(
    nodes: &mut [Flat<S>],
    index: usize,
    taffy: &mut TaffyTree<usize>,
    ids: &mut Vec<NodeId>,
) -> Result<NodeId, Error> {
    ids.push(NodeId::from(0u64));

    let mut children = Vec::new();
    let mut next = index + 1;
    while next < nodes[index].end {
        children.push(build_taffy(nodes, next, taffy, ids)?);
        next = nodes[next].end;
    }

    let style = nodes[index].style.take().unwrap_or_default();
    let id = if children.is_empty() {
        taffy.new_leaf_with_context(style, index)?
    } else {
        taffy.new_with_children(style, &children)?
    };

    ids[index] = id;
    Ok(id)
}
