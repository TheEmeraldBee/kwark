use kwark_text_buffer::CursorOptions;

use crate::{
    plugins::{cursor_overrides::CursorOverrides, internal_utils::*},
    prelude::*,
};

fn create_move_cursor(
    lines: isize,
    columns: isize,
    options: CursorOverrides,
) -> impl Fn(&mut State) -> anyhow::Result<()> {
    move |s: &mut State| {
        let opts = options.combine(*s.get::<&CursorOptions>());

        let Some(mut buf) = active_buffer(s) else {
            return Ok(());
        };

        buf.move_(lines, columns, &opts);

        Ok(())
    }
}

/// Initializes the editor to work with this module
pub fn init(editor: &mut Editor) -> anyhow::Result<()> {
    editor.insert(CursorOptions::new());

    Ok(())
}

/// Registers the `move`, `change_cursor`, and `toggle_all_cursors` commands, calling [`init`] itself
pub struct Plugin;

impl crate::Plugin for Plugin {
    fn register(&self, editor: &mut Editor) {
        let _ = init(editor);

        let registry = editor.get::<&mut CommandRegistry>();

        registry.register("move", |s, args| {
            let dir = args.str("dir", 0, "right");
            let count = args.int("count", 1, 1);
            let extend = args.bool("extend", 2, false);
            let wrap = args.bool("wrap", 3, false);
            let all = args.bool("all", 4, false);

            let (lines, cols) = match dir {
                "left" | "h" => (0, -1),
                "right" | "l" => (0, 1),
                "up" | "k" => (-1, 0),
                "down" | "j" => (1, 0),
                _ => return Err(anyhow::anyhow!("unknown move direction `{dir}`")),
            };

            let overrides = CursorOverrides::new().extend(extend).wrap(wrap).all(all);
            let opts = overrides.combine(*s.get::<&CursorOptions>());

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.move_(lines * count as isize, cols * count as isize, &opts);

            Ok(())
        });

        registry.register("change_cursor", |s, args| {
            let dist = args.int("dist", 0, 1);

            let Some(mut buf) = active_buffer(s) else {
                return Ok(());
            };

            buf.change_primary(dist as isize);

            Ok(())
        });

        registry.register("toggle_all_cursors", |s, _| {
            let options = s.get::<&mut CursorOptions>();
            options.all = !options.all;
            Ok(())
        });
    }
}

/// Uses [`bind_movement`] to bind the arrow keys to movement
///
/// Use a modifier prefix to apply a modifier string to each of the arrows
/// Put `""` to ignore the modifier prefix
///
/// For more info, see [`bind_movement`]
pub fn bind_arrows(
    tree: &mut InputTree,
    prefix: &[&str],
    modifier_prefix: &str,
    overrides: CursorOverrides,
    describe: bool,
) -> anyhow::Result<()> {
    if modifier_prefix.is_empty() {
        bind(
            tree, prefix, "left", "right", "up", "down", overrides, describe,
        )
    } else {
        bind(
            tree,
            prefix,
            &format!("{}-left", modifier_prefix),
            &format!("{}-right", modifier_prefix),
            &format!("{}-up", modifier_prefix),
            &format!("{}-down", modifier_prefix),
            overrides,
            describe,
        )
    }
}

/// Uses [`bind_movement`] to bind `hjkl` to movement
///
/// Use a modifier prefix to apply a modifier string to each of the arrows
/// Put `""` to ignore the modifier prefix
///
/// For more info, see [`bind_movement`]
pub fn bind_hjkl(
    tree: &mut InputTree,
    prefix: &[&str],
    modifier_prefix: &str,
    overrides: CursorOverrides,
    describe: bool,
) -> anyhow::Result<()> {
    if modifier_prefix.is_empty() {
        bind(tree, prefix, "h", "l", "k", "j", overrides, describe)
    } else {
        bind(
            tree,
            prefix,
            &format!("{}-h", modifier_prefix),
            &format!("{}-l", modifier_prefix),
            &format!("{}-k", modifier_prefix),
            &format!("{}-j", modifier_prefix),
            overrides,
            describe,
        )
    }
}

/// Bind 4 movement keys under a prefix (empty to not have one)
///
/// `overrides` allows you to override specific parts of the globally stored cursor
#[allow(clippy::too_many_arguments)]
pub fn bind(
    tree: &mut InputTree,
    prefix_bind: &[&str],
    left: &str,
    right: &str,
    up: &str,
    down: &str,
    overrides: CursorOverrides,
    describe: bool,
) -> anyhow::Result<()> {
    let left_bind = prefix_bind
        .iter()
        .chain(std::iter::once(&left))
        .collect::<Vec<_>>();
    let right_bind = prefix_bind
        .iter()
        .chain(std::iter::once(&right))
        .collect::<Vec<_>>();
    let up_bind = prefix_bind
        .iter()
        .chain(std::iter::once(&up))
        .collect::<Vec<_>>();
    let down_bind = prefix_bind
        .iter()
        .chain(std::iter::once(&down))
        .collect::<Vec<_>>();

    tree.bind(
        &left_bind,
        match describe {
            true => "Move cursor left",
            false => "",
        },
        create_move_cursor(0, -1, overrides),
    )?;
    tree.bind(
        &right_bind,
        match describe {
            true => "Move cursor right",
            false => "",
        },
        create_move_cursor(0, 1, overrides),
    )?;
    tree.bind(
        &up_bind,
        match describe {
            true => "Move cursor up",
            false => "",
        },
        create_move_cursor(-1, 0, overrides),
    )?;
    tree.bind(
        &down_bind,
        match describe {
            true => "Move cursor down",
            false => "",
        },
        create_move_cursor(1, 0, overrides),
    )?;

    Ok(())
}
