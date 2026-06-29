use crate::Vector2;

pub mod math;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Curve {
    Bezier {
        p1: Vector2,
        p2: Vector2,
    },
    Spring {
        mass: f64,
        stiffness: f64,
        dampening: f64,
    },
}

impl Curve {
    pub fn ease_out() -> Self {
        Self::Bezier {
            p1: (0.0, 0.0).into(),
            p2: (0.2, 1.0).into(),
        }
    }

    pub fn bouncy() -> Self {
        Self::Bezier {
            p1: (0.05, 0.9).into(),
            p2: (0.1, 1.05).into(),
        }
    }

    pub fn linear() -> Self {
        Self::Bezier {
            p1: (0.0, 0.0).into(),
            p2: (1.0, 1.0).into(),
        }
    }

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

pub trait Animatable: Clone {
    fn interpolate(start: &Self, end: &Self, t: f64) -> Self;
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

#[derive(Debug, Clone)]
pub struct AnimatedValue<T> {
    pub current: T,
    pub target: T,
    pub start: T,
    pub start_time: f64, // ms
    pub duration: f64,   // ms
    pub curve: Curve,
}

impl<T: Animatable> AnimatedValue<T> {
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

    pub fn set_target(&mut self, new_target: T, now: f64, duration: f64, curve: Curve) {
        if !self.target.is_finished(&new_target) {
            self.start = self.current.clone();
            self.target = new_target;
            self.start_time = now;
            self.duration = duration;
            self.curve = curve;
        }
    }

    /// Advances the animation. Returns `true` if still animating, `false` if done.
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
