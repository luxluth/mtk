#![allow(non_upper_case_globals)]

use bytemuck::{Pod, Zeroable};

/// RGBA defined color values (8 bits per channel: Red, Green, Blue, Alpha).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Encodes this color into a 32-bit integer matching the native RGBA8 memory layout (`[R, G, B, A]`).
    #[inline]
    pub const fn to_rgba_u32(&self) -> u32 {
        u32::from_ne_bytes([self.r, self.g, self.b, self.a])
    }

    /// Decodes a 32-bit RGBA8 value into a `Color`.
    #[inline]
    pub const fn from_rgba_u32(val: u32) -> Self {
        let [r, g, b, a] = val.to_ne_bytes();
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn as_u32(&self) -> u32 {
        self.to_rgba_u32()
    }

    /// Linearly interpolates between `self` and `target` by factor `t`.
    ///
    /// Each channel (R, G, B, A) is interpolated independently:
    ///
    /// ```text
    /// channel = self + (target - self) * t
    /// ```
    ///
    /// ## Arguments
    ///
    /// * `target` - The destination color to interpolate toward.
    /// * `t` - The interpolation parameter. Clamped to the range `[0.0, 1.0]`.
    ///   A value of `0.0` yields `self`, while `1.0` yields `target`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use mtk::Color;
    ///
    /// let black = Color::black;
    /// let white = Color::white;
    /// let mid_gray = black.lerp(&white, 0.5);
    ///
    /// assert_eq!(mid_gray.r, 128);
    /// assert_eq!(mid_gray.g, 128);
    /// assert_eq!(mid_gray.b, 128);
    /// ```
    pub fn lerp(&self, target: &Color, t: f64) -> Color {
        let t_f = t.clamp(0.0, 1.0) as f32;
        let r = (self.r as f32 + (target.r as f32 - self.r as f32) * t_f).round() as u8;
        let g = (self.g as f32 + (target.g as f32 - self.g as f32) * t_f).round() as u8;
        let b = (self.b as f32 + (target.b as f32 - self.b as f32) * t_f).round() as u8;
        let a = (self.a as f32 + (target.a as f32 - self.a as f32) * t_f).round() as u8;

        Color { r, g, b, a }
    }

    /// Computes the relative luminance of the color according to the WCAG 2.1 standard.
    ///
    /// Relative luminance represents the perceived brightness of any color,
    /// normalized from `0.0` (pure black) to `1.0` (pure white).
    ///
    /// ## Algorithm
    ///
    /// 1. Converts non-linear sRGB channels (`0` to `255`) to linear light (`0.0` to `1.0`):
    ///
    ///    ```text
    ///    c = channel / 255.0
    ///
    ///    linear = if c <= 0.04045 {
    ///        c / 12.92
    ///    } else {
    ///        ((c + 0.055) / 1.055) ^ 2.4
    ///    }
    ///    ```
    ///
    /// 2. Applies ITU-R BT.709 spectral sensitivity weights:
    ///
    ///    ```text
    ///    luminance = 0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear
    ///    ```
    ///
    /// ## Notes
    ///
    /// * Operates only on RGB channels and ignores alpha transparency.
    /// * Translucent colors should be composited over an opaque surface first.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use mtk::Color;
    ///
    /// assert_eq!(Color::black.relative_luminance(), 0.0);
    /// assert_eq!(Color::white.relative_luminance(), 1.0);
    /// ```
    pub fn relative_luminance(&self) -> f64 {
        let to_linear = |channel: u8| -> f64 {
            let c = channel as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };

        let r_lin = to_linear(self.r);
        let g_lin = to_linear(self.g);
        let b_lin = to_linear(self.b);

        0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin
    }

    /// Computes the WCAG 2.1 contrast ratio between this color and another.
    ///
    /// Returns a value ranging from `1.0` (no contrast) up to `21.0`
    /// (maximum contrast, such as black against white).
    ///
    /// ## Formula
    ///
    /// ```text
    /// ratio = (lighter_luminance + 0.05) / (darker_luminance + 0.05)
    /// ```
    ///
    /// ### WCAG 2.1 Standards
    ///
    /// * `3.0:1` - Minimum for large text (18pt+ or 14pt+ bold) and UI components (AA).
    /// * `4.5:1` - Minimum for regular body text (AA).
    /// * `7.0:1` - Enhanced contrast for regular body text (AAA).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use mtk::Color;
    ///
    /// let ratio = Color::white.contrast_ratio(&Color::black);
    /// assert!((ratio - 21.0).abs() < 0.01);
    /// ```
    pub fn contrast_ratio(&self, other: &Color) -> f64 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();

        let lighter = l1.max(l2);
        let darker = l1.min(l2);

        (lighter + 0.05) / (darker + 0.05)
    }

    /// Selects pure white or pure black to maximize contrast against this background.
    ///
    /// Matches standard design system utilities (such as Material UI `getContrastText`).
    /// Computes contrast against both [`Color::white`] and [`Color::black`],
    /// returning the one with the higher ratio.
    ///
    /// Due to the `+ 0.05` offset in the WCAG contrast formula, the crossover
    /// point is roughly `0.179` relative luminance rather than `0.5`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use mtk::Color;
    ///
    /// assert_eq!(Color::black.get_contrast_text(), Color::white);
    /// assert_eq!(Color::white.get_contrast_text(), Color::black);
    /// ```
    pub fn get_contrast_text(&self) -> Color {
        let white_ratio = self.contrast_ratio(&Self::white);
        let black_ratio = self.contrast_ratio(&Self::black);

        if white_ratio >= black_ratio {
            Self::white
        } else {
            Self::black
        }
    }

    /// Generates a softened, tinted text color derived from this background
    /// while guaranteeing a minimum WCAG contrast ratio.
    ///
    /// Instead of returning pure black or white, this method blends a fraction
    /// of the background color into an off-white ([`Color::off_white`]) or
    /// off-black ([`Color::off_black`]) base tone.
    ///
    /// ## Selection Logic
    ///
    /// 1. Picks [`Color::off_white`] or [`Color::off_black`] based on which gives
    ///    higher contrast.
    /// 2. If the chosen base cannot reach `min_contrast` even without tinting,
    ///    falls back directly to pure [`Color::white`] or [`Color::black`].
    /// 3. Tests candidate colors by stepping down the tint factor from `max_tint`
    ///    to `0.0` until the contrast ratio meets or exceeds `min_contrast`.
    ///
    /// ## Arguments
    ///
    /// * `min_contrast` - Minimum contrast ratio to satisfy (for example, `4.5` for AA body text).
    /// * `max_tint` - Maximum background blend factor in the range `[0.0, 1.0]`.
    ///   Values between `0.08` and `0.15` (8% to 15%) provide subtle tinting without
    ///   breaking accessibility.
    ///
    /// ## Examples
    ///
    /// ```
    /// use mtk::Color;
    ///
    /// let navy = Color::Hex(0x0a192fff);
    /// let text_color = navy.get_tinted_contrast_text(4.5, 0.12);
    ///
    /// assert!(navy.contrast_ratio(&text_color) >= 4.5);
    /// ```
    pub fn get_tinted_contrast_text(&self, min_contrast: f64, max_tint: f64) -> Color {
        let white_ratio = self.contrast_ratio(&Self::off_white);
        let black_ratio = self.contrast_ratio(&Self::off_black);

        let base_text = if white_ratio >= black_ratio {
            Self::off_white
        } else {
            Self::off_black
        };

        // If the base color cannot hit the target, return untinted pure fallback
        if self.contrast_ratio(&base_text) < min_contrast {
            return if white_ratio >= black_ratio {
                Self::white
            } else {
                Self::black
            };
        }

        let steps = 15;
        for step in (0..=steps).rev() {
            let current_tint = (step as f64 / steps as f64) * max_tint;
            let candidate = base_text.lerp(self, current_tint);

            if self.contrast_ratio(&candidate) >= min_contrast {
                return candidate;
            }
        }

        base_text
    }
}

impl Color {
    /// Composites this color over an opaque backdrop using standard alpha blending.
    pub fn over(&self, backdrop: &Color) -> Color {
        if self.a == 255 {
            return *self;
        }
        if self.a == 0 {
            return *backdrop;
        }

        let alpha = self.a as f32 / 255.0;
        let blend = |fg: u8, bg: u8| -> u8 {
            ((fg as f32 * alpha) + (bg as f32 * (1.0 - alpha))).round() as u8
        };

        Color {
            r: blend(self.r, backdrop.r),
            g: blend(self.g, backdrop.g),
            b: blend(self.b, backdrop.b),
            a: 255,
        }
    }
}

impl Color {
    /// Determines an accessible, tinted text color for this background.
    /// `backdrop` is the surface behind this color (used if `self.a < 255`).
    pub fn get_tinted_contrast_text_on(
        &self,
        backdrop: &Color,
        min_contrast: f64,
        max_tint: f64,
    ) -> Color {
        let effective_bg = self.over(backdrop);

        let white_ratio = effective_bg.contrast_ratio(&Self::off_white);
        let black_ratio = effective_bg.contrast_ratio(&Self::off_black);

        let base_text = if white_ratio >= black_ratio {
            Self::off_white
        } else {
            Self::off_black
        };

        if effective_bg.contrast_ratio(&base_text) < min_contrast {
            return if white_ratio >= black_ratio {
                Self::white
            } else {
                Self::black
            };
        }

        let steps = 15;
        for step in (0..=steps).rev() {
            let current_tint = (step as f64 / steps as f64) * max_tint;
            let mut candidate = base_text.lerp(&effective_bg, current_tint);
            candidate.a = 255;

            if effective_bg.contrast_ratio(&candidate) >= min_contrast {
                return candidate;
            }
        }

        base_text
    }
}

impl Color {
    /// Produces the photographic negative by inverting RGB channels.
    #[inline]
    pub const fn invert(&self) -> Color {
        Color {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
            a: self.a,
        }
    }
}

impl From<Color> for u32 {
    #[inline]
    fn from(c: Color) -> Self {
        c.to_rgba_u32()
    }
}

impl From<u32> for Color {
    #[inline]
    fn from(val: u32) -> Self {
        Color::from_rgba_u32(val)
    }
}

impl Color {
    /// Red color
    pub const red: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Green color
    pub const green: Color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };

    /// Blue color
    pub const blue: Color = Color {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };

    /// White color
    pub const white: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    /// Black color
    pub const black: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    // Off Black Color
    pub const off_black: Color = Color {
        r: 18,
        g: 18,
        b: 24,
        a: 255,
    };

    // Off White Color
    pub const off_white: Color = Color {
        r: 248,
        g: 249,
        b: 250,
        a: 255,
    };

    /// Transparent color
    pub const transparent: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    /// Dodger Blue - a nice color
    pub const dodger_blue: Color = Color {
        r: 30,
        g: 144,
        b: 255,
        a: 255,
    };

    /// LL Blue - I like this one a lot
    pub const ll_blue: Color = Color::Hex(0x4455eeff);

    /// Firefox Blue - Firefox selection blue color
    pub const firefox_blue: Color = Color::Hex(0x3584e4ff);

    /// Transform raw hex into RGBA componnent
    /// **FORMAT: RRGGBBAA**
    #[allow(non_snake_case)]
    pub const fn Hex(hex: u32) -> Color {
        let r = ((hex >> (8 * 3)) & 0xFF) as u8;
        let g = ((hex >> (8 * 2)) & 0xFF) as u8;
        let b = ((hex >> (8 * 1)) & 0xFF) as u8;
        let a = (hex & 0xFF) as u8;

        Color { r, g, b, a }
    }

    /// Set an alpha value for the color
    pub fn with_alpha(mut self, value: u8) -> Self {
        self.a = value;
        self
    }
}

impl Color {
    /// Creates a new Color from HSL values.
    ///
    /// * `h` - Hue in degrees (0.0 - 360.0)
    /// * `s` - Saturation (0.0 - 1.0)
    /// * `l` - Lightness (0.0 - 1.0)
    pub const fn from_hsl(h: f32, mut s: f32, mut l: f32) -> Self {
        if s > 1.0 {
            s /= 100.0;
        }
        if l > 1.0 {
            l /= 100.0;
        }
        let r;
        let g;
        let b;

        if s == 0.0 {
            // Achromatic (Grey)
            r = l;
            g = l;
            b = l;
        } else {
            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;

            // Normalize Hue to 0.0 - 1.0
            let h_norm = h / 360.0;

            r = hue_to_rgb(p, q, h_norm + 1.0 / 3.0);
            g = hue_to_rgb(p, q, h_norm);
            b = hue_to_rgb(p, q, h_norm - 1.0 / 3.0);
        }

        Self {
            r: (r * 255.0).round() as u8,
            g: (g * 255.0).round() as u8,
            b: (b * 255.0).round() as u8,
            a: 255, // Default opaque
        }
    }

    /// Same as from_hsl, but with an alpha channel (0.0 - 1.0)
    pub const fn from_hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let mut color = Self::from_hsl(h, s, l);
        color.a = (a * 255.0).round() as u8;
        color
    }
}

// Helper function for HSL conversion
const fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }

    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    return p;
}

impl Default for Color {
    fn default() -> Self {
        Color::transparent
    }
}

fn srgb_to_linear(c: u8) -> f32 {
    let f = c as f32 / 255.0;
    if f <= 0.04045 {
        f / 12.92
    } else {
        ((f + 0.055) / 1.055).powf(2.4)
    }
}

impl Color {
    /// Converts the sRGB color to linear RGB (often required for GPU shaders).
    pub fn to_linear_rgba_f32(&self) -> [f32; 4] {
        [
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
            self.a as f32 / 255.0,
        ]
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        color.to_linear_rgba_f32()
    }
}

impl From<Color> for [u8; 4] {
    fn from(color: Color) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

pub mod macros {
    /// Defines a color using a named preset or a Hex literal.
    ///
    /// # Hex Format
    /// When using a literal, the format must be **0xRRGGBBAA**.
    /// * **RR**: Red (00-FF)
    /// * **GG**: Green (00-FF)
    /// * **BB**: Blue (00-FF)
    /// * **AA**: Alpha (00-FF, where FF is opaque)
    ///
    /// # Examples
    /// ```rust,ignore
    /// clr!(red);           // Named color
    /// clr!(0xFF0000FF);    // Opaque Red
    /// clr!(0x00FF0080);    // Semi-transparent Green
    /// ```
    #[macro_export]
    macro_rules! clr {
        ($name:ident) => {
            $crate::colors::Color::$name
        };
        ($hex:literal) => {
            $crate::colors::Color::Hex($hex)
        };
    }

    /// Creates a solid opaque color from RGB components.
    ///
    /// Arguments should be `u8` (0-255). Alpha is set to 255 (Opaque).
    ///
    /// # Example
    /// ```rust,ignore
    /// rgb!(255, 0, 0) // Red
    /// ```
    #[macro_export]
    macro_rules! rgb {
        ($r:expr, $g:expr, $b:expr) => {
            $crate::colors::Color {
                r: $r,
                g: $g,
                b: $b,
                a: 255,
            }
        };
    }

    /// Creates a color from RGBA components.
    ///
    /// Arguments should be `u8` (0-255).
    ///
    /// # Example
    /// ```rust,ignore
    /// rgba!(255, 0, 0, 128) // 50% transparent Red
    /// ```
    #[macro_export]
    macro_rules! rgba {
        ($r:expr, $g:expr, $b:expr, $a:expr) => {
            $crate::colors::Color {
                r: $r,
                g: $g,
                b: $b,
                a: $a,
            }
        };
    }

    /// Creates a color from Hue, Saturation, and Lightness.
    ///
    /// # Arguments
    /// * `h` - Hue in degrees (0 - 360)
    /// * `s` - Saturation (0.0 - 1.0)
    /// * `l` - Lightness (0.0 - 1.0)
    ///
    /// # Example
    /// ```rust,ignore
    /// let red = hsl!(0, 1.0, 0.5);
    /// let pastel_blue = hsl!(200, 0.7, 0.8);
    /// ```
    #[macro_export]
    macro_rules! hsl {
        ($h:expr, $s:expr, $l:expr) => {
            $crate::colors::Color::from_hsl($h as f32, $s as f32, $l as f32)
        };
    }

    /// Creates a color from Hue, Saturation, Lightness, and Alpha.
    ///
    /// # Arguments
    /// * `h` - Hue in degrees (0 - 360)
    /// * `s` - Saturation (0.0 - 1.0)
    /// * `l` - Lightness (0.0 - 1.0)
    /// * `a` - Alpha (0.0 - 1.0)
    ///
    /// # Example
    /// ```rust,ignore
    /// let transparent_red = hsla!(0, 1.0, 0.5, 0.5);
    /// ```
    #[macro_export]
    macro_rules! hsla {
        ($h:expr, $s:expr, $l:expr, $a:expr) => {
            $crate::colors::Color::from_hsla($h as f32, $s as f32, $l as f32, $a as f32)
        };
    }
}
