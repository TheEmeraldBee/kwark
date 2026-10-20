// Editor prelude, re-exports the common editor types and crates

pub use kwark_core::prelude::*;

pub use kwark_window::{LeafId, Window, WindowCtx, WindowLayout, WindowPipeline};

pub use kwark_ui as ui;

pub use crate::replace as replace_term;
