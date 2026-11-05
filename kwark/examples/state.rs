use std::time::Duration;

use kwark::prelude::*;

#[kwark::state]
struct State {
    // Here, we define the **initial** value that is taken when `kwark::init` runs.
    #[default(0)]
    value: i32,
}

fn main() -> anyhow::Result<()> {
    // Most of the code here is similar to that of `minimal.rs`, so we won't be explaining it.
    let mut editor = kwark::init::<State>()?;

    let normal = editor.core.input.tree("normal");

    normal.bind(&["ctrl-c"], "Check Quit", |e| {
        // Here is where some things are different, we are only going to exit the editor **if** we press
        // ctrl-c twice within 0.5 seconds, this way, we don't accidentally do it and forget to save our work
        e.state.value += 1;

        // If we **haven't** reset the state here, and it's 2 (double-press of ctrl-c), then we'll quit the editor
        if e.state.value == 2 {
            e.core.running.quit();

            // The editor is exiting, let's not start a timer by exiting early
            return Ok(());
        }

        // We're going to use this again, so let's create it here
        //
        // This function selects and clears all text in the active buffer, and replacees it with the passed text
        fn set_buffer_text(text: &str, e: &mut Editor<State>) {
            // Let's also update our buffer to show some different text!
            let mut primary_buffer = e
                .core
                .active_buffer()
                .expect("`instructions` buffer should be primary")
                .as_text()
                .expect("active buffer should be a text buffer");

            // Go to the 0, 0 position on the buffer (the start of the file)
            primary_buffer.set(0, 0, &text::CursorOptions::new());

            // Select the entire buffer (to the end of the file)
            primary_buffer.goto(
                Some(text::Target::End), // Select to the last line
                Some(text::Target::End), // Select to the last column
                &text::CursorOptions::new().extend(true),
            );

            // Delete the selection
            primary_buffer.delete(&text::CursorOptions::new());

            // Insert our new text
            primary_buffer.insert(text, &text::CursorOptions::new());
        }

        // We'll set the buffers text here
        set_buffer_text("Press ctrl-c 1 more time to exit", e);

        // Register a timer that, after 0.1 seconds, set the value of our state to 0.
        e.core.timers.after(Duration::from_secs_f32(0.5), |e| {
            e.state.value = 0;

            // Reset the buffer text after a delay
            set_buffer_text("Press ctrl-c 2 times to exit", e);
            Ok(())
        });

        Ok(())
    })?;

    let id = editor
        .core
        .buffers
        .insert(text::BufferEntry::new_scratch("instructions"));

    editor.core.windows.active_mut().select(id);

    // Here, let's initialize the buffer's text
    editor
        .core
        .active_buffer()
        .expect("We should have an active buffer")
        .as_text()
        .expect("Buffer should be text")
        .insert("Press ctrl-c 2 times to exit", &text::CursorOptions::new());

    kwark::run(editor);

    Ok(())
}
