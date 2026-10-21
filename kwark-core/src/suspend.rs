use crate::State;

pub type SuspendFn = Box<dyn FnOnce(&mut State) -> anyhow::Result<()>>;

/// Work that runs while another program owns the terminal
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
    /// Runs `f` with the terminal handed back to the shell, for programs such as fzf
    pub fn suspend(&mut self, f: impl FnOnce(&mut State) -> anyhow::Result<()> + 'static) {
        self.get::<&mut SuspendQueue>().0.push(Box::new(f));
    }
}
