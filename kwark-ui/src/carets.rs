use std::collections::HashMap;

pub use kui::CaretShape;
use kui::{CaretStyle, Color};
use kwark_core::Editor;
use kwark_render::{CaretCtx, CaretStyler};

const PRIMARY_BG: Color = Color::rgb(0xF5E0DC);
const PRIMARY_FG: Color = Color::rgb(0x1E1E2E);
const ALL_BG: Color = Color::rgb(0x89B4FA);
const OTHER_BG: Color = Color::rgb(0x585B70);

/// The look of every caret, with a shape for each input mode
///
/// The theme keys `cursor.primary`, `cursor.secondary`, and `cursor.all` set the colors, with `bg`
/// for the caret and `fg` for the character under a block
pub struct Shaped {
    modes: HashMap<String, CaretShape>,
    /// The shape for modes without their own
    pub fallback: CaretShape,
    /// The shape in windows without focus
    pub inactive: CaretShape,
}

impl Default for Shaped {
    fn default() -> Self {
        Self::new()
            .mode("insert", CaretShape::Bar(2.0))
            .mode("command", CaretShape::Hollow(1.0))
    }
}

impl Shaped {
    /// Returns a painter that draws blocks, with outlines in windows without focus
    pub fn new() -> Self {
        Self {
            modes: HashMap::new(),
            fallback: CaretShape::Block,
            inactive: CaretShape::Hollow(1.0),
        }
    }

    /// Sets the shape for an input mode
    pub fn mode(mut self, mode: impl Into<String>, shape: CaretShape) -> Self {
        self.modes.insert(mode.into(), shape);
        self
    }

    fn shape<S>(&self, state: &Editor<S>, active: bool) -> CaretShape {
        if !active {
            return self.inactive;
        }

        let mode = state.core.input.mode();
        self.modes.get(mode).copied().unwrap_or(self.fallback)
    }
}

impl<S: 'static> CaretStyler<Editor<S>> for Shaped {
    fn style(&self, state: &Editor<S>, ctx: CaretCtx) -> CaretStyle {
        let shape = self.shape(state, ctx.active);
        let theme = &state.core.theme;
        let all = state.core.cursor.is_all();

        let (style, bg) = match (ctx.primary, all) {
            (true, _) => (theme.get("cursor.primary"), PRIMARY_BG),
            (false, true) => (theme.get("cursor.all"), ALL_BG),
            (false, false) => (theme.get("cursor.secondary"), OTHER_BG),
        };

        CaretStyle::new(shape, style.bg.unwrap_or(bg)).text(style.fg.unwrap_or(PRIMARY_FG))
    }
}
