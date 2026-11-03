use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    time::{Duration, Instant},
};

type OnceFn<C> = Box<dyn FnOnce(&mut C) -> anyhow::Result<()>>;
type EveryFn<C> = Box<dyn FnMut(&mut C) -> anyhow::Result<()>>;

/// An identifier for a scheduled timer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerId(u64);

enum Timer<C> {
    Once(Option<OnceFn<C>>),
    Every(Duration, EveryFn<C>),
}

/// A timer that is due
pub struct Due<C> {
    id: TimerId,
    at: Instant,
    timer: Timer<C>,
}

impl<C> Due<C> {
    /// Runs the timer's callback
    pub fn run(&mut self, state: &mut C) -> anyhow::Result<()> {
        match &mut self.timer {
            Timer::Once(f) => match f.take() {
                Some(f) => f(state),
                None => Ok(()),
            },
            Timer::Every(_, f) => f(state),
        }
    }
}

/// Delayed and repeating callbacks
pub struct Timers<C> {
    heap: BinaryHeap<Reverse<(Instant, TimerId)>>,
    entries: HashMap<TimerId, Timer<C>>,
    keys: HashMap<String, TimerId>,
    next_id: u64,

    running: Option<TimerId>,
    running_cancelled: bool,
}

impl<C> Default for Timers<C> {
    fn default() -> Self {
        Self {
            heap: BinaryHeap::new(),
            entries: HashMap::new(),
            keys: HashMap::new(),
            next_id: 0,

            running: None,
            running_cancelled: false,
        }
    }
}

impl<C> Timers<C> {
    fn schedule(&mut self, at: Instant, timer: Timer<C>) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;

        self.heap.push(Reverse((at, id)));
        self.entries.insert(id, timer);

        id
    }

    /// Runs `f` once after `delay`
    pub fn after(
        &mut self,
        delay: Duration,
        f: impl FnOnce(&mut C) -> anyhow::Result<()> + 'static,
    ) -> TimerId {
        self.schedule(Instant::now() + delay, Timer::Once(Some(Box::new(f))))
    }

    /// Runs `f` once every `period` until cancelled
    pub fn every(
        &mut self,
        period: Duration,
        f: impl FnMut(&mut C) -> anyhow::Result<()> + 'static,
    ) -> TimerId {
        self.schedule(Instant::now() + period, Timer::Every(period, Box::new(f)))
    }

    /// Runs `f` once after `delay`, replacing any timer still pending under `key`
    pub fn debounce(
        &mut self,
        key: impl Into<String>,
        delay: Duration,
        f: impl FnOnce(&mut C) -> anyhow::Result<()> + 'static,
    ) -> TimerId {
        let key = key.into();
        self.cancel_key(&key);

        let id = self.after(delay, f);
        self.keys.insert(key, id);

        id
    }

    /// Cancels a timer
    pub fn cancel(&mut self, id: TimerId) {
        self.entries.remove(&id);

        if self.running == Some(id) {
            self.running_cancelled = true;
        }
    }

    /// Cancels the timer pending under `key`
    pub fn cancel_key(&mut self, key: &str) {
        if let Some(id) = self.keys.remove(key) {
            self.cancel(id);
        }
    }

    /// Returns when the earliest timer is due
    pub fn next_deadline(&mut self) -> Option<Instant> {
        while let Some(Reverse((at, id))) = self.heap.peek() {
            if self.entries.contains_key(id) {
                return Some(*at);
            }

            self.heap.pop();
        }

        None
    }

    /// Removes and returns a timer that is due at `now`
    pub fn pop_due(&mut self, now: Instant) -> Option<Due<C>> {
        loop {
            let Reverse((at, id)) = *self.heap.peek()?;

            if at > now {
                return None;
            }

            self.heap.pop();

            let Some(timer) = self.entries.remove(&id) else {
                continue;
            };

            if matches!(timer, Timer::Once(_)) {
                self.keys.retain(|_, x| *x != id);
            }

            self.running = Some(id);
            self.running_cancelled = false;

            return Some(Due { id, at, timer });
        }
    }

    /// Reschedules a repeating timer
    pub fn finish(&mut self, due: Due<C>) {
        let cancelled = self.running_cancelled;
        self.running = None;
        self.running_cancelled = false;

        let Timer::Every(period, _) = &due.timer else {
            return;
        };

        if cancelled {
            return;
        }

        let now = Instant::now();
        let next = match due.at + *period {
            next if next > now => next,
            _ => now + *period,
        };

        self.heap.push(Reverse((next, due.id)));
        self.entries.insert(due.id, due.timer);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn run_due(timers: &mut Timers<()>, state: &mut (), now: Instant) -> usize {
        let mut count = 0;

        while let Some(mut due) = timers.pop_due(now) {
            due.run(state).unwrap();
            timers.finish(due);
            count += 1;
        }

        count
    }

    #[test]
    pub fn after_fires_once() {
        let mut timers = Timers::default();
        let mut state = ();

        timers.after(Duration::ZERO, |_| Ok(()));
        let later = Instant::now() + Duration::from_millis(1);

        assert_eq!(run_due(&mut timers, &mut state, later), 1);
        assert_eq!(timers.next_deadline(), None);
    }

    #[test]
    pub fn cancel_skips_the_timer() {
        let mut timers = Timers::default();
        let mut state = ();

        let id = timers.after(Duration::ZERO, |_| Ok(()));
        timers.cancel(id);

        assert_eq!(timers.next_deadline(), None);
        assert_eq!(run_due(&mut timers, &mut state, Instant::now()), 0);
    }

    #[test]
    pub fn debounce_replaces_the_pending_timer() {
        let mut timers = Timers::default();
        let mut state = ();

        timers.debounce("key", Duration::ZERO, |_| Ok(()));
        timers.debounce("key", Duration::ZERO, |_| Ok(()));
        let later = Instant::now() + Duration::from_millis(1);

        assert_eq!(run_due(&mut timers, &mut state, later), 1);
    }

    #[test]
    pub fn every_rearms_after_it_runs() {
        let mut timers = Timers::default();
        let mut state = ();

        timers.every(Duration::from_secs(60), |_| Ok(()));
        let first = timers.next_deadline().unwrap();

        assert_eq!(run_due(&mut timers, &mut state, first), 1);
        assert!(timers.next_deadline().unwrap() > first);
    }
}
