use std::str::FromStr;

use ratatui::layout::Direction;

/// A screen direction
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

impl Dir {
    /// Returns the (lines, columns) offset of one step in this direction
    pub fn offset(self) -> (isize, isize) {
        match self {
            Dir::Left => (0, -1),
            Dir::Right => (0, 1),
            Dir::Up => (-1, 0),
            Dir::Down => (1, 0),
        }
    }

    /// Returns the layout axis of this direction
    pub fn axis(self) -> Direction {
        match self {
            Dir::Left | Dir::Right => Direction::Horizontal,
            Dir::Up | Dir::Down => Direction::Vertical,
        }
    }

    /// Returns whether this direction points right or down
    pub fn is_forward(self) -> bool {
        matches!(self, Dir::Right | Dir::Down)
    }
}

impl FromStr for Dir {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "left" | "h" => Ok(Dir::Left),
            "right" | "l" => Ok(Dir::Right),
            "up" | "k" => Ok(Dir::Up),
            "down" | "j" => Ok(Dir::Down),
            _ => Err(anyhow::anyhow!("unknown direction `{s}`")),
        }
    }
}
