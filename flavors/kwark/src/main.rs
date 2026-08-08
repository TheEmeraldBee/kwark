use std::rc::Rc;

pub use kwark::prelude::*;

fn main() -> anyhow::Result<()> {
    // Initialize the editor
    let mut editor = kwark::init();

    let input = editor.get::<&mut InputState>();
    input.tree("normal").bind(
        &["ctrl-c"],
        "quit the editor",
        Rc::new(|s| {
            s.get::<&mut Running>().quit();

            Ok(())
        }),
    )?;

    editor.get::<&mut BufferList>().file("./Cargo.toml")?;

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`");

    Ok(())
}
