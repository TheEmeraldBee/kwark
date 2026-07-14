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

    fn get_mut(&mut self, name: impl Into<String>) -> Option<&mut Value> {
        let name = name.into();

        self.frames
            .iter_mut()
            .rev()
            .find_map(|x| x.variables.get_mut(&name))
    }

    /// Retrieves a value from the scope
    pub fn get(&self, name: impl Into<String>) -> Option<&Value> {
        let name = name.into();

        self.frames
            .iter()
            .rev()
            .find_map(|x| x.variables.get(&name))
    }

    /// The name of every variable
    pub fn names(&self) -> impl Iterator<Item = &str> {
        let mut seen = std::collections::HashSet::new();

        self.frames
            .iter()
            .rev()
            .flat_map(|frame| frame.variables.keys().map(String::as_str))
            .filter(move |name| seen.insert(*name))
    }
}

#[derive(Default, Clone)]
struct Frame {
    pub variables: HashMap<String, Value>,
}
