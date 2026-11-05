use std::{collections::HashMap, time::Duration};

use kwark_core::{events, prelude::*};

/// The longest frame time one animation step covers
const MAX_STEP: Duration = Duration::from_millis(18);

/// Animates each window's scroll toward its target line
pub struct Plugin {
    pub speed: f32,
}

impl Default for Plugin {
    fn default() -> Self {
        Self { speed: 20.0 }
    }
}

impl Plugin {
    pub fn new(speed: f32) -> Self {
        Self { speed }
    }
}

/// The animated position of a buffer in a window
#[derive(Default)]
pub struct SmoothScroll {
    pos: Option<f32>,
}

impl SmoothScroll {
    /// Moves toward `target`, returning the line to draw or None once it arrives
    fn step(&mut self, target: usize, dt: Duration, speed: f32) -> Option<usize> {
        let target = target as f32;
        let pos = self.pos.get_or_insert(target);

        let t = 1.0 - (-dt.min(MAX_STEP).as_secs_f32() * speed).exp();
        *pos += (target - *pos) * t;

        if (target - *pos).abs() < 0.5 {
            *pos = target;
            return None;
        }

        Some(pos.round() as usize)
    }
}

impl PluginState for Plugin {
    type State = HashMap<(LeafId, buffer::ID), SmoothScroll>;

    fn init(&self) -> anyhow::Result<Self::State> {
        Ok(HashMap::new())
    }
}

impl<S: Has<Plugin> + 'static> kwark_core::Plugin<S> for Plugin {
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()> {
        let speed = self.speed;

        editor.events.on::<events::Frame>(move |s, frame| {
            let (core, animations) = s.split::<Plugin>();

            let mut next = HashMap::with_capacity(animations.len());
            let mut moving = false;

            for (leaf, window) in core.windows.leaves_mut() {
                let Some(id) = window.selected() else {
                    continue;
                };
                let Some(view) = window.view_mut() else {
                    continue;
                };

                let key = (leaf, id);
                let mut smooth = animations.remove(&key).unwrap_or_default();

                view.scroll.shown = smooth.step(view.scroll.top, frame.info.dt, speed);
                moving |= view.scroll.shown.is_some();

                next.insert(key, smooth);
            }

            *animations = next;

            if moving {
                s.redraw();
            }

            Ok(())
        });

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const FRAME: Duration = Duration::from_millis(16);

    #[test]
    pub fn first_step_starts_at_the_target() {
        let mut smooth = SmoothScroll::default();

        assert_eq!(smooth.step(10, FRAME, 20.0), None);
    }

    #[test]
    pub fn steps_move_toward_the_target_and_arrive() {
        let mut smooth = SmoothScroll::default();
        smooth.step(0, FRAME, 20.0);

        let first = smooth.step(40, FRAME, 20.0).unwrap();
        assert!(first > 0 && first < 40);

        let mut last = first;
        while let Some(line) = smooth.step(40, FRAME, 20.0) {
            assert!(line >= last);
            last = line;
        }

        assert_eq!(smooth.pos, Some(40.0));
    }

    #[test]
    pub fn long_frames_do_not_skip_the_animation() {
        let mut smooth = SmoothScroll::default();
        smooth.step(0, FRAME, 20.0);

        assert!(smooth.step(40, Duration::from_secs(5), 20.0).is_some());
    }
}
