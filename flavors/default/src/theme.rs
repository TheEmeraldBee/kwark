use kwark::{State, prelude::Theme};
use ratatui::style::{Color, Style};

fn f(c: Color) -> Style {
    Style::new().fg(c)
}

pub fn init_theme(state: &mut State) {
    let theme = state.get::<&mut Theme>();

    const ROSEWATER: Color = Color::from_u32(0x00F5E0DC);
    const PINK: Color = Color::from_u32(0x00F5C2E7);
    const MAUVE: Color = Color::from_u32(0x00CBA6F7);
    const RED: Color = Color::from_u32(0x00F38BA8);
    const MAROON: Color = Color::from_u32(0x00EBA0AC);
    const PEACH: Color = Color::from_u32(0x00FAB387);
    const YELLOW: Color = Color::from_u32(0x00F9E2AF);
    const GREEN: Color = Color::from_u32(0x00A6E3A1);
    const TEAL: Color = Color::from_u32(0x0094E2D5);
    const SKY: Color = Color::from_u32(0x0089DCEB);
    const SAPPHIRE: Color = Color::from_u32(0x0074C7EC);
    const BLUE: Color = Color::from_u32(0x0089B4FA);
    const LAVENDER: Color = Color::from_u32(0x00B4BEFE);
    const TEXT: Color = Color::from_u32(0x00CDD6F4);
    const TEXT_DARK: Color = Color::from_u32(0x00333654);
    const OVERLAY2: Color = Color::from_u32(0x009399B2);

    theme.set_all([
        ("ts.tag", f(BLUE)),
        ("ts.attribute", f(YELLOW)),
        ("ts.comment", f(OVERLAY2).italic()),
        ("ts.constant", f(PEACH)),
        ("ts.constant.numeric", f(PEACH)),
        ("ts.constant.builtin", f(PEACH)),
        ("ts.constant.character.escape", f(PINK)),
        ("ts.constructor", f(SAPPHIRE)),
        ("ts.function", f(BLUE)),
        ("ts.function.builtin", f(BLUE)),
        ("ts.function.macro", f(ROSEWATER)),
        ("ts.keyword", f(MAUVE)),
        ("ts.keyword.control", f(MAUVE).italic()),
        ("ts.keyword.control.import", f(MAUVE)),
        ("ts.keyword.directive", f(MAUVE)),
        ("ts.keyword.storage", f(MAUVE)),
        ("ts.label", f(SAPPHIRE)),
        ("ts.namespace", f(YELLOW).italic()),
        ("ts.operator", f(SKY)),
        ("ts.keyword.operator", f(SKY)),
        ("ts.special", f(BLUE)),
        ("ts.string", f(GREEN)),
        ("ts.type", f(YELLOW)),
        ("ts.type.builtin", f(MAUVE)),
        ("ts.variable", f(TEXT)),
        ("ts.variable.builtin", f(RED)),
        ("ts.variable.parameter", f(MAROON).italic()),
        ("ts.variable.other.member", f(BLUE)),
        ("ts.punctuation", f(OVERLAY2)),
        ("ts.punctuation.special", f(SKY)),
    ]);

    theme.set_all([
        ("ts.markup.heading", f(RED)),
        ("ts.markup.raw.inline", f(GREEN)),
        ("ts.markup.bold", f(RED).bold()),
        ("ts.markup.italic", f(RED).italic()),
        ("ts.markup.list", f(TEAL)),
        ("ts.markup.quote", f(PINK)),
        ("ts.markup.link.url", f(BLUE).italic().underlined()),
        ("ts.markup.link.text", f(LAVENDER)),
    ]);

    theme.set_all([
        ("gutter", f(OVERLAY2)),
        ("bar.window.selected", f(GREEN)),
        ("bar.tab", f(TEXT)),
        ("bar.tab.primary", f(MAUVE)),
        ("bar.clock", f(PINK)),
        ("borders.active", f(GREEN)),
        ("borders", f(OVERLAY2)),
        ("mode.normal", f(TEXT_DARK).bg(TEAL)),
        ("mode.insert", f(TEXT_DARK).bg(MAUVE)),
        ("mode.command", f(TEXT_DARK).bg(LAVENDER)),
        ("statusline.command", f(MAUVE)),
        ("statusline.log.error", f(MAROON)),
        ("statusline.log.warn", f(RED)),
        ("statusline.log.info", f(SKY)),
    ]);

    theme.set("freda", f(LAVENDER));
}
