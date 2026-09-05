use std::rc::Rc;

use kwark::prelude::text_buffer::{BufferEntry, CursorOptions, CursorSet};
pub use kwark::prelude::*;

fn main() -> anyhow::Result<()> {
    // Initialize the editor
    let mut editor = kwark::init();

    editor.insert(text_buffer::CursorSet::new());

    // Retrieve the input state from the editor
    let input = editor.get::<&mut InputState>();

    // Bind a ton of normal-mode keybinds
    {
        let normal = input.tree("normal");

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

        insert.bind(
            &["escape"],
            "Switch to Normal mode",
            Rc::new(|s| {
                s.get::<&mut InputState>().set_mode("normal");
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

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`");

    Ok(())
}
