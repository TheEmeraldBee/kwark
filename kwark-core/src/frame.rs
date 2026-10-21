use std::time::{Duration, Instant};

use crate::State;

/// The flag that shows the frame count in the bar
pub const FRAME_COUNT_FLAG: &str = "debug_frame_count";

/// Frame requests for the main loop
#[derive(Default)]
pub struct Redraw {
    now: bool,
    at: Option<Instant>,
}

impl Redraw {
    /// Requests a frame as soon as the frame budget allows
    pub fn request(&mut self) {
        self.now = true;
    }

    /// Requests a frame at a later time
    pub fn request_at(&mut self, at: Instant) {
        self.at = Some(self.at.map_or(at, |x| x.min(at)));
    }

    /// Returns when the next frame is due, no sooner than `earliest`
    pub fn deadline(&self, earliest: Instant) -> Option<Instant> {
        if self.now {
            return Some(earliest);
        }

        self.at.map(|at| at.max(earliest))
    }

    /// Clears the requests that a frame drawn at `now` satisfies
    pub fn clear(&mut self, now: Instant) {
        self.now = false;

        if self.at.is_some_and(|at| at <= now) {
            self.at = None;
        }
    }
}

/// The pacing for frames, at most one per interval
pub struct FrameClock {
    interval: Duration,
    last: Option<Instant>,
}

impl FrameClock {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
        }
    }

    /// Returns the earliest time the next frame can draw
    pub fn next_allowed(&self) -> Instant {
        match self.last {
            Some(last) => last + self.interval,
            None => Instant::now(),
        }
    }

    /// Records a frame drawn at `at`
    pub fn presented(&mut self, at: Instant) {
        self.last = Some(at);
    }
}

/// Timing for the frame being drawn
#[derive(Clone, Copy)]
pub struct FrameInfo {
    /// Number of frames drawn so far
    pub count: u64,
    /// Time the frame started
    pub now: Instant,
    /// Time since the previous frame
    pub dt: Duration,
}

impl Default for FrameInfo {
    fn default() -> Self {
        Self {
            count: 0,
            now: Instant::now(),
            dt: Duration::ZERO,
        }
    }
}

impl FrameInfo {
    /// Moves on to a new frame starting at `now`
    pub fn advance(&mut self, now: Instant) {
        self.dt = now.duration_since(self.now);
        self.now = now;
        self.count += 1;
    }
}

impl State {
    /// Requests a frame as soon as the frame budget allows
    pub fn redraw(&mut self) {
        self.get::<&mut Redraw>().request();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn idle_has_no_deadline() {
        let redraw = Redraw::default();

        assert_eq!(redraw.deadline(Instant::now()), None);
    }

    #[test]
    pub fn request_waits_for_the_frame_budget() {
        let mut redraw = Redraw::default();
        let earliest = Instant::now() + Duration::from_millis(10);

        redraw.request();

        assert_eq!(redraw.deadline(earliest), Some(earliest));
    }

    #[test]
    pub fn request_at_keeps_the_earliest_time() {
        let mut redraw = Redraw::default();
        let now = Instant::now();

        redraw.request_at(now + Duration::from_secs(5));
        redraw.request_at(now + Duration::from_secs(1));

        assert_eq!(redraw.deadline(now), Some(now + Duration::from_secs(1)));
    }

    #[test]
    pub fn clear_keeps_future_requests() {
        let mut redraw = Redraw::default();
        let now = Instant::now();
        let later = now + Duration::from_secs(60);

        redraw.request();
        redraw.request_at(later);
        redraw.clear(now);

        assert_eq!(redraw.deadline(now), Some(later));

        redraw.clear(later);

        assert_eq!(redraw.deadline(now), None);
    }

    #[test]
    pub fn clock_paces_frames() {
        let mut clock = FrameClock::new(Duration::from_millis(16));
        let now = Instant::now();

        clock.presented(now);

        assert_eq!(clock.next_allowed(), now + Duration::from_millis(16));
    }
}
