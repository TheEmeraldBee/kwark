use kwark::prelude::{Color, Style, Theme};

fn f(c: Color) -> Style {
    Style::new().fg(c)
}

pub fn init_theme(theme: &mut Theme) {
    const ROSEWATER: Color = Color::rgb(0xF5E0DC);
    const PINK: Color = Color::rgb(0xF5C2E7);
    const MAUVE: Color = Color::rgb(0xCBA6F7);
    const RED: Color = Color::rgb(0xF38BA8);
    const MAROON: Color = Color::rgb(0xEBA0AC);
    const PEACH: Color = Color::rgb(0xFAB387);
    const YELLOW: Color = Color::rgb(0xF9E2AF);
    const GREEN: Color = Color::rgb(0xA6E3A1);
    const TEAL: Color = Color::rgb(0x94E2D5);
    const SKY: Color = Color::rgb(0x89DCEB);
    const SAPPHIRE: Color = Color::rgb(0x74C7EC);
    const BLUE: Color = Color::rgb(0x89B4FA);
    const LAVENDER: Color = Color::rgb(0xB4BEFE);
    const TEXT: Color = Color::rgb(0xCDD6F4);
    const TEXT_DARK: Color = Color::rgb(0x333654);
    const OVERLAY2: Color = Color::rgb(0x9399B2);
    const MANTLE: Color = Color::rgb(0x181825);

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
        ("bar.tab.hovered", f(MAROON)),
        ("bar.clock", f(PINK)),
        ("bar.frames", f(OVERLAY2)),
        ("bar.spinner", f(MAUVE)),
        ("borders.active", f(GREEN)),
        ("borders", f(OVERLAY2)),
        ("mode.normal", f(TEXT_DARK).bg(TEAL)),
        ("mode.insert", f(TEXT_DARK).bg(MAUVE)),
        ("mode.command", f(TEXT_DARK).bg(LAVENDER)),
        ("statusline.cursor", f(TEXT)),
        ("statusline.command", f(MAUVE)),
        ("statusline.log.error", f(MAROON)),
        ("statusline.log.warn", f(RED)),
        ("statusline.log.info", f(SKY)),
    ]);

    theme.set_all([
        ("text", f(TEXT)),
        ("popup", f(TEXT).bg(MANTLE)),
        ("popup.border", f(OVERLAY2)),
    ]);
}
