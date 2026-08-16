//! Keyframe timeline sequences, multi-stop animations, and loop playback modes.

use crate::animation::{Animatable, Curve};
use crate::style::Style;

/// Repetition and loop playback behavior for [`Keyframes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    /// Plays the animation sequence once from 0.0 to 1.0, then stops at the final frame.
    #[default]
    Once,
    /// Repeats the animation sequence a specific number of times.
    Times(u32),
    /// Loops the animation continuously in an infinite cycle.
    Infinite,
    /// Oscillates back and forth between 0.0 -> 1.0 -> 0.0 continuously.
    PingPong,
}

/// A single stop in a [`Keyframes`] animation track.
#[derive(Debug, Clone)]
pub struct Keyframe<T> {
    /// Normalized timeline offset between `0.0` and `1.0`.
    pub offset: f64,
    /// Value at this keyframe.
    pub value: T,
    /// Optional easing curve applied when interpolating towards this keyframe.
    pub curve: Option<Curve>,
}

/// A multi-stop keyframe timeline that interpolates values over time according to easing curves and loop modes.
#[derive(Debug, Clone)]
pub struct Keyframes<T = Style> {
    /// Ordered keyframe stops.
    pub frames: Vec<Keyframe<T>>,
    /// Total duration of one timeline cycle in milliseconds.
    pub duration_ms: f64,
    /// Loop playback mode.
    pub repeat: Repeat,
    /// Default easing curve between keyframe stops.
    pub default_curve: Curve,
}

impl<T: Animatable> Default for Keyframes<T> {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            duration_ms: 1000.0,
            repeat: Repeat::Once,
            default_curve: Curve::linear(),
        }
    }
}

impl<T: Animatable> Keyframes<T> {
    /// Creates a new empty keyframe timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a keyframe stop at normalized progress `offset` (`0.0` to `1.0`).
    pub fn keyframe(mut self, offset: f64, value: T) -> Self {
        self.frames.push(Keyframe {
            offset: offset.clamp(0.0, 1.0),
            value,
            curve: None,
        });
        self.frames
            .sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
        self
    }

    /// Adds a keyframe stop with a specific easing curve.
    pub fn keyframe_with_curve(mut self, offset: f64, value: T, curve: Curve) -> Self {
        self.frames.push(Keyframe {
            offset: offset.clamp(0.0, 1.0),
            value,
            curve: Some(curve),
        });
        self.frames
            .sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
        self
    }

    /// Sets the total duration of one animation cycle in milliseconds.
    pub fn duration_ms(mut self, ms: f64) -> Self {
        self.duration_ms = ms.max(1.0);
        self
    }

    /// Sets the repetition mode (e.g. `Repeat::Infinite`, `Repeat::PingPong`).
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Sets the default easing curve used between stops when not explicitly specified on a keyframe.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.default_curve = curve;
        self
    }

    /// Evaluates the timeline at `elapsed_ms`. Returns `(current_value, is_active)`.
    pub fn evaluate(&self, elapsed_ms: f64) -> (T, bool) {
        if self.frames.is_empty() {
            panic!("Cannot evaluate Keyframes with 0 frames");
        }

        if self.frames.len() == 1 {
            return (self.frames[0].value.clone(), false);
        }

        let duration = self.duration_ms.max(1.0);
        let raw_cycles = elapsed_ms / duration;

        let (t, is_active) = match self.repeat {
            Repeat::Once => {
                if raw_cycles >= 1.0 {
                    (1.0, false)
                } else {
                    (raw_cycles.clamp(0.0, 1.0), true)
                }
            }
            Repeat::Times(n) => {
                if raw_cycles >= n as f64 {
                    (1.0, false)
                } else {
                    (raw_cycles.fract(), true)
                }
            }
            Repeat::Infinite => (raw_cycles.fract(), true),
            Repeat::PingPong => {
                let cycle_idx = raw_cycles.floor() as u64;
                let fract = raw_cycles.fract();
                let pingpong_t = if cycle_idx % 2 == 0 {
                    fract
                } else {
                    1.0 - fract
                };
                (pingpong_t, true)
            }
        };

        // Find surrounding keyframes
        let mut prev_idx = 0;
        let mut next_idx = self.frames.len() - 1;

        for (i, frame) in self.frames.iter().enumerate() {
            if frame.offset <= t {
                prev_idx = i;
            }
            if frame.offset >= t {
                next_idx = i;
                break;
            }
        }

        if prev_idx == next_idx {
            return (self.frames[prev_idx].value.clone(), is_active);
        }

        let start_frame = &self.frames[prev_idx];
        let end_frame = &self.frames[next_idx];

        let span = (end_frame.offset - start_frame.offset).max(1e-6);
        let local_t = ((t - start_frame.offset) / span).clamp(0.0, 1.0);

        let curve = end_frame.curve.as_ref().unwrap_or(&self.default_curve);
        let progress = curve.eval(local_t);

        let interpolated = T::interpolate(&start_frame.value, &end_frame.value, progress);
        (interpolated, is_active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyframes_evaluation() {
        let kf = Keyframes::<f32>::new()
            .keyframe(0.0, 0.0)
            .keyframe(0.5, 100.0)
            .keyframe(1.0, 0.0)
            .duration_ms(1000.0)
            .curve(Curve::linear());

        let (val_0, active_0) = kf.evaluate(0.0);
        assert_eq!(val_0, 0.0);
        assert!(active_0);

        let (val_250, _) = kf.evaluate(250.0);
        assert!((val_250 - 50.0).abs() < 1e-3);

        let (val_500, _) = kf.evaluate(500.0);
        assert!((val_500 - 100.0).abs() < 1e-3);

        let (val_750, _) = kf.evaluate(750.0);
        assert!((val_750 - 50.0).abs() < 1e-3);

        let (val_1000, active_1000) = kf.evaluate(1000.0);
        assert_eq!(val_1000, 0.0);
        assert!(!active_1000);
    }

    #[test]
    fn test_keyframes_ping_pong() {
        let kf = Keyframes::<f32>::new()
            .keyframe(0.0, 10.0)
            .keyframe(1.0, 20.0)
            .duration_ms(1000.0)
            .repeat(Repeat::PingPong)
            .curve(Curve::linear());

        let (val_forward, _) = kf.evaluate(500.0);
        assert!((val_forward - 15.0).abs() < 1e-3);

        let (val_backward, _) = kf.evaluate(1500.0);
        assert!((val_backward - 15.0).abs() < 1e-3);
    }
}
