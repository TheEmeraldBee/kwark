mod simple;
pub use simple::*;

mod low_profile;
pub use low_profile::*;

use kui::Color;
use kwark_core::Editor;
use kwark_window::WindowCtx;

/// Fallback line color for a theme without border styles
const FALLBACK: Color = Color::rgb(0x6C7086);

/// Returns the line color for a window's border from the theme
fn line_color<S: 'static>(state: &Editor<S>, ctx: &WindowCtx) -> Color {
    let name = match ctx.active {
        true => "borders.active",
        false => "borders",
    };

    state.core.theme.get(name).fg.unwrap_or(FALLBACK)
}
