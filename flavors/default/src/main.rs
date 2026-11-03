use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use kwark::prelude::*;
use kwark::{
    plugins::{self, cursor_overrides::CursorOverrides},
    prelude::{
        text::{BoundCursorSet, BufferEntry},
        tree_sitter::LanguageStore,
    },
};
use ratatui::{layout::Position, widgets::BorderType};

mod theme;

kwark::state! {
    /// The state of the default flavor
    pub struct DefaultState {
        visual: plugins::visual::Plugin,
        editor_commands: plugins::editor_commands::Plugin,
        movement: plugins::movement::Plugin,
        insert: plugins::insert::Plugin,
        window: plugins::window::Plugin,
        command_line: plugins::command_line::Plugin,
        smooth_scroll: plugins::smooth_scroll::Plugin = plugins::smooth_scroll::Plugin::new(40.0),
    }
}

type Editor = kwark::Editor<DefaultState>;

fn active_buffer(e: &mut Editor) -> Option<BoundCursorSet<'_>> {
    let id = e.core.windows.active().selected()?;
    e.core.buffers.get_mut(&id)?.as_text()
}

fn commit_changes(e: &mut Editor) {
    let Some(mut buf) = active_buffer(e) else {
        return;
    };

    buf.commit_change();
}

/// Opens a file in the active window
fn open_file(e: &mut Editor, path: PathBuf) {
    e.spawn_blocking(
        move || BufferEntry::new_file(path),
        |e, buf| {
            let id = e.core.buffers.insert(buf?);
            e.core.windows.active_mut().select(id);

            Ok(())
        },
    );
}

fn buffer_len_lines(storage: &buffer::Storage, id: &buffer::ID) -> usize {
    match storage.get(id) {
        Some(buffer::Buffer::Text { buf, .. }) => buf.buffer.rope().len_lines(),
        _ => 0,
    }
}

fn scroll_window(e: &mut Editor, leaf: LeafId, delta: isize) {
    let Core {
        windows, buffers, ..
    } = &mut e.core;

    let Some(window) = windows.window_mut(leaf) else {
        return;
    };
    let Some(id) = window.selected() else {
        return;
    };

    window.scroll_by(delta, buffer_len_lines(buffers, &id));
}

fn create_scroll(delta: isize) -> impl Fn(&mut Editor) -> anyhow::Result<()> {
    move |e: &mut Editor| {
        let pos = e.core.mouse;
        let point = Position::new(pos.column, pos.row);

        let Some((leaf, _)) = e.core.windows.locate(point) else {
            return Ok(());
        };

        scroll_window(e, leaf, delta);

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let mut editor = DefaultState::editor()?;

    // Language store with the path to the queries directory
    editor.core.languages =
        LanguageStore::new(format!("{}/queries", env!("CARGO_MANIFEST_DIR")).into())?;

    editor
        .core
        .languages
        .register(
            "rust",
            tree_sitter::Language::new(tree_sitter_rust::LANGUAGE),
        )?
        .with_extensions(["rs"]);

    let input = &mut editor.core.input;

    {
        let normal = input.tree("normal");

        normal.bind(&["ctrl-d"], "Scroll Down", |e| {
            let leaf = e.core.windows.active_id();
            scroll_window(e, leaf, 40);
            Ok(())
        })?;

        normal.bind(&["ctrl-u"], "Scroll Up", |e| {
            let leaf = e.core.windows.active_id();
            scroll_window(e, leaf, -40);
            Ok(())
        })?;

        normal.bind(&["scroll_up"], "", create_scroll(-3))?;

        normal.bind(&["scroll_down"], "", create_scroll(3))?;

        normal.group(&["space"], "General Actions", |g| {
            g.group(&["f"], "File Pickers", |g| {
                g.bind(&["f"], "Open FZF file picker", |e| {
                    e.suspend(|e| {
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
                            return Ok(());
                        }

                        open_file(e, out.trim().into());

                        Ok(())
                    });

                    Ok(())
                })?;

                g.bind(&["t"], "Open YAZI file picker", |e| {
                    e.suspend(|e| {
                        let _ = std::fs::remove_file("/tmp/chooser");
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

                        for file in lines {
                            let path = PathBuf::from(file).canonicalize()?;
                            if !path.is_file() {
                                continue;
                            }

                            open_file(e, path);
                        }

                        Ok(())
                    });

                    Ok(())
                })?;

                Ok(())
            })
        })?;

        normal.group(&["g"], "Goto Prefix", |g| {
            g.bind_cmd(
                &["p"],
                "Goto Previous Buffer in Window",
                ["buffer dir=prev"],
            )?;
            g.bind_cmd(&["n"], "Goto Next Buffer in Window", ["buffer dir=next"])?;
            Ok(())
        })?;

        normal.group(&[";"], "Shortcut Menu", |g| {
            g.bind_cmd(&["Q"], "Quit the Editor", ["quit"])?;
            g.bind_cmd(&["n"], "Split window right", ["split dir=right"])?;
            g.bind_cmd(&["N"], "Split window down", ["split dir=down"])?;
            g.bind_cmd(
                &["q"],
                "Close active buffer (closes the window if it was the last buffer in the window)",
                ["close_buffer"],
            )?;
            Ok(())
        })?;

        // alt-h is one chord, not a sequence, so it stays flat rather than nested
        normal.bind_cmd(&["alt-h"], "Focus window left", ["focus dir=left"])?;
        normal.bind_cmd(&["alt-l"], "Focus window right", ["focus dir=right"])?;
        normal.bind_cmd(&["alt-k"], "Focus window up", ["focus dir=up"])?;
        normal.bind_cmd(&["alt-j"], "Focus window down", ["focus dir=down"])?;
        normal.bind_cmd(
            &["alt-H"],
            "Move border left",
            ["resize dir=left amount=0.05"],
        )?;
        normal.bind_cmd(
            &["alt-L"],
            "Move border right",
            ["resize dir=right amount=0.05"],
        )?;
        normal.bind_cmd(&["alt-K"], "Move border up", ["resize dir=up amount=0.05"])?;
        normal.bind_cmd(
            &["alt-J"],
            "Move border down",
            ["resize dir=down amount=0.05"],
        )?;

        normal.bind_cmd(&["w"], "Move word", ["motion kind=w"])?;
        normal.bind_cmd(&["W"], "Move WORD", ["motion kind=W"])?;
        normal.bind_cmd(&["e"], "Move word end", ["motion kind=e"])?;
        normal.bind_cmd(&["E"], "Move WORD end", ["motion kind=E"])?;
        normal.bind_cmd(&["b"], "Move word back", ["motion kind=b"])?;
        normal.bind_cmd(&["B"], "Move WORD back", ["motion kind=B"])?;

        normal.bind_cmd(&["alt-w"], "Extend word", ["motion kind=w extend=true"])?;
        normal.bind_cmd(&["alt-W"], "Extend WORD", ["motion kind=W extend=true"])?;
        normal.bind_cmd(&["alt-e"], "Extend word end", ["motion kind=e extend=true"])?;
        normal.bind_cmd(&["alt-E"], "Extend WORD end", ["motion kind=E extend=true"])?;
        normal.bind_cmd(
            &["alt-b"],
            "Extend word back",
            ["motion kind=b extend=true"],
        )?;
        normal.bind_cmd(
            &["alt-B"],
            "Extend WORD back",
            ["motion kind=B extend=true"],
        )?;

        normal.bind_cmd(&["."], "Go to Line End", ["set col=end"])?;
        normal.bind_cmd(&[","], "Go to Line Start", ["set col=0"])?;

        normal.bind_cmd(&[">"], "Extend Line End", ["set col=end extend=true"])?;
        normal.bind_cmd(&["<"], "Extend Line Start", ["set col=0 extend=true"])?;

        normal.bind_cmd(&["u"], "Undo", ["undo"])?;
        normal.bind_cmd(&["U"], "Redo", ["redo"])?;
        normal.bind_cmd(&["c"], "Toggle All-Cursor Mode", ["toggle_all_cursors"])?;

        normal.bind(&["i"], "Enter Insert Mode", |e| {
            e.core.input.set_mode("insert");

            commit_changes(e);

            Ok(())
        })?;

        normal.bind(&["d"], "Delete Selection", |e| {
            let opts = e.core.cursor;

            let Some(mut buf) = active_buffer(e) else {
                return Ok(());
            };

            buf.delete(&opts);

            buf.commit_change();

            Ok(())
        })?;

        normal.bind(&["C"], "Create new cursor below", |e| {
            let Some(mut buf) = active_buffer(e) else {
                return Ok(());
            };

            buf.duplicate();
            buf.move_(1, 0, &text::CursorOptions::new());

            Ok(())
        })?;

        normal.bind(&["m"], "Delete other cursors", |e| {
            let Some(mut buf) = active_buffer(e) else {
                return Ok(());
            };

            buf.remove_other();

            Ok(())
        })?;

        normal.bind_cmd(
            &["o"],
            "Insert newline below",
            [
                "set col=end",
                "move dir=right",
                "insert text=\"\n\"",
                "mode insert",
            ],
        )?;

        normal.bind_cmd(
            &["O"],
            "Insert newline below",
            [
                "set col=0",
                "insert text=\"\n\"",
                "move dir=up",
                "mode insert",
            ],
        )?;

        normal.bind(&["["], "Change Cursor Up", |e| {
            let Some(mut buf) = active_buffer(e) else {
                return Ok(());
            };

            buf.change_primary(-1);

            Ok(())
        })?;

        normal.bind(&["]"], "Change Cursor Down", |e| {
            let Some(mut buf) = active_buffer(e) else {
                return Ok(());
            };

            buf.change_primary(1);

            Ok(())
        })?;

        normal.bind(&[":"], "Command Mode", |e| {
            plugins::command_line::command_mode(e);
            Ok(())
        })?;

        plugins::movement::bind_hjkl(normal, &[], "", CursorOverrides::new().wrap(true), false)?;
        plugins::movement::bind_arrows(normal, &[], "", CursorOverrides::new().wrap(true), false)?;

        plugins::movement::bind_hjkl(
            normal,
            &[],
            "shift",
            CursorOverrides::new().wrap(true).extend(true),
            false,
        )?;
        plugins::movement::bind_arrows(
            normal,
            &[],
            "shift",
            CursorOverrides::new().wrap(true).extend(true),
            false,
        )?;

        normal.bind(&["?"], "Show layer inputs", |e| {
            e.core.flags.set("input_tree_show", true);
            Ok(())
        })?;
    }

    {
        let insert = input.tree("insert");

        plugins::movement::bind_arrows(insert, &[], "", CursorOverrides::new().wrap(true), false)?;
        plugins::movement::bind_arrows(
            insert,
            &[],
            "shift",
            CursorOverrides::new().extend(true).wrap(true),
            false,
        )?;

        insert.bind(&["escape"], "Switch to Normal mode", |e| {
            e.core.input.set_mode("normal");

            commit_changes(e);

            Ok(())
        })?;

        insert.bind_cmd(&["backspace"], "Delete Char", ["delete before=true"])?;

        plugins::insert::bind_insert(insert, Vec::<&str>::new(), CursorOverrides::new())?;
    }

    let id = editor
        .core
        .buffers
        .insert(text::BufferEntry::new_scratch("scratch_0"));
    editor.core.windows.active_mut().select(id);

    editor
        .core
        .pipeline
        .highlighters
        .push(ui::highlighters::TreeSitter)
        .push(ui::highlighters::Cursor);

    editor
        .core
        .pipeline
        .gutters
        .push(ui::gutters::Gap(1))
        .push(ui::gutters::LineNumber {
            relative: true,
            ..Default::default()
        })
        .push(ui::gutters::Gap(1))
        .push(ui::gutters::VerticalBar);

    editor
        .core
        .window_pipeline
        .top
        .push_row(|e| e.push_center(ui::bar::Tabs));

    editor
        .core
        .window_pipeline
        .set_border(ui::borders::LowProfile(BorderType::Thick));

    editor
        .core
        .editor_pipeline
        .bottom
        .push_row(|e| {
            e.push_left(ui::bar::Gap(2))
                .push_left(ui::bar::Mode)
                .push_left(ui::bar::Gap(1))
                .push_left(ui::bar::Spinner::new())
                .push_right(ui::bar::Gap(2))
                .push_right(ui::bar::Clock::new("%I:%M %P"))
        })
        .push_row(|e| {
            e.push_left(ui::bar::Gap(2))
                .push_left(plugins::command_line::StatusLine)
        });

    editor.core.overlays.push(ui::overlays::InputTreePopup);

    theme::init_theme(&mut editor.core.theme);

    kwark::run(editor);

    println!("Goodbye from `kwark`");

    Ok(())
}
