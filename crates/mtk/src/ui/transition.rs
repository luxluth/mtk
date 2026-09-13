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

/// Positional offset specification for animated motions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionOffset {
    None,
    /// Offset proportional to viewport/container dimension (e.g. 1.0 = 100%, -0.3 = -30%).
    Fraction(f32),
    /// Fixed offset in logical pixels.
    Pixels(f32),
}

impl MotionOffset {
    #[inline]
    pub fn resolve(self, dimension: f32) -> f32 {
        match self {
            MotionOffset::None => 0.0,
            MotionOffset::Fraction(frac) => frac * dimension,
            MotionOffset::Pixels(px) => px,
        }
    }
}

/// An atomic motion specification defining opacity, translation offsets, and scale interpolations.
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    pub start_opacity: f32,
    pub end_opacity: f32,
    pub start_offset_x: MotionOffset,
    pub end_offset_x: MotionOffset,
    pub start_offset_y: MotionOffset,
    pub end_offset_y: MotionOffset,
    pub start_scale: f32,
    pub end_scale: f32,
}

impl Default for Motion {
    fn default() -> Self {
        Self::stationary()
    }
}

impl Motion {
    /// Creates a stationary motion (remains in place at 1.0 opacity and 1.0 scale).
    pub fn stationary() -> Self {
        Self {
            start_opacity: 1.0,
            end_opacity: 1.0,
            start_offset_x: MotionOffset::None,
            end_offset_x: MotionOffset::None,
            start_offset_y: MotionOffset::None,
            end_offset_y: MotionOffset::None,
            start_scale: 1.0,
            end_scale: 1.0,
        }
    }

    /// Alias for [`stationary`](Self::stationary).
    pub fn none() -> Self {
        Self::stationary()
    }

    /// Fades in from 0.0 to 1.0 opacity.
    pub fn fade_in() -> Self {
        Self {
            start_opacity: 0.0,
            end_opacity: 1.0,
            ..Self::stationary()
        }
    }

    /// Fades out from 1.0 to 0.0 opacity.
    pub fn fade_out() -> Self {
        Self {
            start_opacity: 1.0,
            end_opacity: 0.0,
            ..Self::stationary()
        }
    }

    /// Custom opacity interpolation from `start` to `end`.
    pub fn fade(start: f32, end: f32) -> Self {
        Self {
            start_opacity: start,
            end_opacity: end,
            ..Self::stationary()
        }
    }

    /// Slides in horizontally from the right edge (100% -> 0%).
    pub fn slide_in_right() -> Self {
        Self {
            start_offset_x: MotionOffset::Fraction(1.0),
            end_offset_x: MotionOffset::None,
            ..Self::stationary()
        }
    }

    /// Slides out horizontally to the left edge (0% -> -100%).
    pub fn slide_out_left() -> Self {
        Self {
            start_offset_x: MotionOffset::None,
            end_offset_x: MotionOffset::Fraction(-1.0),
            ..Self::stationary()
        }
    }

    /// Slides in horizontally from the left edge (-100% -> 0%).
    pub fn slide_in_left() -> Self {
        Self {
            start_offset_x: MotionOffset::Fraction(-1.0),
            end_offset_x: MotionOffset::None,
            ..Self::stationary()
        }
    }

    /// Slides out horizontally to the right edge (0% -> 100%).
    pub fn slide_out_right() -> Self {
        Self {
            start_offset_x: MotionOffset::None,
            end_offset_x: MotionOffset::Fraction(1.0),
            ..Self::stationary()
        }
    }

    /// Slides in vertically from the top edge (-100% -> 0%).
    pub fn slide_in_top() -> Self {
        Self {
            start_offset_y: MotionOffset::Fraction(-1.0),
            end_offset_y: MotionOffset::None,
            ..Self::stationary()
        }
    }

    /// Slides out vertically to the top edge (0% -> -100%).
    pub fn slide_out_top() -> Self {
        Self {
            start_offset_y: MotionOffset::None,
            end_offset_y: MotionOffset::Fraction(-1.0),
            ..Self::stationary()
        }
    }

    /// Slides in vertically from the bottom edge (100% -> 0%).
    pub fn slide_in_bottom() -> Self {
        Self {
            start_offset_y: MotionOffset::Fraction(1.0),
            end_offset_y: MotionOffset::None,
            ..Self::stationary()
        }
    }

    /// Slides out vertically to the bottom edge (0% -> 100%).
    pub fn slide_out_bottom() -> Self {
        Self {
            start_offset_y: MotionOffset::None,
            end_offset_y: MotionOffset::Fraction(1.0),
            ..Self::stationary()
        }
    }

    /// Scales up from `from_scale` to 1.0 with simultaneous fade in.
    pub fn scale_in(from_scale: f32) -> Self {
        Self {
            start_scale: from_scale,
            end_scale: 1.0,
            start_opacity: 0.0,
            end_opacity: 1.0,
            ..Self::stationary()
        }
    }

    /// Scales down from 1.0 to `to_scale` with simultaneous fade out.
    pub fn scale_out(to_scale: f32) -> Self {
        Self {
            start_scale: 1.0,
            end_scale: to_scale,
            start_opacity: 1.0,
            end_opacity: 0.0,
            ..Self::stationary()
        }
    }

    /// Combines this motion with another motion (e.g. slide combined with fade).
    pub fn combined(mut self, other: Self) -> Self {
        if other.start_opacity != 1.0 || other.end_opacity != 1.0 {
            self.start_opacity = other.start_opacity;
            self.end_opacity = other.end_opacity;
        }
        if other.start_offset_x != MotionOffset::None || other.end_offset_x != MotionOffset::None {
            self.start_offset_x = other.start_offset_x;
            self.end_offset_x = other.end_offset_x;
        }
        if other.start_offset_y != MotionOffset::None || other.end_offset_y != MotionOffset::None {
            self.start_offset_y = other.start_offset_y;
            self.end_offset_y = other.end_offset_y;
        }
        if (other.start_scale - 1.0).abs() > 1e-4 || (other.end_scale - 1.0).abs() > 1e-4 {
            self.start_scale = other.start_scale;
            self.end_scale = other.end_scale;
        }
        self
    }

    #[inline]
    pub fn resolve_offset_x(&self, progress: f32, width: f32) -> f32 {
        let start = self.start_offset_x.resolve(width);
        let end = self.end_offset_x.resolve(width);
        start + (end - start) * progress
    }

    #[inline]
    pub fn resolve_offset_y(&self, progress: f32, height: f32) -> f32 {
        let start = self.start_offset_y.resolve(height);
        let end = self.end_offset_y.resolve(height);
        start + (end - start) * progress
    }

    #[inline]
    pub fn resolve_opacity(&self, progress: f32) -> f32 {
        self.start_opacity + (self.end_opacity - self.start_opacity) * progress
    }

    #[inline]
    pub fn resolve_scale(&self, progress: f32) -> f32 {
        self.start_scale + (self.end_scale - self.start_scale) * progress
    }
}

/// Layering order for concurrently transitioning views.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransitionOrder {
    /// Incoming view is rendered on top of the outgoing view (push, modals).
    #[default]
    IncomingOnTop,
    /// Outgoing view is rendered on top of the incoming view (pop, dismiss, curtain peel).
    OutgoingOnTop,
    /// Both views share the same local stacking level (cross-fade).
    SameLevel,
}

/// Defines an asymmetric transition with distinct enter and exit motions, layering order,
/// duration, and easing curve.
#[derive(Clone, Debug, PartialEq)]
pub struct PageTransition {
    pub enter: Motion,
    pub exit: Motion,
    pub order: TransitionOrder,
    pub duration_ms: f32,
    pub curve: Curve,
}

impl Default for PageTransition {
    fn default() -> Self {
        Self::fade()
    }
}

impl PageTransition {
    /// Creates an asymmetric transition with distinct enter and exit motions.
    pub fn asymmetric(enter: Motion, exit: Motion) -> Self {
        Self {
            enter,
            exit,
            order: TransitionOrder::default(),
            duration_ms: 220.0,
            curve: Curve::ease_out(),
        }
    }

    /// Sets the stacking order during transition.
    pub fn order(mut self, order: TransitionOrder) -> Self {
        self.order = order;
        self
    }

    /// Sets transition duration in milliseconds.
    pub fn duration_ms(mut self, duration_ms: f32) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Sets transition easing curve.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Standard cross-fade transition (180ms ease-out).
    pub fn fade() -> Self {
        Self {
            enter: Motion::fade_in(),
            exit: Motion::fade_out(),
            order: TransitionOrder::SameLevel,
            duration_ms: 180.0,
            curve: Curve::ease_out(),
        }
    }

    /// Standard navigation push transition (incoming slides in on top, outgoing translates -30% with dimming).
    pub fn push() -> Self {
        Self {
            enter: Motion::slide_in_right(),
            exit: Motion {
                start_offset_x: MotionOffset::None,
                end_offset_x: MotionOffset::Fraction(-0.3),
                start_opacity: 1.0,
                end_opacity: 0.7,
                ..Motion::stationary()
            },
            order: TransitionOrder::IncomingOnTop,
            duration_ms: 260.0,
            curve: Curve::ease_out(),
        }
    }

    /// Standard navigation pop transition (outgoing slides out to right on top, incoming un-dims).
    pub fn pop() -> Self {
        Self {
            enter: Motion {
                start_offset_x: MotionOffset::Fraction(-0.3),
                end_offset_x: MotionOffset::None,
                start_opacity: 0.7,
                end_opacity: 1.0,
                ..Motion::stationary()
            },
            exit: Motion::slide_out_right(),
            order: TransitionOrder::OutgoingOnTop,
            duration_ms: 260.0,
            curve: Curve::ease_out(),
        }
    }

    /// Modal pop-in transition (scaling from 0.94 on top, background view dims to 0.8).
    pub fn modal() -> Self {
        Self {
            enter: Motion::scale_in(0.94),
            exit: Motion {
                start_scale: 1.0,
                end_scale: 0.96,
                start_opacity: 1.0,
                end_opacity: 0.8,
                ..Motion::stationary()
            },
            order: TransitionOrder::IncomingOnTop,
            duration_ms: 200.0,
            curve: Curve::ease_out(),
        }
    }

    /// Slide-over exit where outgoing view animates on top of a stationary incoming view.
    pub fn slide_over_exit() -> Self {
        Self {
            enter: Motion::stationary(),
            exit: Motion::slide_out_right(),
            order: TransitionOrder::OutgoingOnTop,
            duration_ms: 240.0,
            curve: Curve::ease_out(),
        }
    }
}

impl From<Transition> for PageTransition {
    fn from(t: Transition) -> Self {
        match t {
            Transition::SlideUp { duration_ms, curve } => Self {
                enter: Motion::slide_in_bottom(),
                exit: Motion::fade(1.0, 0.5),
                order: TransitionOrder::IncomingOnTop,
                duration_ms,
                curve,
            },
            Transition::SlideDown { duration_ms, curve } => Self {
                enter: Motion::slide_in_top(),
                exit: Motion::fade(1.0, 0.5),
                order: TransitionOrder::IncomingOnTop,
                duration_ms,
                curve,
            },
            Transition::SlideRight { duration_ms, curve } => Self {
                enter: Motion::slide_in_right(),
                exit: Motion::slide_out_left(),
                order: TransitionOrder::IncomingOnTop,
                duration_ms,
                curve,
            },
            Transition::SlideLeft { duration_ms, curve } => Self {
                enter: Motion::slide_in_left(),
                exit: Motion::slide_out_right(),
                order: TransitionOrder::IncomingOnTop,
                duration_ms,
                curve,
            },
            Transition::Fade { duration_ms, curve } => Self {
                enter: Motion::fade_in(),
                exit: Motion::fade_out(),
                order: TransitionOrder::SameLevel,
                duration_ms,
                curve,
            },
            Transition::Scale {
                from_scale,
                duration_ms,
                curve,
            } => Self {
                enter: Motion::scale_in(from_scale),
                exit: Motion::fade_out(),
                order: TransitionOrder::IncomingOnTop,
                duration_ms,
                curve,
            },
            Transition::None => Self {
                enter: Motion::stationary(),
                exit: Motion::stationary(),
                order: TransitionOrder::SameLevel,
                duration_ms: 0.0,
                curve: Curve::linear(),
            },
        }
    }
}
