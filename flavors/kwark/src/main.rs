use std::rc::Rc;

pub use kwark::prelude::*;

fn main() -> anyhow::Result<()> {
    // Initialize the editor
    let mut editor = kwark::init();

    editor.insert(CursorSet::new());

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
                let bufs = s.get::<&mut BufferList>();
                bufs.get(0).unwrap().insert(0, 0, "\n")?;

                Ok(())
            }),
        )?;

        insert.bind(
            &["space"],
            "Insert Space",
            Rc::new(|s| {
                let (bufs, cursor) = s.get::<(&mut BufferList, &mut CursorSet)>();
                let buf = bufs.get(0).unwrap();
                let mut bound = cursor.bind(buf);

                bound.insert(" ", &CursorOptions::default());

                Ok(())
            }),
        )?;

        insert.set_backup(
            Rc::new(|s, chord| {
                let (bufs, cursor) = s.get::<(&mut BufferList, &mut CursorSet)>();
                let buf = bufs.get(0).unwrap();
                let mut bound = cursor.bind(buf);

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

                bound.insert(key_string.as_str(), &CursorOptions::default());

                Ok(())
            }),
            "Any Other Key - Insert Text",
        );
    }

    editor.get::<&mut BufferList>().file("./Cargo.toml")?;

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`");

    Ok(())
}
