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

    /// Returns whether the point lies inside, counting the top and left edges only
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.left() && p.x < self.right() && p.y >= self.top() && p.y < self.bottom()
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
}
