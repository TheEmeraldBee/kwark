use crate::State;

pub type SuspendFn = Box<dyn FnOnce(&mut State) -> anyhow::Result<()>>;

/// Work queued for when another program owns the terminal
#[derive(Default)]
pub struct SuspendQueue(Vec<SuspendFn>);

impl SuspendQueue {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes and returns every queued suspend
    pub fn take(&mut self) -> Vec<SuspendFn> {
        std::mem::take(&mut self.0)
    }
}

impl State {
    /// Runs `f` while the shell owns the terminal
    pub fn suspend(&mut self, f: impl FnOnce(&mut State) -> anyhow::Result<()> + 'static) {
        self.get::<&mut SuspendQueue>().0.push(Box::new(f));
    }
}
