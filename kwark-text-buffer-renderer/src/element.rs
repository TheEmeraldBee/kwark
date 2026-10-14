use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Something that can claim space in a header or footer row, then draw into it
///
/// `C` carries whatever facts an embedder's elements need that `state` can't
/// supply on its own, for example which pane is active or which pane is
/// currently being drawn
pub trait Element<S, C> {
    /// Width this element wants, given the width left in its row
    fn width(&mut self, state: &mut S, ctx: &C, available: u16) -> u16;

    /// Fixed height this element wants; a row takes the tallest claim
    fn height(&mut self, _state: &mut S, _ctx: &C) -> u16 {
        1
    }

    /// Draws the element into exactly the rect it claimed
    fn render(&mut self, state: &mut S, ctx: &C, area: Rect, buf: &mut Buffer);
}

/// A header or footer row, packed from a left group, a right group, and a
/// centered remainder
pub struct ElementRow<S, C> {
    left: Vec<Box<dyn Element<S, C>>>,
    center: Vec<Box<dyn Element<S, C>>>,
    right: Vec<Box<dyn Element<S, C>>>,
}

impl<S, C> Default for ElementRow<S, C> {
    fn default() -> Self {
        Self {
            left: Vec::new(),
            center: Vec::new(),
            right: Vec::new(),
        }
    }
}

impl<S, C> ElementRow<S, C> {
    /// Appends an element to the left group
    pub fn push_left(&mut self, e: impl Element<S, C> + 'static) -> &mut Self {
        self.left.push(Box::new(e));
        self
    }

    /// Appends an element to the center group
    pub fn push_center(&mut self, e: impl Element<S, C> + 'static) -> &mut Self {
        self.center.push(Box::new(e));
        self
    }

    /// Appends an element to the right group
    ///
    /// The right group packs from the row's right edge inward, so the first
    /// element pushed sits closest to that edge and each one after it sits
    /// further to the left
    pub fn push_right(&mut self, e: impl Element<S, C> + 'static) -> &mut Self {
        self.right.push(Box::new(e));
        self
    }

    /// Tallest claim across every element, 0 when the row is empty
    pub fn height(&mut self, state: &mut S, ctx: &C) -> u16 {
        self.left
            .iter_mut()
            .chain(self.center.iter_mut())
            .chain(self.right.iter_mut())
            .map(|e| e.height(state, ctx))
            .max()
            .unwrap_or(0)
    }

    /// Renders every element in the row into its claimed slice of `area`
    pub fn render(&mut self, state: &mut S, ctx: &C, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let right_edge = area.x + area.width;

        let mut left_x = area.x;
        let mut left_rects = Vec::with_capacity(self.left.len());
        for e in &mut self.left {
            let available = right_edge.saturating_sub(left_x);
            let width = e.width(state, ctx, available).min(available);
            left_rects.push(Rect {
                x: left_x,
                y: area.y,
                width,
                height: area.height,
            });
            left_x += width;
        }

        let mut right_x = right_edge;
        let mut right_rects = Vec::with_capacity(self.right.len());
        for e in &mut self.right {
            let available = right_x.saturating_sub(left_x);
            let width = e.width(state, ctx, available).min(available);
            right_x -= width;
            right_rects.push(Rect {
                x: right_x,
                y: area.y,
                width,
                height: area.height,
            });
        }

        let gap_x = left_x;
        let gap_width = right_x.saturating_sub(left_x);

        let mut center_widths = Vec::with_capacity(self.center.len());
        let mut remaining = gap_width;
        for e in &mut self.center {
            let width = e.width(state, ctx, remaining).min(remaining);
            remaining = remaining.saturating_sub(width);
            center_widths.push(width);
        }

        let total_center: u16 = center_widths.iter().sum();
        let mut center_x = gap_x + (gap_width.saturating_sub(total_center)) / 2;
        let mut center_rects = Vec::with_capacity(center_widths.len());
        for width in center_widths {
            center_rects.push(Rect {
                x: center_x,
                y: area.y,
                width,
                height: area.height,
            });
            center_x += width;
        }

        for (e, rect) in self.left.iter_mut().zip(left_rects) {
            e.render(state, ctx, rect, buf);
        }
        for (e, rect) in self.center.iter_mut().zip(center_rects) {
            e.render(state, ctx, rect, buf);
        }
        for (e, rect) in self.right.iter_mut().zip(right_rects) {
            e.render(state, ctx, rect, buf);
        }
    }
}

/// A stack of [`ElementRow`]s, laid out top to bottom
///
/// Lets a header or footer be more than one row deep, for example a tab bar
/// with its own divider line underneath
pub struct ElementGrid<S, C>(Vec<ElementRow<S, C>>);

impl<S, C> Default for ElementGrid<S, C> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<S, C> ElementGrid<S, C> {
    /// Builds a row and appends it to the bottom of the grid
    pub fn push_row(
        &mut self,
        build: impl FnOnce(&mut ElementRow<S, C>) -> &mut ElementRow<S, C>,
    ) -> &mut Self {
        let mut row = ElementRow::default();
        build(&mut row);
        self.0.push(row);
        self
    }

    /// Total height of however many whole rows, from the top, fit within `available`
    ///
    /// A row that would not fit whole is dropped, along with every row after it
    pub fn height(&mut self, state: &mut S, ctx: &C, available: u16) -> u16 {
        let mut total = 0;
        for row in &mut self.0 {
            let h = row.height(state, ctx);
            if total + h > available {
                break;
            }
            total += h;
        }
        total
    }

    /// Renders as many whole rows, top to bottom, as fit in `area`
    pub fn render(&mut self, state: &mut S, ctx: &C, area: Rect, buf: &mut Buffer) {
        let bottom = area.y + area.height;
        let mut y = area.y;

        for row in &mut self.0 {
            let h = row.height(state, ctx);
            if y + h > bottom {
                break;
            }

            row.render(
                state,
                ctx,
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: h,
                },
                buf,
            );
            y += h;
        }
    }
}
