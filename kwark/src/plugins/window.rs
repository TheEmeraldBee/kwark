use ratatui::layout::Direction;

use crate::prelude::*;

/// Registers the `split`, `close_window`, `close_buffer`, `buffer`, `focus`, `resize`, `mode`, and `quit` commands
pub struct Plugin;

impl crate::Plugin for Plugin {
    fn register(&self, editor: &mut Editor) {
        let registry = editor.get::<&mut CommandRegistry>();

        registry.register("split", |s, args| {
            let dir = args.str("dir", 0, "right");
            let direction = match dir {
                "right" | "left" => Direction::Horizontal,
                "down" | "up" => Direction::Vertical,
                _ => return Err(anyhow::anyhow!("unknown split direction `{dir}`")),
            };

            let (layout, storage) = s.get::<(&mut WindowLayout, &mut buffer::Storage)>();
            layout.split(direction);

            const NAME_ROOT: &str = "scratch";
            let mut num = 0;
            while storage
                .get(&buffer::ID::scratch(format!("{NAME_ROOT}_{num}")))
                .is_some()
            {
                num += 1;
            }

            let buf = text_buffer::BufferEntry::new_scratch(format!("{NAME_ROOT}_{num}"));
            let id = storage.insert(buf);
            layout.active_mut().select(id);

            Ok(())
        });

        registry.register("close_window", |s, _| {
            let windows = s.get::<&mut WindowLayout>();
            let active = windows.active_id();
            windows.close(active);
            Ok(())
        });

        registry.register("close_buffer", |s, _| {
            let windows = s.get::<&mut WindowLayout>();
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

            s.get::<&mut WindowLayout>().active_mut().move_primary(dist);

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

            s.get::<&mut WindowLayout>()
                .focus_direction(direction, forward);

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

            s.get::<&mut WindowLayout>().resize(direction, delta);

            Ok(())
        });

        registry.register("mode", |s, args| {
            let name = args.str("name", 0, "normal").to_string();
            s.get::<&mut InputState>().set_mode(name);
            Ok(())
        });

        registry.register("quit", |s, _| {
            s.get::<&mut Running>().quit();
            Ok(())
        });
    }
}
