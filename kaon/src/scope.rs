use std::collections::HashMap;

use crate::value::Value;

/// A set of frames that are accessible for variables
#[derive(Clone)]
pub struct Scope {
    frames: Vec<Frame>,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            frames: vec![Frame::default()],
        }
    }
}

impl Scope {
    /// Pushes a frame onto the stack, creating a new logical "scope" that inherits all parent scopes
    pub fn push_frame(&mut self) {
        self.frames.push(Frame::default())
    }

    /// Pushes a frame that blocks lookups from reaching frames beneath it, other than the global frame
    pub fn push_blocking_frame(&mut self) {
        self.frames.push(Frame {
            blocking: true,
            ..Frame::default()
        })
    }

    /// Removes a single frame from the system, not removing the bottom of the stack
    pub fn pop_frame(&mut self) {
        if self.frames.len() <= 1 {
            return;
        }

        self.frames.pop();
    }

    /// Creates a **new** variable for the top of the stack
    pub fn register(&mut self, name: impl Into<String>, value: Value) {
        self.frames
            .last_mut()
            .expect("there should always be at least 1 frame")
            .variables
            .insert(name.into(), value);
    }

    /// Sets an existing variable's value, returns false if the value doesn't exist
    pub fn set(&mut self, name: impl Into<String>, value: Value) -> bool {
        let var = self.get_mut(name);

        let Some(var) = var else {
            return false;
        };

        *var = value;

        true
    }

    /// The index of the topmost blocking frame, or 0 if there is none
    fn boundary(&self) -> usize {
        self.frames.iter().rposition(|x| x.blocking).unwrap_or(0)
    }

    fn get_mut(&mut self, name: impl Into<String>) -> Option<&mut Value> {
        let name = name.into();
        let boundary = self.boundary();

        if boundary == 0 {
            return self
                .frames
                .iter_mut()
                .rev()
                .find_map(|x| x.variables.get_mut(&name));
        }

        let (global, rest) = self.frames.split_at_mut(1);
        rest[boundary - 1..]
            .iter_mut()
            .rev()
            .find_map(|x| x.variables.get_mut(&name))
            .or_else(|| global[0].variables.get_mut(&name))
    }

    /// Retrieves a value from the scope
    pub fn get(&self, name: impl Into<String>) -> Option<&Value> {
        let name = name.into();
        let boundary = self.boundary();

        self.frames[boundary..]
            .iter()
            .rev()
            .find_map(|x| x.variables.get(&name))
            .or_else(|| (boundary > 0).then(|| self.frames[0].variables.get(&name)).flatten())
    }

    /// The name of every variable visible from the current frame
    pub fn names(&self) -> impl Iterator<Item = &str> {
        let mut seen = std::collections::HashSet::new();
        let boundary = self.boundary();

        self.frames[boundary..]
            .iter()
            .rev()
            .chain((boundary > 0).then(|| &self.frames[0]))
            .flat_map(|frame| frame.variables.keys().map(String::as_str))
            .filter(move |name| seen.insert(*name))
    }
}

#[derive(Default, Clone)]
struct Frame {
    pub variables: HashMap<String, Value>,
    pub blocking: bool,
}
