use kwark_buffer::Buffer;
use kwark_text::BoundCursorSet;

use kwark_core::prelude::*;
use kwark_window::WindowLayout;

pub(crate) fn active_buffer(s: &mut State) -> Option<BoundCursorSet<'_>> {
    let id = s.get::<&WindowLayout>().active().selected()?;
    s.get::<&mut buffer::Storage>().get_mut(&id)?.as_text()
}

pub(crate) fn active_buffer_normal(s: &mut State) -> Option<&mut Buffer> {
    let id = s.get::<&WindowLayout>().active().selected()?;
    s.get::<&mut buffer::Storage>().get_mut(&id)
}
