//! Tabs in a flex row that react to hover and click

use kui::{
    App, Color, CursorIcon, Cx, Family, Node, Shadow, Size, TextStyle, WindowOptions, div, rich,
    text,
};

const BG: Color = Color::rgb(0x1E1E2E);
const BAR: Color = Color::rgb(0x181825);
const TAB: Color = Color::rgb(0x313244);
const TAB_HOVER: Color = Color::rgb(0x45475A);
const ACTIVE: Color = Color::rgb(0x89B4FA);
const FG: Color = Color::rgb(0xCDD6F4);
const MUTED: Color = Color::rgb(0x7F849C);
const RED: Color = Color::rgb(0xF38BA8);

struct Tab {
    id: u32,
    name: String,
}

struct Bar {
    tabs: Vec<Tab>,
    active: u32,
    next: u32,
    clicks: u32,
}

impl Bar {
    fn new() -> Self {
        let names = [
            "main.rs",
            "shell.rs",
            "a_file_with_a_name_long_enough_to_truncate.rs",
            "Cargo.toml",
        ];

        Self {
            tabs: names
                .iter()
                .enumerate()
                .map(|(i, name)| Tab {
                    id: i as u32,
                    name: name.to_string(),
                })
                .collect(),
            active: 0,
            next: names.len() as u32,
            clicks: 0,
        }
    }

    fn tab(&self, tab: &Tab) -> Node<Self> {
        let id = tab.id;
        let active = id == self.active;
        let bg = if active { TAB } else { BAR };

        let close = div()
            .px(4.0)
            .rounded(4.0)
            .color(MUTED)
            .hover(|v| v.bg(TAB_HOVER).color(RED))
            .on_click(move |s: &mut Self, cx| {
                s.tabs.retain(|t| t.id != id);
                if s.active == id {
                    s.active = s.tabs.first().map_or(0, |t| t.id);
                }
                cx.request_redraw();
            })
            .child(text("×"));

        div()
            .id(id)
            .flex_row()
            .items_center()
            .gap(6.0)
            .h(30.0)
            .px(10.0)
            .max_w(220.0)
            .rounded(6.0)
            .bg(bg)
            .color(if active { FG } else { MUTED })
            .border_b(2.0, if active { ACTIVE } else { Color::TRANSPARENT })
            .hover(move |v| v.bg(if active { TAB } else { TAB_HOVER }).color(FG))
            .cursor(CursorIcon::Pointer)
            .on_click(move |s: &mut Self, cx| {
                s.active = id;
                s.clicks += 1;
                cx.request_redraw();
            })
            .child(text(tab.name.clone()).shrink(1.0).truncate())
            .child(close)
    }

    fn add_button(&self) -> Node<Self> {
        div()
            .h(30.0)
            .w(30.0)
            .items_center()
            .justify_center()
            .rounded(6.0)
            .color(MUTED)
            .hover(|v| v.bg(TAB_HOVER).color(FG))
            .cursor(CursorIcon::Pointer)
            .on_click(|s: &mut Self, cx| {
                let id = s.next;
                s.next += 1;
                s.tabs.push(Tab {
                    id,
                    name: format!("untitled-{id}"),
                });
                s.active = id;
                cx.request_redraw();
            })
            .child(text("+"))
    }

    fn status(&self) -> Node<Self> {
        let mono = TextStyle::default().size(13.0);
        let name = self
            .tabs
            .iter()
            .find(|t| t.id == self.active)
            .map_or("no tab", |t| t.name.as_str());

        div()
            .flex_row()
            .justify_between()
            .items_center()
            .h(26.0)
            .px(12.0)
            .bg(BAR)
            .border_t(1.0, TAB)
            .child(rich([
                (" NORMAL ".to_string(), mono.clone().color(BAR).bold()),
                ("  ".to_string(), mono.clone()),
                (name.to_string(), mono.clone().color(FG)),
            ]))
            .child(
                text(format!("{} tabs   {} clicks", self.tabs.len(), self.clicks))
                    .size(13.0)
                    .color(MUTED),
            )
    }
}

impl App for Bar {
    type State = Self;
    type Message = ();

    fn window(&self) -> WindowOptions {
        WindowOptions {
            title: "kui bar".into(),
            size: Size::new(900.0, 500.0),
            background: BG,
        }
    }

    fn state(&mut self) -> &mut Self {
        self
    }

    fn view(&mut self, _cx: &mut Cx) -> Node<Self> {
        let tabs = div()
            .flex_row()
            .items_center()
            .gap(4.0)
            .p(6.0)
            .bg(BAR)
            .children(self.tabs.iter().map(|t| self.tab(t)))
            .child(self.add_button());

        let card = div()
            .flex_col()
            .gap(8.0)
            .p(20.0)
            .max_w(420.0)
            .rounded(10.0)
            .bg(BAR)
            .border(1.0, TAB)
            .shadow(Shadow::new(24.0, Color::BLACK.opacity(0.5)).offset(0.0, 8.0))
            .child(text("Element tree").size(18.0).bold().color(FG))
            .child(text("Hover a tab, click to select it, or close it with ×.").color(MUTED))
            .child(
                text("Monospace, italic, underlined")
                    .font(Family::Monospace)
                    .italic()
                    .underline()
                    .color(ACTIVE),
            );

        let body = div().flex_1().items_center().justify_center().child(card);

        div()
            .flex_col()
            .font(Family::SansSerif)
            .size(14.0)
            .child(tabs)
            .child(body)
            .child(self.status())
    }
}

fn main() {
    if let Err(e) = kui::run(Bar::new()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
