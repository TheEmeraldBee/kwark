use kwark_core::prelude::*;

use crate::{cursor_overrides::CursorOverrides, internal_utils::*};

/// Registers the `insert`, `delete`, `undo`, and `redo` commands
#[derive(Default)]
pub struct Plugin;

impl PluginState for Plugin {
    type State = ();

    fn init<S: 'static>(&self, _: &Senders<S>) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<S: 'static> kwark_core::Plugin<S> for Plugin {
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()> {
        let registry = &mut editor.core.commands;

        registry.register("insert", |s, args| {
            insert(s, args.str("text", 0, ""), CursorOverrides::new())
        });

        registry.register("delete", |s, args| delete(s, args.bool("before", 0, false)));

        registry.register("undo", |s, _| undo(s));

        registry.register("redo", |s, _| redo(s));

        Ok(())
    }
}

fn join_prefix<'a>(prefix: &'a [&'a str], bind: &'a &'a str) -> Vec<&'a &'a str> {
    prefix
        .iter()
        .chain(std::iter::once(bind))
        .collect::<Vec<_>>()
}

/// Inserts `text` at the cursors
pub fn insert<S: 'static>(
    e: &mut Editor<S>,
    text: &str,
    overrides: CursorOverrides,
) -> anyhow::Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    let opts = overrides.combine(e.core.cursor);

    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.insert(text, &opts);

    Ok(())
}

/// Deletes the selection, or the char before or after the cursors when not extending
pub fn delete<S: 'static>(e: &mut Editor<S>, before: bool) -> anyhow::Result<()> {
    let opts = e.core.cursor;

    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    if opts.extend {
        buf.delete(&opts);
    } else {
        buf.delete_char(before, &opts);
    }

    Ok(())
}

/// Undoes the last change in the active buffer
pub fn undo<S: 'static>(e: &mut Editor<S>) -> anyhow::Result<()> {
    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.undo();

    Ok(())
}

/// Redoes the last undone change in the active buffer
pub fn redo<S: 'static>(e: &mut Editor<S>) -> anyhow::Result<()> {
    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.redo();

    Ok(())
}

/// Binds all important insertion keys for use in a insertion-like system
pub fn bind_insert<'a, S: 'static>(
    tree: &mut InputTree<S>,
    prefix: impl IntoIterator<Item = &'a str>,
    overrides: CursorOverrides,
) -> anyhow::Result<()> {
    let prefix = prefix.into_iter().collect::<Vec<_>>();

    tree.bind(
        &join_prefix(&prefix, &"space"),
        "Insert Space",
        move |e: &mut Editor<S>| insert(e, " ", overrides),
    )?;

    tree.bind(
        &join_prefix(&prefix, &"enter"),
        "Insert Newline",
        move |e: &mut Editor<S>| insert(e, "\n", overrides),
    )?;

    tree.bind(
        &join_prefix(&prefix, &"tab"),
        "Insert Tab",
        move |e: &mut Editor<S>| insert(e, "\t", overrides),
    )?;

    tree.set_backup(
        |s, chord| {
            let opts = s.core.cursor;

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            // Don't handle anything other than shift
            if chord.mods != KeyModifiers::SHIFT && chord.mods != KeyModifiers::empty() {
                return Ok(());
            }

            let key_string = match chord.code {
                Code::Key(KeyCode::Char(c)) => match chord.mods.contains(KeyModifiers::SHIFT) {
                    false => c.to_string(),
                    true => c.to_ascii_uppercase().to_string(),
                },
                _ => return Ok(()),
            };

            buf.insert(key_string.as_str(), &opts);

            Ok(())
        },
        "Insert Pressed Key",
    );

    Ok(())
}
