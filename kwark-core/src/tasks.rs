use std::{collections::HashSet, future::Future};

use tokio::task::AbortHandle;

use crate::{CallbackSender, Runtime, State};

/// An identifier for a background task
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

/// The background tasks that are still running
#[derive(Default)]
pub struct Tasks {
    running: HashSet<TaskId>,
    next_id: u64,
}

impl Tasks {
    fn begin(&mut self) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        self.running.insert(id);

        id
    }

    fn end(&mut self, id: TaskId) {
        self.running.remove(&id);
    }

    /// Returns whether any task is still running
    pub fn busy(&self) -> bool {
        !self.running.is_empty()
    }

    /// Number of tasks still running
    pub fn len(&self) -> usize {
        self.running.len()
    }

    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }
}

/// A handle to a running background task
pub struct TaskHandle {
    id: TaskId,
    abort: AbortHandle,
}

impl TaskHandle {
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Cancels the task
    pub fn abort(&self) {
        self.abort.abort();
    }

    pub fn is_finished(&self) -> bool {
        self.abort.is_finished()
    }
}

/// The end marker for a task
struct EndGuard {
    sender: CallbackSender,
    id: TaskId,
}

impl Drop for EndGuard {
    fn drop(&mut self) {
        let id = self.id;
        self.sender.send_fn(move |e| {
            e.get::<&mut Tasks>().end(id);
            Ok(())
        });
    }
}

impl State {
    /// Runs a future in the background, then `done` with its result
    pub fn spawn<T: Send + 'static>(
        &mut self,
        fut: impl Future<Output = T> + Send + 'static,
        done: impl FnOnce(&mut State, T) -> anyhow::Result<()> + Send + 'static,
    ) -> TaskHandle {
        let mut sender = self.sender();
        let id = self.get::<&mut Tasks>().begin();
        let guard = EndGuard {
            sender: sender.clone(),
            id,
        };

        let handle = self.get::<&Runtime>().spawn(async move {
            let _guard = guard;
            let value = fut.await;

            sender.send_fn(move |e| done(&mut e.state, value));
        });

        TaskHandle {
            id,
            abort: handle.abort_handle(),
        }
    }

    /// Runs blocking work in the background, then `done` with its result
    pub fn spawn_blocking<T: Send + 'static>(
        &mut self,
        work: impl FnOnce() -> T + Send + 'static,
        done: impl FnOnce(&mut State, T) -> anyhow::Result<()> + Send + 'static,
    ) -> TaskHandle {
        let mut sender = self.sender();
        let id = self.get::<&mut Tasks>().begin();
        let guard = EndGuard {
            sender: sender.clone(),
            id,
        };

        let handle = self.get::<&Runtime>().spawn_blocking(move || {
            let _guard = guard;
            let value = work();

            sender.send_fn(move |e| done(&mut e.state, value));
        });

        TaskHandle {
            id,
            abort: handle.abort_handle(),
        }
    }
}
