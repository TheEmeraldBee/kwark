# Plugins

## What is a plugin
It is a `rust` struct that implements a trait that can be added to your project, often coming from external sources (hence the name plugin)

## Why support plugins
While the editor I primarily use, `helix`, has been in debate around this for a very long time, I already know what I want kwark to be, as **EXTENSIBLE** as possible,
while still having a good core (much like `nvim`). So, plugins are a must-have, and make for a reasonable, and simple abstraction for people to use to add things.

Plugins are also a great way of allowing me to handle the desire for me to add "built-in" features that many people **but not all** would want.

## How do I create a plugin
Plugins are farily simple to create, but are slighly hard to understand initially, because in order to maintani
strong type safety and discoverability, Kwark utilizes a macro in order to store the state for each plugin

Plugins require us to implement 2 traits:  
1. `kwark::PluginState` - This is the trait that tells the editor what the plugin's state needs to be, as well as
providing a function in order to create it.
2. `kwark::Plugin` - This is the trait that, given a state, tells the editor how we use it.

It makes more sense when looking at the following example:
```rust
{{#include ../../kwark/examples/plugin.rs}}
```

Let's explain some more code:
- The `PluginState` implementation:
Kwark uses this in order to know what data must be stored within your `MyState` type that uses the macro.
We also utilize this in order to have our plugin return a valid initial state in order to create it.
This init method is run very early, while creating `MyState`, and is how we insert all of the important items
for our plugin into the editor.
- The `Plugin` implementation:
Kwark uses this in order to actually do some operation to the `Editor` when it is registered. This is
where we should add things like new commands, event-watchers, and anything else your plugin needs
to modify within the editor in order to function well.
- The `#[kwark::state]` attribute macro:
This attribute macro allows us to modify the `MyState` type to have the correct trait implementations and function
layout that kwark expects in order to run the Editor, without this, we would be unable to run `kwark::init::<MyState>()?`.
  - The `#[plugin(MyPlugin { })]` attribute:
  This attribute forces us to correctly create the plugin type with its info needed to run (just an initializer for the type).
  This can be any code, so if `MyPlugin` had a `new` method, we could use it here instead of the struct definition
