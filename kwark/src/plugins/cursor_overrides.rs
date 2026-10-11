use kwark_text_buffer::CursorOptions;

/// A set of overrides that we can force onto movement over the globals
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorOverrides {
    /// Whether, when the cursor moves, to extend the selection
    pub extend: Option<bool>,
    /// Whether, when the cursor's column goes past the end of line, to move it to the beginning of the next line
    pub wrap: Option<bool>,
    /// Whether or not to apply this to all cursors in the set
    pub all: Option<bool>,
}

impl CursorOverrides {
    /// Creates a new set of overrides
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets extend to the given boolean, causing it to override apon application
    pub fn extend(mut self, extend: bool) -> Self {
        self.extend = Some(extend);
        self
    }

    /// Unsets extend so it doesn't override
    pub fn unset_extend(mut self) -> Self {
        self.extend = None;
        self
    }

    /// Sets wrap to the given boolean, causing it to override apon application
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self
    }

    /// Unsets wrap so it doesn't override
    pub fn unset_wrap(mut self) -> Self {
        self.wrap = None;
        self
    }

    /// Sets all to the given boolean, causing it to override apon application
    pub fn all(mut self, all: bool) -> Self {
        self.all = Some(all);
        self
    }

    /// Unsets all so it doesn't override
    pub fn unset_all(mut self) -> Self {
        self.all = None;
        self
    }

    /// Converts the overrides to a single options
    ///
    /// Defaults all settings to false if not overriding
    pub fn to_options(self) -> CursorOptions {
        CursorOptions {
            extend: self.extend.unwrap_or_default(),
            wrap: self.wrap.unwrap_or_default(),
            all: self.all.unwrap_or_default(),
        }
    }

    /// Combine these overrides with a set of cursor options
    pub fn combine(self, mut opts: CursorOptions) -> CursorOptions {
        if let Some(extend) = self.extend {
            opts.extend = extend
        }

        if let Some(wrap) = self.wrap {
            opts.wrap = wrap
        }

        if let Some(all) = self.all {
            opts.all = all
        }
        opts
    }
}
