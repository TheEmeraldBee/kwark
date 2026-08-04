fn main() {
    // Initialize the editor
    let mut editor = kwark::init();

    // Run some default configuration code or something
    editor
        .exec(
            r#"
            let id = buffer::open("./Cargo.toml");
            if id == 0 {
                input::bind("normal", ["ctrl-f"], fn() { quit() });
            }

            input::bind("normal", ["ctrl-c"], fn() { quit() });
            input::bind("normal", [";", "Q"], fn() { quit() });

            input::backup("normal", fn(key) { if len(key) == 1 { quit() } });
        "#,
        )
        .unwrap();

    // Run the actual editor
    editor.run();

    // Print your goodbyes
    println!("Goodbye from `kwark`")
}
