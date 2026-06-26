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

#[derive(Debug, Clone)]
pub struct AnimatedValue {
    pub current: f64,
    pub target: f64,
    pub start: f64,
    pub start_time: f64, // ms
    pub duration: f64,   // ms
    pub curve: Curve,
}

impl AnimatedValue {
    pub fn new(initial: f64) -> Self {
        Self {
            current: initial,
            target: initial,
            start: initial,
            start_time: 0.0,
            duration: 0.0,
            curve: Curve::ease_out(),
        }
    }

    pub fn set_target(&mut self, new_target: f64, now: f64, duration: f64, curve: Curve) {
        if (self.target - new_target).abs() > 1e-5 {
            self.start = self.current;
            self.target = new_target;
            self.start_time = now;
            self.duration = duration;
            self.curve = curve;
        }
    }

    /// Advances the animation. Returns `true` if still animating, `false` if done.
    pub fn tick(&mut self, now: f64) -> bool {
        if (self.current - self.target).abs() < 1e-5 {
            self.current = self.target;
            return false;
        }

        if self.duration <= 0.0 {
            self.current = self.target;
            return false;
        }

        let elapsed = now - self.start_time;
        if elapsed >= self.duration {
            self.current = self.target;
            return false;
        }

        let t = elapsed / self.duration;
        let progress = self.curve.eval(t);
        self.current = self.start + (self.target - self.start) * progress;

        true
    }
}
