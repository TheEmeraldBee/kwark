use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub use kwark::prelude::*;
use kwark::prelude::{
    text_buffer::{BoundCursorSet, BufferEntry, CursorOptions, CursorSet},
    tree_sitter::LanguageStore,
};
use ratatui::layout::Direction;

mod highlighters;
mod theme;
use highlighters::*;

fn active_buffer(s: &mut State) -> Option<BoundCursorSet<'_>> {
    let id = s.get::<&WindowLayout>().active().selected()?;
    s.get::<&mut buffer::Storage>().get_mut(&id)?.as_text()
}

fn commit_changes(state: &mut State) {
    let Some(mut buf) = active_buffer(state) else {
        return;
    };

    buf.commit_change();
}

fn create_move_cursor(
    lines: isize,
    columns: isize,
    options: CursorOptions,
) -> impl Fn(&mut State) -> anyhow::Result<()> {
    move |s: &mut State| {
        let opts = s.get::<&CursorOptions>().join(options);

        let Some(mut buf) = active_buffer(s) else {
            return Ok(());
        };

        buf.move_(lines, columns, &opts);

        Ok(())
    }
}

fn bind_movement(
    tree: &mut InputTree,
    options: CursorOptions,
    left: &str,
    right: &str,
    up: &str,
    down: &str,
) -> anyhow::Result<()> {
    tree.bind(&[left], "", create_move_cursor(0, -1, options))?;

    tree.bind(&[right], "", create_move_cursor(0, 1, options))?;

    tree.bind(&[up], "", create_move_cursor(-1, 0, options))?;

    tree.bind(&[down], "", create_move_cursor(1, 0, options))?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Initialize the editor
    let mut editor = kwark::init();

    editor.insert(LanguageStore::new("./queries".into())?);
    editor.insert(CursorOptions::new());

    // Retrieve the input state from the editor
    let input = editor.get::<&mut InputState>();

    // Bind a ton of normal-mode keybinds
    {
        let normal = input.tree("normal");

        normal.desc(&["space"], "General Actions")?;

        normal.desc(&["space", "f"], "File Pickers")?;

        normal.bind(&["space", "f", "f"], "Open FZF file picker", |s| {
            let value = replace_term(s.sender(), || {
                let out = String::from_utf8_lossy(
                    &Command::new("sh")
                        .args(["-c", "find \"$PWD\" -type f | fzf"])
                        .stdout(Stdio::piped())
                        .spawn()?
                        .wait_with_output()?
                        .stdout,
                )
                .to_string();

                if out.trim().is_empty() {
                    return Ok(None);
                }

                Ok(Some(out.trim().to_string()))
            })?;

            if let Some(path) = value {
                let (layout, storage) = s.get::<(&mut WindowLayout, &mut buffer::Storage)>();

                let buf = BufferEntry::new_file(path.into())?;
                let id = storage.insert(buffer::Buffer::Text {
                    id: buf.kind().clone(),
                    buf: Box::new(buf),
                    cursors: Box::new(CursorSet::new()),
                    tree: None,
                    parsed_version: 0,
                });

                layout.active_mut().select(id);
            }

            Ok(())
        })?;

        normal.bind(&["space", "f", "t"], "Open YAZI file picker", |s| {
            let value = replace_term(s.sender(), || {
                Command::new("yazi")
                    .args(["--chooser-file", "/tmp/chooser"])
                    .spawn()?
                    .wait()?;

                let lines = std::fs::read_to_string("/tmp/chooser")?
                    .split("\n")
                    .filter_map(|x| {
                        let out = x.trim().to_string();
                        if out.is_empty() { None } else { Some(out) }
                    })
                    .collect::<Vec<_>>();

                Ok(lines)
            })?;

            let (layout, storage) = s.get::<(&mut WindowLayout, &mut buffer::Storage)>();

            for file in value {
                let path = PathBuf::from(file).canonicalize()?;
                if !path.is_file() {
                    continue;
                }

                let buf = BufferEntry::new_file(path)?;
                let id = storage.insert(buffer::Buffer::Text {
                    id: buf.kind().clone(),
                    buf: Box::new(buf),
                    cursors: Box::new(CursorSet::new()),
                    tree: None,
                    parsed_version: 0,
                });

                layout.active_mut().select(id);
            }

            Ok(())
        })?;

        bind_movement(
            normal,
            CursorOptions::default().wrap(true),
            "h",
            "l",
            "k",
            "j",
        )?;

        bind_movement(
            normal,
            CursorOptions::default().wrap(true),
            "left",
            "right",
            "up",
            "down",
        )?;

        normal.bind(&["g", "p"], "Goto Previous Buffer in Window", |s| {
            let layout = s.get::<&mut WindowLayout>();

            layout.active_mut().move_primary(-1);

            Ok(())
        })?;

        normal.bind(&["g", "n"], "Goto Next Buffer in Window", |s| {
            let layout = s.get::<&mut WindowLayout>();

            layout.active_mut().move_primary(1);

            Ok(())
        })?;

        normal.desc(&[";"], "Shortcut Menu")?;

        normal.bind(&[";", "Q"], "Quit the Editor", |s| {
            s.get::<&mut Running>().quit();

            Ok(())
        })?;

        normal.bind(&[";", "n"], "Split window right", |s| {
            let (layout, storage) = s.get::<(&mut WindowLayout, &mut buffer::Storage)>();

            layout.split(Direction::Horizontal);

            let buf = BufferEntry::new_scratch("helloworld".to_string());
            let id = storage.insert(buffer::Buffer::Text {
                id: buf.kind().clone(),
                buf: Box::new(buf),
                cursors: Box::new(CursorSet::new()),
                tree: None,
                parsed_version: 0,
            });

            layout.active_mut().select(id);

            Ok(())
        })?;

        normal.bind(&[";", "N"], "Split window down", |s| {
            let (layout, storage) = s.get::<(&mut WindowLayout, &mut buffer::Storage)>();

            layout.split(Direction::Vertical);

            let buf = BufferEntry::new_scratch("helloworld".to_string());
            let id = storage.insert(buffer::Buffer::Text {
                id: buf.kind().clone(),
                buf: Box::new(buf),
                cursors: Box::new(CursorSet::new()),
                tree: None,
                parsed_version: 0,
            });

            layout.active_mut().select(id);

            Ok(())
        })?;

        normal.bind(
            &[";", "q"],
            "Close active buffer (closes the window if it was the last buffer in the window)",
            |s| {
                let windows = s.get::<&mut WindowLayout>();
                windows.active_mut().close_primary();
                if windows.active_mut().is_empty() {
                    let active = windows.active_id();
                    windows.close(active);
                }
                Ok(())
            },
        )?;

        normal.bind(&[";", "h"], "Focus window left", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Horizontal, false);
            Ok(())
        })?;

        normal.bind(&[";", "l"], "Focus window right", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Horizontal, true);
            Ok(())
        })?;

        normal.bind(&[";", "k"], "Focus window up", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Vertical, false);
            Ok(())
        })?;

        normal.bind(&[";", "j"], "Focus window down", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Vertical, true);
            Ok(())
        })?;

        normal.bind(&[";", "H"], "Move border left", |s| {
            s.get::<&mut WindowLayout>()
                .resize(Direction::Horizontal, -0.05);
            Ok(())
        })?;

        normal.bind(&[";", "L"], "Move border right", |s| {
            s.get::<&mut WindowLayout>()
                .resize(Direction::Horizontal, 0.05);
            Ok(())
        })?;

        normal.bind(&[";", "K"], "Move border up", |s| {
            s.get::<&mut WindowLayout>()
                .resize(Direction::Vertical, -0.05);
            Ok(())
        })?;

        normal.bind(&[";", "J"], "Move border down", |s| {
            s.get::<&mut WindowLayout>()
                .resize(Direction::Vertical, 0.05);
            Ok(())
        })?;

        normal.bind(&["u"], "Undo", |s| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.undo();

            Ok(())
        })?;

        normal.bind(&["U"], "Redo", |s| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.redo();

            Ok(())
        })?;

        normal.bind(&["i"], "Enter Insert Mode", |s| {
            s.get::<&mut InputState>().set_mode("insert");

            commit_changes(s);

            Ok(())
        })?;

        normal.bind(&["c"], "Toggle All-Cursor Mode", |s| {
            let options = s.get::<&mut CursorOptions>();

            *options = options.all(!options.is_all());

            Ok(())
        })?;

        normal.bind(&["d"], "Delete Selection", |s| {
            let opts = *s.get::<&CursorOptions>();

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.move_(0, 1, &opts.extend(true).wrap(true));

            buf.delete(&opts);

            buf.commit_change();

            Ok(())
        })?;

        normal.bind(&["C"], "Create new cursor below", |s| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.duplicate();
            buf.move_(1, 0, &CursorOptions::new());

            Ok(())
        })?;

        normal.bind(&[","], "Delete other cursors", |s| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.remove_other();

            Ok(())
        })?;

        normal.bind(&["["], "Change Cursor Up", |s| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.change_primary(-1);

            Ok(())
        })?;

        normal.bind(&["]"], "Change Cursor Down", |s| {
            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.change_primary(1);

            Ok(())
        })?;

        bind_movement(
            normal,
            CursorOptions::new().extend(true).wrap(true),
            "shift-left",
            "shift-right",
            "shift-up",
            "shift-down",
        )?;

        bind_movement(
            normal,
            CursorOptions::new().extend(true).wrap(true),
            "H",
            "L",
            "K",
            "J",
        )?;

        normal.bind(&["?"], "Show layer inputs", |s| {
            s.get::<&mut Flags>().set("input_tree_show", true);
            Ok(())
        })?;
    }

    // Bind a bunch of insert mode keybinds
    {
        let insert = input.tree("insert");

        bind_movement(
            insert,
            CursorOptions::new().extend(true).wrap(true),
            "left",
            "right",
            "up",
            "down",
        )?;

        bind_movement(
            insert,
            CursorOptions::new().extend(true).wrap(true),
            "shift-left",
            "shift-right",
            "shift-up",
            "shift-down",
        )?;

        insert.bind(&["escape"], "Switch to Normal mode", |s| {
            s.get::<&mut InputState>().set_mode("normal");

            commit_changes(s);

            Ok(())
        })?;

        insert.bind(&["backspace"], "Delete Char", |s| {
            let opts = *s.get::<&CursorOptions>();

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.move_(0, -1, &opts.wrap(true).extend(true));
            buf.delete(&opts);

            Ok(())
        })?;

        insert.bind(&["enter"], "Insert Newline", |s| {
            let opts = *s.get::<&CursorOptions>();

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.insert("\n", &opts);

            Ok(())
        })?;

        insert.bind(&["space"], "Insert Space", |s| {
            let opts = *s.get::<&CursorOptions>();

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.insert(" ", &opts);

            Ok(())
        })?;

        insert.set_backup(
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
                    KeyCode::Char(c) => match chord.mods.contains(KeyModifiers::SHIFT) {
                        false => c.to_string(),
                        true => c.to_ascii_uppercase().to_string(),
                    },
                    _ => return Ok(()),
                };

                buf.insert(key_string.as_str(), &opts);

                Ok(())
            },
            "Insert Pressed",
        );
    }

    let store = editor.get::<&mut tree_sitter::LanguageStore>();
    store.register(
        "rust",
        tree_sitter::Language::new(tree_sitter_rust::LANGUAGE),
    )?;

    editor
        .get::<&mut Pipeline>()
        .highlighters
        .push(TSHighlighter);

    editor
        .get::<&mut Pipeline>()
        .highlighters
        .push(CursorHighlighter);

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`");

    Ok(())
}
