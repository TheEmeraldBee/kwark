use kwark::prelude::*;

/// A basic plugin that prints its state when it's registered
pub struct MyPlugin {
    initial: bool,
}

impl kwark::PluginState for MyPlugin {
    // Put any state type here, this is what you can use wherever
    type State = bool;

    fn init(&self) -> anyhow::Result<Self::State> {
        // must run and return a valid state from the editor
        Ok(self.initial)
    }
}

impl<S: Has<MyPlugin> + 'static> kwark::Plugin<S> for MyPlugin {
    // This method runs once **after** our state is created, this should be used to make the plugin be prepared to work, IE: when events occur
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()> {
        // This is how we access the data from our plugin. Without the `Has<MyPlugin>` above, this line wouldn't compile!
        let state = editor.plugin::<MyPlugin>();

        // For this example, let's just print out our state's value :)
        println!("My Plugin Is Registered with data, and it's initial is set to: {state}!");

        Ok(())
    }
}

#[kwark::state]
struct MyState {
    #[plugin(MyPlugin { initial: true })]
    my_plugin: MyPlugin,
}

fn main() -> anyhow::Result<()> {
    // Initialize the editor, this will also register our plugin!
    let mut editor = kwark::init::<MyState>()?;

    // Bind a quit control
    editor
        .core
        .input
        .tree("normal")
        .bind(&["ctrl-c"], "Exit", |s| {
            s.core.running.quit();
            Ok(())
        })?;

    // Insert a new scratch buffer
    let id = editor
        .core
        .buffers
        .insert(text::BufferEntry::new_scratch("instructions"));

    // Set it to the active buffer on the main window
    editor.core.windows.active_mut().select(id);

    // Insert some text into our buffer
    editor
        .core
        .active_buffer()
        .expect("Just set to something")
        .as_text()
        .expect("Should be text buffer")
        .insert(
            "Press `ctrl-c` to exit :), and you'll see a fun message :)",
            &text::CursorOptions::new(),
        );

    println!(
        "We can also access the info here! The value is {}",
        editor.state.my_plugin
    );

    // Actually run the editor
    kwark::run(editor);

    // We get control back afterwards
    println!("Thank you for using kwark!!!");

    Ok(())
}
