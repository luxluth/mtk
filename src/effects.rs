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
        }
    };
}

#[derive(Clone, Debug, PartialEq, Copy, Default)]
pub struct Border {
    pub color: Color,
    pub radius: Radius,
}

#[derive(Clone, Debug, PartialEq, Copy, Default)]
pub struct Shadow {
    pub color: Color,
    pub spread: f32,
    pub power: f32,
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
    pub shadow: Shadow,
    pub filters: Vec<Filter>,
    pub opacity: f32,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            background_color: Color::transparent,
            opacity: 1.,
            border: Border::default(),
            shadow: Shadow::default(),
            filters: Vec::new(),
        }
    }
}

corners_component!(Radius, f32);
