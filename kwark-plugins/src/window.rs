use ratatui::layout::Direction;

use kwark_core::prelude::*;

use crate::dir::Dir;

/// Registers the `split`, `close_window`, `close_buffer`, `buffer`, `focus`, `resize`, and `mode` commands
#[derive(Default)]
pub struct Plugin;

impl PluginState for Plugin {
    type State = ();

    fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<S: 'static> kwark_core::Plugin<S> for Plugin {
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()> {
        let registry = &mut editor.core.commands;

        registry.register("split", |s, args| {
            let dir: Dir = args.str("dir", 0, "right").parse()?;
            split(s, dir.axis())
        });

        registry.register("close_window", |s, _| close_window(s));

        registry.register("close_buffer", |s, _| close_buffer(s));

        registry.register("buffer", |s, args| {
            let dir = args.str("dir", 0, "next");
            let dist = match dir {
                "next" => 1,
                "prev" | "previous" => -1,
                _ => return Err(anyhow::anyhow!("unknown buffer direction `{dir}`")),
            };

            cycle_buffer(s, dist)
        });

        registry.register("focus", |s, args| {
            focus(s, args.str("dir", 0, "right").parse()?)
        });

        registry.register("resize", |s, args| {
            let dir: Dir = args.str("dir", 0, "right").parse()?;
            resize(s, dir, args.float("amount", 1, 0.05) as f32)
        });

        registry.register("mode", |s, args| {
            let name = args.str("name", 0, "normal").to_string();
            s.core.input.set_mode(name);
            Ok(())
        });

        Ok(())
    }
}

/// Splits the active window along `direction`, opening a new scratch buffer in it
pub fn split<S: 'static>(e: &mut Editor<S>, direction: Direction) -> anyhow::Result<()> {
    let Core {
        windows: layout,
        buffers: storage,
        ..
    } = &mut e.core;
    layout.split(direction);

    const NAME_ROOT: &str = "scratch";
    let mut num = 0;
    while storage
        .get(&buffer::ID::scratch(format!("{NAME_ROOT}_{num}")))
        .is_some()
    {
        num += 1;
    }

    let buf = text::BufferEntry::new_scratch(format!("{NAME_ROOT}_{num}"));

    let id = storage.insert(buf);
    layout.active_mut().select(id);

    Ok(())
}

/// Closes the active window
pub fn close_window<S: 'static>(e: &mut Editor<S>) -> anyhow::Result<()> {
    let windows = &mut e.core.windows;
    let active = windows.active_id();
    windows.close(active);
    Ok(())
}

/// Closes the active buffer, and the window if it has no buffers left
pub fn close_buffer<S: 'static>(e: &mut Editor<S>) -> anyhow::Result<()> {
    let windows = &mut e.core.windows;
    windows.active_mut().close_primary();

    if windows.active_mut().is_empty() {
        let active = windows.active_id();
        windows.close(active);
    }

    Ok(())
}

/// Selects the buffer `dist` places away in the active window
pub fn cycle_buffer<S: 'static>(e: &mut Editor<S>, dist: isize) -> anyhow::Result<()> {
    e.core.windows.active_mut().move_primary(dist);
    Ok(())
}

/// Focuses the window in `dir` from the active window
pub fn focus<S: 'static>(e: &mut Editor<S>, dir: Dir) -> anyhow::Result<()> {
    e.core.windows.focus_direction(dir.axis(), dir.is_forward());
    Ok(())
}

/// Moves the border of the active window in `dir` by `amount`
pub fn resize<S: 'static>(e: &mut Editor<S>, dir: Dir, amount: f32) -> anyhow::Result<()> {
    let delta = if dir.is_forward() { amount } else { -amount };
    e.core.windows.resize(dir.axis(), delta);
    Ok(())
}
