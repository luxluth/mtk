//! Animation curves, spring physics, interpolations, and keyframe sequences.

use crate::Vector2;
use crate::colors::Color;
use crate::effects::{Effects, Radius, Shadow};
use crate::style::{Edges, Size, Style};

pub mod keyframes;
pub mod math;

pub use keyframes::{Keyframe, Keyframes, Repeat};

/// Spring physics configuration (mass, stiffness, dampening).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// Mass of the animated object (higher = more inertia/weight).
    pub mass: f64,
    /// Spring stiffness (higher = faster snap).
    pub stiffness: f64,
    /// Damping ratio (lower = more oscillations/bounce, higher = less overshoot).
    pub dampening: f64,
}

impl Spring {
    /// Creates a custom spring configuration.
    pub const fn new(mass: f64, stiffness: f64, dampening: f64) -> Self {
        Self {
            mass,
            stiffness,
            dampening,
        }
    }

    /// Gentle, smooth spring with minimal overshoot.
    pub const fn gentle() -> Self {
        Self::new(1.0, 120.0, 14.0)
    }

    /// Playful, bouncy spring with noticeable oscillations.
    pub const fn bouncy() -> Self {
        Self::new(1.0, 180.0, 12.0)
    }

    /// Responsive, stiff spring with rapid settling time.
    pub const fn stiff() -> Self {
        Self::new(1.0, 300.0, 26.0)
    }

    /// Slow, cinematic spring with prolonged ease.
    pub const fn slow() -> Self {
        Self::new(1.0, 80.0, 18.0)
    }
}

impl Default for Spring {
    fn default() -> Self {
        Self::gentle()
    }
}

/// Easing curves and physics solvers for transitions and keyframes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    /// Cubic Bezier curve defined by control points `p1` and `p2`.
    Bezier { p1: Vector2, p2: Vector2 },
    /// Harmonic spring oscillator physics curve.
    Spring {
        mass: f64,
        stiffness: f64,
        dampening: f64,
    },
}

impl Curve {
    /// Standard ease-out curve.
    pub fn ease_out() -> Self {
        Self::Bezier {
            p1: (0.0, 0.0).into(),
            p2: (0.2, 1.0).into(),
        }
    }

    /// Standard ease-in curve.
    pub fn ease_in() -> Self {
        Self::Bezier {
            p1: (0.42, 0.0).into(),
            p2: (1.0, 1.0).into(),
        }
    }

    /// Standard symmetric ease-in-out curve.
    pub fn ease_in_out() -> Self {
        Self::Bezier {
            p1: (0.42, 0.0).into(),
            p2: (0.58, 1.0).into(),
        }
    }

    /// Bouncy curve with pleasant overshoot.
    pub fn bouncy() -> Self {
        Self::Bezier {
            p1: (0.05, 0.9).into(),
            p2: (0.1, 1.05).into(),
        }
    }

    /// Constant-speed linear progression.
    pub fn linear() -> Self {
        Self::Bezier {
            p1: (0.0, 0.0).into(),
            p2: (1.0, 1.0).into(),
        }
    }

    /// Harmonic spring curve based on a [`Spring`] configuration.
    pub fn spring(spring: Spring) -> Self {
        Self::Spring {
            mass: spring.mass,
            stiffness: spring.stiffness,
            dampening: spring.dampening,
        }
    }

    /// Evaluates curve progress `[0.0, 1.0]` at normalized time `time` (`0.0` to `1.0`).
    pub fn eval(&self, time: f64) -> f64 {
        let time = time.clamp(0.0, 1.0);

        match self {
            Curve::Bezier { p1, p2 } => {
                let t = math::solve_curve_t(time, p1.x as f64, p2.x as f64);
                math::sample_curve_y(t, p1.y as f64, p2.y as f64)
            }
            Curve::Spring {
                mass,
                stiffness,
                dampening,
            } => {
                if *mass <= 0.0 {
                    return 1.0;
                }

                let w0 = (stiffness / mass).sqrt();
                let zeta = dampening / (2.0 * (mass * stiffness).sqrt());

                if zeta < 1.0 {
                    let wd = w0 * (1.0 - zeta * zeta).sqrt();
                    let a = zeta * w0 / wd;
                    1.0 - (-zeta * w0 * time).exp() * ((wd * time).cos() + a * (wd * time).sin())
                } else {
                    1.0 - (-w0 * time).exp() * (1.0 + w0 * time)
                }
            }
        }
    }
}

/// Trait implemented by types that support smooth mathematical interpolation across transitions.
pub trait Animatable: Clone {
    /// Linearly or mathematically interpolates between `start` and `end` at progress `t` (`0.0` to `1.0`).
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self;

    /// Returns `true` if this value has settled at or reached `target`.
    fn is_finished(&self, target: &Self) -> bool;
}

impl Animatable for f64 {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        *start + (*end - *start) * t
    }
    fn is_finished(&self, target: &Self) -> bool {
        (*self - *target).abs() < 1e-5
    }
}

impl Animatable for f32 {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        *start + (*end - *start) * (t as f32)
    }
    fn is_finished(&self, target: &Self) -> bool {
        (*self - *target).abs() < 1e-5
    }
}

impl Animatable for i32 {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        (*start as f64 + (*end - *start) as f64 * t).round() as i32
    }
    fn is_finished(&self, target: &Self) -> bool {
        self == target
    }
}

impl Animatable for u32 {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        (*start as f64 + (*end as f64 - *start as f64) * t).round() as u32
    }
    fn is_finished(&self, target: &Self) -> bool {
        self == target
    }
}

impl Animatable for Color {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t.clamp(0.0, 1.0) as f32;
        let r = (start.r as f32 + (end.r as f32 - start.r as f32) * t_f).round() as u8;
        let g = (start.g as f32 + (end.g as f32 - start.g as f32) * t_f).round() as u8;
        let b = (start.b as f32 + (end.b as f32 - start.b as f32) * t_f).round() as u8;
        let a = (start.a as f32 + (end.a as f32 - start.a as f32) * t_f).round() as u8;
        Color { r, g, b, a }
    }
    fn is_finished(&self, target: &Self) -> bool {
        self == target
    }
}

impl Animatable for Vector2 {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t as f32;
        Vector2 {
            x: start.x + (end.x - start.x) * t_f,
            y: start.y + (end.y - start.y) * t_f,
        }
    }
    fn is_finished(&self, target: &Self) -> bool {
        (self.x - target.x).abs() < 1e-4 && (self.y - target.y).abs() < 1e-4
    }
}

impl Animatable for Edges {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t as f32;
        Edges {
            top: start.top + (end.top - start.top) * t_f,
            right: start.right + (end.right - start.right) * t_f,
            bottom: start.bottom + (end.bottom - start.bottom) * t_f,
            left: start.left + (end.left - start.left) * t_f,
        }
    }
    fn is_finished(&self, target: &Self) -> bool {
        (self.top - target.top).abs() < 1e-4
            && (self.right - target.right).abs() < 1e-4
            && (self.bottom - target.bottom).abs() < 1e-4
            && (self.left - target.left).abs() < 1e-4
    }
}

impl Animatable for Radius {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t as f32;
        Radius {
            tl: start.tl + (end.tl - start.tl) * t_f,
            tr: start.tr + (end.tr - start.tr) * t_f,
            bl: start.bl + (end.bl - start.bl) * t_f,
            br: start.br + (end.br - start.br) * t_f,
        }
    }
    fn is_finished(&self, target: &Self) -> bool {
        (self.tl - target.tl).abs() < 1e-4
            && (self.tr - target.tr).abs() < 1e-4
            && (self.bl - target.bl).abs() < 1e-4
            && (self.br - target.br).abs() < 1e-4
    }
}

impl Animatable for Shadow {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t as f32;
        Shadow {
            color: Color::interpolate(&start.color, &end.color, t),
            spread: start.spread + (end.spread - start.spread) * t_f,
            power: start.power + (end.power - start.power) * t_f,
        }
    }
    fn is_finished(&self, target: &Self) -> bool {
        self.color.is_finished(&target.color)
            && (self.spread - target.spread).abs() < 1e-4
            && (self.power - target.power).abs() < 1e-4
    }
}

impl Animatable for Size {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t as f32;
        match (start, end) {
            (Size::Fixed(s), Size::Fixed(e)) => {
                Size::Fixed((*s as f32 + (*e as f32 - *s as f32) * t_f).round() as u32)
            }
            (Size::Percent(s), Size::Percent(e)) => Size::Percent(*s + (*e - *s) * t_f),
            (_, end) => *end,
        }
    }
    fn is_finished(&self, target: &Self) -> bool {
        self == target
    }
}

impl Animatable for Effects {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t as f32;
        Effects {
            background_color: Color::interpolate(&start.background_color, &end.background_color, t),
            border: crate::effects::Border {
                color: Color::interpolate(&start.border.color, &end.border.color, t),
                radius: Radius::interpolate(&start.border.radius, &end.border.radius, t),
            },
            shadow: Shadow::interpolate(&start.shadow, &end.shadow, t),
            filters: if t >= 0.5 {
                end.filters.clone()
            } else {
                start.filters.clone()
            },
            opacity: start.opacity + (end.opacity - start.opacity) * t_f,
            scale: start.scale + (end.scale - start.scale) * t_f,
        }
    }
    fn is_finished(&self, target: &Self) -> bool {
        self.background_color.is_finished(&target.background_color)
            && self.border.color.is_finished(&target.border.color)
            && self.border.radius.is_finished(&target.border.radius)
            && self.shadow.is_finished(&target.shadow)
            && (self.opacity - target.opacity).abs() < 1e-4
            && (self.scale - target.scale).abs() < 1e-4
    }
}

impl Animatable for Style {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self {
        let t_f = t as f32;
        let mut interpolated = end.clone();
        interpolated.base_effects = Effects::interpolate(&start.base_effects, &end.base_effects, t);
        interpolated.base_constraints.padding = Edges::interpolate(
            &start.base_constraints.padding,
            &end.base_constraints.padding,
            t,
        );
        interpolated.base_constraints.border = Edges::interpolate(
            &start.base_constraints.border,
            &end.base_constraints.border,
            t,
        );
        interpolated.base_constraints.width = Size::interpolate(
            &start.base_constraints.width,
            &end.base_constraints.width,
            t,
        );
        interpolated.base_constraints.height = Size::interpolate(
            &start.base_constraints.height,
            &end.base_constraints.height,
            t,
        );
        interpolated.base_constraints.gap = start.base_constraints.gap
            + (end.base_constraints.gap - start.base_constraints.gap) * t_f;
        interpolated.base_text_style.font_size = start.base_text_style.font_size
            + (end.base_text_style.font_size - start.base_text_style.font_size) * t_f;
        interpolated.base_text_style.color =
            Color::interpolate(&start.base_text_style.color, &end.base_text_style.color, t);
        interpolated
    }
    fn is_finished(&self, target: &Self) -> bool {
        self.base_effects.is_finished(&target.base_effects)
            && self
                .base_constraints
                .padding
                .is_finished(&target.base_constraints.padding)
            && self
                .base_constraints
                .border
                .is_finished(&target.base_constraints.border)
            && (self.base_constraints.gap - target.base_constraints.gap).abs() < 1e-4
            && (self.base_text_style.font_size - target.base_text_style.font_size).abs() < 1e-4
            && self
                .base_text_style
                .color
                .is_finished(&target.base_text_style.color)
    }
}

/// A reactive animatable value that smoothly transitions between states over time.
#[derive(Debug, Clone)]
pub struct AnimatedValue<T> {
    /// Current interpolated value.
    pub current: T,
    /// Target value to transition towards.
    pub target: T,
    /// Starting value of the current transition segment.
    pub start: T,
    /// Timestamp in milliseconds when the current animation began.
    pub start_time: f64,
    /// Duration of the transition in milliseconds.
    pub duration: f64,
    /// Active easing or spring curve.
    pub curve: Curve,
}

impl<T: Animatable> AnimatedValue<T> {
    /// Creates a new `AnimatedValue` initialized at `initial`.
    pub fn new(initial: T) -> Self {
        Self {
            current: initial.clone(),
            target: initial.clone(),
            start: initial,
            start_time: 0.0,
            duration: 0.0,
            curve: Curve::ease_out(),
        }
    }

    /// Returns a copy of the current interpolated value.
    #[inline]
    pub fn get(&self) -> T {
        self.current.clone()
    }

    /// Immediately snaps the current value to `value` without animating.
    pub fn snap_to(&mut self, value: T) {
        self.current = value.clone();
        self.target = value.clone();
        self.start = value;
        self.duration = 0.0;
    }

    /// Sets a new target value to animate towards starting from `now` ms.
    pub fn set_target(&mut self, new_target: T, now: f64, duration: f64, curve: Curve) {
        if !self.target.is_finished(&new_target) {
            self.start = self.current.clone();
            self.target = new_target;
            self.start_time = now;
            self.duration = duration;
            self.curve = curve;
        }
    }

    /// Transitions to `target` using a spring configuration.
    pub fn spring_to(&mut self, target: T, now: f64, spring: Spring) {
        self.set_target(target, now, 500.0, Curve::spring(spring));
    }

    /// Transitions to `target` over `duration_ms` with `curve`.
    pub fn animate_to(&mut self, target: T, now: f64, duration_ms: f64, curve: Curve) {
        self.set_target(target, now, duration_ms, curve);
    }

    /// Returns `true` if the value is currently actively animating.
    #[inline]
    pub fn is_animating(&self) -> bool {
        !self.current.is_finished(&self.target)
    }

    /// Advances the animation according to current timestamp `now` in milliseconds.
    /// Returns `true` if still animating, `false` if settled.
    pub fn tick(&mut self, now: f64) -> bool {
        if self.current.is_finished(&self.target) {
            self.current = self.target.clone();
            return false;
        }

        if self.duration <= 0.0 {
            self.current = self.target.clone();
            return false;
        }

        let elapsed = now - self.start_time;
        if elapsed >= self.duration {
            self.current = self.target.clone();
            return false;
        }

        let t = elapsed / self.duration;
        let progress = self.curve.eval(t);
        self.current = T::interpolate(&self.start, &self.target, progress);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_animatable() {
        let red = Color::new(255, 0, 0, 255);
        let blue = Color::new(0, 0, 255, 255);
        let mid = Color::interpolate(&red, &blue, 0.5);

        assert_eq!(mid.r, 128);
        assert_eq!(mid.g, 0);
        assert_eq!(mid.b, 128);
        assert_eq!(mid.a, 255);
    }

    #[test]
    fn test_animated_value_lifecycle() {
        let mut val = AnimatedValue::new(0.0f32);
        assert_eq!(val.get(), 0.0);
        assert!(!val.is_animating());

        val.animate_to(100.0, 1000.0, 1000.0, Curve::linear());
        assert!(val.is_animating());

        let still_animating = val.tick(1500.0);
        assert!(still_animating);
        assert!((val.get() - 50.0).abs() < 1e-3);

        let still_animating = val.tick(2000.0);
        assert!(!still_animating);
        assert_eq!(val.get(), 100.0);
        assert!(!val.is_animating());
    }

    #[test]
    fn test_style_interpolation() {
        let start = Style::new().padding(10.0).scale(1.0).opacity(1.0);
        let end = Style::new().padding(20.0).scale(2.0).opacity(0.0);

        let mid = Style::interpolate(&start, &end, 0.5);
        assert!((mid.base_constraints.padding.top - 15.0).abs() < 1e-3);
        assert!((mid.base_effects.scale - 1.5).abs() < 1e-3);
        assert!((mid.base_effects.opacity - 0.5).abs() < 1e-3);
    }
}
