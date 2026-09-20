use std::rc::Rc;

use kwark::prelude::text_buffer::{BufferEntry, CursorOptions, CursorSet};
pub use kwark::prelude::*;

mod highlighter;
use highlighter::CursorHighlighter;

fn create_move_cursor(
    lines: isize,
    columns: isize,
    options: CursorOptions,
) -> Rc<dyn Fn(&mut State) -> anyhow::Result<()>> {
    return Rc::new(move |s: &mut State| {
        let bufs = s.get::<&mut buffer::Storage>();

        let Some(buf) = bufs.iter_mut().next() else {
            return Ok(());
        };

        buf.1.move_(lines, columns, &options);

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
    tree.bind(
        &[left],
        "Move Cursor Left",
        create_move_cursor(0, -1, options),
    )?;
    tree.bind(
        &[right],
        "Move Cursor Right",
        create_move_cursor(0, 1, options),
    )?;
    tree.bind(&[up], "Move Cursor Up", create_move_cursor(-1, 0, options))?;
    tree.bind(
        &[down],
        "Move Cursor Down",
        create_move_cursor(1, 0, options),
    )?;

    // tree.desc_multi(
    //     &[&[up], &[down], &[right], &[left]],
    //     format!("{up}|{down}|{left}|{right}"),
    //     "Move Cursor",
    // );

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Initialize the editor
    let mut editor = kwark::init();

    // Retrieve the input state from the editor
    let input = editor.get::<&mut InputState>();

    // Bind a ton of normal-mode keybinds
    {
        let normal = input.tree("normal");

        bind_movement(normal, CursorOptions::default(), "h", "l", "k", "j")?;
        bind_movement(
            normal,
            CursorOptions::default(),
            "left",
            "right",
            "up",
            "down",
        )?;

        normal.desc(&[";"], "Shortcut Menu")?;

        normal.bind(
            &[";", "Q"],
            "quit the editor",
            Rc::new(|s| {
                s.get::<&mut Running>().quit();

                Ok(())
            }),
        )?;

        normal.bind(
            &["ctrl-c"],
            "quit the editor",
            Rc::new(|s| {
                s.get::<&mut Running>().quit();

                Ok(())
            }),
        )?;

        normal.bind(
            &["i"],
            "Enter Insert Mode",
            Rc::new(|s| {
                s.get::<&mut InputState>().set_mode("insert");
                Ok(())
            }),
        )?;

        normal.bind(
            &["?"],
            "Show possible keys",
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
                Ok(())
            }),
        )?;

        insert.bind(
            &["backspace"],
            "Delete Char",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();

                let buf = bufs.iter_mut().next().unwrap().1;

                buf.move_(0, -1, &CursorOptions::new().extend(true).wrap(true));
                buf.delete(&CursorOptions::new());

                Ok(())
            }),
        )?;

        insert.bind(
            &["enter"],
            "Insert Newline",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();

                bufs.iter_mut()
                    .next()
                    .unwrap()
                    .1
                    .insert("\n", &CursorOptions::default());

                Ok(())
            }),
        )?;

        insert.bind(
            &["space"],
            "Insert Space",
            Rc::new(|s| {
                let bufs = s.get::<&mut buffer::Storage>();

                bufs.iter_mut()
                    .next()
                    .unwrap()
                    .1
                    .insert(" ", &text_buffer::CursorOptions::default());

                Ok(())
            }),
        )?;

        insert.set_backup(
            |s, chord| {
                let bufs = s.get::<&mut buffer::Storage>();

                let buf = bufs.iter_mut().next().unwrap().1;

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

                buf.insert(key_string.as_str(), &CursorOptions::default());

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
        .push(CursorHighlighter);

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`");

    Ok(())
}
