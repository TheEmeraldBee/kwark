static TEXT: &str = r#"
    let id = buffer::open("./Cargo.toml");
    if id == 0 {
        input::bind("normal", ["ctrl-f"], fn() { quit() });
    }

    // Register a mode renderer to the bottom left corner
    // bar::register(-1, -1, fn() { input::get_mode() });

    input::bind("normal", ["ctrl-c"], fn() { quit() });
    input::bind("normal", [";", "Q"], fn() { quit() });

    let cursor_row = 0;
    let cursor_col = 0;

    input::bind("normal", ["k"], fn() {
        cursor_col = 0;
        if cursor_row > 0 {
            cursor_row = cursor_row - 1;
        }
    });
    input::bind("normal", ["j"], fn() {
        cursor_row = cursor_row + 1;
        cursor_col = 0;
    });

    input::bind("normal", ["i"], fn() { input::set_mode("insert") });
    input::bind("insert", ["esc"], fn() { input::set_mode("normal") });

    input::bind("insert", ["space"], fn() {
        buffer::insert(cursor_row, cursor_col, " ");
        cursor_col = cursor_col + 1;
    });

    input::backup("insert", fn(key) {
        if len(key) == 1 {
            buffer::insert(cursor_row, cursor_col, key);
            cursor_col = cursor_col + 1;
        }
    });
"#;

fn main() {
    // Initialize the editor
    let mut editor = kwark::init();

    // Run some default configuration code or something
    match editor.exec(TEXT).map_err(|e| {
        let message = e.value.to_string();
        e.point_at(message, TEXT)
    }) {
        Ok(_) => (),
        Err(e) => panic!("{}", e),
    }

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`")
}
