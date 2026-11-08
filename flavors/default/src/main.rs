use kwark::prelude::*;
use kwark::{
    plugins::{
        self,
        cursor_overrides::CursorOverrides,
        dir::Dir,
        editor_commands, insert,
        movement::{self, Motion},
        window,
    },
    prelude::{text::BoundCursorSet, tree_sitter::LanguageStore},
};

mod theme;

#[kwark::state]
pub struct DefaultState {
    #[plugin]
    visual: plugins::visual::Plugin,
    #[plugin]
    editor_commands: plugins::editor_commands::Plugin,
    #[plugin]
    movement: plugins::movement::Plugin,
    #[plugin]
    insert: plugins::insert::Plugin,
    #[plugin]
    window: plugins::window::Plugin,
    #[plugin]
    command_line: plugins::command_line::Plugin,
    #[plugin(plugins::smooth_scroll::Plugin::new(40.0))]
    smooth_scroll: plugins::smooth_scroll::Plugin,
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
        let Some((leaf, _)) = e.core.windows.locate(e.core.mouse) else {
            return Ok(());
        };

        scroll_window(e, leaf, delta);

        Ok(())
    }
}

fn create_motion(kind: Motion, extend: bool) -> impl Fn(&mut Editor) -> anyhow::Result<()> {
    move |e: &mut Editor| {
        let overrides = CursorOverrides::new().extend(extend).wrap(true);
        movement::motion(e, kind, 1, overrides)
    }
}

fn main() -> anyhow::Result<()> {
    let mut editor = kwark::init::<DefaultState>()?;

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

        normal.bind(&["scroll_up"], "", create_scroll(-5))?;

        normal.bind(&["scroll_down"], "", create_scroll(5))?;

        normal.group(&["g"], "Goto Prefix", |g| {
            g.bind(&["p"], "Goto Previous Buffer in Window", |e| {
                window::cycle_buffer(e, -1)
            })?;
            g.bind(&["n"], "Goto Next Buffer in Window", |e| {
                window::cycle_buffer(e, 1)
            })?;
            Ok(())
        })?;

        normal.group(&[";"], "Shortcut Menu", |g| {
            g.bind(&["Q"], "Quit the Editor", editor_commands::quit)?;
            g.bind(&["n"], "Split window right", |e| {
                window::split(e, Direction::Horizontal)
            })?;
            g.bind(&["N"], "Split window down", |e| {
                window::split(e, Direction::Vertical)
            })?;
            g.bind(
                &["q"],
                "Close active buffer (closes the window if it was the last buffer in the window)",
                window::close_buffer,
            )?;
            Ok(())
        })?;

        normal.bind(&["alt-h"], "Focus window left", |e| {
            window::focus(e, Dir::Left)
        })?;
        normal.bind(&["alt-l"], "Focus window right", |e| {
            window::focus(e, Dir::Right)
        })?;
        normal.bind(&["alt-k"], "Focus window up", |e| window::focus(e, Dir::Up))?;
        normal.bind(&["alt-j"], "Focus window down", |e| {
            window::focus(e, Dir::Down)
        })?;
        normal.bind(&["alt-H"], "Move border left", |e| {
            window::resize(e, Dir::Left, 0.05)
        })?;
        normal.bind(&["alt-L"], "Move border right", |e| {
            window::resize(e, Dir::Right, 0.05)
        })?;
        normal.bind(&["alt-K"], "Move border up", |e| {
            window::resize(e, Dir::Up, 0.05)
        })?;
        normal.bind(&["alt-J"], "Move border down", |e| {
            window::resize(e, Dir::Down, 0.05)
        })?;

        normal.bind(&["w"], "Move word", create_motion(Motion::WordStart, false))?;
        normal.bind(
            &["W"],
            "Move WORD",
            create_motion(Motion::BigWordStart, false),
        )?;
        normal.bind(
            &["e"],
            "Move word end",
            create_motion(Motion::WordEnd, false),
        )?;
        normal.bind(
            &["E"],
            "Move WORD end",
            create_motion(Motion::BigWordEnd, false),
        )?;
        normal.bind(
            &["b"],
            "Move word back",
            create_motion(Motion::WordBack, false),
        )?;
        normal.bind(
            &["B"],
            "Move WORD back",
            create_motion(Motion::BigWordBack, false),
        )?;

        normal.bind(
            &["alt-w"],
            "Extend word",
            create_motion(Motion::WordStart, true),
        )?;
        normal.bind(
            &["alt-W"],
            "Extend WORD",
            create_motion(Motion::BigWordStart, true),
        )?;
        normal.bind(
            &["alt-e"],
            "Extend word end",
            create_motion(Motion::WordEnd, true),
        )?;
        normal.bind(
            &["alt-E"],
            "Extend WORD end",
            create_motion(Motion::BigWordEnd, true),
        )?;
        normal.bind(
            &["alt-b"],
            "Extend word back",
            create_motion(Motion::WordBack, true),
        )?;
        normal.bind(
            &["alt-B"],
            "Extend WORD back",
            create_motion(Motion::BigWordBack, true),
        )?;

        normal.bind(&["."], "Go to Line End", |e| {
            movement::goto(e, None, Some(text::Target::End), CursorOverrides::new())
        })?;
        normal.bind(&[","], "Go to Line Start", |e| {
            movement::goto(e, None, Some(text::Target::At(0)), CursorOverrides::new())
        })?;

        normal.bind(&[">"], "Extend Line End", |e| {
            let overrides = CursorOverrides::new().extend(true);
            movement::goto(e, None, Some(text::Target::End), overrides)
        })?;
        normal.bind(&["<"], "Extend Line Start", |e| {
            let overrides = CursorOverrides::new().extend(true);
            movement::goto(e, None, Some(text::Target::At(0)), overrides)
        })?;

        normal.bind(&["u"], "Undo", insert::undo)?;
        normal.bind(&["U"], "Redo", insert::redo)?;
        normal.bind(
            &["c"],
            "Toggle All-Cursor Mode",
            movement::toggle_all_cursors,
        )?;

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

        normal.bind(&["o"], "Insert newline below", |e| {
            let plain = CursorOverrides::new().extend(false).wrap(false);
            movement::goto(e, None, Some(text::Target::End), CursorOverrides::new())?;
            movement::move_cursor(e, 0, 1, plain)?;
            insert::insert(e, "\n", CursorOverrides::new())?;
            e.core.input.set_mode("insert");
            Ok(())
        })?;

        normal.bind(&["O"], "Insert newline above", |e| {
            let plain = CursorOverrides::new().extend(false).wrap(false);
            movement::goto(e, None, Some(text::Target::At(0)), CursorOverrides::new())?;
            insert::insert(e, "\n", CursorOverrides::new())?;
            movement::move_cursor(e, -1, 0, plain)?;
            e.core.input.set_mode("insert");
            Ok(())
        })?;

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

        insert.bind(&["backspace"], "Delete Char", |e| {
            insert::delete(e, true, CursorOverrides::new().extend(true))
        })?;

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
        .set_border(ui::borders::LowProfile(2.0));

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

    editor.core.window_config = WindowOptions {
        title: "kwark".into(),
        size: Size::new(1200.0, 800.0),
        background: Color::rgba(0x1E1E2EDD),
        decorations: Decorations::None,
        resizable: true,
        transparent: true,
        blur: true,
    };

    let metrics = &mut editor.core.pipeline.metrics;

    *metrics =
        metrics.with_text(TextStyle::default().family(Family::Named("Hack Nerd Font Mono".into())));

    metrics.fonts.set_hinting(true);
    metrics.fonts.set_embolden(0.02);

    kwark::run(editor);

    println!("Goodbye from `kwark`");

    Ok(())
}
