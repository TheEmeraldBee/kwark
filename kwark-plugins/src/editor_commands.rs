use std::path::PathBuf;

use kwark_buffer::Buffer;

use kwark_core::{Editor, PluginState, commands::Args, frame::FRAME_COUNT_FLAG, log::Severity};

use crate::internal_utils::active_buffer_normal;

/// Registers the `quit`, `write`, `wq`, and `toggle_frame_count` commands
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

        registry.register("quit", |s, _| quit(s));
        registry.register("q", |s, _| quit(s));

        registry.register("write", |s, args| write(s, path_arg(args)));
        registry.register("w", |s, args| write(s, path_arg(args)));

        registry.register("wq", |s, args| write_quit(s, path_arg(args)));
        registry.register("write-quit", |s, args| write_quit(s, path_arg(args)));

        registry.register("toggle_frame_count", |s, _| {
            s.core.flags.toggle(FRAME_COUNT_FLAG);
            Ok(())
        });

        Ok(())
    }
}

fn path_arg(args: &Args) -> Option<PathBuf> {
    args.get("path", 0)
        .and_then(|x| x.as_str())
        .map(PathBuf::from)
}

/// Quits the editor
pub fn quit<S: 'static>(s: &mut Editor<S>) -> anyhow::Result<()> {
    s.core.running.quit();

    Ok(())
}

/// Writes the active buffer to `path`, or to its own file when `None`
pub fn write<S: 'static>(s: &mut Editor<S>, path: Option<PathBuf>) -> anyhow::Result<()> {
    write_then(s, path, |_| Ok(()))
}

/// Writes the active buffer, then quits
pub fn write_quit<S: 'static>(s: &mut Editor<S>, path: Option<PathBuf>) -> anyhow::Result<()> {
    write_then(s, path, quit)
}

/// Writes the active buffer in the background, then runs `then`
pub fn write_then<S: 'static>(
    s: &mut Editor<S>,
    path: Option<PathBuf>,
    then: impl FnOnce(&mut Editor<S>) -> anyhow::Result<()> + Send + 'static,
) -> anyhow::Result<()> {
    let job = match active_buffer_normal(s) {
        Some(Buffer::Text { buf, .. }) => buf.save_job(path),
        _ => return then(s),
    };

    let name = job.name();

    s.spawn_blocking(
        move || job.run(),
        move |s, res| {
            res?;

            s.core.log.log(format!("wrote {name}"), Severity::Info);

            then(s)
        },
    );

    Ok(())
}
