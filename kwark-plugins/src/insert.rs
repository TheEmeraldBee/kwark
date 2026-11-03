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
            let text = args.str("text", 0, "");
            if text.is_empty() {
                return Ok(());
            }

            let opts = s.core.cursor;

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.insert(text, &opts);

            Ok(())
        });

        registry.register("delete", |s, args| {
            let before = args.bool("before", 0, false);
            let opts = s.core.cursor;

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            if opts.extend {
                buf.delete(&opts);
            } else {
                buf.delete_char(before, &opts);
            }

            Ok(())
        });

        registry.register("undo", |s, _| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.undo();

            Ok(())
        });

        registry.register("redo", |s, _| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.redo();

            Ok(())
        });

        Ok(())
    }
}

fn join_prefix<'a>(prefix: &'a [&'a str], bind: &'a &'a str) -> Vec<&'a &'a str> {
    prefix
        .iter()
        .chain(std::iter::once(bind))
        .collect::<Vec<_>>()
}

/// Creates an insertion function for use in [`InputTree::bind`]
pub fn create_insertion<S: 'static>(
    text: &str,
    overrides: CursorOverrides,
) -> impl Fn(&mut Editor<S>) -> anyhow::Result<()> {
    move |s: &mut Editor<S>| {
        let opts = overrides.combine(s.core.cursor);

        let Some(mut buf) = active_buffer(s) else {
            return Ok(());
        };

        buf.insert(text, &opts);

        Ok(())
    }
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
        create_insertion(" ", overrides),
    )?;

    tree.bind(
        &join_prefix(&prefix, &"enter"),
        "Insert Newline",
        create_insertion("\n", overrides),
    )?;

    tree.bind(
        &join_prefix(&prefix, &"tab"),
        "Insert Tab",
        create_insertion("\t", overrides),
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
