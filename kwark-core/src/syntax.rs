use kwark_buffer as buffer;

use crate::{
    Editor,
    events::{BufferChanged, BufferOpened},
};

/// Registers syntax parsing for buffers
pub(crate) fn register<S: 'static>(editor: &mut Editor<S>) {
    editor
        .events
        .on::<BufferOpened>(|e, ev| start_parse(e, ev.id.clone()));

    // Edits can reveal a language
    editor
        .events
        .on::<BufferChanged>(|e, ev| start_parse(e, ev.id.clone()));
}

fn start_parse<S: 'static>(e: &mut Editor<S>, id: buffer::ID) -> anyhow::Result<()> {
    let core = &mut e.core;

    let Some(buf) = core.buffers.get_mut(&id) else {
        return Ok(());
    };
    let Some(job) = buf.parse_job(&core.languages)? else {
        return Ok(());
    };

    e.spawn_blocking(
        move || job.run(),
        move |e, parsed| {
            let core = &mut e.core;

            if let Some(buf) = core.buffers.get_mut(&id) {
                buf.install(parsed, &mut core.languages)?;
            }

            Ok(())
        },
    );

    Ok(())
}
