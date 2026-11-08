use std::time::Duration;

use kwark::prelude::*;
use kwark_plugins::cursor_overrides::CursorOverrides;

// Here, let's create a new state for kwark, and install some plugins to make a functional editor
//
// The editor in this file just uses 2 modes, "normal" for regular insertion, and "command" for running
// individual commands through a palette
#[kwark::state]
struct State {
    /// This plugin makes the mouse work for us, like dragging and double/triple clicking
    #[plugin]
    visual: plugins::visual::Plugin,

    /// This plugin gives us some state so we can have a command palette
    #[plugin]
    command: plugins::command_line::Plugin,

    /// This plugin makes creating insertion keybinds a lot simpler
    #[plugin]
    insert: plugins::insert::Plugin,

    /// This plugin makes it easier to bind buffer-movement
    #[plugin]
    movement: plugins::movement::Plugin,

    /// This plugin registers more commands for the command_line plugin to use
    #[plugin]
    commands: plugins::editor_commands::Plugin,
}

fn main() -> anyhow::Result<()> {
    // Initialize the editor, and make it use our plugin list
    let mut editor = kwark::init::<State>()?;

    // Given that most people like it when the cursor wraps, let's make it **always** wrap
    // The cursor is stored as a global option set, so we can set it easily!
    editor.core.cursor.wrap = true;

    // Let's create some keybindings, starting with our insertion
    let normal = editor.core.input.tree("normal");

    // Use scopes in order to make your keybindings easier to read through
    {
        // Using our insert plugin, let's bind insertion keys
        //
        // Here, we bind it with no prefix, and we'll give them no overrides
        // Overrides allow us to make it so things like insertions apply
        // to all cursors, extend the cursor, etc.
        //
        // Feel free to mess with it here
        plugins::insert::bind_insert(normal, [], CursorOverrides::new())?;

        // Bind a backspace, which will delete text backwards
        normal.bind(&["backspace"], "Backspae", |e| {
            plugins::insert::delete(e, true, CursorOverrides::new().extend(true))
        })?;

        // Bind a delete, which will delete text forwards
        normal.bind(&["delete"], "Delete", |e| {
            plugins::insert::delete(e, false, CursorOverrides::new().extend(true))
        })?;

        // Now let's use the movement plugin to bind some movement!
        //
        // Let's bind the arrow keys to move the cursor, with no overrides
        //
        // Here, we won't describe them so they don't show up in a popup as they'd clutter it
        plugins::movement::bind_arrows(normal, &[], "", CursorOverrides::new(), false)?;

        // Here, let's also bind bind the arrow keys with the **shift** prefix to extend our cursor
        plugins::movement::bind_arrows(
            normal,
            &[],
            "shift",
            CursorOverrides::new().extend(true),
            false,
        )?;
    }

    // This is our misc bindings, for things like the command-palette, quitting, etc.
    {
        // Let's create a bind to enter command mode and see the command palette
        normal.bind(&["ctrl-p"], "Command Palette", |e| {
            plugins::command_line::command_mode(e);
            Ok(())
        })?;

        normal.bind(&["ctrl-w"], "Quit", |e| {
            e.core.running.quit();
            Ok(())
        })?;

        normal.bind(&["ctrl-z"], "Undo", plugins::insert::undo)?;
        normal.bind(&["ctrl-Z"], "Redo", plugins::insert::redo)?;
    }

    // Save out the undo/redo state every 500 ms
    editor.core.timers.every(Duration::from_millis(500), |e| {
        let Some(id) = e.core.windows.active().selected() else {
            return Ok(());
        };

        let Some(mut text_buffer) = e
            .core
            .buffers
            .get_mut(&id)
            .expect("Rendered buffer should exist")
            .as_text()
        else {
            return Ok(());
        };

        text_buffer.commit_change();

        Ok(())
    });

    // Now it's time to set up our render pipeline (what we want to render to the screen)
    //
    // This is very easy, and there are a lot of widgets already available to us, we're just
    // going to use those for now.

    // Start by adding line numbers in the gutters, with a slight gap on the right before the text
    //
    // You'll also see that we can chain methods that push to the gutters :)
    editor
        .core
        .pipeline
        .gutters
        .push(ui::gutters::LineNumber::default()) // We can configure the LineNumber widget to change some stuff about it, mess around!
        .push(ui::gutters::Gap(1)); // Gap exists in most parts of the pipeline, this one just renders empty space in the gutters!

    // Next, let's add some pretty displays for the bottom bar, such as the mode, time, and status line!
    editor
        .core
        .editor_pipeline
        .bottom
        // Each row takes a closure that adds items to the row, so we make a row with the mode and clock
        //
        // Clock takes a format, similarly to the time command on the cli, this format is HH:MM am/pm
        .push_row(|r| {
            r.push_left(ui::bar::Mode)
                .push_right(ui::bar::Clock::new("%I:%M %P"))
        })
        // Here, we take from the command_line plugin and push onto the renderer the StatusLine
        .push_row(|r| r.push_left(plugins::command_line::StatusLine));

    // Next, lets add a tab bar to each window
    editor
        .core
        .window_pipeline
        .top
        .push_row(|r| r.push_center(ui::bar::Tabs));

    // Finally, draw a caret for each cursor, in a shape that follows the input mode
    editor
        .core
        .pipeline
        .carets
        .set(ui::carets::Shaped::default());

    // A highlighter colors text, here the text that each cursor selects
    editor
        .core
        .pipeline
        .highlighters
        .push(ui::highlighters::Selection);

    // Now, in this example, we just create a scratch buffer for users to play with,
    // but you can get creative! Do fun stuff with your editor, integrate it with cli tools,
    // find plugins to do what you want, and have fun!

    // Create and insert a new scratch buffer into the editor
    let id = editor
        .core
        .buffers
        .insert(text::BufferEntry::new_scratch("scratch_buffer"));

    // Make the primary window use the buffer's ID as the one it's looking at
    editor.core.windows.active_mut().select(id);

    // Actually run the editor
    kwark::run(editor);

    // Return a good exit code
    Ok(())
}
