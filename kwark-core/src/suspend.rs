use crate::Editor;

pub type SuspendFn<C> = Box<dyn FnOnce(&mut C) -> anyhow::Result<()>>;

/// Work queued for when another program owns the terminal
pub struct SuspendQueue<C>(Vec<SuspendFn<C>>);

impl<C> Default for SuspendQueue<C> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<C> SuspendQueue<C> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes and returns every queued suspend
    pub fn take(&mut self) -> Vec<SuspendFn<C>> {
        std::mem::take(&mut self.0)
    }
}

impl<S: 'static> Editor<S> {
    /// Runs `f` while the shell owns the terminal
    pub fn suspend(&mut self, f: impl FnOnce(&mut Editor<S>) -> anyhow::Result<()> + 'static) {
        self.core.suspend.0.push(Box::new(f));
    }
}
