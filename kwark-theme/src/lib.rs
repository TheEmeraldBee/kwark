use std::collections::HashMap;

use ratatui::style::Style;

/// A set of styles for given names that are split up by `.` characters
#[derive(Default, Clone)]
pub struct Theme {
    styles: HashMap<String, Style>,
    backup_style: Style,
}

impl Theme {
    /// Sets the backup style for the theme
    pub fn set_backup(&mut self, style: Style) -> &mut Self {
        self.backup_style = style;
        self
    }

    /// Sets the value of the name to the style
    pub fn set(&mut self, name: impl Into<String>, style: Style) -> &mut Self {
        self.styles.insert(name.into(), style);
        self
    }

    /// Sets an iterator of items into the theme
    pub fn set_all(
        &mut self,
        styles: impl IntoIterator<Item = (impl Into<String>, Style)>,
    ) -> &mut Self {
        for style in styles {
            self.set(style.0, style.1);
        }

        self
    }

    /// Retrieve the closest match to the name
    ///
    /// Splits `.` down from the right side until we either find a style or return the default
    ///
    /// # Examples:
    /// Theme includes `a.b.c`, we ask for `a.b.c`, return `a.b.c`
    /// Theme includes `a`, we ask for `a.b`, no `a.b`, return `a`
    /// Theme includes `a`, we ask for `a.b.c`, no `a.b.c`, no `a.b`, return `a`
    ///
    /// Theme includes Nothing, we ask for `a.b`, no `a.b`, no `a`, return whatever the default style is.
    pub fn get(&self, mut name: &str) -> Style {
        if let Some(s) = self.styles.get(name) {
            return *s;
        }

        // Take the left side over and over
        while let Some((left, _)) = name.rsplit_once('.') {
            name = left;

            if let Some(s) = self.styles.get(name) {
                return *s;
            }
        }

        self.backup_style
    }
}
