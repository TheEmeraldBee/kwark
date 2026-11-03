use ratatui::layout::Direction;

use kwark_core::prelude::*;

/// Registers the `split`, `close_window`, `close_buffer`, `buffer`, `focus`, `resize`, `mode`, and `quit` commands
#[derive(Default)]
pub struct Plugin;

impl PluginState for Plugin {
    type State = ();

    fn init<S: 'static>(&self, _: &Senders<S>) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<S: 'static> kwark_core::Plugin<S> for Plugin {
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()> {
        let registry = &mut editor.core.commands;

        registry.register("split", |s, args| {
            let dir = args.str("dir", 0, "right");
            let direction = match dir {
                "right" | "left" => Direction::Horizontal,
                "down" | "up" => Direction::Vertical,
                _ => return Err(anyhow::anyhow!("unknown split direction `{dir}`")),
            };

            let Core {
                windows: layout,
                buffers: storage,
                ..
            } = &mut s.core;
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
        });

        registry.register("close_window", |s, _| {
            let windows = &mut s.core.windows;
            let active = windows.active_id();
            windows.close(active);
            Ok(())
        });

        registry.register("close_buffer", |s, _| {
            let windows = &mut s.core.windows;
            windows.active_mut().close_primary();

            if windows.active_mut().is_empty() {
                let active = windows.active_id();
                windows.close(active);
            }

            Ok(())
        });

        registry.register("buffer", |s, args| {
            let dir = args.str("dir", 0, "next");
            let dist = match dir {
                "next" => 1,
                "prev" | "previous" => -1,
                _ => return Err(anyhow::anyhow!("unknown buffer direction `{dir}`")),
            };

            s.core.windows.active_mut().move_primary(dist);

            Ok(())
        });

        registry.register("focus", |s, args| {
            let dir = args.str("dir", 0, "right");
            let (direction, forward) = match dir {
                "left" => (Direction::Horizontal, false),
                "right" => (Direction::Horizontal, true),
                "up" => (Direction::Vertical, false),
                "down" => (Direction::Vertical, true),
                _ => return Err(anyhow::anyhow!("unknown focus direction `{dir}`")),
            };

            s.core.windows.focus_direction(direction, forward);

            Ok(())
        });

        registry.register("resize", |s, args| {
            let dir = args.str("dir", 0, "right");
            let amount = args.float("amount", 1, 0.05) as f32;
            let (direction, delta) = match dir {
                "left" => (Direction::Horizontal, -amount),
                "right" => (Direction::Horizontal, amount),
                "up" => (Direction::Vertical, -amount),
                "down" => (Direction::Vertical, amount),
                _ => return Err(anyhow::anyhow!("unknown resize direction `{dir}`")),
            };

            s.core.windows.resize(direction, delta);

            Ok(())
        });

        registry.register("mode", |s, args| {
            let name = args.str("name", 0, "normal").to_string();
            s.core.input.set_mode(name);
            Ok(())
        });

        Ok(())
    }
}
