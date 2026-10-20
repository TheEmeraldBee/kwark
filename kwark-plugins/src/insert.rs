use kwark_text::CursorOptions;

use kwark_core::prelude::*;

use crate::{cursor_overrides::CursorOverrides, internal_utils::*};

/// Initializes the editor to work with this module
pub fn init(editor: &mut Editor) -> anyhow::Result<()> {
    editor.insert(CursorOptions::new());

    Ok(())
}

/// Registers the `insert`, `delete`, `undo`, and `redo` commands, calling [`init`] itself
pub struct Plugin;

impl kwark_core::Plugin for Plugin {
    fn register(&self, editor: &mut Editor) -> anyhow::Result<()> {
        let _ = init(editor);

        let registry = editor.get::<&mut CommandRegistry>();

        registry.register("insert", |s, args| {
            let text = args.str("text", 0, "");
            if text.is_empty() {
                return Ok(());
            }

            let opts = *s.get::<&CursorOptions>();

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.insert(text, &opts);

            Ok(())
        });

        registry.register("delete", |s, args| {
            let before = args.bool("before", 0, false);
            let opts = *s.get::<&CursorOptions>();

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            if !opts.extend {
                let delta = if before { -1 } else { 1 };
                buf.move_(0, delta, &opts.wrap(true).extend(true));
            }

            buf.delete(&opts);

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
pub fn create_insertion(
    text: &str,
    overrides: CursorOverrides,
) -> impl Fn(&mut State) -> anyhow::Result<()> {
    move |s: &mut State| {
        let opts = overrides.combine(*s.get::<&CursorOptions>());

        let Some(mut buf) = active_buffer(s) else {
            return Ok(());
        };

        buf.insert(text, &opts);

        Ok(())
    }
}

/// Binds all important insertion keys for use in a insertion-like system
pub fn bind_insert<'a>(
    tree: &mut InputTree,
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

    tree.set_backup(
        |s, chord| {
            let opts = *s.get::<&CursorOptions>();

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
