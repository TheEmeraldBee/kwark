use kwark_buffer::Buffer;

use kwark_core::{
    Flags, Running, State,
    commands::{Args, CommandRegistry},
    frame::FRAME_COUNT_FLAG,
    log::{LogState, Severity},
};

use crate::internal_utils::active_buffer_normal;

pub struct Plugin;

impl kwark_core::Plugin for Plugin {
    fn register(&self, editor: &mut kwark_core::prelude::Editor) -> anyhow::Result<()> {
        let registry = editor.get::<&mut CommandRegistry>();

        registry.register("quit", quit);
        registry.register("q", quit);

        registry.register("write", write);
        registry.register("w", write);

        registry.register("wq", write_quit);
        registry.register("write-quit", write_quit);

        registry.register("toggle_frame_count", |s, _| {
            s.get::<&mut Flags>().toggle(FRAME_COUNT_FLAG);
            Ok(())
        });

        Ok(())
    }
}

pub fn quit(s: &mut State, _args: &Args) -> anyhow::Result<()> {
    s.get::<&mut Running>().quit();

    Ok(())
}

pub fn write(s: &mut State, args: &Args) -> anyhow::Result<()> {
    write_then(s, args, |_| Ok(()))
}

/// Writes the active buffer, then quits once the write succeeds
pub fn write_quit(s: &mut State, args: &Args) -> anyhow::Result<()> {
    write_then(s, args, |s| quit(s, &Args::default()))
}

/// Writes the active buffer in the background, then runs `then` once the write succeeds
pub fn write_then(
    s: &mut State,
    args: &Args,
    then: impl FnOnce(&mut State) -> anyhow::Result<()> + Send + 'static,
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

            s.get::<&mut LogState>()
                .log(format!("wrote {name}"), Severity::Info);

            then(s)
        },
    );

    Ok(())
}
