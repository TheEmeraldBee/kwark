use std::str::FromStr;

use kwark_core::prelude::*;

use crate::{cursor_overrides::CursorOverrides, dir::Dir, internal_utils::*};

/// Moves the cursor by the given lines and columns
pub fn move_cursor<S: 'static>(
    e: &mut Editor<S>,
    lines: isize,
    columns: isize,
    overrides: CursorOverrides,
) -> anyhow::Result<()> {
    let opts = overrides.combine(e.core.cursor);

    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.move_(lines, columns, &opts);

    Ok(())
}

#[derive(PartialEq, Eq)]
enum CharClass {
    Blank,
    Eol,
    Word,
    Punct,
}

/// A helix-style motion
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Motion {
    /// The start of the next word (`w`)
    WordStart,

    /// The start of the next WORD (`W`)
    BigWordStart,

    /// The end of the next word (`e`)
    WordEnd,

    /// The end of the next WORD (`E`)
    BigWordEnd,

    /// The start of the previous word (`b`)
    WordBack,

    /// The start of the previous WORD (`B`)
    BigWordBack,
}

impl Motion {
    fn is_big(self) -> bool {
        matches!(
            self,
            Motion::BigWordStart | Motion::BigWordEnd | Motion::BigWordBack
        )
    }

    fn is_back(self) -> bool {
        matches!(self, Motion::WordBack | Motion::BigWordBack)
    }

    /// Returns whether the motion stops between `prev` and `next`
    fn reached(self, prev: char, next: char) -> bool {
        let big = self.is_big();

        if char_class(prev, big) == char_class(next, big) {
            return false;
        }

        match self {
            Motion::WordStart | Motion::BigWordStart => is_eol(next) || !next.is_whitespace(),
            _ => is_eol(next) || !prev.is_whitespace(),
        }
    }
}

impl FromStr for Motion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "word_start" | "w" => Ok(Motion::WordStart),
            "big_word_start" | "W" => Ok(Motion::BigWordStart),
            "word_end" | "e" => Ok(Motion::WordEnd),
            "big_word_end" | "E" => Ok(Motion::BigWordEnd),
            "word_back" | "b" => Ok(Motion::WordBack),
            "big_word_back" | "B" => Ok(Motion::BigWordBack),
            _ => Err(anyhow::anyhow!("unknown motion `{s}`")),
        }
    }
}

/// Returns the unit for `kind` repeated `count` times, for use with [`select_to`]
pub fn motion_unit(
    kind: Motion,
    count: usize,
) -> impl Fn(usize, usize, &text::Rope) -> (usize, usize) {
    move |anchor, caret, rope| {
        let len = rope.len_chars();

        // A selection covers both ends, so its range ends past the last char
        let start = if anchor <= caret {
            (anchor, (caret + 1).min(len))
        } else {
            ((anchor + 1).min(len), caret)
        };

        let (new_anchor, head) = word_move(rope, start, count.max(1), kind);

        if (new_anchor, head) == start {
            (anchor, caret)
        } else if new_anchor < head {
            (new_anchor, head - 1)
        } else if new_anchor > head {
            (new_anchor - 1, head)
        } else {
            (head, head)
        }
    }
}

fn is_eol(c: char) -> bool {
    matches!(c, '\n' | '\r')
}

/// Returns the class of `c`, where a WORD (`big`) treats all non-blanks alike
fn char_class(c: char, big: bool) -> CharClass {
    if is_eol(c) {
        CharClass::Eol
    } else if c.is_whitespace() {
        CharClass::Blank
    } else if big || c.is_alphanumeric() || c == '_' {
        CharClass::Word
    } else {
        CharClass::Punct
    }
}

/// Returns the (anchor, head) range after moving `count` times
fn word_move(
    rope: &text::Rope,
    (anchor, head): (usize, usize),
    count: usize,
    kind: Motion,
) -> (usize, usize) {
    let len = rope.len_chars();
    let back = kind.is_back();

    if (back && head == 0) || (!back && head == len) {
        return (anchor, head);
    }

    // Start from the char under the cursor, facing the motion
    let mut range = match (back, anchor < head) {
        (true, true) => (head, head - 1),
        (true, false) => ((head + 1).min(len), head),
        (false, true) => (head - 1, head),
        (false, false) => (head, (head + 1).min(len)),
    };

    for _ in 0..count {
        let next = range_to_target(rope, range, kind);

        if next == range {
            break;
        }

        range = next;
    }

    range
}

/// Returns the (anchor, head) range after moving once
fn range_to_target(
    rope: &text::Rope,
    (mut anchor, mut head): (usize, usize),
    kind: Motion,
) -> (usize, usize) {
    let back = kind.is_back();

    let behind = |head: usize| match back {
        true => rope.get_char(head),
        false => head.checked_sub(1).map(|i| rope.char(i)),
    };
    let ahead = |head: usize| match back {
        true => head.checked_sub(1).map(|i| rope.char(i)),
        false => rope.get_char(head),
    };
    let step = |head: &mut usize| match back {
        true => *head -= 1,
        false => *head += 1,
    };

    let mut prev = behind(head);

    while let Some(c) = ahead(head).filter(|&c| is_eol(c)) {
        prev = Some(c);
        step(&mut head);
    }

    if prev.is_some_and(is_eol) {
        anchor = head;
    }

    let head_start = head;

    while let Some(next) = ahead(head) {
        if prev.is_none_or(|prev| kind.reached(prev, next)) {
            if head != head_start {
                break;
            }

            anchor = head;
        }

        prev = Some(next);
        step(&mut head);
    }

    (anchor, head)
}

/// Selects to the anchor and caret returned by `unit`
pub fn select_to<S: 'static>(
    e: &mut Editor<S>,
    unit: impl Fn(usize, usize, &text::Rope) -> (usize, usize),
    overrides: CursorOverrides,
) -> anyhow::Result<()> {
    let opts = overrides.combine(e.core.cursor);

    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.select_to(&opts, &unit);

    Ok(())
}

/// Selects to the end of `kind` repeated `count` times
pub fn motion<S: 'static>(
    e: &mut Editor<S>,
    kind: Motion,
    count: usize,
    overrides: CursorOverrides,
) -> anyhow::Result<()> {
    select_to(e, motion_unit(kind, count), overrides)
}

/// Moves the cursor to the given line and column, leaving `None` targets unchanged
pub fn goto<S: 'static>(
    e: &mut Editor<S>,
    line: Option<text::Target>,
    col: Option<text::Target>,
    overrides: CursorOverrides,
) -> anyhow::Result<()> {
    let opts = overrides.combine(e.core.cursor);

    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.goto(line, col, &opts);

    Ok(())
}

/// Moves the primary cursor by `dist` cursors
pub fn change_cursor<S: 'static>(e: &mut Editor<S>, dist: isize) -> anyhow::Result<()> {
    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.change_primary(dist);

    Ok(())
}

/// Toggles whether actions apply to all cursors
pub fn toggle_all_cursors<S: 'static>(e: &mut Editor<S>) -> anyhow::Result<()> {
    let options = &mut e.core.cursor;
    options.all = !options.all;
    Ok(())
}

/// Clears the selection of the cursors
pub fn clear_selection<S: 'static>(
    e: &mut Editor<S>,
    overrides: CursorOverrides,
) -> anyhow::Result<()> {
    let opts = overrides.combine(e.core.cursor);

    let Some(mut buf) = active_buffer(e) else {
        return Ok(());
    };

    buf.clear_selection(&opts);

    Ok(())
}

/// Returns the line or column target named `name`, or `None` if it is missing
fn target_arg(args: &Args, name: &str, pos: usize) -> anyhow::Result<Option<text::Target>> {
    let Some(value) = args.get(name, pos) else {
        return Ok(None);
    };

    if value.as_str() == Some("end") {
        return Ok(Some(text::Target::End));
    }

    match value.as_int() {
        Some(i) if i >= 0 => Ok(Some(text::Target::At(i as usize))),
        _ => Err(anyhow::anyhow!("invalid {name} `{value:?}`")),
    }
}

/// Registers the `move`, `motion`, `set`, `change_cursor`, `toggle_all_cursors`, and `clear_selection` commands
#[derive(Default)]
pub struct Plugin;

impl PluginState for Plugin {
    type State = ();

    fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<S: 'static> kwark_core::Plugin<S> for Plugin {
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()> {
        let registry = &mut editor.core.commands;

        registry.register("move", |s, args| {
            let dir: Dir = args.str("dir", 0, "right").parse()?;
            let count = args.int("count", 1, 1) as isize;
            let overrides = CursorOverrides::new()
                .extend(args.bool("extend", 2, false))
                .wrap(args.bool("wrap", 3, false))
                .all(args.bool("all", 4, false));

            let (lines, cols) = dir.offset();
            move_cursor(s, lines * count, cols * count, overrides)
        });

        registry.register("motion", |s, args| {
            let kind: Motion = args.str("kind", 0, "w").parse()?;
            let count = args.int("count", 1, 1).max(1) as usize;
            let overrides = CursorOverrides::new()
                .extend(args.bool("extend", 2, false))
                .wrap(args.bool("wrap", 3, true))
                .all(args.bool("all", 4, false));

            motion(s, kind, count, overrides)
        });

        registry.register("change_cursor", |s, args| {
            change_cursor(s, args.int("dist", 0, 1) as isize)
        });

        registry.register("toggle_all_cursors", |s, _| toggle_all_cursors(s));

        registry.register("clear_selection", |s, args| {
            let overrides = CursorOverrides {
                all: args.get("all", 0).and_then(ArgValue::as_bool),
                ..CursorOverrides::new()
            };

            clear_selection(s, overrides)
        });

        registry.register("set", |s, args| {
            let line = target_arg(args, "line", 0)?;
            let col = target_arg(args, "col", 1)?;
            let overrides = CursorOverrides {
                extend: args.get("extend", 2).and_then(ArgValue::as_bool),
                all: args.get("all", 3).and_then(ArgValue::as_bool),
                ..CursorOverrides::new()
            };

            goto(s, line, col, overrides)
        });

        Ok(())
    }
}

/// Uses [`bind_movement`] to bind the arrow keys to movement
///
/// Use a modifier prefix to apply a modifier string to each of the arrows
/// Put `""` to ignore the modifier prefix
///
/// For more info, see [`bind_movement`]
pub fn bind_arrows<S: 'static>(
    tree: &mut InputTree<S>,
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
pub fn bind_hjkl<S: 'static>(
    tree: &mut InputTree<S>,
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
pub fn bind<S: 'static>(
    tree: &mut InputTree<S>,
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
        move |e: &mut Editor<S>| move_cursor(e, 0, -1, overrides),
    )?;
    tree.bind(
        &right_bind,
        match describe {
            true => "Move cursor right",
            false => "",
        },
        move |e: &mut Editor<S>| move_cursor(e, 0, 1, overrides),
    )?;
    tree.bind(
        &up_bind,
        match describe {
            true => "Move cursor up",
            false => "",
        },
        move |e: &mut Editor<S>| move_cursor(e, -1, 0, overrides),
    )?;
    tree.bind(
        &down_bind,
        match describe {
            true => "Move cursor down",
            false => "",
        },
        move |e: &mut Editor<S>| move_cursor(e, 1, 0, overrides),
    )?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn stops(kind: Motion, src: &str, start: (usize, usize)) -> Vec<(usize, usize)> {
        let rope = text::Rope::from_str(src);
        let unit = motion_unit(kind, 1);
        let mut out = vec![];
        let mut range = start;

        loop {
            let next = unit(range.0, range.1, &rope);

            if next == range {
                break;
            }

            out.push(next);
            range = next;
        }

        out
    }

    #[test]
    fn word_start_selects_word_and_trailing_space() {
        assert_eq!(
            stops(Motion::WordStart, "foo bar baz", (0, 0)),
            vec![(0, 3), (4, 7), (8, 10)]
        );
    }

    #[test]
    fn word_start_advances_anchor_from_boundary() {
        assert_eq!(stops(Motion::WordStart, " foo bar", (0, 0))[0], (1, 4));
    }

    #[test]
    fn word_start_ignores_previous_anchor() {
        assert_eq!(stops(Motion::WordStart, "foo bar baz", (9, 0))[0], (0, 3));
    }

    #[test]
    fn word_start_stops_at_line_end() {
        assert_eq!(
            stops(Motion::BigWordStart, "hi;\nfoo", (1, 1)),
            vec![(1, 2), (4, 6)]
        );
        assert_eq!(
            stops(Motion::WordStart, "foo\r\nbar", (0, 0)),
            vec![(0, 2), (5, 7)]
        );
    }

    #[test]
    fn word_start_repeats_count() {
        let rope = text::Rope::from_str("foo bar baz");
        assert_eq!(motion_unit(Motion::WordStart, 2)(0, 0, &rope), (4, 7));
    }

    #[test]
    fn big_word_start_ignores_punct() {
        assert_eq!(
            stops(Motion::BigWordStart, "foo.bar baz", (0, 0)),
            vec![(0, 7), (8, 10)]
        );
    }

    #[test]
    fn word_end_selects_leading_space_and_word() {
        assert_eq!(
            stops(Motion::WordEnd, "foo bar", (0, 0)),
            vec![(0, 2), (3, 6)]
        );
    }

    #[test]
    fn word_end_stops_at_line_end() {
        assert_eq!(
            stops(Motion::WordEnd, "hi;\nfoo", (0, 0)),
            vec![(0, 1), (2, 2), (4, 6)]
        );
    }

    #[test]
    fn word_back_selects_word() {
        assert_eq!(
            stops(Motion::WordBack, "foo bar", (6, 6)),
            vec![(6, 4), (3, 0)]
        );
    }

    #[test]
    fn word_back_stops_at_line_start() {
        assert_eq!(
            stops(Motion::WordBack, "foo\n  bar", (6, 6)),
            vec![(5, 4), (2, 0)]
        );
    }

    #[test]
    fn big_word_back_ignores_punct() {
        assert_eq!(
            stops(Motion::BigWordBack, "foo.bar baz", (10, 10)),
            vec![(10, 8), (7, 0)]
        );
    }
}
