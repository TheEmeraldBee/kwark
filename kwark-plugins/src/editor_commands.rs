use kwark_buffer::Buffer;

use kwark_core::{
    Editor, PluginState, Senders, commands::Args, frame::FRAME_COUNT_FLAG, log::Severity,
};

use crate::internal_utils::active_buffer_normal;

/// Registers the `quit`, `write`, `wq`, and `toggle_frame_count` commands
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

        registry.register("quit", quit);
        registry.register("q", quit);

        registry.register("write", write);
        registry.register("w", write);

        registry.register("wq", write_quit);
        registry.register("write-quit", write_quit);

        registry.register("toggle_frame_count", |s, _| {
            s.core.flags.toggle(FRAME_COUNT_FLAG);
            Ok(())
        });

        Ok(())
    }
}

pub fn quit<S: 'static>(s: &mut Editor<S>, _args: &Args) -> anyhow::Result<()> {
    s.core.running.quit();

    Ok(())
}

pub fn write<S: 'static>(s: &mut Editor<S>, args: &Args) -> anyhow::Result<()> {
    write_then(s, args, |_| Ok(()))
}

/// Writes the active buffer, then quits
pub fn write_quit<S: 'static>(s: &mut Editor<S>, args: &Args) -> anyhow::Result<()> {
    write_then(s, args, |s| quit(s, &Args::default()))
}

/// Writes the active buffer in the background, then runs `then`
pub fn write_then<S: 'static>(
    s: &mut Editor<S>,
    args: &Args,
    then: impl FnOnce(&mut Editor<S>) -> anyhow::Result<()> + Send + 'static,
) -> anyhow::Result<()> {
    let job = match active_buffer_normal(s) {
        Some(Buffer::Text { buf, .. }) => {
            let filepath = args.get("path", 0).and_then(|x| x.as_str());
            buf.save_job(filepath.map(|x| x.into()))
        }
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
