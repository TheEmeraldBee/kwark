use kwark_buffer as buffer;
use kwark_core::{
    Editor, State,
    events::{BufferChanged, BufferOpened},
};
use kwark_tree_sitter::store::LanguageStore;

/// Registers syntax parsing for buffers
pub(crate) fn register(editor: &mut Editor) {
    editor
        .events
        .on::<BufferOpened>(|s, e| start_parse(s, e.id.clone()));

    // Edits can reveal a language
    editor
        .events
        .on::<BufferChanged>(|s, e| start_parse(s, e.id.clone()));
}

fn start_parse(s: &mut State, id: buffer::ID) -> anyhow::Result<()> {
    if !s.has::<LanguageStore>() {
        return Ok(());
    }

    let (bufs, store) = s.get::<(&mut buffer::Storage, &LanguageStore)>();

    let Some(buf) = bufs.get_mut(&id) else {
        return Ok(());
    };
    let Some(job) = buf.parse_job(store)? else {
        return Ok(());
    };

    s.spawn_blocking(
        move || job.run(),
        move |s, parsed| {
            let (bufs, store) = s.get::<(&mut buffer::Storage, &mut LanguageStore)>();

            if let Some(buf) = bufs.get_mut(&id) {
                buf.install(parsed, store)?;
            }

            Ok(())
        },
    );

    Ok(())
}
