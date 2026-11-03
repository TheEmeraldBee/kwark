use kwark::prelude::*;

fn main() -> anyhow::Result<()> {
  // Initialize the editor so you can configure it.
  let mut editor = kwark::init();

  let input = editor.get::<&mut InputState>();

  {
    // Create a "normal" mode for your editor.
    // This is the default mode you end up in
    let normal = input.tree("normal");

    // Create a binding with the "chord" ctrl-c
    // Set the bind's description to "Quit", and make it so it runs a function that sets the editor's running state to false
    normal.bind(&["ctrl-c"], "Quit", |s| {
      s.get::<&mut Running>().quit();
      Ok(())
    })?;
  }

  // Now that kwark is configured, allow the editor to take control and run
  // Whenever Running ends up being false at the end of a frame, this run command will exit, so we can do any cleanup we want afterwards
  kwark::run(editor);

  Ok(())
}
