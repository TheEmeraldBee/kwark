//! A caret, a drag selection, and a popup with a shadow

use kui::{
    App, Color, Cx, Event, MouseButton, Node, Painter, Point, Rect, Size, Span, TextStyle,
    WindowOptions, canvas,
};

const BG: Color = Color::rgb(0x11111B);
const PANEL: Color = Color::rgb(0x1E1E2E);
const BORDER: Color = Color::rgb(0x313244);
const FG: Color = Color::rgb(0xCDD6F4);
const MUTED: Color = Color::rgb(0x7F849C);
const SELECTION: Color = Color::rgba(0x89B4FA40);
const CARET: Color = Color::rgb(0xF5E0DC);
const POPUP: Color = Color::rgb(0x181825);
const HOVER: Color = Color::rgb(0x45475A);
const ACCENT: Color = Color::rgb(0x89B4FA);

const PAD: f32 = 24.0;
const INSET: f32 = 20.0;

const SOURCE: &[&str] = &[
    "fn render(cx: &mut Cx, lines: &[Line]) {",
    "    let height = cx.fonts().line_height(&style);",
    "    for (row, line) in lines.iter().enumerate() {",
    "        let top = row as f32 * height;",
    "        cx.text(line, Point::new(0.0, top));",
    "    }",
    "}",
    "",
    "fn main() {",
    "    kui::run(Editor::new()).unwrap();",
    "}",
];

const ITEMS: &[(&str, &str)] = &[
    ("line_height", "fn(&TextStyle) -> f32"),
    ("line", "fn(&[Span]) -> Arc<TextLayout>"),
    ("char_width", "fn(&TextStyle) -> f32"),
    ("a_name_long_enough_to_need_the_clip_rect", "fn()"),
    ("new", "fn() -> Fonts"),
];

/// A row and a byte index in that row
type Pos = (usize, usize);

struct Demo {
    style: TextStyle,
    anchor: Pos,
    head: Pos,
    dragging: bool,
}

impl Demo {
    fn panel(size: Size) -> Rect {
        Rect::new(PAD, PAD, size.width - PAD * 2.0, size.height - PAD * 2.0)
    }

    fn origin(size: Size) -> Point {
        let panel = Self::panel(size);
        Point::new(panel.left() + INSET, panel.top() + INSET)
    }

    fn span(&self, row: usize) -> Span<'static> {
        Span::new(SOURCE[row], self.style.clone())
    }

    /// Returns the text position under a point
    fn hit(&self, cx: &mut Cx, p: Point) -> Pos {
        let origin = Self::origin(cx.size());
        let height = cx.fonts().line_height(&self.style);

        let row = ((p.y - origin.y) / height).max(0.0) as usize;
        let row = row.min(SOURCE.len() - 1);
        let layout = cx.fonts().line(&[self.span(row)]);

        (row, layout.index_for_x(p.x - origin.x))
    }

    fn popup(&self, p: &mut Painter, at: Point) {
        let style = self.style.clone().size(13.0);
        let height = p.fonts().line_height(&style) + 8.0;
        let rect = Rect::new(at.x, at.y, 300.0, height * ITEMS.len() as f32 + 40.0);

        let shadow = Rect::new(
            rect.left(),
            rect.top() + 6.0,
            rect.size.width,
            rect.size.height,
        );
        p.shadow(shadow, 8.0, 24.0, Color::BLACK.opacity(0.6));
        p.fill_rounded(rect, 8.0, POPUP);

        let item = |i: usize| {
            Rect::new(
                rect.left() + 4.0,
                rect.top() + 4.0 + i as f32 * height,
                rect.size.width - 8.0,
                height,
            )
        };
        let hovered = p
            .pointer()
            .and_then(|ptr| (0..ITEMS.len()).find(|&i| item(i).contains(ptr)));

        p.clip(rect.inset(4.0), |p| {
            for (i, (name, _)) in ITEMS.iter().enumerate() {
                let item = item(i);

                if hovered == Some(i) {
                    p.fill_rounded(item, 5.0, HOVER);
                }

                let name = p.fonts().line(&[Span::new(name, style.clone().color(FG))]);
                p.text(&name, Point::new(item.left() + 8.0, item.top() + 4.0));
            }
        });

        let divider = rect.bottom() - 32.0;
        p.line(
            Point::new(rect.left(), divider),
            Point::new(rect.right(), divider),
            1.0,
            BORDER,
        );

        let detail = ITEMS[hovered.unwrap_or(0)].1;
        let detail = p
            .fonts()
            .line(&[Span::new(detail, style.clone().color(MUTED).italic())]);
        p.text(&detail, Point::new(rect.left() + 12.0, divider + 8.0));

        p.stroke(rect, 8.0, 1.0, BORDER);
    }
}

impl App for Demo {
    type State = Self;
    type Message = ();

    fn state(&mut self) -> &mut Self {
        self
    }

    fn view(&mut self, _cx: &mut Cx) -> Node<Self> {
        canvas(|s: &mut Self, rect, p| s.paint_frame(p, rect.size)).size_full()
    }

    fn window(&self) -> WindowOptions {
        WindowOptions {
            title: "kui paint".into(),
            size: Size::new(900.0, 600.0),
            background: BG,
            ..WindowOptions::default()
        }
    }

    fn event(&mut self, event: Event, cx: &mut Cx) {
        match event {
            Event::MouseDown {
                position,
                button: MouseButton::Left,
            } => {
                self.head = self.hit(cx, position);
                self.anchor = self.head;
                self.dragging = true;
            }
            Event::MouseMove { position } if self.dragging => {
                self.head = self.hit(cx, position);
            }
            Event::MouseUp {
                button: MouseButton::Left,
                ..
            } => {
                self.dragging = false;
            }
            _ => {}
        }

        // The popup hover follows the pointer, so every event redraws
        cx.request_redraw();
    }
}

impl Demo {
    fn paint_frame(&mut self, p: &mut Painter, size: Size) {
        let panel = Self::panel(size);
        let origin = Self::origin(size);
        let height = p.fonts().line_height(&self.style);
        let (start, end) = (self.anchor.min(self.head), self.anchor.max(self.head));

        p.fill_rounded(panel, 10.0, PANEL);

        let mut caret = Point::default();

        p.clip(panel, |p| {
            for row in 0..SOURCE.len() {
                let layout = p.fonts().line(&[self.span(row)]);
                let top = origin.y + row as f32 * height;

                if start != end && (start.0..=end.0).contains(&row) {
                    let from = if row == start.0 { start.1 } else { 0 };
                    let x0 = layout.x_for_index(from);
                    let x1 = if row == end.0 {
                        layout.x_for_index(end.1)
                    } else {
                        layout.width() + 6.0
                    };

                    let rect = Rect::new(origin.x + x0, top, x1 - x0, height);
                    p.fill_rounded(rect, 3.0, SELECTION);
                }

                p.text(&layout, Point::new(origin.x, top));

                if row == self.head.0 {
                    caret = Point::new(origin.x + layout.x_for_index(self.head.1), top);
                    p.fill(Rect::new(caret.x, caret.y, 2.0, height), CARET);
                }
            }
        });

        p.stroke(panel, 10.0, 1.0, BORDER);
        p.line(
            Point::new(caret.x - 4.0, caret.y + height + 2.0),
            Point::new(caret.x + 24.0, caret.y + height + 2.0),
            2.0,
            ACCENT,
        );

        self.popup(p, Point::new(caret.x, caret.y + height + 8.0));
    }
}

fn main() {
    let demo = Demo {
        style: TextStyle::default().size(15.0).color(FG),
        anchor: (1, 8),
        head: (1, 14),
        dragging: false,
    };

    if let Err(e) = kui::run(demo) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
