use crate::{Color, Corners, Family, Point, TextStyle};

/// A soft shadow under a node
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset: Point,
    pub blur: f32,
    pub color: Color,
}

impl Shadow {
    pub const fn new(blur: f32, color: Color) -> Self {
        Self {
            offset: Point::new(0.0, 0.0),
            blur,
            color,
        }
    }

    pub const fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Point::new(x, y);
        self
    }
}

/// The look of a node that hover can change
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Visual {
    pub bg: Option<Color>,
    pub border_color: Color,
    pub radius: Corners,
    pub shadow: Option<Shadow>,
    pub opacity: f32,
    /// Text color for this node and its children
    pub color: Option<Color>,
}

impl Default for Visual {
    fn default() -> Self {
        Self {
            bg: None,
            border_color: Color::TRANSPARENT,
            radius: Corners::ZERO,
            shadow: None,
            opacity: 1.0,
            color: None,
        }
    }
}

impl Visual {
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn rounded(mut self, radius: impl Into<Corners>) -> Self {
        self.radius = radius.into();
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Text options that a node sets for itself and its children
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TextPatch {
    pub family: Option<Family>,
    pub size: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
}

impl TextPatch {
    /// Returns the inherited style with this patch on top
    pub fn apply(&self, base: &TextStyle) -> TextStyle {
        TextStyle {
            family: self.family.clone().unwrap_or_else(|| base.family.clone()),
            size: self.size.unwrap_or(base.size),
            color: base.color,
            bold: self.bold.unwrap_or(base.bold),
            italic: self.italic.unwrap_or(base.italic),
            underline: self.underline.unwrap_or(base.underline),
            strike: self.strike.unwrap_or(base.strike),
        }
    }
}
