//! Highlighted lines with a click to place the caret

use std::time::{Duration, Instant};

use kui::{
    App, Color, Cx, Event, MouseButton, Painter, Point, Rect, ScrollDelta, Size, Span, TextStyle,
    WindowOptions,
};

const LINES: usize = 200;
const GUTTER: f32 = 56.0;
const HEADER: f32 = 28.0;

const BG: Color = Color::rgb(0x1E1E2E);
const PANEL: Color = Color::rgb(0x181825);
const FG: Color = Color::rgb(0xCDD6F4);
const MUTED: Color = Color::rgb(0x6C7086);
const KEYWORD: Color = Color::rgb(0xCBA6F7);
const STRING: Color = Color::rgb(0xA6E3A1);
const NUMBER: Color = Color::rgb(0xFAB387);
const COMMENT: Color = Color::rgb(0x7F849C);
const TYPE: Color = Color::rgb(0xF9E2AF);
const CARET: Color = Color::rgb(0xF5E0DC);

const SOURCE: &[&str] = &[
    "/// Returns the sum of the first n squares",
    "pub fn squares(n: u64) -> u64 {",
    "    let mut total = 0;",
    "    for i in 1..=n {",
    "        total += i * i; // grows fast",
    "    }",
    "    total",
    "}",
    "",
    "struct Point { x: f32, y: f32 }",
    "",
    "impl Point {",
    "    const ORIGIN: Point = Point { x: 0.0, y: 0.0 };",
    "    fn label(&self) -> String {",
    "        format!(\"({}, {})\", self.x, self.y)",
    "    }",
    "}",
    "",
    "fn main() { println!(\"{}\", squares(42)); }",
    "",
];

const KEYWORDS: &[&str] = &[
    "pub", "fn", "let", "mut", "for", "in", "struct", "impl", "const", "self", "return", "if",
    "else", "match", "use", "mod",
];

/// A kind of token for the highlighter
#[derive(Clone, Copy)]
enum Kind {
    Plain,
    Keyword,
    String,
    Number,
    Comment,
    Type,
}

impl Kind {
    fn style(self, base: &TextStyle) -> TextStyle {
        let style = base.clone();

        match self {
            Kind::Plain => style.color(FG),
            Kind::Keyword => style.color(KEYWORD).bold(),
            Kind::String => style.color(STRING),
            Kind::Number => style.color(NUMBER),
            Kind::Comment => style.color(COMMENT).italic(),
            Kind::Type => style.color(TYPE),
        }
    }
}

/// Splits a line into byte ranges and token kinds
fn highlight(line: &str) -> Vec<(usize, usize, Kind)> {
    let bytes = line.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let start = i;
        let c = bytes[i];

        let kind = if line[i..].starts_with("//") {
            i = bytes.len();
            Kind::Comment
        } else if c == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            i = (i + 1).min(bytes.len());
            Kind::String
        } else if c.is_ascii_digit() {
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            Kind::Number
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }

            let word = &line[start..i];
            if KEYWORDS.contains(&word) {
                Kind::Keyword
            } else if c.is_ascii_uppercase() || matches!(word, "u64" | "f32") {
                Kind::Type
            } else {
                Kind::Plain
            }
        } else {
            i += 1;
            Kind::Plain
        };

        tokens.push((start, i, kind));
    }

    tokens
}

struct Viewer {
    lines: Vec<String>,
    tokens: Vec<Vec<(usize, usize, Kind)>>,
    style: TextStyle,
    /// Pixels scrolled from the top
    scroll: f32,
    /// Line and byte index of the caret
    caret: (usize, usize),
    paint_time: Duration,
}

impl Viewer {
    fn new() -> Self {
        let lines: Vec<String> = (0..LINES)
            .map(|i| SOURCE[i % SOURCE.len()].to_string())
            .collect();
        let tokens = lines.iter().map(|l| highlight(l)).collect();

        Self {
            lines,
            tokens,
            style: TextStyle::default().size(15.0),
            scroll: 0.0,
            caret: (0, 0),
            paint_time: Duration::ZERO,
        }
    }

    fn spans(&self, line: usize) -> Vec<Span<'_>> {
        let text = &self.lines[line];

        self.tokens[line]
            .iter()
            .map(|&(start, end, kind)| Span::new(&text[start..end], kind.style(&self.style)))
            .collect()
    }

    fn max_scroll(&self, line_height: f32, size: Size) -> f32 {
        (LINES as f32 * line_height - (size.height - HEADER)).max(0.0)
    }
}

impl App for Viewer {
    fn window(&self) -> WindowOptions {
        WindowOptions {
            title: "kui text".into(),
            size: Size::new(900.0, 600.0),
            background: BG,
        }
    }

    fn event(&mut self, event: Event, cx: &mut Cx) {
        let line_height = cx.fonts().line_height(&self.style);

        match event {
            Event::MouseDown {
                position,
                button: MouseButton::Left,
            } => {
                if position.y < HEADER {
                    return;
                }

                let row = ((position.y - HEADER + self.scroll) / line_height) as usize;
                let row = row.min(LINES - 1);
                let spans = self.spans(row);
                let layout = cx.fonts().line(&spans);

                self.caret = (row, layout.index_for_x(position.x - GUTTER));
                cx.request_redraw();
            }
            Event::Scroll { delta, .. } => {
                let dy = match delta {
                    ScrollDelta::Lines(p) => p.y * line_height * 3.0,
                    ScrollDelta::Pixels(p) => p.y,
                };

                let max = self.max_scroll(line_height, cx.size());
                self.scroll = (self.scroll - dy).clamp(0.0, max);
                cx.request_redraw();
            }
            _ => {}
        }
    }

    fn paint(&mut self, p: &mut Painter, size: Size) {
        let start = Instant::now();
        let line_height = p.fonts().line_height(&self.style);
        let numbers = self.style.clone().color(MUTED);

        p.fill(Rect::new(0.0, HEADER, GUTTER, size.height - HEADER), PANEL);

        for row in 0..LINES {
            let top = HEADER + row as f32 * line_height - self.scroll;

            let number = (row + 1).to_string();
            let number = p.fonts().line(&[Span::new(&number, numbers.clone())]);
            let x = GUTTER - 12.0 - number.width();
            p.text(&number, Point::new(x, top));

            let spans = self.spans(row);
            let layout = p.fonts().line(&spans);
            p.text(&layout, Point::new(GUTTER, top));

            if row == self.caret.0 {
                let x = GUTTER + layout.x_for_index(self.caret.1);
                p.fill(Rect::new(x, top, 2.0, line_height), CARET);
            }
        }

        let (row, index) = self.caret;
        let column = self.lines[row][..index].chars().count() + 1;
        let status = format!(
            "line {} col {}   paint {:.2} ms",
            row + 1,
            column,
            self.paint_time.as_secs_f64() * 1000.0
        );
        let status = p
            .fonts()
            .line(&[Span::new(&status, self.style.clone().color(FG))]);

        p.fill(Rect::new(0.0, 0.0, size.width, HEADER), PANEL);
        p.text(&status, Point::new(12.0, (HEADER - status.height()) / 2.0));

        self.paint_time = start.elapsed();
    }
}

fn main() {
    if let Err(e) = kui::run(Viewer::new()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
