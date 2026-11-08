//! A window with a clear color

use kui::{App, Color, Cx, Node, Size, WindowOptions, div};

struct Hello;

impl App for Hello {
    type State = Self;

    fn state(&mut self) -> &mut Self {
        self
    }

    fn view(&mut self, _cx: &mut Cx) -> Node<Self> {
        div()
    }

    fn window(&self) -> WindowOptions {
        WindowOptions {
            title: "kui hello".into(),
            size: Size::new(800.0, 500.0),
            background: Color::rgb(0x1E1E2E),
        }
    }
}

fn main() {
    if let Err(e) = kui::run(Hello) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
