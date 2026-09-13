use parley::layout::Alignment;
use parley::style::{FontStyle, FontWeight, OverflowWrap};
use std::borrow::Cow;

pub use mtk_layout::{
    AbsoluteBuilder, AlignItems, AlignSelf, Computed, Constraints, Edges, FlexDirection, FlexWrap,
    IntoPositionStrategy, JustifyContent, Overflow, PositionStrategy, Rect, Size, Vector2,
};

use crate::animation::Curve;
use crate::clr;
use crate::colors::Color;
use crate::effects::{BoxShadow, Effects, Filter, Radius};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum LineHeight {
    Relative(f32),
    #[default]
    Auto,
}

impl LineHeight {
    pub fn resolve(&self) -> f32 {
        match self {
            LineHeight::Relative(f) => *f,
            LineHeight::Auto => 1.2, // I think this was 1.2
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: LineHeight,
    pub color: Color,
    pub font_family: Cow<'static, str>,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub alignment: Alignment,
    pub vertical_alignment: VerticalAlignment,
    pub wrap: bool,
    pub overflow_wrap: OverflowWrap,
    pub selection_color: Color,
    pub selection_bg: Color,
    pub caret_color: Color,
    pub strikethrough: bool,
    pub underline: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: LineHeight::Auto,
            color: clr!(black),
            font_family: Cow::Borrowed("system-ui"),
            font_weight: FontWeight::default(),
            font_style: FontStyle::default(),
            alignment: Alignment::Start,
            vertical_alignment: VerticalAlignment::Top,
            wrap: false,
            overflow_wrap: OverflowWrap::default(),
            selection_color: clr!(white),
            selection_bg: clr!(ll_blue),
            caret_color: clr!(black),
            strikethrough: false,
            underline: false,
        }
    }
}

/// Target properties that can be transitioned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionProperty {
    /// Transitions all changing properties simultaneously.
    All,
    Padding,
    Border,
    Width,
    Height,
    Size,
    Gap,
    BackgroundColor,
    BorderColor,
    CornerRadius,
    Scale,
    Opacity,
    BoxShadow,
    TextColor,
    FontSize,
    Scrollbar,
}

/// Backwards compatibility alias for [`TransitionProperty`].
pub type AnimationTarget = TransitionProperty;

/// Configuration for a smooth property transition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub property: TransitionProperty,
    pub duration_ms: f64,
    pub curve: Curve,
}

impl Transition {
    pub fn new(property: TransitionProperty, duration_ms: f64, curve: Curve) -> Self {
        Self {
            property,
            duration_ms,
            curve,
        }
    }
}

impl Effects {
    /// Merges non-default visual effects from `other` into `self`.
    pub fn merge(&mut self, other: &Effects) {
        if other.background_color != Color::transparent {
            self.background_color = other.background_color;
        }
        if other.border.color != Color::transparent {
            self.border.color = other.border.color;
        }
        if other.border.radius != crate::effects::Radius::all(0.0) {
            self.border.radius = other.border.radius;
        }
        if other.box_shadow != BoxShadow::default() {
            self.box_shadow = other.box_shadow;
        }
        if !other.additional_shadows.is_empty() {
            self.additional_shadows = other.additional_shadows.clone();
        }
        if !other.filters.is_empty() {
            self.filters = other.filters.clone();
        }
        if other.explicit_opacity || (other.opacity - 1.0).abs() > 1e-4 {
            self.opacity = other.opacity;
            self.explicit_opacity = true;
        }
        if other.explicit_scale || (other.scale - 1.0).abs() > 1e-4 {
            self.scale = other.scale;
            self.explicit_scale = true;
        }
    }
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn size(self, size: f32) -> Self {
        self.font_size(size)
    }

    pub fn line_height(mut self, line_height: LineHeight) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn font_family(mut self, family: impl Into<Cow<'static, str>>) -> Self {
        self.font_family = family.into();
        self
    }

    pub fn family(self, family: impl Into<Cow<'static, str>>) -> Self {
        self.font_family(family)
    }

    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }

    pub fn weight(self, weight: FontWeight) -> Self {
        self.font_weight(weight)
    }

    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.font_style = style;
        self
    }

    pub fn italic(self) -> Self {
        self.font_style(FontStyle::Italic)
    }

    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn align(self, alignment: Alignment) -> Self {
        self.alignment(alignment)
    }

    pub fn vertical_alignment(mut self, alignment: VerticalAlignment) -> Self {
        self.vertical_alignment = alignment;
        self
    }

    pub fn vertical_align(self, alignment: VerticalAlignment) -> Self {
        self.vertical_alignment(alignment)
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn overflow_wrap(mut self, overflow_wrap: OverflowWrap) -> Self {
        self.overflow_wrap = overflow_wrap;
        self
    }

    pub fn selection_color(mut self, color: Color) -> Self {
        self.selection_color = color;
        self
    }

    pub fn selection_bg(mut self, color: Color) -> Self {
        self.selection_bg = color;
        self
    }

    pub fn caret_color(mut self, color: Color) -> Self {
        self.caret_color = color;
        self
    }

    pub fn strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = strikethrough;
        self
    }

    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// Merges non-default typography styling from `other` into `self`.
    pub fn merge(&mut self, other: &TextStyle) {
        if (other.font_size - 16.0).abs() > 1e-4 {
            self.font_size = other.font_size;
        }
        if other.line_height != LineHeight::Auto {
            self.line_height = other.line_height;
        }
        if other.color != clr!(black) {
            self.color = other.color;
        }
        if other.font_family != "system-ui" {
            self.font_family = other.font_family.clone();
        }
        if other.font_weight != FontWeight::default() {
            self.font_weight = other.font_weight;
        }
        if other.font_style != FontStyle::default() {
            self.font_style = other.font_style;
        }
        if other.alignment != Alignment::Start {
            self.alignment = other.alignment;
        }
        if other.vertical_alignment != VerticalAlignment::Top {
            self.vertical_alignment = other.vertical_alignment;
        }
        if other.wrap {
            self.wrap = other.wrap;
        }
        if other.strikethrough {
            self.strikethrough = other.strikethrough;
        }
        if other.underline {
            self.underline = other.underline;
        }
        if other.caret_color != clr!(black) {
            self.caret_color = other.caret_color;
        }
    }
}

/// Visibility mode for a container's scrollbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScrollbarVisibility {
    /// Shown automatically when content overflows the viewport.
    #[default]
    Auto,
    /// Always visible even if content fits without overflowing.
    Always,
    /// Never visible (disables rendering and hit-testing; same as `no_scrollbar`).
    Never,
}

/// Declarative styling parameters for Material-style segmented scrollbars.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollbarStyle {
    /// Thickness of the scrollbar thumb and track in logical pixels.
    pub width: f32,
    /// Inset margin from the viewport edge in logical pixels.
    pub margin: f32,
    /// Air gap between the thumb and the top/bottom track segments.
    pub gap: f32,
    /// Color of the draggable thumb pill.
    pub thumb_color: Color,
    /// Optional color for the top/bottom track segments (None = no track rendered).
    pub track_color: Option<Color>,
    /// Corner border radii for the thumb and track pieces.
    pub radius: Radius,
    /// Minimum thumb length in logical pixels.
    pub min_thumb_len: f32,
    /// Visibility mode.
    pub visibility: ScrollbarVisibility,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            width: 4.0,
            margin: 2.0,
            gap: 2.0,
            thumb_color: Color::new(102, 102, 102, 128),
            track_color: None,
            radius: Radius::all(2.0),
            min_thumb_len: 20.0,
            visibility: ScrollbarVisibility::Auto,
        }
    }
}

impl ScrollbarStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: &ScrollbarStyle) {
        if (other.width - 4.0).abs() > 1e-4 {
            self.width = other.width;
        }
        if (other.margin - 2.0).abs() > 1e-4 {
            self.margin = other.margin;
        }
        if (other.gap - 2.0).abs() > 1e-4 {
            self.gap = other.gap;
        }
        if other.thumb_color != Color::new(102, 102, 102, 128) {
            self.thumb_color = other.thumb_color;
        }
        if other.track_color.is_some() {
            self.track_color = other.track_color;
        }
        if other.radius != Radius::all(2.0) {
            self.radius = other.radius;
        }
        if (other.min_thumb_len - 20.0).abs() > 1e-4 {
            self.min_thumb_len = other.min_thumb_len;
        }
        if other.visibility != ScrollbarVisibility::Auto {
            self.visibility = other.visibility;
        }
    }
}

/// Compact bitmask tracking which properties have been explicitly set on a [`Style`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StyleFlags(pub u64);

impl StyleFlags {
    pub const EMPTY: Self = Self(0);
    pub const WIDTH: Self = Self(1 << 0);
    pub const HEIGHT: Self = Self(1 << 1);
    pub const MIN_WIDTH: Self = Self(1 << 2);
    pub const MAX_WIDTH: Self = Self(1 << 3);
    pub const MIN_HEIGHT: Self = Self(1 << 4);
    pub const MAX_HEIGHT: Self = Self(1 << 5);
    pub const PADDING: Self = Self(1 << 6);
    pub const BORDER_WIDTH: Self = Self(1 << 7);
    pub const GAP: Self = Self(1 << 8);
    pub const FLEX_GROW: Self = Self(1 << 9);
    pub const FLEX_SHRINK: Self = Self(1 << 10);
    pub const FLEX_BASIS: Self = Self(1 << 11);
    pub const FLEX_DIRECTION: Self = Self(1 << 12);
    pub const FLEX_WRAP: Self = Self(1 << 13);
    pub const JUSTIFY_CONTENT: Self = Self(1 << 14);
    pub const ALIGN_ITEMS: Self = Self(1 << 15);
    pub const ALIGN_SELF: Self = Self(1 << 16);
    pub const ASPECT_RATIO: Self = Self(1 << 17);
    pub const OVERFLOW: Self = Self(1 << 18);
    pub const POSITIONING: Self = Self(1 << 19);
    pub const Z_INDEX: Self = Self(1 << 20);

    pub const BG_COLOR: Self = Self(1 << 21);
    pub const BORDER_COLOR: Self = Self(1 << 22);
    pub const BORDER_RADIUS: Self = Self(1 << 23);
    pub const BOX_SHADOW: Self = Self(1 << 24);
    pub const OPACITY: Self = Self(1 << 25);
    pub const SCALE: Self = Self(1 << 26);
    pub const FILTERS: Self = Self(1 << 27);

    pub const FONT_SIZE: Self = Self(1 << 28);
    pub const FONT_FAMILY: Self = Self(1 << 29);
    pub const FONT_WEIGHT: Self = Self(1 << 30);
    pub const FONT_STYLE: Self = Self(1 << 31);
    pub const TEXT_COLOR: Self = Self(1 << 32);
    pub const TEXT_ALIGN: Self = Self(1 << 33);
    pub const VERTICAL_ALIGN: Self = Self(1 << 34);
    pub const WRAP: Self = Self(1 << 35);
    pub const LINE_HEIGHT: Self = Self(1 << 36);
    pub const UNDERLINE: Self = Self(1 << 37);
    pub const STRIKETHROUGH: Self = Self(1 << 38);
    pub const CARET_COLOR: Self = Self(1 << 39);
    pub const SELECTION_COLOR: Self = Self(1 << 40);
    pub const SELECTION_BG: Self = Self(1 << 41);

    pub const SCROLLBAR: Self = Self(1 << 42);

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    #[inline]
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

impl std::ops::BitOr for StyleFlags {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for StyleFlags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for StyleFlags {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Declarative styling overrides for interactive pseudo-states (hover, active, focus, disabled).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InteractiveStyles {
    pub hover: Option<Style>,
    pub active: Option<Style>,
    pub focus: Option<Style>,
    pub disabled: Option<Style>,
}

/// Declarative styling container defining layout constraints, visual effects, typography, pseudo-states, and transitions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub base_constraints: Constraints,
    pub base_effects: Effects,
    pub base_text_style: TextStyle,
    pub flex_direction: Option<FlexDirection>,
    pub scrollbar: Option<Box<ScrollbarStyle>>,
    pub flags: StyleFlags,
    pub interaction: Option<Box<InteractiveStyles>>,
    pub transitions: Vec<Transition>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn hover(&self) -> Option<&Style> {
        self.interaction.as_ref().and_then(|i| i.hover.as_ref())
    }

    #[inline]
    pub fn active(&self) -> Option<&Style> {
        self.interaction.as_ref().and_then(|i| i.active.as_ref())
    }

    #[inline]
    pub fn focus(&self) -> Option<&Style> {
        self.interaction.as_ref().and_then(|i| i.focus.as_ref())
    }

    #[inline]
    pub fn disabled(&self) -> Option<&Style> {
        self.interaction.as_ref().and_then(|i| i.disabled.as_ref())
    }

    /// Merges `other` into `self`, overriding conflicting properties while preserving non-conflicting base styles.
    pub fn merge(mut self, other: Style) -> Self {
        if other.base_constraints != Constraints::default() {
            self.base_constraints.merge(&other.base_constraints);
        }
        if other.base_effects != Effects::default() {
            self.base_effects.merge(&other.base_effects);
        }
        if other.base_text_style != TextStyle::default() {
            self.base_text_style.merge(&other.base_text_style);
        }

        if let Some(dir) = other.flex_direction {
            self.flex_direction = Some(dir);
            self.base_constraints.flex_direction = dir;
        }

        if !other.flags.is_empty() {
            if other.flags.contains(StyleFlags::WIDTH) {
                self.base_constraints.width = other.base_constraints.width;
            }
            if other.flags.contains(StyleFlags::HEIGHT) {
                self.base_constraints.height = other.base_constraints.height;
            }
            if other.flags.contains(StyleFlags::MIN_WIDTH) {
                self.base_constraints.min_width = other.base_constraints.min_width;
            }
            if other.flags.contains(StyleFlags::MAX_WIDTH) {
                self.base_constraints.max_width = other.base_constraints.max_width;
            }
            if other.flags.contains(StyleFlags::MIN_HEIGHT) {
                self.base_constraints.min_height = other.base_constraints.min_height;
            }
            if other.flags.contains(StyleFlags::MAX_HEIGHT) {
                self.base_constraints.max_height = other.base_constraints.max_height;
            }
            if other.flags.contains(StyleFlags::ASPECT_RATIO) {
                self.base_constraints.aspect_ratio = other.base_constraints.aspect_ratio;
            }
            if other.flags.contains(StyleFlags::PADDING) {
                self.base_constraints.padding = other.base_constraints.padding;
            }
            if other.flags.contains(StyleFlags::BORDER_WIDTH) {
                self.base_constraints.border = other.base_constraints.border;
            }
            if other.flags.contains(StyleFlags::GAP) {
                self.base_constraints.gap = other.base_constraints.gap;
            }
            if other.flags.contains(StyleFlags::FLEX_GROW) {
                self.base_constraints.flex_grow = other.base_constraints.flex_grow;
            }
            if other.flags.contains(StyleFlags::FLEX_SHRINK) {
                self.base_constraints.flex_shrink = other.base_constraints.flex_shrink;
            }
            if other.flags.contains(StyleFlags::FLEX_BASIS) {
                self.base_constraints.flex_basis = other.base_constraints.flex_basis;
            }
            if other.flags.contains(StyleFlags::FLEX_DIRECTION) {
                self.flex_direction = other.flex_direction;
                if let Some(dir) = other.flex_direction {
                    self.base_constraints.flex_direction = dir;
                }
            }
            if other.flags.contains(StyleFlags::FLEX_WRAP) {
                self.base_constraints.flex_wrap = other.base_constraints.flex_wrap;
            }
            if other.flags.contains(StyleFlags::JUSTIFY_CONTENT) {
                self.base_constraints.justify_content = other.base_constraints.justify_content;
            }
            if other.flags.contains(StyleFlags::ALIGN_ITEMS) {
                self.base_constraints.align_items = other.base_constraints.align_items;
            }
            if other.flags.contains(StyleFlags::ALIGN_SELF) {
                self.base_constraints.align_self = other.base_constraints.align_self;
            }
            if other.flags.contains(StyleFlags::OVERFLOW) {
                self.base_constraints.overflow = other.base_constraints.overflow;
            }
            if other.flags.contains(StyleFlags::POSITIONING) {
                self.base_constraints.positioning = other.base_constraints.positioning;
            }
            if other.flags.contains(StyleFlags::Z_INDEX) {
                self.base_constraints.z_index = other.base_constraints.z_index;
            }

            if other.flags.contains(StyleFlags::BG_COLOR) {
                self.base_effects.background_color = other.base_effects.background_color;
            }
            if other.flags.contains(StyleFlags::BORDER_COLOR) {
                self.base_effects.border.color = other.base_effects.border.color;
            }
            if other.flags.contains(StyleFlags::BORDER_RADIUS) {
                self.base_effects.border.radius = other.base_effects.border.radius;
            }
            if other.flags.contains(StyleFlags::BOX_SHADOW) {
                self.base_effects.box_shadow = other.base_effects.box_shadow;
                self.base_effects.additional_shadows = other.base_effects.additional_shadows;
            }
            if other.flags.contains(StyleFlags::OPACITY) {
                self.base_effects.opacity = other.base_effects.opacity;
                self.base_effects.explicit_opacity = true;
            }
            if other.flags.contains(StyleFlags::SCALE) {
                self.base_effects.scale = other.base_effects.scale;
                self.base_effects.explicit_scale = true;
            }
            if other.flags.contains(StyleFlags::FILTERS) {
                self.base_effects.filters = other.base_effects.filters;
            }

            if other.flags.contains(StyleFlags::FONT_SIZE) {
                self.base_text_style.font_size = other.base_text_style.font_size;
            }
            if other.flags.contains(StyleFlags::FONT_FAMILY) {
                self.base_text_style.font_family = other.base_text_style.font_family;
            }
            if other.flags.contains(StyleFlags::FONT_WEIGHT) {
                self.base_text_style.font_weight = other.base_text_style.font_weight;
            }
            if other.flags.contains(StyleFlags::FONT_STYLE) {
                self.base_text_style.font_style = other.base_text_style.font_style;
            }
            if other.flags.contains(StyleFlags::TEXT_COLOR) {
                self.base_text_style.color = other.base_text_style.color;
            }
            if other.flags.contains(StyleFlags::TEXT_ALIGN) {
                self.base_text_style.alignment = other.base_text_style.alignment;
            }
            if other.flags.contains(StyleFlags::VERTICAL_ALIGN) {
                self.base_text_style.vertical_alignment = other.base_text_style.vertical_alignment;
            }
            if other.flags.contains(StyleFlags::WRAP) {
                self.base_text_style.wrap = other.base_text_style.wrap;
            }
            if other.flags.contains(StyleFlags::LINE_HEIGHT) {
                self.base_text_style.line_height = other.base_text_style.line_height;
            }
            if other.flags.contains(StyleFlags::UNDERLINE) {
                self.base_text_style.underline = other.base_text_style.underline;
            }
            if other.flags.contains(StyleFlags::STRIKETHROUGH) {
                self.base_text_style.strikethrough = other.base_text_style.strikethrough;
            }
            if other.flags.contains(StyleFlags::CARET_COLOR) {
                self.base_text_style.caret_color = other.base_text_style.caret_color;
            }
            if other.flags.contains(StyleFlags::SELECTION_COLOR) {
                self.base_text_style.selection_color = other.base_text_style.selection_color;
            }
            if other.flags.contains(StyleFlags::SELECTION_BG) {
                self.base_text_style.selection_bg = other.base_text_style.selection_bg;
            }
            self.flags |= other.flags;
        }

        if let Some(sb) = other.scrollbar {
            self.scrollbar = Some(match self.scrollbar {
                Some(mut existing) => {
                    existing.merge(&sb);
                    existing
                }
                None => sb,
            });
            self.flags.insert(StyleFlags::SCROLLBAR);
        }

        if let Some(other_inter) = other.interaction {
            let mut my_inter = self.interaction.take().unwrap_or_default();
            if let Some(h) = other_inter.hover {
                my_inter.hover = Some(match my_inter.hover {
                    Some(existing) => existing.merge(h),
                    None => h,
                });
            }
            if let Some(a) = other_inter.active {
                my_inter.active = Some(match my_inter.active {
                    Some(existing) => existing.merge(a),
                    None => a,
                });
            }
            if let Some(f) = other_inter.focus {
                my_inter.focus = Some(match my_inter.focus {
                    Some(existing) => existing.merge(f),
                    None => f,
                });
            }
            if let Some(d) = other_inter.disabled {
                my_inter.disabled = Some(match my_inter.disabled {
                    Some(existing) => existing.merge(d),
                    None => d,
                });
            }
            self.interaction = Some(my_inter);
        }

        self.transitions.extend(other.transitions);
        self
    }

    /// Applies a reusable style mixin function `f`.
    pub fn apply(self, f: impl FnOnce(Style) -> Style) -> Self {
        f(self)
    }

    /// Applies this style's base constraints, effects, and text style directly to a layout node.
    pub fn apply_to_node(&self, ctx: &mut crate::Context, node: crate::Node) {
        node.update_constraints(ctx, |c| {
            let overflow = c.overflow;
            let scroll = c.scroll;
            let flex_dir = self.flex_direction.unwrap_or(c.flex_direction);
            *c = self.base_constraints;
            c.flex_direction = flex_dir;
            if self.base_constraints.overflow == Overflow::Visible && overflow != Overflow::Visible
            {
                c.overflow = overflow;
            }
            c.scroll = scroll;
        });

        node.set_effects(ctx, self.base_effects.clone());

        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            if node
                .get_text_userdata::<crate::TextRenderInfo>(ctx)
                .is_none()
                && node.get_text_userdata::<TextStyle>(ctx).is_none()
            {
                node.set_text_with_userdata(ctx, &text_owned, self.base_text_style.clone());
            }
        }
    }

    /// Conditionally applies style modifications when `condition` is `true`.
    pub fn when(self, condition: bool, f: impl FnOnce(Style) -> Style) -> Self {
        if condition { f(self) } else { self }
    }

    pub fn padding(mut self, val: f32) -> Self {
        self.base_constraints.padding = Edges::all(val);
        self.flags.insert(StyleFlags::PADDING);
        self
    }

    /// Sets symmetric horizontal (`x`) and vertical (`y`) padding edges.
    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.base_constraints.padding = Edges {
            top: y,
            right: x,
            bottom: y,
            left: x,
        };
        self.flags.insert(StyleFlags::PADDING);
        self
    }

    pub fn padding_edges(mut self, edges: Edges) -> Self {
        self.base_constraints.padding = edges;
        self.flags.insert(StyleFlags::PADDING);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border = Edges::all(width);
        self.base_effects.border.color = color;
        self.flags
            .insert(StyleFlags::BORDER_WIDTH | StyleFlags::BORDER_COLOR);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.base_effects.border.color = color;
        self.flags.insert(StyleFlags::BORDER_COLOR);
        self
    }

    pub fn flex_wrap(mut self, wrap: FlexWrap) -> Self {
        self.base_constraints.flex_wrap = wrap;
        self.flags.insert(StyleFlags::FLEX_WRAP);
        self
    }

    pub fn wrap(self) -> Self {
        self.flex_wrap(FlexWrap::Wrap)
    }

    pub fn border_edges(mut self, edges: Edges, color: Color) -> Self {
        self.base_constraints.border = edges;
        self.base_effects.border.color = color;
        self.flags
            .insert(StyleFlags::BORDER_WIDTH | StyleFlags::BORDER_COLOR);
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.bottom = width;
        self.base_effects.border.color = color;
        self.flags
            .insert(StyleFlags::BORDER_WIDTH | StyleFlags::BORDER_COLOR);
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.top = width;
        self.base_effects.border.color = color;
        self.flags
            .insert(StyleFlags::BORDER_WIDTH | StyleFlags::BORDER_COLOR);
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.left = width;
        self.base_effects.border.color = color;
        self.flags
            .insert(StyleFlags::BORDER_WIDTH | StyleFlags::BORDER_COLOR);
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.right = width;
        self.base_effects.border.color = color;
        self.flags
            .insert(StyleFlags::BORDER_WIDTH | StyleFlags::BORDER_COLOR);
        self
    }

    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.base_effects.border.radius = Radius::all(radius);
        self.flags.insert(StyleFlags::BORDER_RADIUS);
        self
    }

    pub fn corner_radius_top(mut self, radius: f32) -> Self {
        self.base_effects.border.radius.tl = radius;
        self.base_effects.border.radius.tr = radius;
        self.flags.insert(StyleFlags::BORDER_RADIUS);
        self
    }

    pub fn corner_radius_bottom(mut self, radius: f32) -> Self {
        self.base_effects.border.radius.bl = radius;
        self.base_effects.border.radius.br = radius;
        self.flags.insert(StyleFlags::BORDER_RADIUS);
        self
    }

    pub fn corner_radius_precise(mut self, radius: Radius) -> Self {
        self.base_effects.border.radius = radius;
        self.flags.insert(StyleFlags::BORDER_RADIUS);
        self
    }

    pub fn box_shadow(mut self, shadow: BoxShadow) -> Self {
        self.base_effects.box_shadow = shadow;
        self.flags.insert(StyleFlags::BOX_SHADOW);
        self
    }

    pub fn add_box_shadow(mut self, shadow: BoxShadow) -> Self {
        self.base_effects.additional_shadows.push(shadow);
        self.flags.insert(StyleFlags::BOX_SHADOW);
        self
    }

    pub fn box_shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        let mut iter = shadows.into_iter();
        if let Some(first) = iter.next() {
            self.base_effects.box_shadow = first;
            self.base_effects.additional_shadows = iter.collect();
        } else {
            self.base_effects.box_shadow = BoxShadow::default();
            self.base_effects.additional_shadows.clear();
        }
        self.flags.insert(StyleFlags::BOX_SHADOW);
        self
    }

    pub fn blur(mut self, vibrancy: f32) -> Self {
        self.base_effects.filters.push(Filter::Blur {
            vibrancy,
            vibrancy_darkness: 0.2,
            passes: 4.0,
        });
        self.flags.insert(StyleFlags::FILTERS);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.base_effects.opacity = opacity;
        self.base_effects.explicit_opacity = true;
        self.flags.insert(StyleFlags::OPACITY);
        self
    }

    pub fn z_index(mut self, z_index: i32) -> Self {
        self.base_constraints.z_index = z_index;
        self.flags.insert(StyleFlags::Z_INDEX);
        self
    }

    pub fn absolute(mut self, left: f32, top: f32) -> Self {
        self.base_constraints.positioning = PositionStrategy::Absolute {
            left,
            top,
            right: f32::NAN,
            bottom: f32::NAN,
        };
        self.flags.insert(StyleFlags::POSITIONING);
        self
    }

    pub fn position(mut self, positioning: impl IntoPositionStrategy) -> Self {
        self.base_constraints.positioning = positioning.into_strategy();
        self.flags.insert(StyleFlags::POSITIONING);
        self
    }

    pub fn width(mut self, size: Size) -> Self {
        self.base_constraints.width = size;
        self.flags.insert(StyleFlags::WIDTH);
        self
    }

    pub fn min_width(mut self, min_w: f32) -> Self {
        self.base_constraints.min_width = min_w;
        self.flags.insert(StyleFlags::MIN_WIDTH);
        self
    }

    pub fn max_width(mut self, max_w: f32) -> Self {
        self.base_constraints.max_width = max_w;
        self.flags.insert(StyleFlags::MAX_WIDTH);
        self
    }

    pub fn height(mut self, size: Size) -> Self {
        self.base_constraints.height = size;
        self.flags.insert(StyleFlags::HEIGHT);
        self
    }

    pub fn min_height(mut self, min_h: f32) -> Self {
        self.base_constraints.min_height = min_h;
        self.flags.insert(StyleFlags::MIN_HEIGHT);
        self
    }

    pub fn max_height(mut self, max_h: f32) -> Self {
        self.base_constraints.max_height = max_h;
        self.flags.insert(StyleFlags::MAX_HEIGHT);
        self
    }

    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.base_constraints.aspect_ratio = ratio;
        self.flags.insert(StyleFlags::ASPECT_RATIO);
        self
    }

    pub fn justify_content(mut self, j: JustifyContent) -> Self {
        self.base_constraints.justify_content = j;
        self.flags.insert(StyleFlags::JUSTIFY_CONTENT);
        self
    }

    pub fn align_items(mut self, a: AlignItems) -> Self {
        self.base_constraints.align_items = a;
        self.flags.insert(StyleFlags::ALIGN_ITEMS);
        self
    }

    pub fn align_self(mut self, a: AlignSelf) -> Self {
        self.base_constraints.align_self = a;
        self.flags.insert(StyleFlags::ALIGN_SELF);
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.base_constraints.flex_shrink = shrink;
        self.flags.insert(StyleFlags::FLEX_SHRINK);
        self
    }

    pub fn flex_basis(mut self, basis: Size) -> Self {
        self.base_constraints.flex_basis = basis;
        self.flags.insert(StyleFlags::FLEX_BASIS);
        self
    }

    pub fn gap(mut self, val: f32) -> Self {
        self.base_constraints.gap = val;
        self.flags.insert(StyleFlags::GAP);
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.base_constraints.overflow = overflow;
        self.flags.insert(StyleFlags::OVERFLOW);
        self
    }

    pub fn bg_color(mut self, color: Color) -> Self {
        self.base_effects.background_color = color;
        self.flags.insert(StyleFlags::BG_COLOR);
        self
    }

    pub fn scale(mut self, s: f32) -> Self {
        self.base_effects.scale = s;
        self.base_effects.explicit_scale = true;
        self.flags.insert(StyleFlags::SCALE);
        self
    }

    pub fn flex_direction(mut self, dir: FlexDirection) -> Self {
        self.base_constraints.flex_direction = dir;
        self.flex_direction = Some(dir);
        self.flags.insert(StyleFlags::FLEX_DIRECTION);
        self
    }

    /// Sets flex grow factor for layout flexing inside flex containers.
    pub fn flex_grow(mut self, val: f32) -> Self {
        self.base_constraints.flex_grow = val;
        self.flags.insert(StyleFlags::FLEX_GROW);
        self
    }

    pub fn set_constraints(mut self, c: Constraints) -> Self {
        self.base_constraints = c;
        self
    }

    pub fn update_constraints(mut self, f: impl FnOnce(&mut Constraints)) -> Self {
        f(&mut self.base_constraints);
        self
    }

    pub fn set_effects(mut self, e: Effects) -> Self {
        self.base_effects = e;
        self
    }

    pub fn update_effects(mut self, f: impl FnOnce(&mut Effects)) -> Self {
        f(&mut self.base_effects);
        self
    }

    pub fn set_text_style(mut self, t: TextStyle) -> Self {
        self.base_text_style = t;
        self
    }

    pub fn update_text_style(mut self, f: impl FnOnce(&mut TextStyle)) -> Self {
        f(&mut self.base_text_style);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.base_text_style.font_size = size;
        self.flags.insert(StyleFlags::FONT_SIZE);
        self
    }

    pub fn font_family(mut self, family: impl Into<Cow<'static, str>>) -> Self {
        self.base_text_style.font_family = family.into();
        self.flags.insert(StyleFlags::FONT_FAMILY);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.base_text_style.color = color;
        self.flags.insert(StyleFlags::TEXT_COLOR);
        self
    }

    /// Sets whether text should wrap across multiple lines when width is constrained.
    pub fn text_wrap(mut self, wrap: bool) -> Self {
        self.base_text_style.wrap = wrap;
        self.flags.insert(StyleFlags::WRAP);
        self
    }

    /// Disables text wrapping, forcing text to render on a single line.
    pub fn text_nowrap(self) -> Self {
        self.text_wrap(false)
    }

    /// Declares style overrides applied when the mouse cursor hovers over the element.
    pub fn on_hover(mut self, hover_fn: impl FnOnce(Style) -> Style) -> Self {
        let hover_style = hover_fn(Style::new());
        let mut inter = self.interaction.take().map(|b| *b).unwrap_or_default();
        inter.hover = Some(hover_style);
        self.interaction = Some(Box::new(inter));
        self
    }

    /// Declares style overrides applied when the mouse button is pressed over the element (active state).
    pub fn on_active(mut self, active_fn: impl FnOnce(Style) -> Style) -> Self {
        let active_style = active_fn(Style::new());
        let mut inter = self.interaction.take().map(|b| *b).unwrap_or_default();
        inter.active = Some(active_style);
        self.interaction = Some(Box::new(inter));
        self
    }

    /// Declares style overrides applied when the element receives focus.
    pub fn on_focus(mut self, focus_fn: impl FnOnce(Style) -> Style) -> Self {
        let focus_style = focus_fn(Style::new());
        let mut inter = self.interaction.take().map(|b| *b).unwrap_or_default();
        inter.focus = Some(focus_style);
        self.interaction = Some(Box::new(inter));
        self
    }

    /// Declares style overrides applied when the element is disabled.
    pub fn on_disabled(mut self, disabled_fn: impl FnOnce(Style) -> Style) -> Self {
        let disabled_style = disabled_fn(Style::new());
        let mut inter = self.interaction.take().map(|b| *b).unwrap_or_default();
        inter.disabled = Some(disabled_style);
        self.interaction = Some(Box::new(inter));
        self
    }

    /// Smoothly transitions all changing properties over `duration_ms` with easing/spring `curve`.
    pub fn transition_all(mut self, duration_ms: f64, curve: Curve) -> Self {
        self.transitions
            .push(Transition::new(TransitionProperty::All, duration_ms, curve));
        self
    }

    /// Smoothly transitions a specific property over `duration_ms` with easing/spring `curve`.
    pub fn transition(
        mut self,
        property: TransitionProperty,
        duration_ms: f64,
        curve: Curve,
    ) -> Self {
        self.transitions
            .push(Transition::new(property, duration_ms, curve));
        self
    }

    /// Backwards-compatible animation transition builder.
    pub fn animate(mut self, target: TransitionProperty, duration_ms: f64, curve: Curve) -> Self {
        self.transitions
            .push(Transition::new(target, duration_ms, curve));
        self
    }

    pub fn scrollbar(mut self, style: ScrollbarStyle) -> Self {
        if style.visibility == ScrollbarVisibility::Never {
            self.base_constraints.scrollbar_visible = false;
        }
        self.scrollbar = Some(Box::new(style));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn scrollbar_width(mut self, width: f32) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.width = width;
        self.scrollbar = Some(Box::new(sb));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn scrollbar_margin(mut self, margin: f32) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.margin = margin;
        self.scrollbar = Some(Box::new(sb));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn scrollbar_gap(mut self, gap: f32) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.gap = gap;
        self.scrollbar = Some(Box::new(sb));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn scrollbar_thumb(mut self, color: Color) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.thumb_color = color;
        self.scrollbar = Some(Box::new(sb));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn scrollbar_thumb_color(self, color: Color) -> Self {
        self.scrollbar_thumb(color)
    }

    pub fn scrollbar_track(mut self, color: Color) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.track_color = Some(color);
        self.scrollbar = Some(Box::new(sb));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn scrollbar_track_color(self, color: Color) -> Self {
        self.scrollbar_track(color)
    }

    pub fn scrollbar_radius(mut self, radius: Radius) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.radius = radius;
        self.scrollbar = Some(Box::new(sb));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.visibility = visibility;
        if visibility == ScrollbarVisibility::Never {
            self.base_constraints.scrollbar_visible = false;
        } else {
            self.base_constraints.scrollbar_visible = true;
        }
        self.scrollbar = Some(Box::new(sb));
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }

    pub fn no_scrollbar(mut self) -> Self {
        let mut sb = self.scrollbar.take().map(|b| *b).unwrap_or_default();
        sb.visibility = ScrollbarVisibility::Never;
        self.scrollbar = Some(Box::new(sb));
        self.base_constraints.scrollbar_visible = false;
        self.flags.insert(StyleFlags::SCROLLBAR);
        self
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    pub(crate) fn test_row_flex_direction_preserved_when_styled() {
        let style = Style::new();
        assert_eq!(style.base_constraints.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn test_flexbox_new_properties() {
        let style = Style::new()
            .align_self(AlignSelf::End)
            .flex_grow(2.0)
            .flex_shrink(0.5)
            .flex_basis(Size::Fixed(100));

        assert_eq!(style.base_constraints.align_self, AlignSelf::End);
        assert_eq!(style.base_constraints.flex_grow, 2.0);
        assert_eq!(style.base_constraints.flex_shrink, 0.5);
        assert_eq!(style.base_constraints.flex_basis, Size::Fixed(100));
    }

    #[test]
    fn test_root_node_size_fill() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fill;
            c.height = Size::Fill;
        });
        ctx.root_attach(root);
        ctx.compute_layout(1024.0, 768.0);

        let bounds = root.get_computed(&ctx).unwrap();
        assert_eq!(bounds.w, 1024.0);
        assert_eq!(bounds.h, 768.0);
    }

    #[test]
    fn test_flex_shrink_distribution() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(100);
            c.height = Size::Fixed(50);
            c.flex_direction = FlexDirection::Row;
        });

        let child1 = ctx.create_node();
        child1.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(80);
            c.height = Size::Fixed(50);
            c.flex_shrink = 1.0;
        });

        let child2 = ctx.create_node();
        child2.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(40);
            c.height = Size::Fixed(50);
            c.flex_shrink = 1.0;
        });

        root.append(&mut ctx, child1);
        root.append(&mut ctx, child2);
        ctx.root_attach(root);
        ctx.compute_layout(100.0, 50.0);

        let c1_bounds = child1.get_computed(&ctx).unwrap();
        let c2_bounds = child2.get_computed(&ctx).unwrap();

        // 80 / 120 * 20 = 13.333... -> 80 - 13.333 = 66.666...
        // 40 / 120 * 20 = 6.666... -> 40 - 6.666 = 33.333...
        assert!((c1_bounds.w - 66.666).abs() < 0.1);
        assert!((c2_bounds.w - 33.333).abs() < 0.1);
        assert!(((c1_bounds.w + c2_bounds.w) - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_flex_grow_distribution() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(50);
            c.flex_direction = FlexDirection::Row;
        });

        let child1 = ctx.create_node();
        child1.update_constraints(&mut ctx, |c| {
            c.flex_grow = 1.0;
            c.height = Size::Fixed(50);
        });

        let child2 = ctx.create_node();
        child2.update_constraints(&mut ctx, |c| {
            c.flex_grow = 3.0;
            c.height = Size::Fixed(50);
        });

        root.append(&mut ctx, child1);
        root.append(&mut ctx, child2);
        ctx.root_attach(root);
        ctx.compute_layout(200.0, 50.0);

        let c1_bounds = child1.get_computed(&ctx).unwrap();
        let c2_bounds = child2.get_computed(&ctx).unwrap();

        assert_eq!(c1_bounds.w, 50.0);
        assert_eq!(c2_bounds.w, 150.0);
    }

    #[test]
    fn test_flex_wrap_layout() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(120);
            c.height = Size::Fit;
            c.flex_direction = FlexDirection::Row;
            c.flex_wrap = FlexWrap::Wrap;
            c.gap = 10.0;
        });

        let child1 = ctx.create_node();
        child1.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(50);
            c.height = Size::Fixed(30);
        });

        let child2 = ctx.create_node();
        child2.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(50);
            c.height = Size::Fixed(30);
        });

        let child3 = ctx.create_node();
        child3.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(50);
            c.height = Size::Fixed(30);
        });

        root.append(&mut ctx, child1);
        root.append(&mut ctx, child2);
        root.append(&mut ctx, child3);
        ctx.root_attach(root);
        ctx.compute_layout(120.0, 100.0);

        let c1_bounds = child1.get_computed(&ctx).unwrap();
        let c2_bounds = child2.get_computed(&ctx).unwrap();
        let c3_bounds = child3.get_computed(&ctx).unwrap();
        let root_bounds = root.get_computed(&ctx).unwrap();

        assert_eq!(c1_bounds.x, 0.0);
        assert_eq!(c1_bounds.y, 0.0);

        assert_eq!(c2_bounds.x, 60.0);
        assert_eq!(c2_bounds.y, 0.0);

        assert_eq!(c3_bounds.x, 0.0);
        assert_eq!(c3_bounds.y, 40.0);

        assert_eq!(root_bounds.h, 70.0);
    }

    #[test]
    fn test_flex_basis_with_grow() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(50);
            c.flex_direction = FlexDirection::Row;
        });

        let child1 = ctx.create_node();
        child1.update_constraints(&mut ctx, |c| {
            c.flex_basis = Size::Fixed(40);
            c.flex_grow = 1.0;
            c.height = Size::Fixed(50);
        });

        let child2 = ctx.create_node();
        child2.update_constraints(&mut ctx, |c| {
            c.flex_basis = Size::Fixed(60);
            c.flex_grow = 1.0;
            c.height = Size::Fixed(50);
        });

        root.append(&mut ctx, child1);
        root.append(&mut ctx, child2);
        ctx.root_attach(root);
        ctx.compute_layout(200.0, 50.0);

        // Total basis = 40 + 60 = 100. Free space = 200 - 100 = 100.
        // Child 1 = 40 + (1/2)*100 = 90.
        // Child 2 = 60 + (1/2)*100 = 110.
        let c1_bounds = child1.get_computed(&ctx).unwrap();
        let c2_bounds = child2.get_computed(&ctx).unwrap();

        assert_eq!(c1_bounds.w, 90.0);
        assert_eq!(c2_bounds.w, 110.0);
    }

    #[test]
    fn test_pre_distribution_cross_axis_stretch() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(300);
            c.height = Size::Fixed(400);
            c.flex_direction = FlexDirection::Column;
            c.align_items = AlignItems::Stretch;
        });

        let card = ctx.create_node();
        card.update_constraints(&mut ctx, |c| {
            c.height = Size::Fixed(80);
            c.flex_direction = FlexDirection::Row;
        });

        let card_inner_fill = ctx.create_node();
        card_inner_fill.update_constraints(&mut ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fixed(30);
        });

        card.append(&mut ctx, card_inner_fill);
        root.append(&mut ctx, card);
        ctx.root_attach(root);
        ctx.compute_layout(300.0, 400.0);

        // Card is stretched by root column to 300px width.
        // Inner fill child resolves its 100% width against card's 300px width!
        let card_bounds = card.get_computed(&ctx).unwrap();
        let inner_bounds = card_inner_fill.get_computed(&ctx).unwrap();

        assert_eq!(card_bounds.w, 300.0);
        assert_eq!(inner_bounds.w, 300.0);
    }

    #[test]
    fn test_incremental_dirty_layout_speed() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(1000);
            c.height = Size::Fixed(1000);
            c.flex_direction = FlexDirection::Column;
        });

        let mut leaf_nodes = Vec::new();
        for _ in 0..10 {
            let row = ctx.create_node();
            row.update_constraints(&mut ctx, |c| {
                c.width = Size::Percent(1.0);
                c.height = Size::Fixed(50);
                c.flex_direction = FlexDirection::Row;
            });
            for _ in 0..10 {
                let leaf = ctx.create_node();
                leaf.update_constraints(&mut ctx, |c| {
                    c.width = Size::Fixed(50);
                    c.height = Size::Fixed(50);
                });
                row.append(&mut ctx, leaf);
                leaf_nodes.push(leaf);
            }
            root.append(&mut ctx, row);
        }

        ctx.root_attach(root);
        ctx.compute_layout(1000.0, 1000.0);

        // Initial layout complete. Now modify only one single leaf
        let target_leaf = leaf_nodes[42];
        target_leaf.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(80);
        });

        // Incremental re-layout
        ctx.compute_layout(1000.0, 1000.0);

        let target_bounds = target_leaf.get_computed(&ctx).unwrap();
        assert_eq!(target_bounds.w, 80.0);

        // Untouched leaf retains its computed size
        let untouched_leaf = leaf_nodes[0];
        let untouched_bounds = untouched_leaf.get_computed(&ctx).unwrap();
        assert_eq!(untouched_bounds.w, 50.0);
    }

    #[test]
    fn test_aspect_ratio_resolution_modes() {
        let mut ctx = crate::Context::new();

        // 1. Fixed width + Fit height + Aspect Ratio 2.0
        let node1 = ctx.create_node();
        node1.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(100);
            c.height = Size::Fit;
            c.aspect_ratio = 2.0;
        });
        ctx.root_attach(node1);
        ctx.compute_layout(500.0, 500.0);
        let b1 = node1.get_computed(&ctx).unwrap();
        assert_eq!(b1.w, 100.0);
        assert_eq!(b1.h, 50.0);

        // 2. Fixed height + Fit width + Aspect Ratio 2.0
        let node2 = ctx.create_node();
        node2.update_constraints(&mut ctx, |c| {
            c.width = Size::Fit;
            c.height = Size::Fixed(50);
            c.aspect_ratio = 2.0;
        });
        ctx.root_attach(node2);
        ctx.compute_layout(500.0, 500.0);
        let b2 = node2.get_computed(&ctx).unwrap();
        assert_eq!(b2.w, 100.0);
        assert_eq!(b2.h, 50.0);

        // 3. Child with height: Fill + width: Fit in a Row (Cross-axis stretch)
        let row = ctx.create_node();
        row.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(400);
            c.height = Size::Fixed(40);
            c.flex_direction = FlexDirection::Row;
        });
        let child_fill_h = ctx.create_node();
        child_fill_h.update_constraints(&mut ctx, |c| {
            c.height = Size::Fill;
            c.width = Size::Fit;
            c.aspect_ratio = 1.5; // w = h * 1.5 = 40 * 1.5 = 60
        });
        row.append(&mut ctx, child_fill_h);
        ctx.root_attach(row);
        ctx.compute_layout(400.0, 400.0);
        let b3 = child_fill_h.get_computed(&ctx).unwrap();
        assert_eq!(b3.h, 40.0);
        assert_eq!(b3.w, 60.0);

        // 4. Child with width: Fill + height: Fit in a Column
        let col = ctx.create_node();
        col.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(300);
            c.height = Size::Fixed(500);
            c.flex_direction = FlexDirection::Column;
        });
        let child_fill_w = ctx.create_node();
        child_fill_w.update_constraints(&mut ctx, |c| {
            c.width = Size::Fill;
            c.height = Size::Fit;
            c.aspect_ratio = 1.5; // h = w / 1.5 = 300 / 1.5 = 200
        });
        col.append(&mut ctx, child_fill_w);
        ctx.root_attach(col);
        ctx.compute_layout(300.0, 500.0);
        let b4 = child_fill_w.get_computed(&ctx).unwrap();
        assert_eq!(b4.w, 300.0);
        assert_eq!(b4.h, 200.0);
    }

    #[test]
    fn test_scroll_clamping_on_content_shrink() {
        let mut ctx = crate::Context::new();

        let container = ctx.create_node();
        container.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(200);
            c.overflow = Overflow::Scroll;
            c.flex_direction = FlexDirection::Column;
        });

        let child = ctx.create_node();
        child.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(600);
        });
        container.append(&mut ctx, child);
        ctx.root_attach(container);

        // Scroll to 300px (max scroll is 400px)
        container.update_constraints(&mut ctx, |c| {
            c.scroll.y = 300.0;
        });

        ctx.compute_layout(400.0, 400.0);
        let cons = container.get_constraints(&ctx).unwrap();
        assert_eq!(cons.scroll.y, 300.0);

        // Shrink child to 100px (now fits in 200px container, max scroll is 0.0)
        child.update_constraints(&mut ctx, |c| {
            c.height = Size::Fixed(100);
        });

        ctx.compute_layout(400.0, 400.0);
        let cons = container.get_constraints(&ctx).unwrap();
        assert_eq!(cons.scroll.y, 0.0);

        let child_comp = child.get_computed(&ctx).unwrap();
        let container_comp = container.get_computed(&ctx).unwrap();
        assert_eq!(child_comp.y, container_comp.y);
    }

    #[test]
    fn test_flex_fill_hover_incremental_layout() {
        let mut ctx = crate::Context::new();

        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(400);
            c.height = Size::Fixed(400);
            c.flex_direction = FlexDirection::Column;
        });

        let row = ctx.create_node();
        row.update_constraints(&mut ctx, |c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Fill;
            c.gap = 10.0;
            c.flex_direction = FlexDirection::Row;
        });
        root.append(&mut ctx, row);

        let mut buttons = Vec::new();
        for _ in 0..4 {
            let btn = ctx.create_node();
            btn.update_constraints(&mut ctx, |c| {
                c.width = Size::Fill;
                c.height = Size::Fill;
            });
            row.append(&mut ctx, btn);
            buttons.push(btn);
        }

        ctx.root_attach(root);
        ctx.compute_layout(400.0, 400.0);

        // Expected button width: (400 - 3 * 10) / 4 = 370 / 4 = 92.5
        for &btn in &buttons {
            let comp = btn.get_computed(&ctx).unwrap();
            assert!((comp.w - 92.5).abs() < 1e-3, "Initial width was {}", comp.w);
        }

        // Simulate hover on button 0 by setting it dirty
        buttons[0].set_dirty(&mut ctx);
        ctx.compute_layout(400.0, 400.0);

        // Verify that incremental layout maintains exact geometries for all buttons
        for &btn in &buttons {
            let comp = btn.get_computed(&ctx).unwrap();
            assert!(
                (comp.w - 92.5).abs() < 1e-3,
                "Button {:?} width corrupted after hover on button 0: {}",
                btn,
                comp.w
            );
        }

        // Simulate hover on button 2
        buttons[2].set_dirty(&mut ctx);
        ctx.compute_layout(400.0, 400.0);

        for &btn in &buttons {
            let comp = btn.get_computed(&ctx).unwrap();
            assert!(
                (comp.w - 92.5).abs() < 1e-3,
                "Button {:?} width corrupted after hover on button 2: {}",
                btn,
                comp.w
            );
        }
    }

    #[test]
    fn test_scroll_offset_percentage_initial_position() {
        let mut ctx = crate::Context::new();

        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(200);
            c.overflow = Overflow::Scroll;
            c.scroll.y = -1.0001;
        });

        let child = ctx.create_node();
        child.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(1000);
        });

        let grandchild = ctx.create_node();
        grandchild.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(50);
        });

        child.append(&mut ctx, grandchild);
        root.append(&mut ctx, child);
        ctx.root_attach(root);

        ctx.compute_layout(200.0, 200.0);

        let root_cons = root.get_constraints(&ctx).unwrap();
        let child_comp = child.get_computed(&ctx).unwrap();
        let grandchild_comp = grandchild.get_computed(&ctx).unwrap();

        assert!(
            (root_cons.scroll.y - 800.0).abs() < 1e-3,
            "scroll.y was {}",
            root_cons.scroll.y
        );
        assert!(
            (child_comp.y - (-800.0)).abs() < 1e-3,
            "child.y was {}",
            child_comp.y
        );
        assert!(
            (grandchild_comp.y - (-800.0)).abs() < 1e-3,
            "grandchild.y was {}",
            grandchild_comp.y
        );
    }

    #[test]
    fn test_scroll_offset_percentage_horizontal_and_partial() {
        let mut ctx = crate::Context::new();

        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(200);
            c.overflow = Overflow::Scroll;
            c.scroll.y = -0.5001;
            c.scroll.x = -1.0001;
        });

        let child = ctx.create_node();
        child.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(600);
            c.height = Size::Fixed(1000);
        });

        root.append(&mut ctx, child);
        ctx.root_attach(root);

        ctx.compute_layout(200.0, 200.0);

        let root_cons = root.get_constraints(&ctx).unwrap();
        let child_comp = child.get_computed(&ctx).unwrap();

        assert!(
            (root_cons.scroll.y - 400.0).abs() < 1e-3,
            "scroll.y was {}",
            root_cons.scroll.y
        );
        assert!(
            (child_comp.y - (-400.0)).abs() < 1e-3,
            "child.y was {}",
            child_comp.y
        );

        assert!(
            (root_cons.scroll.x - 400.0).abs() < 1e-3,
            "scroll.x was {}",
            root_cons.scroll.x
        );
        assert!(
            (child_comp.x - (-400.0)).abs() < 1e-3,
            "child.x was {}",
            child_comp.x
        );
    }

    #[test]
    fn test_style_box_shadows() {
        let s1 = BoxShadow::sm();
        let s2 = BoxShadow::md();
        let style = Style::new().box_shadow(s1).add_box_shadow(s2);

        assert_eq!(style.base_effects.box_shadow, s1);
        assert_eq!(style.base_effects.additional_shadows, vec![s2]);

        let s3 = BoxShadow::lg();
        let style2 = Style::new().box_shadows([s1, s2, s3]);
        assert_eq!(style2.base_effects.box_shadow, s1);
        assert_eq!(style2.base_effects.additional_shadows, vec![s2, s3]);
    }

    #[test]
    fn test_style_flags_and_explicit_sentinel_overrides() {
        // Transparent color override (previously broken by magic sentinel comparisons)
        let base = Style::new().bg_color(Color::new(255, 0, 0, 255));
        let override_style = Style::new().bg_color(Color::transparent);
        let merged = base.merge(override_style);
        assert_eq!(merged.base_effects.background_color, Color::transparent);

        // Explicit font size 16.0 override (previously ignored because 16.0 was default)
        let base = Style::new().font_size(24.0);
        let override_style = Style::new().font_size(16.0);
        let merged = base.merge(override_style);
        assert_eq!(merged.base_text_style.font_size, 16.0);

        // Explicit padding 0.0 override (previously ignored if checking default edges)
        let base = Style::new().padding(20.0);
        let override_style = Style::new().padding(0.0);
        let merged = base.merge(override_style);
        assert_eq!(merged.base_constraints.padding, Edges::all(0.0));
    }

    #[test]
    fn test_style_interaction_consolidation() {
        let style = Style::new()
            .bg_color(Color::new(10, 20, 30, 255))
            .on_hover(|s| s.bg_color(Color::new(40, 50, 60, 255)))
            .on_active(|s| s.scale(0.95))
            .on_focus(|s| s.border(2.0, Color::new(0, 100, 200, 255)))
            .on_disabled(|s| s.opacity(0.5));

        assert!(style.interaction.is_some());
        assert_eq!(
            style.hover().unwrap().base_effects.background_color,
            Color::new(40, 50, 60, 255)
        );
        assert_eq!(style.active().unwrap().base_effects.scale, 0.95);
        assert_eq!(
            style.focus().unwrap().base_constraints.border,
            Edges::all(2.0)
        );
        assert_eq!(style.disabled().unwrap().base_effects.opacity, 0.5);

        // Unconfigured style has None for interaction -> zero heap allocation
        let plain_style = Style::new().padding(10.0);
        assert!(plain_style.interaction.is_none());
        assert!(plain_style.hover().is_none());
    }

    #[test]
    fn test_style_memory_footprint() {
        use std::mem::size_of;
        // ScrollbarStyle is now boxed, saving 56 bytes
        assert_eq!(size_of::<Option<Box<ScrollbarStyle>>>(), 8);
        // Interaction is now consolidated into a single Option<Box>, saving 24 bytes on stack
        assert_eq!(size_of::<Option<Box<InteractiveStyles>>>(), 8);
        // Flags is a compact 8-byte bitmask
        assert_eq!(size_of::<StyleFlags>(), 8);
        println!("Style size: {} bytes", size_of::<Style>());
    }
}
