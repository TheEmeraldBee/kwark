use std::{collections::HashMap, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

impl std::fmt::Display for Chord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (flag, name) in [
            (KeyModifiers::CONTROL, "ctrl"),
            (KeyModifiers::ALT, "alt"),
            (KeyModifiers::SHIFT, "shift"),
            (KeyModifiers::SUPER, "super"),
            (KeyModifiers::META, "meta"),
            (KeyModifiers::HYPER, "hyper"),
        ] {
            if self.mods.contains(flag) {
                write!(f, "{name}-")?;
            }
        }

        match self.code {
            KeyCode::Char(' ') => write!(f, "space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Tab => write!(f, "tab"),
            KeyCode::Enter => write!(f, "enter"),
            KeyCode::Esc => write!(f, "esc"),
            KeyCode::Backspace => write!(f, "backspace"),
            KeyCode::Delete => write!(f, "delete"),
            KeyCode::Insert => write!(f, "insert"),
            KeyCode::Home => write!(f, "home"),
            KeyCode::End => write!(f, "end"),
            KeyCode::PageUp => write!(f, "pageup"),
            KeyCode::PageDown => write!(f, "pagedown"),
            KeyCode::Up => write!(f, "up"),
            KeyCode::Down => write!(f, "down"),
            KeyCode::Left => write!(f, "left"),
            KeyCode::Right => write!(f, "right"),
            KeyCode::F(n) => write!(f, "f{n}"),
            other => write!(f, "{other:?}"),
        }
    }
}

type Event<S> = Rc<dyn Fn(&mut S) -> anyhow::Result<()> + 'static>;
type EventBackup<S> = Rc<dyn Fn(&mut S, Chord) -> anyhow::Result<()> + 'static>;

#[derive(Clone)]
pub enum InputNode<S: 'static> {
    Node {
        desc: String,
        key: Chord,
        children: Vec<Self>,
    },
    Leaf {
        desc: String,
        key: Chord,
        event: Event<S>,
    },
}

impl<S: 'static> InputNode<S> {
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

    pub fn desc(&self) -> &str {
        match self {
            Self::Node { desc, .. } => &desc,
            Self::Leaf { desc, .. } => &desc,
        }
    }

    pub fn key(&self) -> Chord {
        match self {
            Self::Node { key, .. } => *key,
            Self::Leaf { key, .. } => *key,
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

fn get_or_insert<'a, S: 'static>(
    nodes: &'a mut Vec<InputNode<S>>,
    chord: &Chord,
) -> &'a mut InputNode<S> {
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
pub enum Step<S: 'static> {
    Failed,
    Step,
    Complete(Event<S>, Vec<Chord>),
}

pub struct InputTree<S: 'static> {
    root: Vec<InputNode<S>>,
    current: Vec<Chord>,

    backup: Option<EventBackup<S>>,
    backup_desc: Option<String>,
}

impl<S: 'static> Default for InputTree<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: 'static> InputTree<S> {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            root: vec![],
            current: vec![],

            backup: None,
            backup_desc: None,
        }
    }

    /// Retrieves the node found when using the chords
    fn find(&self, chords: &[Chord]) -> Option<&InputNode<S>> {
        let mut chords = chords.iter();
        let chord = chords.next()?;

        let mut elem = self.root.iter().find(|x| x.is_chord(chord))?;

        for chord in chords {
            elem = elem.children()?.iter().find(|x| x.is_chord(chord))?;
        }

        Some(elem)
    }

    /// Feed a single chord into the tree, advancing from wherever the last `step` left off
    pub fn step(&mut self, chord: Chord) -> Step<S> {
        self.current.push(chord);

        match self.find(&self.current) {
            Some(InputNode::Node { .. }) => Step::Step,
            Some(InputNode::Leaf { event, .. }) => {
                let event = event.clone();
                let chords = std::mem::take(&mut self.current);
                Step::Complete(event, chords)
            }
            None => {
                // Only 1 chord input and a backup method exists
                if self.current.len() == 1
                    && let Some(backup) = &self.backup
                {
                    let chords = std::mem::take(&mut self.current);
                    let last = chords.last().expect("Length checked to be 1").clone();
                    let backup = backup.clone();

                    return Step::Complete(Rc::new(move |state| backup(state, last)), chords);
                }

                self.current.clear();
                Step::Failed
            }
        }
    }

    /// Discard any in-progress chord sequence
    pub fn reset(&mut self) {
        self.current.clear();
    }

    pub fn find_or_create(&mut self, chords: &[Chord]) -> &mut InputNode<S> {
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

    pub fn set_backup(
        &mut self,
        backup: Rc<dyn Fn(&mut S, Chord) -> anyhow::Result<()> + 'static>,
        desc: impl Into<String>,
    ) {
        self.backup = Some(backup);
        self.backup_desc = Some(desc.into());
    }

    pub fn clear_backup(&mut self) {
        self.backup = None;
        self.backup_desc = None;
    }

    pub fn bind(
        &mut self,
        keys: &[impl AsRef<str>],
        desc: impl Into<String>,
        event: Event<S>,
    ) -> anyhow::Result<()> {
        let chords = keys
            .iter()
            .map(|x| parse_chord(x.as_ref()))
            .collect::<Result<Vec<Chord>, ParseError>>()?;

        self.register(&chords, desc, event);

        Ok(())
    }

    pub fn register(&mut self, chords: &[Chord], desc: impl Into<String>, event: Event<S>) {
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

    pub fn desc(
        &mut self,
        keys: &[impl AsRef<str>],
        desc: impl Into<String>,
    ) -> Result<(), ParseError> {
        let chords = keys
            .iter()
            .map(|x| parse_chord(x.as_ref()))
            .collect::<Result<Vec<_>, ParseError>>()?;

        self.describe(&chords, desc);

        Ok(())
    }

    pub fn describe(&mut self, chords: &[Chord], desc: impl Into<String>) {
        let node = self.find_or_create(chords);
        node.set_desc(desc);
    }
}

/// A set of user-defined modes, each with its own `InputTree`, with stepping routed to whichever mode is current
pub struct InputState<S: 'static> {
    trees: HashMap<String, InputTree<S>>,
    mode: String,
}

impl<S: 'static> InputState<S> {
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
    pub fn tree(&mut self, mode: impl Into<String>) -> &mut InputTree<S> {
        self.trees.entry(mode.into()).or_default()
    }

    /// Feed a single chord into the current mode's tree
    pub fn step(&mut self, chord: Chord) -> Step<S> {
        self.trees.entry(self.mode.clone()).or_default().step(chord)
    }

    /// Returns if a chord is currently in progress
    pub fn is_active(&self) -> bool {
        self.trees
            .get(&self.mode)
            .map(|x| x.current.len() > 0)
            .unwrap_or(false)
    }

    /// Returns a list of items at the current layer of the input node
    pub fn get_layer(&self) -> Vec<(Option<Chord>, String)> {
        let mut out = vec![];
        let Some(tree) = self.trees.get(&self.mode) else {
            return out;
        };

        let found = match tree.find(&tree.current) {
            Some(node) => match node.children() {
                Some(nodes) => nodes,
                None => return out,
            },
            None => &tree.root,
        };

        // Add all children to the layer
        for child in found {
            out.push((Some(child.key()), child.desc().to_string()));
        }

        if let Some(backup_desc) = tree.backup_desc.clone() {
            out.push((None, backup_desc))
        }

        out
    }
}
