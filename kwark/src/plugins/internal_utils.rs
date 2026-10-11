use kwark_text_buffer::BoundCursorSet;

use crate::prelude::*;

pub(crate) fn active_buffer(s: &mut State) -> Option<BoundCursorSet<'_>> {
    let id = s.get::<&WindowLayout>().active().selected()?;
    s.get::<&mut buffer::Storage>().get_mut(&id)?.as_text()
}
