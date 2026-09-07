//! Reusable kinetic velocity and momentum physics simulation primitive.
//!
//! Provides [`KineticTracker`] for tracking pointer flick gestures and simulating
//! realistic momentum decay on scrollable containers, infinite 2D viewports, and custom dials.

use std::time::Instant;

const SAMPLE_CAPACITY: usize = 6;
const DEFAULT_FRICTION: f32 = 4.8;
const MIN_VELOCITY_CUTOFF: f32 = 1.0;
const MAX_FLING_WINDOW_SECS: f32 = 0.12;

/// Samples timestamped pointer positions to compute launch velocity and simulates
/// exponential velocity decay across successive animation frames.
#[derive(Clone, Debug)]
pub struct KineticTracker {
    friction: f32,
    velocity: (f32, f32),
    samples: [(f32, f32, Instant); SAMPLE_CAPACITY],
    sample_count: usize,
    sample_idx: usize,
}

impl Default for KineticTracker {
    fn default() -> Self {
        Self::new(DEFAULT_FRICTION)
    }
}

impl KineticTracker {
    /// Creates a new `KineticTracker` configured with the specified exponential friction coefficient.
    ///
    /// Typical friction values range between `3.0` (slippery) and `6.0` (tight resistance).
    /// Defaults to `4.8`.
    pub fn new(friction: f32) -> Self {
        let now = Instant::now();
        Self {
            friction: friction.max(0.1),
            velocity: (0.0, 0.0),
            samples: [(0.0, 0.0, now); SAMPLE_CAPACITY],
            sample_count: 0,
            sample_idx: 0,
        }
    }

    /// Resets the tracker history and starts recording position samples from pointer press.
    pub fn on_press(&mut self, x: f32, y: f32) {
        let now = Instant::now();
        self.velocity = (0.0, 0.0);
        self.samples[0] = (x, y, now);
        self.sample_count = 1;
        self.sample_idx = 1;
    }

    /// Records a new timestamped pointer position sample during dragging or scrubbing.
    pub fn on_move(&mut self, x: f32, y: f32) {
        let now = Instant::now();
        self.samples[self.sample_idx] = (x, y, now);
        self.sample_idx = (self.sample_idx + 1) % SAMPLE_CAPACITY;
        if self.sample_count < SAMPLE_CAPACITY {
            self.sample_count += 1;
        }
    }

    /// Calculates and sets the launch velocity (pixels/second) from recent position history.
    ///
    /// If the cursor remained stationary immediately before release (elapsed > 120ms),
    /// launch velocity drops to zero.
    pub fn on_release(&mut self) -> (f32, f32) {
        if self.sample_count < 2 {
            self.sample_count = 0;
            self.velocity = (0.0, 0.0);
            return (0.0, 0.0);
        }

        let now = Instant::now();
        let newest_idx = (self.sample_idx + SAMPLE_CAPACITY - 1) % SAMPLE_CAPACITY;
        let newest = self.samples[newest_idx];

        // If user paused before releasing, do not fling
        if now.duration_since(newest.2).as_secs_f32() > MAX_FLING_WINDOW_SECS {
            self.sample_count = 0;
            self.velocity = (0.0, 0.0);
            return (0.0, 0.0);
        }

        // Find the oldest sample within MAX_FLING_WINDOW_SECS
        let mut oldest = newest;
        for i in 1..self.sample_count {
            let idx = (self.sample_idx + SAMPLE_CAPACITY - 1 - i) % SAMPLE_CAPACITY;
            let sample = self.samples[idx];
            if newest.2.duration_since(sample.2).as_secs_f32() <= MAX_FLING_WINDOW_SECS {
                oldest = sample;
            } else {
                break;
            }
        }

        let dt = newest.2.duration_since(oldest.2).as_secs_f32();
        if dt > 0.005 {
            let vx = (newest.0 - oldest.0) / dt;
            let vy = (newest.1 - oldest.1) / dt;
            self.velocity = (vx, vy);
        } else {
            self.velocity = (0.0, 0.0);
        }

        self.sample_count = 0;
        self.velocity
    }

    /// Explicitly injects a velocity vector (pixels/second), e.g. from touchpad gesture events.
    pub fn set_velocity(&mut self, vx: f32, vy: f32) {
        self.velocity = (vx, vy);
    }

    /// Returns the current simulated velocity vector `(vx, vy)` in pixels/second.
    pub fn velocity(&self) -> (f32, f32) {
        self.velocity
    }

    /// Updates the friction resistance coefficient.
    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction.max(0.1);
    }

    /// Returns the current friction resistance coefficient.
    pub fn friction(&self) -> f32 {
        self.friction
    }

    /// Advances the kinetic momentum simulation by `dt` seconds using exponential velocity decay.
    ///
    /// Returns `Some((delta_x, delta_y))` displacement for this frame, or `None` once velocity
    /// decays below the minimum cutoff threshold.
    pub fn update(&mut self, dt: f32) -> Option<(f32, f32)> {
        if !self.is_active() {
            self.velocity = (0.0, 0.0);
            return None;
        }

        let dt_clamped = dt.clamp(0.001, 0.05);
        let decay = (-self.friction * dt_clamped).exp();

        let dx = self.velocity.0 * dt_clamped;
        let dy = self.velocity.1 * dt_clamped;

        self.velocity.0 *= decay;
        self.velocity.1 *= decay;

        if self.velocity.0.abs() <= MIN_VELOCITY_CUTOFF {
            self.velocity.0 = 0.0;
        }
        if self.velocity.1.abs() <= MIN_VELOCITY_CUTOFF {
            self.velocity.1 = 0.0;
        }

        Some((dx, dy))
    }

    /// Returns `true` if the simulation currently has active momentum above cutoff.
    pub fn is_active(&self) -> bool {
        self.velocity.0.abs() > MIN_VELOCITY_CUTOFF || self.velocity.1.abs() > MIN_VELOCITY_CUTOFF
    }

    /// Immediately halts momentum simulation and zeroes velocity.
    pub fn stop(&mut self) {
        self.velocity = (0.0, 0.0);
        self.sample_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_kinetic_velocity_and_decay() {
        let mut tracker = KineticTracker::new(4.8);
        assert!(!tracker.is_active());

        // Simulate steady horizontal drag of 100px over ~20ms
        tracker.on_press(0.0, 0.0);
        sleep(Duration::from_millis(10));
        tracker.on_move(50.0, 0.0);
        sleep(Duration::from_millis(10));
        tracker.on_move(100.0, 0.0);

        let (vx, vy) = tracker.on_release();
        assert!(vx > 1000.0, "vx should be > 1000 px/sec, got {}", vx);
        assert_eq!(vy, 0.0);
        assert!(tracker.is_active());

        // Step simulation forward
        let delta1 = tracker.update(0.016);
        assert!(delta1.is_some());
        let (dx1, dy1) = delta1.unwrap();
        assert!(dx1 > 0.0);
        assert_eq!(dy1, 0.0);

        // Successive updates decay velocity
        let mut total_x = dx1;
        while let Some((dx, _)) = tracker.update(0.016) {
            total_x += dx;
        }

        assert!(!tracker.is_active());
        assert_eq!(tracker.velocity(), (0.0, 0.0));
        assert!(total_x > 0.0);
    }

    #[test]
    fn test_pause_cancels_fling() {
        let mut tracker = KineticTracker::new(4.8);
        tracker.on_press(0.0, 0.0);
        tracker.on_move(100.0, 0.0);
        // Wait longer than MAX_FLING_WINDOW_SECS before release
        sleep(Duration::from_millis(150));
        let (vx, vy) = tracker.on_release();
        assert_eq!(vx, 0.0);
        assert_eq!(vy, 0.0);
        assert!(!tracker.is_active());
    }
}
