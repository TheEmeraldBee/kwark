use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kaon::value::Value;

mod parse;

pub use parse::{ParseError, parse_chord};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chord {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl Chord {
    /// Builds a chord, folding an uppercase `Char` into its lowercase form plus `SHIFT`
    pub fn new(code: KeyCode, mods: KeyModifiers) -> Self {
        match code {
            KeyCode::Char(c) if c.is_uppercase() => Self {
                code: KeyCode::Char(c.to_lowercase().next().unwrap_or(c)),
                mods: mods | KeyModifiers::SHIFT,
            },
            _ => Self { code, mods },
        }
    }
}

impl From<KeyEvent> for Chord {
    fn from(event: KeyEvent) -> Self {
        Self::new(event.code, event.modifiers)
    }
}

#[derive(Debug, Clone)]
pub enum InputNode {
    Node {
        desc: String,
        key: Chord,
        children: Vec<Self>,
    },
    Leaf {
        desc: String,
        key: Chord,
        event: Value,
    },
}

impl InputNode {
    pub fn is_chord(&self, chord: &Chord) -> bool {
        match self {
            Self::Node { key, .. } => key == chord,
            Self::Leaf { key, .. } => key == chord,
        }
    }

    pub fn as_children(&mut self) -> Option<&mut [Self]> {
        match self {
            Self::Node { children, .. } => Some(children.as_mut_slice()),
            Self::Leaf { .. } => None,
        }
    }

    pub fn children(&self) -> Option<&[Self]> {
        match self {
            Self::Node { children, .. } => Some(children.as_slice()),
            Self::Leaf { .. } => None,
        }
    }

    pub fn set_desc(&mut self, new_desc: impl Into<String>) {
        match self {
            Self::Node { desc, .. } => *desc = new_desc.into(),
            Self::Leaf { desc, .. } => *desc = new_desc.into(),
        }
    }

    /// Replace a leaf with an empty node carrying the same desc and key, no-op on a node
    fn ensure_node(&mut self) {
        if let Self::Leaf { desc, key, .. } = self {
            *self = Self::Node {
                desc: std::mem::take(desc),
                key: *key,
                children: vec![],
            };
        }
    }
}

fn get_or_insert<'a>(nodes: &'a mut Vec<InputNode>, chord: &Chord) -> &'a mut InputNode {
    if let Some(idx) = nodes.iter().position(|x| x.is_chord(chord)) {
        return &mut nodes[idx];
    }
    nodes.push(InputNode::Node {
        desc: String::new(),
        key: *chord,
        children: vec![],
    });
    nodes.last_mut().unwrap()
}

/// Result of feeding one chord into `InputTree::step`
pub enum Step {
    Failed,
    Step,
    Complete(Value),
}

pub struct InputTree {
    root: Vec<InputNode>,
    current: Vec<Chord>,
}

impl Default for InputTree {
    fn default() -> Self {
        Self::new()
    }
}

impl InputTree {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            root: vec![],
            current: vec![],
        }
    }

    /// Retrieves the node found when using the chords
    fn find(&self, chords: &[Chord]) -> Option<&InputNode> {
        let mut chords = chords.iter();
        let chord = chords.next()?;

        let mut elem = self.root.iter().find(|x| x.is_chord(chord))?;

        for chord in chords {
            elem = elem.children()?.iter().find(|x| x.is_chord(chord))?;
        }

        Some(elem)
    }

    /// Feed a single chord into the tree, advancing from wherever the last `step` left off
    pub fn step(&mut self, chord: Chord) -> Step {
        self.current.push(chord);

        match self.find(&self.current) {
            Some(InputNode::Node { .. }) => Step::Step,
            Some(InputNode::Leaf { event, .. }) => {
                let event = event.clone();
                self.current.clear();
                Step::Complete(event)
            }
            None => {
                self.current.clear();
                Step::Failed
            }
        }
    }

    /// Discard any in-progress chord sequence
    pub fn reset(&mut self) {
        self.current.clear();
    }

    pub fn find_or_create(&mut self, chords: &[Chord]) -> &mut InputNode {
        let mut chords = chords.iter();
        let chord = chords
            .next()
            .expect("find_or_create requires a non-empty chord path");

        let mut elem = get_or_insert(&mut self.root, chord);

        for chord in chords {
            elem.ensure_node();
            let InputNode::Node { children, .. } = elem else {
                unreachable!("Ensure node forces elem to be a node")
            };
            elem = get_or_insert(children, chord);
        }

        elem
    }

    pub fn register(&mut self, chords: &[Chord], desc: impl Into<String>, event: Value) {
        if chords.is_empty() {
            return;
        }

        let (last, rest) = chords.split_last().expect("chords is checked to be empty");

        let children = if rest.is_empty() {
            &mut self.root
        } else {
            let parent = self.find_or_create(rest);
            parent.ensure_node();
            let InputNode::Node { children, .. } = parent else {
                unreachable!()
            };
            children
        };

        let leaf = InputNode::Leaf {
            desc: desc.into(),
            key: *last,
            event,
        };

        match children.iter_mut().find(|x| x.is_chord(last)) {
            Some(existing) => *existing = leaf,
            None => children.push(leaf),
        }
    }

    pub fn describe(&mut self, chords: &[Chord], desc: impl Into<String>) {
        let node = self.find_or_create(chords);
        node.set_desc(desc);
    }
}

/// A set of user-defined modes, each with its own `InputTree`, with stepping routed to whichever mode is current
pub struct InputState {
    trees: HashMap<String, InputTree>,
    mode: String,
}

impl InputState {
    /// Create a new state starting in the given mode
    pub fn new(mode: impl Into<String>) -> Self {
        let mode = mode.into();
        let mut trees = HashMap::new();
        trees.insert(mode.clone(), InputTree::new());
        Self { trees, mode }
    }

    /// Name of the currently active mode
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// Switch the active mode, discarding any in-progress chord sequence, creating the mode's tree if it doesn't exist yet
    pub fn set_mode(&mut self, mode: impl Into<String>) {
        if let Some(tree) = self.trees.get_mut(&self.mode) {
            tree.reset();
        }
        self.mode = mode.into();
        self.trees.entry(self.mode.clone()).or_default();
    }

    /// Get or create the `InputTree` for a mode, e.g. for `register`/`describe`
    pub fn tree(&mut self, mode: impl Into<String>) -> &mut InputTree {
        self.trees.entry(mode.into()).or_default()
    }

    /// Feed a single chord into the current mode's tree
    pub fn step(&mut self, chord: Chord) -> Step {
        self.trees.entry(self.mode.clone()).or_default().step(chord)
    }
}
