/// A position in logical pixels
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A width and height in logical pixels
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle in logical pixels
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// Returns the rect at the origin with the given size
    pub const fn from_size(size: Size) -> Self {
        Self {
            origin: Point::new(0.0, 0.0),
            size,
        }
    }

    pub fn left(&self) -> f32 {
        self.origin.x
    }

    pub fn top(&self) -> f32 {
        self.origin.y
    }

    pub fn right(&self) -> f32 {
        self.origin.x + self.size.width
    }

    pub fn bottom(&self) -> f32 {
        self.origin.y + self.size.height
    }

    /// Returns the rect shrunk by an amount on every side
    pub fn inset(&self, amount: f32) -> Self {
        let width = (self.size.width - amount * 2.0).max(0.0);
        let height = (self.size.height - amount * 2.0).max(0.0);
        Self::new(self.left() + amount, self.top() + amount, width, height)
    }

    /// Returns whether the point lies inside, counting the top and left edges only
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.left() && p.x < self.right() && p.y >= self.top() && p.y < self.bottom()
    }
}

/// The radius of each corner of a rect in logical pixels
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub const ZERO: Self = Self::all(0.0);

    /// Returns corners that all share one radius
    pub const fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Returns the corners with each radius shrunk by an amount (min 0)
    pub fn shrink(&self, amount: f32) -> Self {
        Self {
            top_left: (self.top_left - amount).max(0.0),
            top_right: (self.top_right - amount).max(0.0),
            bottom_right: (self.bottom_right - amount).max(0.0),
            bottom_left: (self.bottom_left - amount).max(0.0),
        }
    }
}

impl From<f32> for Corners {
    fn from(radius: f32) -> Self {
        Self::all(radius)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn contains_takes_the_near_edges_only() {
        let r = Rect::new(10.0, 10.0, 20.0, 5.0);

        assert!(r.contains(Point::new(10.0, 10.0)));
        assert!(r.contains(Point::new(29.9, 14.9)));
        assert!(!r.contains(Point::new(30.0, 12.0)));
        assert!(!r.contains(Point::new(12.0, 15.0)));
    }

    #[test]
    fn inset_stops_at_an_empty_rect() {
        assert_eq!(
            Rect::new(0.0, 0.0, 10.0, 4.0).inset(1.0),
            Rect::new(1.0, 1.0, 8.0, 2.0)
        );
        assert_eq!(Rect::new(0.0, 0.0, 10.0, 4.0).inset(3.0).size.height, 0.0);
    }
}
