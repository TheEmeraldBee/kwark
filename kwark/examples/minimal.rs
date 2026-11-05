use kwark::prelude::*;

// The state of this editor, with no plugins
#[kwark::state]
pub struct MinimalState {}

fn main() -> anyhow::Result<()> {
    // Build the editor and register every plugin in the state
    let mut editor = kwark::init::<MinimalState>()?;

    // The "normal" mode is the mode the editor starts in
    let normal = editor.core.input.tree("normal");

    // Here, we'll create a binding for the key `ctrl-c`,
    // and we'll give it the description "Quit".
    //
    // The closure below takes in the editor, does something, and returns a result in case we want to return an error
    //
    // We must "try" this method, as the string "ctrl-c" is a shorthand to writing a binding, and it can fail if you put bad text.
    //
    // If the binding is invalid, our editor will crash before it even runs, and end up doing nothing :)
    normal.bind(&["ctrl-c"], "Quit", |e| {
        e.core.running.quit();
        Ok(())
    })?;

    // Create a new buffer and insert it into our **global** buffer list
    // This won't show the buffer, we're just telling the editor that the buffer should exist with this info
    let id = editor
        .core
        .buffers
        .insert(text::BufferEntry::new_scratch("instructions"));

    // This line is what tells the **active** split/window to show the given buffer ID in our frame-loop
    editor.core.windows.active_mut().select(id);

    // Let's insert some text into our just created buffer
    //
    // This is a little bit of work, but it's because even though above we guarenteed the buffer exists,
    // all methods in the `core` are built to assume nothing, but because we're sure, let's just use `.expect()` here
    editor
        .core
        .active_buffer()
        .expect("Just set to something")
        .as_text()
        .expect("Should be text buffer")
        .insert("Press `ctrl-c` to exit :)", &text::CursorOptions::new());

    // Run until a binding quits the editor
    kwark::run(editor);

    // We get control back afterwards
    println!("Thank you for using kwark!!!");

    Ok(())
}
