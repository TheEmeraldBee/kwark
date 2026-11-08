/*!
The GUI layer that kwark draws with: windows, the event loop, text, layout, and paint
*/

mod app;
pub use app::*;

mod color;
pub use color::*;

mod error;
pub use error::*;

mod event;
pub use event::*;

mod geom;
pub use geom::*;

mod gpu;

mod node;
pub use node::{CursorIcon, Node, Shadow, Visual, canvas, div, rich, text};

mod paint;
pub use paint::Painter;

mod shell;
pub use shell::run;

mod text;
pub use text::{Family, Fonts, Span, TextLayout, TextRendering, TextStyle};
