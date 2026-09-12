use parley::layout::Alignment;
use parley::style::{FontStyle, FontWeight, OverflowWrap};

pub use mtk_layout::{
    AbsoluteBuilder, AlignItems, AlignSelf, Computed, Constraints, Edges, FlexDirection, FlexWrap,
    IntoPositionStrategy, JustifyContent, Overflow, PositionStrategy, Rect, Size, Vector2,
};

use crate::animation::Curve;
use crate::clr;
use crate::colors::Color;
use crate::effects::{Effects, Filter, Radius, Shadow};

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
    pub font_family: String,
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
            font_family: "system-ui".to_string(),
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
    Shadow,
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
        if other.shadow != Shadow::default() {
            self.shadow = other.shadow;
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

    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = family.into();
        self
    }

    pub fn family(self, family: impl Into<String>) -> Self {
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

/// Declarative styling container defining layout constraints, visual effects, typography, pseudo-states, and transitions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub base_constraints: Constraints,
    pub base_effects: Effects,
    pub base_text_style: TextStyle,
    pub flex_direction: Option<FlexDirection>,
    pub scrollbar: Option<ScrollbarStyle>,

    pub hover: Option<Box<Style>>,
    pub active: Option<Box<Style>>,
    pub focus: Option<Box<Style>>,
    pub disabled: Option<Box<Style>>,

    pub transitions: Vec<Transition>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges `other` into `self`, overriding conflicting properties while preserving non-conflicting base styles.
    pub fn merge(mut self, other: Style) -> Self {
        self.base_constraints.merge(&other.base_constraints);
        self.base_effects.merge(&other.base_effects);
        self.base_text_style.merge(&other.base_text_style);

        if let Some(dir) = other.flex_direction {
            self.flex_direction = Some(dir);
            self.base_constraints.flex_direction = dir;
        }

        if let Some(sb) = other.scrollbar {
            self.scrollbar = Some(match self.scrollbar {
                Some(mut existing) => {
                    existing.merge(&sb);
                    existing
                }
                None => sb,
            });
        }

        if let Some(h) = other.hover {
            self.hover = Some(Box::new(match self.hover {
                Some(existing) => existing.merge(*h),
                None => *h,
            }));
        }

        if let Some(a) = other.active {
            self.active = Some(Box::new(match self.active {
                Some(existing) => existing.merge(*a),
                None => *a,
            }));
        }

        if let Some(f) = other.focus {
            self.focus = Some(Box::new(match self.focus {
                Some(existing) => existing.merge(*f),
                None => *f,
            }));
        }

        if let Some(d) = other.disabled {
            self.disabled = Some(Box::new(match self.disabled {
                Some(existing) => existing.merge(*d),
                None => *d,
            }));
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
        self
    }

    pub fn padding_edges(mut self, edges: Edges) -> Self {
        self.base_constraints.padding = edges;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border = Edges::all(width);
        self.base_effects.border.color = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.base_effects.border.color = color;
        self
    }

    pub fn flex_wrap(mut self, wrap: FlexWrap) -> Self {
        self.base_constraints.flex_wrap = wrap;
        self
    }

    pub fn wrap(self) -> Self {
        self.flex_wrap(FlexWrap::Wrap)
    }

    pub fn border_edges(mut self, edges: Edges, color: Color) -> Self {
        self.base_constraints.border = edges;
        self.base_effects.border.color = color;
        self
    }

    pub fn border_bottom(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.bottom = width;
        self.base_effects.border.color = color;
        self
    }

    pub fn border_top(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.top = width;
        self.base_effects.border.color = color;
        self
    }

    pub fn border_left(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.left = width;
        self.base_effects.border.color = color;
        self
    }

    pub fn border_right(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border.right = width;
        self.base_effects.border.color = color;
        self
    }

    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.base_effects.border.radius = Radius::all(radius);
        self
    }

    pub fn corner_radius_top(mut self, radius: f32) -> Self {
        self.base_effects.border.radius.tl = radius;
        self.base_effects.border.radius.tr = radius;
        self
    }

    pub fn corner_radius_bottom(mut self, radius: f32) -> Self {
        self.base_effects.border.radius.bl = radius;
        self.base_effects.border.radius.br = radius;
        self
    }

    pub fn corner_radius_precise(mut self, radius: Radius) -> Self {
        self.base_effects.border.radius = radius;
        self
    }

    pub fn shadow(mut self, color: Color, spread: f32, power: f32) -> Self {
        self.base_effects.shadow = Shadow {
            color,
            spread,
            power,
        };
        self
    }

    pub fn blur(mut self, vibrancy: f32) -> Self {
        self.base_effects.filters.push(Filter::Blur {
            vibrancy,
            vibrancy_darkness: 0.2,
            passes: 4.0,
        });
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.base_effects.opacity = opacity;
        self.base_effects.explicit_opacity = true;
        self
    }

    pub fn z_index(mut self, z_index: i32) -> Self {
        self.base_constraints.z_index = z_index;
        self
    }

    pub fn absolute(mut self, left: f32, top: f32) -> Self {
        self.base_constraints.positioning = PositionStrategy::Absolute {
            left,
            top,
            right: f32::NAN,
            bottom: f32::NAN,
        };
        self
    }

    pub fn position(mut self, positioning: impl IntoPositionStrategy) -> Self {
        self.base_constraints.positioning = positioning.into_strategy();
        self
    }

    pub fn width(mut self, size: Size) -> Self {
        self.base_constraints.width = size;
        self
    }

    pub fn min_width(mut self, min_w: f32) -> Self {
        self.base_constraints.min_width = min_w;
        self
    }

    pub fn max_width(mut self, max_w: f32) -> Self {
        self.base_constraints.max_width = max_w;
        self
    }

    pub fn height(mut self, size: Size) -> Self {
        self.base_constraints.height = size;
        self
    }

    pub fn min_height(mut self, min_h: f32) -> Self {
        self.base_constraints.min_height = min_h;
        self
    }

    pub fn max_height(mut self, max_h: f32) -> Self {
        self.base_constraints.max_height = max_h;
        self
    }

    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.base_constraints.aspect_ratio = ratio;
        self
    }

    pub fn justify_content(mut self, j: JustifyContent) -> Self {
        self.base_constraints.justify_content = j;
        self
    }

    pub fn align_items(mut self, a: AlignItems) -> Self {
        self.base_constraints.align_items = a;
        self
    }

    pub fn align_self(mut self, a: AlignSelf) -> Self {
        self.base_constraints.align_self = a;
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.base_constraints.flex_shrink = shrink;
        self
    }

    pub fn flex_basis(mut self, basis: Size) -> Self {
        self.base_constraints.flex_basis = basis;
        self
    }

    pub fn gap(mut self, val: f32) -> Self {
        self.base_constraints.gap = val;
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.base_constraints.overflow = overflow;
        self
    }

    pub fn bg_color(mut self, color: Color) -> Self {
        self.base_effects.background_color = color;
        self
    }

    pub fn scale(mut self, s: f32) -> Self {
        self.base_effects.scale = s;
        self.base_effects.explicit_scale = true;
        self
    }

    pub fn flex_direction(mut self, dir: FlexDirection) -> Self {
        self.base_constraints.flex_direction = dir;
        self.flex_direction = Some(dir);
        self
    }

    /// Sets flex grow factor for layout flexing inside flex containers.
    pub fn flex_grow(mut self, val: f32) -> Self {
        self.base_constraints.flex_grow = val;
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

    /// Sets whether text should wrap across multiple lines when width is constrained.
    pub fn text_wrap(mut self, wrap: bool) -> Self {
        self.base_text_style.wrap = wrap;
        self
    }

    /// Disables text wrapping, forcing text to render on a single line.
    pub fn text_nowrap(self) -> Self {
        self.text_wrap(false)
    }

    /// Declares style overrides applied when the mouse cursor hovers over the element.
    pub fn on_hover(mut self, hover_fn: impl FnOnce(Style) -> Style) -> Self {
        let hover_style = hover_fn(Style::new());
        self.hover = Some(Box::new(hover_style));
        self
    }

    /// Declares style overrides applied when the mouse button is pressed over the element (active state).
    pub fn on_active(mut self, active_fn: impl FnOnce(Style) -> Style) -> Self {
        let active_style = active_fn(Style::new());
        self.active = Some(Box::new(active_style));
        self
    }

    /// Declares style overrides applied when the element receives focus.
    pub fn on_focus(mut self, focus_fn: impl FnOnce(Style) -> Style) -> Self {
        let focus_style = focus_fn(Style::new());
        self.focus = Some(Box::new(focus_style));
        self
    }

    /// Declares style overrides applied when the element is disabled.
    pub fn on_disabled(mut self, disabled_fn: impl FnOnce(Style) -> Style) -> Self {
        let disabled_style = disabled_fn(Style::new());
        self.disabled = Some(Box::new(disabled_style));
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
        self.scrollbar = Some(style);
        self
    }

    pub fn scrollbar_width(mut self, width: f32) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.width = width;
        self.scrollbar = Some(sb);
        self
    }

    pub fn scrollbar_margin(mut self, margin: f32) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.margin = margin;
        self.scrollbar = Some(sb);
        self
    }

    pub fn scrollbar_gap(mut self, gap: f32) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.gap = gap;
        self.scrollbar = Some(sb);
        self
    }

    pub fn scrollbar_thumb(mut self, color: Color) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.thumb_color = color;
        self.scrollbar = Some(sb);
        self
    }

    pub fn scrollbar_thumb_color(self, color: Color) -> Self {
        self.scrollbar_thumb(color)
    }

    pub fn scrollbar_track(mut self, color: Color) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.track_color = Some(color);
        self.scrollbar = Some(sb);
        self
    }

    pub fn scrollbar_track_color(self, color: Color) -> Self {
        self.scrollbar_track(color)
    }

    pub fn scrollbar_radius(mut self, radius: Radius) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.radius = radius;
        self.scrollbar = Some(sb);
        self
    }

    pub fn scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.visibility = visibility;
        if visibility == ScrollbarVisibility::Never {
            self.base_constraints.scrollbar_visible = false;
        } else {
            self.base_constraints.scrollbar_visible = true;
        }
        self.scrollbar = Some(sb);
        self
    }

    pub fn no_scrollbar(mut self) -> Self {
        let mut sb = self.scrollbar.take().unwrap_or_default();
        sb.visibility = ScrollbarVisibility::Never;
        self.scrollbar = Some(sb);
        self.base_constraints.scrollbar_visible = false;
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
}
