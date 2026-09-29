use std::rc::Rc;

use kwark::prelude::text_buffer::{BufferEntry, CursorOptions, CursorSet};
pub use kwark::prelude::*;

mod highlighter;
use highlighter::CursorHighlighter;

use crate::highlighter::{ConstantHighlighter, HelloWorldRenderer};

fn commit_changes(state: &mut State) {
    let bufs = state.get::<&mut buffer::Storage>();
    let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
        return;
    };

    buf.commit_change();
}

fn create_move_cursor(
    lines: isize,
    columns: isize,
    options: CursorOptions,
) -> Rc<dyn Fn(&mut State) -> anyhow::Result<()>> {
    return Rc::new(move |s: &mut State| {
        let (bufs, opts) = s.get::<(&mut buffer::Storage, &CursorOptions)>();

        let opts = opts.join(options);

        let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
            return Ok(());
        };

        buf.move_(lines, columns, &opts);

        Ok(())
    });
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

    editor.insert(CursorOptions::new());

    // Retrieve the input state from the editor
    let input = editor.get::<&mut InputState>();

    // Bind a ton of normal-mode keybinds
    {
        let normal = input.tree("normal");

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

        normal.desc(&[";"], "Shortcut Menu")?;

        normal.bind(
            &[";", "Q"],
            "Quit the Editor",
            Rc::new(|s| {
                s.get::<&mut Running>().quit();

                Ok(())
            }),
        )?;

        normal.bind(
            &["u"],
            "Undo",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();
                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.undo();

                Ok(())
            }),
        )?;

        normal.bind(
            &["U"],
            "Redo",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();
                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.redo();

                Ok(())
            }),
        )?;

        normal.bind(
            &["i"],
            "Enter Insert Mode",
            Rc::new(|s| {
                s.get::<&mut InputState>().set_mode("insert");

                let bufs = s.get::<&mut buffer::Storage>();
                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };
                buf.commit_change();

                Ok(())
            }),
        )?;

        normal.bind(
            &["c"],
            "Toggle All-Cursor Mode",
            Rc::new(|s| {
                let options = s.get::<&mut CursorOptions>();

                *options = options.all(!options.is_all());

                Ok(())
            }),
        )?;

        normal.bind(
            &["d"],
            "Delete Selection",
            Rc::new(|s| {
                let (bufs, opts) = s.get::<(&mut buffer::Storage, &CursorOptions)>();

                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.move_(0, 1, &opts.extend(true).wrap(true));

                buf.delete(&opts);

                buf.commit_change();

                Ok(())
            }),
        )?;

        normal.bind(
            &["C"],
            "Create new cursor below",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();
                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.duplicate();
                buf.move_(1, 0, &CursorOptions::new());

                Ok(())
            }),
        )?;

        normal.bind(
            &[","],
            "Delete other cursors",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();
                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.remove_other();

                Ok(())
            }),
        )?;

        normal.bind(
            &["["],
            "Change Cursor Up",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();
                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.change_primary(-1);

                Ok(())
            }),
        )?;

        normal.bind(
            &["]"],
            "Change Cursor Down",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();
                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.change_primary(1);

                Ok(())
            }),
        )?;

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

        normal.bind(
            &["?"],
            "Show layer inputs",
            Rc::new(|s| {
                s.get::<&mut Flags>().set("input_tree_show", true);
                Ok(())
            }),
        )?;
    }

    // Bind a bunch of insert mode keybinds
    {
        let insert = input.tree("insert");

        bind_movement(
            insert,
            CursorOptions::default(),
            "left",
            "right",
            "up",
            "down",
        )?;

        insert.bind(
            &["escape"],
            "Switch to Normal mode",
            Rc::new(|s| {
                s.get::<&mut InputState>().set_mode("normal");

                commit_changes(s);

                Ok(())
            }),
        )?;

        insert.bind(
            &["backspace"],
            "Delete Char",
            Rc::new(|s| {
                let (bufs, opts) = s.get::<(&mut buffer::Storage, &CursorOptions)>();

                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.move_(0, -1, &opts.wrap(true).extend(true));
                buf.delete(&opts);

                Ok(())
            }),
        )?;

        insert.bind(
            &["enter"],
            "Insert Newline",
            Rc::new(|s| {
                let (bufs, opts) = s.get::<(&mut buffer::Storage, &CursorOptions)>();

                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.insert("\n", opts);

                Ok(())
            }),
        )?;

        insert.bind(
            &["space"],
            "Insert Space",
            Rc::new(|s| {
                let (bufs, opts) = s.get::<(&mut buffer::Storage, &CursorOptions)>();

                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
                    return Ok(());
                };

                buf.insert(" ", &opts);

                Ok(())
            }),
        )?;

        insert.set_backup(
            |s, chord| {
                let (bufs, opts) = s.get::<(&mut buffer::Storage, &CursorOptions)>();

                let Some(mut buf) = bufs.iter_mut().next().and_then(|x| x.1.as_text()) else {
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

    editor
        .get::<&mut buffer::Storage>()
        .insert(buffer::Buffer::Text {
            path: "./Cargo.toml".into(),
            buf: BufferEntry::new_file("./Cargo.toml".into())?,
            cursors: CursorSet::default(),
        });

    editor
        .get::<&mut Pipeline>()
        .highlighters
        .push(ConstantHighlighter);

    editor
        .get::<&mut Pipeline>()
        .highlighters
        .push(CursorHighlighter);

    editor
        .get::<&mut Pipeline>()
        .renderers
        .push(HelloWorldRenderer);

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`");

    Ok(())
}
