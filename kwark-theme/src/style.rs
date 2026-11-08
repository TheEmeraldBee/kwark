use kui::{Color, Node, TextStyle};

/// Colors and text options for one themed name, where unset fields fall through to what is below
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: None,
            italic: None,
            underline: None,
            strike: None,
        }
    }

    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = Some(true);
        self
    }

    pub const fn italic(mut self) -> Self {
        self.italic = Some(true);
        self
    }

    pub const fn underlined(mut self) -> Self {
        self.underline = Some(true);
        self
    }

    pub const fn crossed_out(mut self) -> Self {
        self.strike = Some(true);
        self
    }

    /// Returns this style with the fields that `other` sets laid on top
    pub fn patch(self, other: Self) -> Self {
        Self {
            fg: other.fg.or(self.fg),
            bg: other.bg.or(self.bg),
            bold: other.bold.or(self.bold),
            italic: other.italic.or(self.italic),
            underline: other.underline.or(self.underline),
            strike: other.strike.or(self.strike),
        }
    }

    /// Returns the node with this style's colors and text options set on it
    pub fn apply<S>(&self, mut node: Node<S>) -> Node<S> {
        if let Some(fg) = self.fg {
            node = node.color(fg);
        }
        if let Some(bg) = self.bg {
            node = node.bg(bg);
        }
        if self.bold == Some(true) {
            node = node.bold();
        }
        if self.italic == Some(true) {
            node = node.italic();
        }
        if self.underline == Some(true) {
            node = node.underline();
        }
        if self.strike == Some(true) {
            node = node.strike();
        }

        node
    }

    /// Returns the text style with this style's foreground and text options applied
    pub fn text(&self, base: &TextStyle) -> TextStyle {
        TextStyle {
            color: self.fg.unwrap_or(base.color),
            bold: self.bold.unwrap_or(base.bold),
            italic: self.italic.unwrap_or(base.italic),
            underline: self.underline.unwrap_or(base.underline),
            strike: self.strike.unwrap_or(base.strike),
            ..base.clone()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn patch_keeps_fields_the_top_leaves_unset() {
        let red = Color::rgb(0xFF0000);
        let blue = Color::rgb(0x0000FF);
        let style = Style::new()
            .fg(red)
            .bg(red)
            .patch(Style::new().fg(blue).bold());

        assert_eq!(style.fg, Some(blue));
        assert_eq!(style.bg, Some(red));
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.italic, None);
    }
}
