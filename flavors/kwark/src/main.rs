use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub use kwark::prelude::*;
use kwark::prelude::{
    buffer::Storage,
    text_buffer::{BoundCursorSet, BufferEntry, CursorOptions, CursorSet},
    tree_sitter::LanguageStore,
};
use ratatui::layout::{Direction, Position};

mod highlighters;
mod theme;
use highlighters::*;

mod renderers;
use renderers::*;

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

fn buffer_len_lines(storage: &buffer::Storage, id: &buffer::ID) -> usize {
    match storage.get(id) {
        Some(buffer::Buffer::Text { buf, .. }) => buf.buffer.rope().len_lines(),
        _ => 0,
    }
}

fn scroll_window(s: &mut State, leaf: LeafId, delta: isize) {
    let (layout, storage) = s.get::<(&mut WindowLayout, &buffer::Storage)>();

    let Some(id) = layout.window_mut(leaf).and_then(|w| w.selected()) else {
        return;
    };

    let total_lines = buffer_len_lines(storage, &id);

    if let Some(window) = layout.window_mut(leaf) {
        window.scroll_by(delta, total_lines);
    }
}

fn create_scroll(delta: isize) -> impl Fn(&mut State) -> anyhow::Result<()> {
    move |s: &mut State| {
        let pos = *s.get::<&MousePosition>();
        let point = Position::new(pos.column, pos.row);

        let Some((leaf, _)) = s.get::<&WindowLayout>().locate(point) else {
            return Ok(());
        };

        scroll_window(s, leaf, delta);

        Ok(())
    }
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

        normal.bind(&["ctrl-d"], "Scroll Down", |s| {
            let leaf = s.get::<&WindowLayout>().active_id();
            scroll_window(s, leaf, 10);
            Ok(())
        })?;

        normal.bind(&["ctrl-u"], "Scroll Up", |s| {
            let leaf = s.get::<&WindowLayout>().active_id();
            scroll_window(s, leaf, -10);
            Ok(())
        })?;

        normal.bind(&["scroll_up"], "", create_scroll(-3))?;

        normal.bind(&["scroll_down"], "", create_scroll(3))?;

        normal.bind(&["mouse_1"], "", |s| {
            let pos = *s.get::<&MousePosition>();
            let point = Position::new(pos.column, pos.row);

            let mut taken = s.scope::<(buffer::Storage, Pipeline, WindowLayout)>();
            let ((storage, pipeline, windows), state) = taken.split();

            if let Some((leaf, id, hit)) = windows.hit_test(storage, pipeline, state, point) {
                windows.focus(leaf);
                windows.active_mut().select(id.clone());

                if let Some(mut buf) = storage.get_mut(&id).and_then(|b| b.as_text()) {
                    buf.set(hit.line, hit.col, &CursorOptions::new());
                }
            }

            Ok(())
        })?;

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

            const NAME_ROOT: &str = "scratch";

            let mut num = 0;

            while storage
                .get(&buffer::ID::scratch(format!("{}_{}", NAME_ROOT, num)))
                .is_some()
            {
                num += 1;
            }

            let buf_name = format!("{}_{}", NAME_ROOT, num);

            let buf = BufferEntry::new_scratch(buf_name);
            let id = storage.insert(buf);
            layout.active_mut().select(id);

            Ok(())
        })?;

        normal.bind(&[";", "N"], "Split window down", |s| {
            let (layout, storage) = s.get::<(&mut WindowLayout, &mut buffer::Storage)>();

            layout.split(Direction::Vertical);

            const NAME_ROOT: &str = "scratch";

            let mut num = 0;

            while storage
                .get(&buffer::ID::scratch(format!("{}_{}", NAME_ROOT, num)))
                .is_some()
            {
                num += 1;
            }

            let buf_name = format!("{}_{}", NAME_ROOT, num);

            let buf = BufferEntry::new_scratch(buf_name);
            let id = storage.insert(buf);
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

        normal.bind(&["alt-h"], "Focus window left", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Horizontal, false);
            Ok(())
        })?;

        normal.bind(&["alt-l"], "Focus window right", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Horizontal, true);
            Ok(())
        })?;

        normal.bind(&["alt-k"], "Focus window up", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Vertical, false);
            Ok(())
        })?;

        normal.bind(&["alt-j"], "Focus window down", |s| {
            s.get::<&mut WindowLayout>()
                .focus_direction(Direction::Vertical, true);
            Ok(())
        })?;

        normal.bind(&["alt-H"], "Move border left", |s| {
            s.get::<&mut WindowLayout>()
                .resize(Direction::Horizontal, -0.05);
            Ok(())
        })?;

        normal.bind(&["alt-L"], "Move border right", |s| {
            s.get::<&mut WindowLayout>()
                .resize(Direction::Horizontal, 0.05);
            Ok(())
        })?;

        normal.bind(&["alt-K"], "Move border up", |s| {
            s.get::<&mut WindowLayout>()
                .resize(Direction::Vertical, -0.05);
            Ok(())
        })?;

        normal.bind(&["alt-J"], "Move border down", |s| {
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
                    Code::Key(KeyCode::Char(c)) => match chord.mods.contains(KeyModifiers::SHIFT) {
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

    let (layout, storage) = editor.get::<(&mut WindowLayout, &mut Storage)>();

    let buf = text_buffer::BufferEntry::new_scratch("scratch");
    let id = storage.insert(buf);
    layout.active_mut().select(id);

    let store = editor.get::<&mut tree_sitter::LanguageStore>();
    store
        .register(
            "rust",
            tree_sitter::Language::new(tree_sitter_rust::LANGUAGE),
        )?
        .with_extensions(["rs"]);

    {
        let pipeline = editor.get::<&mut Pipeline>();

        // Highlighters
        pipeline.highlighters.push(TSHighlighter);
        pipeline.highlighters.push(CursorHighlighter);

        // Renderers
        pipeline
            .renderers
            .push(HeaderRenderer::new().block::<CursorTag>());
    }

    theme::init_theme(&mut editor);

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`");

    Ok(())
}
