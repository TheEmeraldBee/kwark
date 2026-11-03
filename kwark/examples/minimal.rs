// The state of this editor, with no plugins
kwark::state! {
    pub struct MinimalState {}
}

fn main() -> anyhow::Result<()> {
    // Build the editor and register every plugin in the state
    let mut editor = MinimalState::editor()?;

    // The "normal" mode is the mode the editor starts in
    let normal = editor.core.input.tree("normal");

    normal.bind(&["ctrl-c"], "Quit", |e| {
        e.core.running.quit();
        Ok(())
    })?;

    // Run until a binding quits the editor
    kwark::run(editor);

    Ok(())
}
