# Installation
- Create a new `cargo` project somewhere.

- Add `kwark` as a git dependency
  - Use `cargo add kwark --git https://github.com/TheEmeraldBee/kwark` in order to add the framework as a dependency

- Add `anyhow` as a dependency
  - Use `cargo add anyhow` to add anyhow to your project

## Where to put the project
While there is no required place to put the project, as kwark is just a library you use to build the editor yourself,
I would recommend using `cargo new kwark` in your `~/.config/` directory so that kwark's "configuration" exists in a common place for configurations to exist.

## Project template
Below is a very basic example project that just creates a binding that, on `ctrl-c` being pressed, the editor exits
```rust
{{#include ../../kwark/examples/minimal.rs}}
```
