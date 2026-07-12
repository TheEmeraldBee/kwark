use std::collections::HashMap;

use crate::value::Value;

/// A set of frames that are accessible for variables
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
    pub fn push_frame(&mut self) {
        self.frames.push(Frame::default())
    }

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

    /// Sets an existing variable's value, value should exist in order to set, returns false if the value doesn't exist
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

    /// Retrieves a value going down the stack to retrieve.
    pub fn get(&self, name: impl Into<String>) -> Option<&Value> {
        let name = name.into();

        self.frames
            .iter()
            .rev()
            .find_map(|x| x.variables.get(&name))
    }
}

#[derive(Default)]
struct Frame {
    pub variables: HashMap<String, Value>,
}
