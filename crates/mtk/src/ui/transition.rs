use crate::animation::Curve;

/// Preset transition motion physics for UI layers and animated surfaces.
#[derive(Clone, Debug, PartialEq)]
pub enum Transition {
    /// Slides vertically up from the bottom of the viewport.
    SlideUp { duration_ms: f32, curve: Curve },
    /// Slides vertically down from the top of the viewport.
    SlideDown { duration_ms: f32, curve: Curve },
    /// Slides horizontally in from the right edge.
    SlideRight { duration_ms: f32, curve: Curve },
    /// Slides horizontally in from the left edge.
    SlideLeft { duration_ms: f32, curve: Curve },
    /// Smooth opacity fade in/out.
    Fade { duration_ms: f32, curve: Curve },
    /// Scale and fade transition (popular for modals, popovers, and command palettes).
    Scale {
        from_scale: f32,
        duration_ms: f32,
        curve: Curve,
    },
    /// Instant display with no animation.
    None,
}

impl Default for Transition {
    fn default() -> Self {
        Transition::Fade {
            duration_ms: 200.0,
            curve: Curve::ease_out(),
        }
    }
}

impl Transition {
    /// Creates a standard slide-up transition (220ms ease-out).
    pub fn slide_up() -> Self {
        Transition::SlideUp {
            duration_ms: 220.0,
            curve: Curve::ease_out(),
        }
    }

    /// Creates a slide-up transition with custom duration in milliseconds.
    pub fn slide_up_ms(duration_ms: f32) -> Self {
        Transition::SlideUp {
            duration_ms,
            curve: Curve::ease_out(),
        }
    }

    /// Creates a standard slide-right transition (220ms ease-out).
    pub fn slide_right() -> Self {
        Transition::SlideRight {
            duration_ms: 220.0,
            curve: Curve::ease_out(),
        }
    }

    /// Creates a standard slide-left transition (220ms ease-out).
    pub fn slide_left() -> Self {
        Transition::SlideLeft {
            duration_ms: 220.0,
            curve: Curve::ease_out(),
        }
    }

    /// Creates a standard fade transition (180ms ease-out).
    pub fn fade() -> Self {
        Transition::Fade {
            duration_ms: 180.0,
            curve: Curve::ease_out(),
        }
    }

    /// Creates a modal/popover scale transition (scaling from 0.94 to 1.0 with fade).
    pub fn scale() -> Self {
        Transition::Scale {
            from_scale: 0.94,
            duration_ms: 180.0,
            curve: Curve::ease_out(),
        }
    }

    /// Gets the duration of the transition in milliseconds.
    pub fn duration_ms(&self) -> f32 {
        match self {
            Transition::SlideUp { duration_ms, .. }
            | Transition::SlideDown { duration_ms, .. }
            | Transition::SlideRight { duration_ms, .. }
            | Transition::SlideLeft { duration_ms, .. }
            | Transition::Fade { duration_ms, .. }
            | Transition::Scale { duration_ms, .. } => *duration_ms,
            Transition::None => 0.0,
        }
    }

    /// Gets the animation curve.
    pub fn curve(&self) -> Curve {
        match self {
            Transition::SlideUp { curve, .. }
            | Transition::SlideDown { curve, .. }
            | Transition::SlideRight { curve, .. }
            | Transition::SlideLeft { curve, .. }
            | Transition::Fade { curve, .. }
            | Transition::Scale { curve, .. } => *curve,
            Transition::None => Curve::linear(),
        }
    }
}
