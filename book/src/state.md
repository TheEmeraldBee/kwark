# State
Your `State` struct (the `#[kwark::state]` macro) is the driver for all [plugins](plugins.md) **and** custom
state you may want within your flavor

If you're looking for how to install plugins into your state, see [plugins](plugins.md).

## Inserting custom state
```rust
{{#include ../../kwark/examples/state.rs}}
```

Now, this above example is pretty complicated, but it's to show you what's possible.
However, this explanation is only going to talk about what `#[default(0)]` means,
the other piece can be ignored.

The `#[default(expr)]` format is used when a `State` element doesn't come from a plugin.
Instead, this is used when the flavor itself wants to use a value for something, and not have any
plugins know about it.

## Why use this over a plugin
This is where it gets tricky, as in most use-cases, users should never touch `#[default]`, but as seen
in the example above, this is a case where maybe a plugin doesn't make sense. So in other words,
you should avoid using `#[default]` for things that would make more sense as a plugin, IE: something
that may or may not be reusable, or something that would make more sense to be abstracted out of the state itself
