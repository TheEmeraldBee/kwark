use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub use kwark::prelude::*;
use kwark::{
    plugins::{self, cursor_overrides::CursorOverrides},
    prelude::{
        buffer::Storage,
        text_buffer::{BoundCursorSet, BufferEntry, CursorOptions, CursorSet},
        tree_sitter::LanguageStore,
    },
};
use ratatui::{layout::Position, widgets::BorderType};

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
    let mut editor = kwark::init();

    editor.insert(LanguageStore::new("./queries".into())?);

    plugins::visual::init(&mut editor);

    // Each plugin seeds the state its own commands need
    editor.install(plugins::movement::Plugin)?;
    editor.install(plugins::insert::Plugin)?;
    editor.install(plugins::window::Plugin)?;
    editor.install(plugins::command_line::Plugin)?;

    let input = editor.get::<&mut InputState>();

    {
        let normal = input.tree("normal");

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

        normal.group(&["space"], "General Actions", |g| {
            g.group(&["f"], "File Pickers", |g| {
                g.bind(&["f"], "Open FZF file picker", |s| {
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
                        let (layout, storage) =
                            s.get::<(&mut WindowLayout, &mut buffer::Storage)>();

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

                g.bind(&["t"], "Open YAZI file picker", |s| {
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

                Ok(())
            })
        })?;

        normal.group(&["g"], "Goto Prefix", |g| {
            g.bind_cmd(&["p"], "Goto Previous Buffer in Window", "buffer dir=prev")?;
            g.bind_cmd(&["n"], "Goto Next Buffer in Window", "buffer dir=next")?;
            Ok(())
        })?;

        normal.group(&[";"], "Shortcut Menu", |g| {
            g.bind_cmd(&["Q"], "Quit the Editor", "quit")?;
            g.bind_cmd(&["n"], "Split window right", "split dir=right")?;
            g.bind_cmd(&["N"], "Split window down", "split dir=down")?;
            g.bind_cmd(
                &["q"],
                "Close active buffer (closes the window if it was the last buffer in the window)",
                "close_buffer",
            )?;
            Ok(())
        })?;

        // alt-h is one chord, not a sequence, so it stays flat rather than nested
        normal.bind_cmd(&["alt-h"], "Focus window left", "focus dir=left")?;
        normal.bind_cmd(&["alt-l"], "Focus window right", "focus dir=right")?;
        normal.bind_cmd(&["alt-k"], "Focus window up", "focus dir=up")?;
        normal.bind_cmd(&["alt-j"], "Focus window down", "focus dir=down")?;
        normal.bind_cmd(
            &["alt-H"],
            "Move border left",
            "resize dir=left amount=0.05",
        )?;
        normal.bind_cmd(
            &["alt-L"],
            "Move border right",
            "resize dir=right amount=0.05",
        )?;
        normal.bind_cmd(&["alt-K"], "Move border up", "resize dir=up amount=0.05")?;
        normal.bind_cmd(
            &["alt-J"],
            "Move border down",
            "resize dir=down amount=0.05",
        )?;

        normal.bind_cmd(&["u"], "Undo", "undo")?;
        normal.bind_cmd(&["U"], "Redo", "redo")?;
        normal.bind_cmd(&["c"], "Toggle All-Cursor Mode", "toggle_all_cursors")?;

        normal.bind(&["i"], "Enter Insert Mode", |s| {
            s.get::<&mut InputState>().set_mode("insert");

            commit_changes(s);

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

        normal.bind(&[":"], "Command Mode", |s| {
            plugins::command_line::command_mode(s);
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

        normal.bind(&["?"], "Show layer inputs", |s| {
            s.get::<&mut Flags>().set("input_tree_show", true);
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

        insert.bind(&["escape"], "Switch to Normal mode", |s| {
            s.get::<&mut InputState>().set_mode("normal");

            commit_changes(s);

            Ok(())
        })?;

        insert.bind_cmd(&["backspace"], "Delete Char", "delete before=true")?;

        plugins::insert::bind_insert(insert, Vec::<&str>::new(), CursorOverrides::new())?;
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

        pipeline
            .highlighters
            .push(TSHighlighter)
            .push(CursorHighlighter);

        pipeline
            .renderers
            .push(HeaderRenderer::new().block::<CursorTag>());

        pipeline
            .gutters
            .push(pipelines::gutters::Gap(1))
            .push(pipelines::gutters::LineNumber {
                relative: true,
                ..Default::default()
            })
            .push(pipelines::gutters::Gap(1))
            .push(pipelines::gutters::VerticalBar);
    }
    {
        let window_pipeline = editor.get::<&mut WindowPipeline>();

        window_pipeline
            .top
            .push_row(|e| e.push_center(pipelines::bar::Tabs));

        window_pipeline.set_border(pipelines::borders::LowProfile(BorderType::Thick));
    }
    {
        let editor_pipeline = editor.get::<&mut EditorPipeline>();

        editor_pipeline.bottom.push_row(|e| {
            e.push_left(pipelines::bar::Gap(2))
                .push_left(pipelines::bar::Mode)
                .push_right(pipelines::bar::Gap(2))
                .push_right(pipelines::bar::Clock::new("%I:%m %P"))
        });
    }

    theme::init_theme(&mut editor);

    editor.run();

    println!("Goodbye from `kwark`");

    Ok(())
}
