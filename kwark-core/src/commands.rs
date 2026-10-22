use std::collections::HashMap;

use crate::{InputTree, State};

/// A single parsed argument
#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl ArgValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ArgValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            ArgValue::Int(i) => Some(*i),
            ArgValue::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            ArgValue::Float(f) => Some(*f),
            ArgValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ArgValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

fn parse_value(raw: &str) -> ArgValue {
    if let Some(stripped) = raw.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return ArgValue::Str(stripped.to_string());
    }

    match raw {
        "true" => return ArgValue::Bool(true),
        "false" => return ArgValue::Bool(false),
        _ => {}
    }

    if let Ok(i) = raw.parse::<i64>() {
        return ArgValue::Int(i);
    }

    if let Ok(f) = raw.parse::<f64>() {
        return ArgValue::Float(f);
    }

    ArgValue::Str(raw.to_string())
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in line.chars() {
        match c {
            '"' => {
                current.push(c);
                in_quotes = !in_quotes;
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Arguments passed to a command
#[derive(Debug, Default, Clone)]
pub struct Args {
    positional: Vec<ArgValue>,
    named: HashMap<String, ArgValue>,
}

impl Args {
    /// Parses a command argument string
    pub fn parse(rest: &str) -> Self {
        let mut positional = Vec::new();
        let mut named = HashMap::new();

        for tok in tokenize(rest) {
            if let Some(eq) = tok.find('=') {
                let (name, val) = (&tok[..eq], &tok[eq + 1..]);
                if is_ident(name) {
                    named.insert(name.to_string(), parse_value(val));
                    continue;
                }
            }

            positional.push(parse_value(&tok));
        }

        Self { positional, named }
    }

    /// Looks up an argument by name then position
    pub fn get(&self, name: &str, pos: usize) -> Option<&ArgValue> {
        self.named.get(name).or_else(|| self.positional.get(pos))
    }

    pub fn str<'a>(&'a self, name: &str, pos: usize, default: &'a str) -> &'a str {
        self.get(name, pos)
            .and_then(ArgValue::as_str)
            .unwrap_or(default)
    }

    pub fn int(&self, name: &str, pos: usize, default: i64) -> i64 {
        self.get(name, pos)
            .and_then(ArgValue::as_int)
            .unwrap_or(default)
    }

    pub fn float(&self, name: &str, pos: usize, default: f64) -> f64 {
        self.get(name, pos)
            .and_then(ArgValue::as_float)
            .unwrap_or(default)
    }

    pub fn bool(&self, name: &str, pos: usize, default: bool) -> bool {
        self.get(name, pos)
            .and_then(ArgValue::as_bool)
            .unwrap_or(default)
    }
}

type CommandFn = Box<dyn Fn(&mut State, &Args) -> anyhow::Result<()>>;

/// The registry of callable commands
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, CommandFn>,
}

impl CommandRegistry {
    /// Registers a command
    pub fn register(
        &mut self,
        name: impl Into<String>,
        func: impl Fn(&mut State, &Args) -> anyhow::Result<()> + 'static,
    ) {
        self.commands.insert(name.into(), Box::new(func));
    }

    /// Runs a command with the given arguments
    pub fn call(&self, state: &mut State, name: &str, args: &Args) -> anyhow::Result<()> {
        match self.commands.get(name) {
            Some(func) => func(state, args),
            None => Err(anyhow::anyhow!("unknown command `{name}`")),
        }
    }

    /// Parses and runs a command
    pub fn execute(&self, state: &mut State, line: &str) -> anyhow::Result<()> {
        let line = line.trim();

        let (name, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));

        if name.is_empty() {
            return Ok(());
        }

        self.call(state, name, &Args::parse(rest))
    }

    /// Lists the names of every command
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.keys().map(String::as_str)
    }
}

pub trait InputTreeExt {
    /// Binds a chord to a named command
    fn bind_cmd(
        &mut self,
        keys: &[&str],
        desc: impl Into<String>,
        commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> anyhow::Result<()>;

    /// Begin a builder for a group under `key`
    ///
    /// Ignores desc if it's empty
    fn group(
        &mut self,
        key: &[&str],
        desc: &str,
        f: impl FnOnce(&mut Group) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;
}

impl InputTreeExt for InputTree {
    fn bind_cmd(
        &mut self,
        keys: &[&str],
        desc: impl Into<String>,
        commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> anyhow::Result<()> {
        let commands = commands.into_iter().map(|x| x.into()).collect::<Vec<_>>();

        self.bind(keys, desc, move |s: &mut State| {
            let mut taken = s.scope::<(CommandRegistry,)>();
            let ((registry,), state) = taken.split();
            for command in commands.iter() {
                registry.execute(state, command)?;
            }

            Ok(())
        })
    }

    fn group(
        &mut self,
        keys: &[&str],
        desc: &str,
        f: impl FnOnce(&mut Group) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        if !desc.is_empty() {
            self.desc(keys, desc)?;
        }

        f(&mut Group {
            tree: self,
            prefix: keys.iter().map(|x| x.to_string()).collect(),
        })
    }
}

/// A builder for a sub-group of [`InputTree`]
pub struct Group<'a> {
    tree: &'a mut InputTree,
    prefix: Vec<String>,
}

impl<'a> Group<'a> {
    /// The underlying tree
    pub fn tree(&mut self) -> &mut InputTree {
        self.tree
    }

    /// The full chord path for a key in the group
    fn keys(&self, keys: &[&str]) -> Vec<String> {
        self.prefix
            .iter()
            .map(Clone::clone)
            .chain(keys.iter().map(|x| x.to_string()))
            .collect()
    }

    /// Binds `key` to a function
    pub fn bind(
        &mut self,
        keys: &[&str],
        desc: &str,
        f: impl Fn(&mut State) -> anyhow::Result<()> + 'static,
    ) -> anyhow::Result<()> {
        self.tree.bind(&self.keys(keys), desc, f)
    }
}

impl<'a> InputTreeExt for Group<'a> {
    fn bind_cmd(
        &mut self,
        keys: &[&str],
        desc: impl Into<String>,
        command: impl IntoIterator<Item = impl Into<String>>,
    ) -> anyhow::Result<()> {
        self.tree.bind_cmd(
            self.keys(keys)
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
            desc,
            command,
        )
    }

    fn group(
        &mut self,
        key: &[&str],
        desc: &str,
        f: impl FnOnce(&mut Group) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let keys = self.keys(key);

        if !desc.is_empty() {
            self.tree.desc(&keys, desc)?;
        }

        f(&mut Group {
            tree: self.tree,
            prefix: keys,
        })
    }
}
