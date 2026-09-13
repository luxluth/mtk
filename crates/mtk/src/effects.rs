use crate::colors::Color;

macro_rules! corners_component {
    ($name:ident, $t:ty) => {
        #[derive(Debug, Default, Clone, Copy, PartialEq)]
        pub struct $name {
            pub tl: $t,
            pub tr: $t,
            pub bl: $t,
            pub br: $t,
        }

        impl $name {
            /// Creates an instance with the same value applied to all four corners.
            pub fn all(val: $t) -> Self {
                Self {
                    tl: val,
                    tr: val,
                    bl: val,
                    br: val,
                }
            }

            /// Creates an instance with the value applied to top corners only.
            pub fn top(val: $t) -> Self
            where
                $t: Default,
            {
                Self {
                    tl: val,
                    tr: val,
                    bl: <$t>::default(),
                    br: <$t>::default(),
                }
            }

            /// Creates an instance with the value applied to bottom corners only.
            pub fn bottom(val: $t) -> Self
            where
                $t: Default,
            {
                Self {
                    tl: <$t>::default(),
                    tr: <$t>::default(),
                    bl: val,
                    br: val,
                }
            }

            /// Creates an instance with the value applied to left corners only.
            pub fn left(val: $t) -> Self
            where
                $t: Default,
            {
                Self {
                    tl: val,
                    tr: <$t>::default(),
                    bl: val,
                    br: <$t>::default(),
                }
            }

            /// Creates an instance with the value applied to right corners only.
            pub fn right(val: $t) -> Self
            where
                $t: Default,
            {
                Self {
                    tl: <$t>::default(),
                    tr: val,
                    bl: <$t>::default(),
                    br: val,
                }
            }
        }
    };
}

#[derive(Clone, Debug, PartialEq, Copy, Default)]
pub struct Border {
    pub color: Color,
    pub radius: Radius,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct BoxShadow {
    pub color: Color,
    pub offset: [f32; 2],
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub inset: bool,
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self {
            color: Color::transparent,
            offset: [0.0, 0.0],
            blur_radius: 0.0,
            spread_radius: 0.0,
            inset: false,
        }
    }
}

impl BoxShadow {
    /// Creates a new outer box shadow with the specified color.
    pub const fn new(color: Color) -> Self {
        Self {
            color,
            offset: [0.0, 0.0],
            blur_radius: 0.0,
            spread_radius: 0.0,
            inset: false,
        }
    }

    /// Creates an outer (drop) shadow with the specified color.
    pub const fn drop(color: Color) -> Self {
        Self::new(color)
    }

    /// Creates an inner (inset) shadow with the specified color.
    pub const fn inset(color: Color) -> Self {
        Self {
            inset: true,
            ..Self::new(color)
        }
    }

    /// Sets the color of the box shadow.
    pub const fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets the horizontal and vertical offset displacement `(x, y)` in logical pixels.
    pub const fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = [x, y];
        self
    }

    /// Sets the blur radius in logical pixels.
    pub const fn blur(mut self, blur: f32) -> Self {
        self.blur_radius = blur;
        self
    }

    /// Sets the spread radius in logical pixels.
    pub const fn spread(mut self, spread: f32) -> Self {
        self.spread_radius = spread;
        self
    }

    /// Sets whether this shadow is an inner/inset shadow.
    pub const fn is_inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    /// Returns a zero-offset, zero-blur transparent shadow.
    pub const fn none() -> Self {
        Self {
            color: Color::transparent,
            offset: [0.0, 0.0],
            blur_radius: 0.0,
            spread_radius: 0.0,
            inset: false,
        }
    }

    /// Small subtle elevation (0 1px 2px 0 rgba(0, 0, 0, 0.05)).
    pub fn sm() -> Self {
        Self::drop(Color::new(0, 0, 0, 13))
            .offset(0.0, 1.0)
            .blur(2.0)
    }

    /// Standard elevation (0 1px 3px 0 rgba(0, 0, 0, 0.1)).
    pub fn base() -> Self {
        Self::drop(Color::new(0, 0, 0, 26))
            .offset(0.0, 1.0)
            .blur(3.0)
    }

    /// Medium elevation for cards and tooltips (0 4px 6px -1px rgba(0, 0, 0, 0.1)).
    pub fn md() -> Self {
        Self::drop(Color::new(0, 0, 0, 26))
            .offset(0.0, 4.0)
            .blur(6.0)
            .spread(-1.0)
    }

    /// Large elevation for floating panels and menus (0 10px 15px -3px rgba(0, 0, 0, 0.1)).
    pub fn lg() -> Self {
        Self::drop(Color::new(0, 0, 0, 26))
            .offset(0.0, 10.0)
            .blur(15.0)
            .spread(-3.0)
    }

    /// Extra large elevation for modals and dialogs (0 20px 25px -5px rgba(0, 0, 0, 0.1)).
    pub fn xl() -> Self {
        Self::drop(Color::new(0, 0, 0, 26))
            .offset(0.0, 20.0)
            .blur(25.0)
            .spread(-5.0)
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Filter {
    Blur {
        vibrancy: f32,
        vibrancy_darkness: f32,
        passes: f32,
    },
}

impl Filter {
    pub fn classic_blur() -> Self {
        Filter::Blur {
            vibrancy: 0.4,
            vibrancy_darkness: 0.2,
            passes: 4.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Effects {
    pub background_color: Color,
    pub border: Border,
    pub box_shadow: BoxShadow,
    pub additional_shadows: Vec<BoxShadow>,
    pub filters: Vec<Filter>,
    pub opacity: f32,
    pub scale: f32,
    pub explicit_opacity: bool,
    pub explicit_scale: bool,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            background_color: Color::transparent,
            opacity: 1.,
            scale: 1.,
            border: Border::default(),
            box_shadow: BoxShadow::default(),
            additional_shadows: Vec::new(),
            filters: Vec::new(),
            explicit_opacity: false,
            explicit_scale: false,
        }
    }
}

impl Effects {
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self.explicit_opacity = true;
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self.explicit_scale = true;
        self
    }

    pub fn box_shadow(mut self, shadow: BoxShadow) -> Self {
        self.box_shadow = shadow;
        self
    }
}

corners_component!(Radius, f32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_shadow_builder() {
        let shadow = BoxShadow::new(Color::new(10, 20, 30, 200))
            .offset(4.0, 8.0)
            .blur(12.0)
            .spread(2.0)
            .is_inset(false);

        assert_eq!(shadow.color, Color::new(10, 20, 30, 200));
        assert_eq!(shadow.offset, [4.0, 8.0]);
        assert_eq!(shadow.blur_radius, 12.0);
        assert_eq!(shadow.spread_radius, 2.0);
        assert!(!shadow.inset);

        let inset_shadow = BoxShadow::inset(Color::new(0, 0, 0, 100))
            .offset(0.0, 2.0)
            .blur(4.0);
        assert!(inset_shadow.inset);
    }

    #[test]
    fn test_box_shadow_presets() {
        let none = BoxShadow::none();
        assert_eq!(none.color, Color::transparent);
        assert_eq!(none.blur_radius, 0.0);

        let sm = BoxShadow::sm();
        assert_eq!(sm.offset, [0.0, 1.0]);
        assert_eq!(sm.blur_radius, 2.0);
        assert!(!sm.inset);

        let xl = BoxShadow::xl();
        assert_eq!(xl.offset, [0.0, 20.0]);
        assert_eq!(xl.blur_radius, 25.0);
        assert_eq!(xl.spread_radius, -5.0);
    }

    #[test]
    fn test_effects_multiple_box_shadows() {
        let s1 = BoxShadow::sm();
        let s2 = BoxShadow::md();
        let effects = Effects {
            box_shadow: s1,
            additional_shadows: vec![s2],
            ..Default::default()
        };

        assert_eq!(effects.box_shadow, s1);
        assert_eq!(effects.additional_shadows.len(), 1);
        assert_eq!(effects.additional_shadows[0], s2);
    }
}
