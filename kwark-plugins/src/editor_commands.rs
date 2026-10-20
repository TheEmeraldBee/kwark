use kwark_buffer::Buffer;

use kwark_core::{
    Running, State,
    commands::{Args, CommandRegistry},
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

        registry.register("wq", |s, a| {
            write(s, a)?;
            quit(s, a)?;

            Ok(())
        });

        registry.register("write-quit", |s, a| {
            write(s, a)?;
            quit(s, a)?;

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
    let Some(buf) = active_buffer_normal(s) else {
        return Ok(());
    };

    let Buffer::Text { buf, .. } = buf else {
        return Ok(());
    };

    let filepath = args.get("path", 0).and_then(|x| x.as_str());

    buf.save(filepath.map(|x| x.into()))?;

    Ok(())
}
