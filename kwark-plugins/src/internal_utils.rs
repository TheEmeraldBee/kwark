use kwark_buffer::Buffer;
use kwark_text::BoundCursorSet;

use kwark_core::prelude::*;

pub(crate) fn active_buffer<S: 'static>(e: &mut Editor<S>) -> Option<BoundCursorSet<'_>> {
    let id = e.core.windows.active().selected()?;
    e.core.buffers.get_mut(&id)?.as_text()
}

pub(crate) fn active_buffer_normal<S: 'static>(e: &mut Editor<S>) -> Option<&mut Buffer> {
    let id = e.core.windows.active().selected()?;
    e.core.buffers.get_mut(&id)
}
