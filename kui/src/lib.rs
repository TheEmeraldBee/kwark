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
pub use node::{CaretShape, CaretStyle, CursorIcon, Node, Shadow, Visual, canvas, div, rich, text};

mod paint;
pub use paint::Painter;

mod shell;
pub use shell::run;

mod text;
pub use text::{Family, Fonts, Span, TextLayout, TextRendering, TextStyle};

/// Lays out and paints a tree without a window, so that tests can check what paint records
#[doc(hidden)]
pub fn paint_offscreen<S>(
    root: Node<S>,
    size: Size,
    fonts: &Fonts,
    state: &mut S,
) -> Result<(), Error> {
    let mut fonts = fonts.clone();
    let mut taffy = taffy::TaffyTree::new();
    let mut frame = node::Frame::build(root, None, size, &mut fonts, &mut taffy)?;

    let mut scene = vello::Scene::new();
    let mut painter = Painter::new(&mut scene, &mut fonts, 1.0, size, None);
    frame.paint(&mut painter, state);

    Ok(())
}
