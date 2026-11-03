use std::collections::HashMap;

/// A set of flags (booleans) that the editor can quickly hold
#[derive(Default)]
pub struct Flags {
    flags: HashMap<&'static str, bool>,
}

impl Flags {
    /// Get the value of a flag
    pub fn get(&self, flag: &str) -> bool {
        self.flags.get(flag).copied().unwrap_or_default()
    }

    /// Set the value of a flag
    pub fn set(&mut self, flag: &'static str, set: bool) {
        self.flags.insert(flag, set);
    }

    /// Toggle the value of a flag
    pub fn toggle(&mut self, flag: &'static str) {
        self.flags.insert(flag, !self.get(flag));
    }
}
