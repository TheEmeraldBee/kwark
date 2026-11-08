use kui::{Node, div};

/// Something that shows in a header or footer row
pub trait Element<S, C> {
    /// Returns the tree for the element, or None to show nothing
    fn view(&mut self, state: &mut S, ctx: &C) -> Option<Node<S>>;
}

/// A header or footer row
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

impl<S: 'static, C> ElementRow<S, C> {
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
    pub fn push_right(&mut self, e: impl Element<S, C> + 'static) -> &mut Self {
        self.right.push(Box::new(e));
        self
    }

    /// Returns the row with its groups at the left, center, and right, or None when nothing shows
    pub fn view(&mut self, state: &mut S, ctx: &C) -> Option<Node<S>> {
        let mut group = |elements: &mut Vec<Box<dyn Element<S, C>>>| -> Vec<Node<S>> {
            elements
                .iter_mut()
                .filter_map(|e| e.view(state, ctx))
                .collect()
        };

        let left = group(&mut self.left);
        let center = group(&mut self.center);
        let right = group(&mut self.right);

        if left.is_empty() && center.is_empty() && right.is_empty() {
            return None;
        }

        // The side groups split the space left over, so the center stays centered
        let side = || div().flex_row().items_center().flex_1().min_w(0.0);

        Some(
            div()
                .flex_row()
                .items_center()
                .child(side().children(left))
                .child(div().flex_row().items_center().children(center))
                .child(side().justify_end().children(right)),
        )
    }
}

/// A stack of [`ElementRow`]s
pub struct ElementGrid<S, C>(Vec<ElementRow<S, C>>);

impl<S, C> Default for ElementGrid<S, C> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<S: 'static, C> ElementGrid<S, C> {
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

    /// Returns every row that shows something, top first
    pub fn view(&mut self, state: &mut S, ctx: &C) -> Vec<Node<S>> {
        self.0
            .iter_mut()
            .filter_map(|row| row.view(state, ctx))
            .collect()
    }
}
