mod hit;
mod layout;
mod paint;
mod style;

use std::hash::{DefaultHasher, Hash, Hasher};

use taffy::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Overflow, Position,
};

pub(crate) use layout::{Frame, Segment};
pub use style::{Shadow, Visual};
pub use winit::window::CursorIcon;

use crate::{Color, Corners, Cx, Family, Painter, Rect, TextStyle};
use style::TextPatch;

/// A handler for a click on a node
pub(crate) type ClickHandler<S> = Box<dyn FnMut(&mut S, &mut Cx)>;

/// A closure that paints a node's content rect
pub(crate) type PaintFn<S> = Box<dyn FnMut(&mut S, Rect, &mut Painter)>;

/// A change to a node's look while the pointer is over it
pub(crate) type HoverFn = Box<dyn Fn(Visual) -> Visual>;

/// What a node holds
pub(crate) enum Kind<S> {
    Div,
    /// Plain text in the inherited style
    Text(String),
    /// Spans with their own styles
    Rich(Vec<(String, TextStyle)>),
    Canvas(PaintFn<S>),
}

/// One element of the tree that an app returns each frame
pub struct Node<S> {
    pub(crate) kind: Kind<S>,
    pub(crate) style: taffy::Style,
    pub(crate) visual: Visual,
    pub(crate) text: TextPatch,
    pub(crate) hover: Option<HoverFn>,
    pub(crate) cursor: Option<CursorIcon>,
    pub(crate) on_click: Option<ClickHandler<S>>,
    pub(crate) id: Option<u64>,
    pub(crate) children: Vec<Node<S>>,
}

/// Returns an empty box that lays out its children in a row
pub fn div<S>() -> Node<S> {
    Node::new(Kind::Div)
}

/// Returns a line of text in the inherited style
pub fn text<S>(text: impl Into<String>) -> Node<S> {
    Node::new(Kind::Text(text.into()))
}

/// Returns a line of text made of spans with their own styles
pub fn rich<S>(spans: impl IntoIterator<Item = (String, TextStyle)>) -> Node<S> {
    Node::new(Kind::Rich(spans.into_iter().collect()))
}

/// Returns a node that a closure paints
pub fn canvas<S>(paint: impl FnMut(&mut S, Rect, &mut Painter) + 'static) -> Node<S> {
    Node::new(Kind::Canvas(Box::new(paint)))
}

fn length(value: f32) -> LengthPercentage {
    LengthPercentage::length(value)
}

fn length_auto(value: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::length(value)
}

impl<S> Node<S> {
    fn new(kind: Kind<S>) -> Self {
        Self {
            kind,
            style: taffy::Style {
                display: Display::Flex,
                ..taffy::Style::default()
            },
            visual: Visual::default(),
            text: TextPatch::default(),
            hover: None,
            cursor: None,
            on_click: None,
            id: None,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: Node<S>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node<S>>) -> Self {
        self.children.extend(children);
        self
    }

    // Flex

    pub fn flex_row(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn flex_col(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Column;
        self
    }

    /// Grows and shrinks from a basis of zero
    pub fn flex_1(mut self) -> Self {
        self.style.flex_grow = 1.0;
        self.style.flex_shrink = 1.0;
        self.style.flex_basis = Dimension::length(0.0);
        self
    }

    /// Keeps the content size
    pub fn flex_none(mut self) -> Self {
        self.style.flex_grow = 0.0;
        self.style.flex_shrink = 0.0;
        self.style.flex_basis = Dimension::auto();
        self
    }

    pub fn grow(mut self, grow: f32) -> Self {
        self.style.flex_grow = grow;
        self
    }

    pub fn shrink(mut self, shrink: f32) -> Self {
        self.style.flex_shrink = shrink;
        self
    }

    pub fn basis(mut self, basis: f32) -> Self {
        self.style.flex_basis = Dimension::length(basis);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = taffy::Size {
            width: length(gap),
            height: length(gap),
        };
        self
    }

    pub fn items_start(mut self) -> Self {
        self.style.align_items = AlignItems::FLEX_START;
        self
    }

    pub fn items_center(mut self) -> Self {
        self.style.align_items = AlignItems::CENTER;
        self
    }

    pub fn items_end(mut self) -> Self {
        self.style.align_items = AlignItems::FLEX_END;
        self
    }

    pub fn justify_start(mut self) -> Self {
        self.style.justify_content = JustifyContent::FLEX_START;
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.style.justify_content = JustifyContent::CENTER;
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.style.justify_content = JustifyContent::FLEX_END;
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.style.justify_content = JustifyContent::SPACE_BETWEEN;
        self
    }

    // Size

    pub fn w(mut self, width: f32) -> Self {
        self.style.size.width = Dimension::length(width);
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.style.size.height = Dimension::length(height);
        self
    }

    pub fn size_full(self) -> Self {
        self.w_full().h_full()
    }

    pub fn w_full(mut self) -> Self {
        self.style.size.width = Dimension::percent(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.style.size.height = Dimension::percent(1.0);
        self
    }

    /// Sets the width as a fraction of the parent's width
    pub fn w_frac(mut self, fraction: f32) -> Self {
        self.style.size.width = Dimension::percent(fraction);
        self
    }

    /// Sets the height as a fraction of the parent's height
    pub fn h_frac(mut self, fraction: f32) -> Self {
        self.style.size.height = Dimension::percent(fraction);
        self
    }

    pub fn min_w(mut self, width: f32) -> Self {
        self.style.min_size.width = length_auto(width);
        self
    }

    pub fn min_h(mut self, height: f32) -> Self {
        self.style.min_size.height = length_auto(height);
        self
    }

    pub fn max_w(mut self, width: f32) -> Self {
        self.style.max_size.width = length_auto(width);
        self
    }

    pub fn max_h(mut self, height: f32) -> Self {
        self.style.max_size.height = length_auto(height);
        self
    }

    // Box

    pub fn p(self, padding: f32) -> Self {
        self.px(padding).py(padding)
    }

    pub fn px(self, padding: f32) -> Self {
        self.pl(padding).pr(padding)
    }

    pub fn py(self, padding: f32) -> Self {
        self.pt(padding).pb(padding)
    }

    pub fn pt(mut self, padding: f32) -> Self {
        self.style.padding.top = length(padding);
        self
    }

    pub fn pb(mut self, padding: f32) -> Self {
        self.style.padding.bottom = length(padding);
        self
    }

    pub fn pl(mut self, padding: f32) -> Self {
        self.style.padding.left = length(padding);
        self
    }

    pub fn pr(mut self, padding: f32) -> Self {
        self.style.padding.right = length(padding);
        self
    }

    /// Sets the margin on every side
    pub fn m(mut self, margin: f32) -> Self {
        let margin = length_auto(margin);
        self.style.margin = taffy::Rect {
            top: margin,
            right: margin,
            bottom: margin,
            left: margin,
        };
        self
    }

    /// Takes the node out of the flow, placed by `inset` within its parent
    pub fn absolute(mut self) -> Self {
        self.style.position = Position::Absolute;
        self
    }

    /// Sets the offset from every side of the parent
    pub fn inset(mut self, inset: f32) -> Self {
        let inset = length_auto(inset);
        self.style.inset = taffy::Rect {
            top: inset,
            right: inset,
            bottom: inset,
            left: inset,
        };
        self
    }

    /// Sets the offset from the top of the parent
    pub fn top(mut self, inset: f32) -> Self {
        self.style.inset.top = length_auto(inset);
        self
    }

    /// Sets the offset from the right of the parent
    pub fn right(mut self, inset: f32) -> Self {
        self.style.inset.right = length_auto(inset);
        self
    }

    /// Sets the offset from the bottom of the parent
    pub fn bottom(mut self, inset: f32) -> Self {
        self.style.inset.bottom = length_auto(inset);
        self
    }

    /// Sets the offset from the left of the parent
    pub fn left(mut self, inset: f32) -> Self {
        self.style.inset.left = length_auto(inset);
        self
    }

    /// Clips the content and children to the node
    pub fn overflow_hidden(mut self) -> Self {
        self.style.overflow = taffy::Point {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        self
    }

    // Visual

    pub fn bg(mut self, color: Color) -> Self {
        self.visual.bg = Some(color);
        self
    }

    /// Sets a border on every side
    pub fn border(mut self, width: f32, color: Color) -> Self {
        let width = length(width);
        self.style.border = taffy::Rect {
            top: width,
            right: width,
            bottom: width,
            left: width,
        };
        self.visual.border_color = color;
        self
    }

    pub fn border_t(mut self, width: f32, color: Color) -> Self {
        self.style.border.top = length(width);
        self.visual.border_color = color;
        self
    }

    pub fn border_b(mut self, width: f32, color: Color) -> Self {
        self.style.border.bottom = length(width);
        self.visual.border_color = color;
        self
    }

    pub fn border_l(mut self, width: f32, color: Color) -> Self {
        self.style.border.left = length(width);
        self.visual.border_color = color;
        self
    }

    pub fn border_r(mut self, width: f32, color: Color) -> Self {
        self.style.border.right = length(width);
        self.visual.border_color = color;
        self
    }

    pub fn rounded(mut self, radius: impl Into<Corners>) -> Self {
        self.visual.radius = radius.into();
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.visual.shadow = Some(shadow);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.visual.opacity = opacity;
        self
    }

    // Text

    pub fn color(mut self, color: Color) -> Self {
        self.visual.color = Some(color);
        self
    }

    pub fn font(mut self, family: Family) -> Self {
        self.text.family = Some(family);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.text.size = Some(size);
        self
    }

    pub fn bold(mut self) -> Self {
        self.text.bold = Some(true);
        self
    }

    pub fn italic(mut self) -> Self {
        self.text.italic = Some(true);
        self
    }

    pub fn underline(mut self) -> Self {
        self.text.underline = Some(true);
        self
    }

    pub fn strike(mut self) -> Self {
        self.text.strike = Some(true);
        self
    }

    /// Lets the text shrink below its width and clips what does not fit
    pub fn truncate(self) -> Self {
        self.overflow_hidden()
    }

    // Input

    /// Changes the look while the pointer is over the node or its children
    pub fn hover(mut self, f: impl Fn(Visual) -> Visual + 'static) -> Self {
        self.hover = Some(Box::new(f));
        self
    }

    /// Sets the pointer icon over the node
    pub fn cursor(mut self, icon: CursorIcon) -> Self {
        self.cursor = Some(icon);
        self
    }

    /// Sets the handler for a press and release on the node
    pub fn on_click(mut self, f: impl FnMut(&mut S, &mut Cx) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    /// Names the node, so that clicks find it across frames when its siblings change
    pub fn id(mut self, id: impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        self.id = Some(hasher.finish());
        self
    }
}
