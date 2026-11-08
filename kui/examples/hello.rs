//! A window with a clear color

use kui::{App, Color, Size, WindowOptions};

struct Hello;

impl App for Hello {
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
