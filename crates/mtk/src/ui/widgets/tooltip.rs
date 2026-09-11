use std::time::{Duration, Instant};

use crate::animation::{AnimatedValue, Curve};
use crate::debugger::SourceLocation;
use crate::effects::Effects;
use crate::style::{PositionStrategy, Style, TextStyle};
use crate::text_property::FontWeight;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, clr, rgb, rgba};

/// Defines the anchor point of the tooltip relative to its child view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TooltipPlacement {
    /// Placed above the child view, centered horizontally.
    #[default]
    Top,
    /// Placed above the child view, aligned to the child's start (left) edge.
    TopStart,
    /// Placed above the child view, aligned to the child's end (right) edge.
    TopEnd,
    /// Placed below the child view, centered horizontally.
    Bottom,
    /// Placed below the child view, aligned to the child's start (left) edge.
    BottomStart,
    /// Placed below the child view, aligned to the child's end (right) edge.
    BottomEnd,
    /// Placed to the left of the child view, centered vertically.
    Left,
    /// Placed to the left of the child view, aligned to the child's top edge.
    LeftStart,
    /// Placed to the left of the child view, aligned to the child's bottom edge.
    LeftEnd,
    /// Placed to the right of the child view, centered vertically.
    Right,
    /// Placed to the right of the child view, aligned to the child's top edge.
    RightStart,
    /// Placed to the right of the child view, aligned to the child's bottom edge.
    RightEnd,
}

/// Visual transition style when entering and exiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TooltipAnimation {
    /// Smooth fade and subtle scaling transition (opacity 0 -> 1, scale 0.95 -> 1.0).
    #[default]
    FadeAndScale,
    /// Opacity fade transition (opacity 0 -> 1, scale fixed at 1.0).
    Fade,
    /// Snaps into and out of visibility instantly with no animation.
    None,
}

/// A hover tooltip wrapper component that displays a floating hint when hovering over its child view.
pub struct Tooltip<V> {
    pub(crate) inner: V,
    pub(crate) text: String,
    pub(crate) placement: TooltipPlacement,
    pub(crate) offset: f32,
    pub(crate) delay: Duration,
    pub(crate) enter_duration: Duration,
    pub(crate) exit_duration: Duration,
    pub(crate) animation: TooltipAnimation,
    pub(crate) custom_style: Option<Style>,
    pub(crate) override_style: Option<Style>,
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Wraps a view with a hover tooltip.
///
/// # Examples
/// ```rust,ignore
/// tooltip(button("Save"), "Saves current changes to disk")
///     .placement(TooltipPlacement::Bottom)
///     .delay_ms(150)
/// ```
#[track_caller]
pub fn tooltip<V>(inner: V, text: impl Into<String>) -> Tooltip<V> {
    Tooltip {
        inner,
        text: text.into(),
        placement: TooltipPlacement::default(),
        offset: 6.0,
        delay: Duration::ZERO,
        enter_duration: Duration::from_millis(150),
        exit_duration: Duration::from_millis(150),
        animation: TooltipAnimation::default(),
        custom_style: None,
        override_style: None,
        source_loc: Some(SourceLocation::here("Tooltip")),
    }
}

impl<V> Tooltip<V> {
    /// Sets the anchor placement of the tooltip relative to its child view.
    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the pixel offset distance between the child view and the tooltip.
    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the hover delay before the tooltip begins its appearance animation.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Sets the hover delay in milliseconds.
    pub fn delay_ms(self, ms: u64) -> Self {
        self.delay(Duration::from_millis(ms))
    }

    /// Sets the entrance animation transition duration.
    pub fn enter_duration(mut self, duration: Duration) -> Self {
        self.enter_duration = duration;
        self
    }

    /// Sets the entrance animation transition duration in milliseconds.
    pub fn enter_duration_ms(self, ms: u64) -> Self {
        self.enter_duration(Duration::from_millis(ms))
    }

    /// Sets the exit animation transition duration.
    pub fn exit_duration(mut self, duration: Duration) -> Self {
        self.exit_duration = duration;
        self
    }

    /// Sets the exit animation transition duration in milliseconds.
    pub fn exit_duration_ms(self, ms: u64) -> Self {
        self.exit_duration(Duration::from_millis(ms))
    }

    /// Configures the entrance and exit animation kind.
    pub fn animation(mut self, animation: TooltipAnimation) -> Self {
        self.animation = animation;
        self
    }

    /// Sets custom styling that will be merged with the default tooltip style.
    pub fn custom_style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Fluent alias for [`custom_style`](Self::custom_style), merging the provided style with default values.
    pub fn style(self, style: Style) -> Self {
        self.custom_style(style)
    }

    /// Completely overrides the default tooltip style, ignoring base defaults.
    pub fn override_style(mut self, style: Style) -> Self {
        self.override_style = Some(style);
        self
    }

    pub(crate) fn default_style() -> Style {
        Style::new()
            .padding_xy(8.0, 4.0)
            .corner_radius(4.0)
            .bg_color(rgb!(15, 23, 42))
            .border(1.0, rgb!(51, 65, 85))
            .shadow(rgba!(0, 0, 0, 80), 8.0, 0.5)
            .set_text_style(TextStyle {
                font_size: 11.0,
                font_weight: FontWeight::MEDIUM,
                color: clr!(white),
                wrap: false,
                ..Default::default()
            })
            .z_index(3000)
    }

    pub(crate) fn resolve_style(&self) -> Style {
        if let Some(ref over) = self.override_style {
            over.clone()
        } else if let Some(ref custom) = self.custom_style {
            Self::default_style().merge(custom.clone())
        } else {
            Self::default_style()
        }
    }
}

pub(crate) fn compute_absolute_position(
    placement: TooltipPlacement,
    offset: f32,
    parent_w: f32,
    parent_h: f32,
    tooltip_w: f32,
    tooltip_h: f32,
) -> PositionStrategy {
    let nan = f32::NAN;
    let (top, left) = match placement {
        TooltipPlacement::Top => (-tooltip_h - offset, (parent_w - tooltip_w) / 2.0),
        TooltipPlacement::TopStart => (-tooltip_h - offset, 0.0),
        TooltipPlacement::TopEnd => (-tooltip_h - offset, parent_w - tooltip_w),
        TooltipPlacement::Bottom => (parent_h + offset, (parent_w - tooltip_w) / 2.0),
        TooltipPlacement::BottomStart => (parent_h + offset, 0.0),
        TooltipPlacement::BottomEnd => (parent_h + offset, parent_w - tooltip_w),
        TooltipPlacement::Left => ((parent_h - tooltip_h) / 2.0, -tooltip_w - offset),
        TooltipPlacement::LeftStart => (0.0, -tooltip_w - offset),
        TooltipPlacement::LeftEnd => (parent_h - tooltip_h, -tooltip_w - offset),
        TooltipPlacement::Right => ((parent_h - tooltip_h) / 2.0, parent_w + offset),
        TooltipPlacement::RightStart => (0.0, parent_w + offset),
        TooltipPlacement::RightEnd => (parent_h - tooltip_h, parent_w + offset),
    };

    PositionStrategy::Absolute {
        top,
        left,
        bottom: nan,
        right: nan,
    }
}

fn measure_tooltip_dimensions(text: &str, style: &Style, ctx: &Context) -> (f32, f32) {
    let out = crate::text::measure_text(
        text,
        &style.base_text_style,
        f32::INFINITY,
        f32::INFINITY,
        &ctx.text_context,
        &[],
    );
    let pad = &style.base_constraints.padding;
    let border = &style.base_constraints.border;
    (
        out.computed_width + pad.left + pad.right + border.left + border.right,
        out.computed_height + pad.top + pad.bottom + border.top + border.bottom,
    )
}

fn get_tooltip_dimensions<E>(
    element: &TooltipElement<E>,
    text: &str,
    style: &Style,
    ctx: &Context,
) -> (f32, f32) {
    if let Some(comp) = element.tooltip_node.get_computed(ctx) {
        if comp.w > 0.0 && comp.h > 0.0 {
            return (comp.w, comp.h);
        }
    }
    measure_tooltip_dimensions(text, style, ctx)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum TooltipAnimPhase {
    #[default]
    Hidden,
    Entering,
    Visible,
    Exiting,
}

pub struct TooltipElement<E> {
    inner_el: E,
    pub(crate) tooltip_node: Node,
    pub(crate) is_attached: bool,
    pub(crate) hover_start: Option<Instant>,
    pub(crate) is_hovered: bool,
    pub(crate) anim_phase: TooltipAnimPhase,
    pub(crate) anim_progress: AnimatedValue<f32>,
    pub(crate) anim_start: Instant,
    pub(crate) base_effects: Effects,
}

impl<E> TooltipElement<E> {
    fn apply_animated_effects(&self, ctx: &mut Context, animation: TooltipAnimation, t: f32) {
        let mut effects = self.base_effects.clone();
        let t_clamped = t.clamp(0.0, 1.0);
        match animation {
            TooltipAnimation::None => {
                effects.opacity = if t_clamped > 0.0 {
                    self.base_effects.opacity
                } else {
                    0.0
                };
            }
            TooltipAnimation::Fade => {
                effects.opacity = self.base_effects.opacity * t_clamped;
            }
            TooltipAnimation::FadeAndScale => {
                effects.opacity = self.base_effects.opacity * t_clamped;
                let scale_factor = 0.92 + 0.08 * t_clamped;
                effects.scale = self.base_effects.scale * scale_factor;
            }
        }
        self.tooltip_node.set_effects(ctx, effects);
    }
}

impl<State, V: View<State>> View<State> for Tooltip<V> {
    type Element = TooltipElement<V::Element>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let inner_el = self.inner.build(ctx);
        let tooltip_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(tooltip_node, loc);
        }

        tooltip_node.set_text(ctx, &self.text);

        let mut final_style = self.resolve_style();
        let base_effects = final_style.base_effects.clone();

        if final_style.base_constraints.positioning == PositionStrategy::Inflow {
            let (tooltip_w, tooltip_h) = measure_tooltip_dimensions(&self.text, &final_style, ctx);
            final_style.base_constraints.positioning = compute_absolute_position(
                self.placement,
                self.offset,
                0.0,
                0.0,
                tooltip_w,
                tooltip_h,
            );
        }

        final_style.apply_to_node(ctx, tooltip_node);

        TooltipElement {
            inner_el,
            tooltip_node,
            is_attached: false,
            hover_start: None,
            is_hovered: false,
            anim_phase: TooltipAnimPhase::Hidden,
            anim_progress: AnimatedValue::new(0.0),
            anim_start: Instant::now(),
            base_effects,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, &mut element.inner_el);

        if ctx.active_layer != crate::layer::ActiveLayerId::Base && element.is_attached {
            element.tooltip_node.remove(ctx);
            element.is_attached = false;
            element.is_hovered = false;
            element.hover_start = None;
            element.anim_phase = TooltipAnimPhase::Hidden;
            element.anim_progress = AnimatedValue::new(0.0);
        }

        let style_changed = self.custom_style != prev.custom_style
            || self.override_style != prev.override_style
            || self.placement != prev.placement
            || self.offset != prev.offset;

        if self.text != prev.text || style_changed {
            let mut final_style = self.resolve_style();
            element.base_effects = final_style.base_effects.clone();

            let parent_node = self.inner.get_node(&element.inner_el);
            let (parent_w, parent_h) = parent_node
                .get_computed(ctx)
                .map(|c| (c.w, c.h))
                .unwrap_or((0.0, 0.0));

            if final_style.base_constraints.positioning == PositionStrategy::Inflow {
                let (tooltip_w, tooltip_h) =
                    get_tooltip_dimensions(element, &self.text, &final_style, ctx);
                final_style.base_constraints.positioning = compute_absolute_position(
                    self.placement,
                    self.offset,
                    parent_w,
                    parent_h,
                    tooltip_w,
                    tooltip_h,
                );
            }

            final_style.apply_to_node(ctx, element.tooltip_node);
            element.tooltip_node.set_text_with_userdata(
                ctx,
                &self.text,
                final_style.base_text_style,
            );

            let t = element.anim_progress.get().clamp(0.0, 1.0);
            element.apply_animated_effects(ctx, self.animation, t);
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if element.is_attached {
            element.tooltip_node.remove(ctx);
        }
        ctx.destroy_node(element.tooltip_node);
        self.inner.teardown(ctx, &mut element.inner_el);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.inner_el)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let node = self.inner.get_node(&element.inner_el);
        let mut tooltip_changed = false;

        if let Event::Tick { .. } = event {
            if self.handle_tick(element, ctx, node) {
                tooltip_changed = true;
            }
        }

        match &event {
            Event::CursorMoved { hit_nodes, .. } => {
                let is_hit = hit_nodes.contains(&node)
                    && ctx.active_layer == crate::layer::ActiveLayerId::Base;
                if is_hit {
                    if !element.is_hovered {
                        element.is_hovered = true;
                        element.hover_start = Some(Instant::now());
                    }

                    if element.anim_phase == TooltipAnimPhase::Hidden {
                        let elapsed = element.hover_start.map(|s| s.elapsed()).unwrap_or_default();
                        if elapsed >= self.delay {
                            self.start_entrance(element, ctx, node);
                            tooltip_changed = true;
                        } else {
                            ctx.request_frame();
                        }
                    } else if element.anim_phase == TooltipAnimPhase::Exiting {
                        self.start_entrance(element, ctx, node);
                        tooltip_changed = true;
                    } else {
                        self.update_position(element, ctx, node);
                    }
                } else if element.is_hovered || element.anim_phase != TooltipAnimPhase::Hidden {
                    element.is_hovered = false;
                    element.hover_start = None;
                    if element.anim_phase != TooltipAnimPhase::Hidden {
                        self.start_exit(element, ctx);
                        tooltip_changed = true;
                    }
                }
            }
            Event::MouseInput { pressed: true, .. } => {
                if element.is_hovered || element.anim_phase != TooltipAnimPhase::Hidden {
                    element.is_hovered = false;
                    element.hover_start = None;
                    element.anim_phase = TooltipAnimPhase::Hidden;
                    element.anim_progress.snap_to(0.0);
                    if element.is_attached {
                        element.tooltip_node.remove(ctx);
                        element.is_attached = false;
                        ctx.request_frame();
                        tooltip_changed = true;
                    }
                }
            }
            _ => {}
        }

        let (inner_res, inner_msg) =
            self.inner
                .handle_event(&mut element.inner_el, state, event, ctx);

        let res = if tooltip_changed {
            EventResult::Handled
        } else {
            inner_res
        };

        (res, inner_msg)
    }
}

impl<V> Tooltip<V> {
    fn update_position<E>(
        &self,
        element: &mut TooltipElement<E>,
        ctx: &mut Context,
        parent_node: Node,
    ) {
        let final_style = self.resolve_style();
        if final_style.base_constraints.positioning != PositionStrategy::Inflow {
            return;
        }

        let (parent_w, parent_h) = parent_node
            .get_computed(ctx)
            .map(|c| (c.w, c.h))
            .unwrap_or((0.0, 0.0));

        let (tooltip_w, tooltip_h) = get_tooltip_dimensions(element, &self.text, &final_style, ctx);
        let pos = compute_absolute_position(
            self.placement,
            self.offset,
            parent_w,
            parent_h,
            tooltip_w,
            tooltip_h,
        );

        element.tooltip_node.update_constraints(ctx, |c| {
            c.positioning = pos;
        });
    }

    fn start_entrance<E>(&self, element: &mut TooltipElement<E>, ctx: &mut Context, node: Node) {
        if !element.is_attached {
            node.append(ctx, element.tooltip_node);
            element.is_attached = true;
        }
        self.update_position(element, ctx, node);

        if self.animation == TooltipAnimation::None || self.enter_duration.is_zero() {
            element.anim_progress.snap_to(1.0);
            element.anim_phase = TooltipAnimPhase::Visible;
            element.apply_animated_effects(ctx, self.animation, 1.0);
        } else {
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            element.anim_progress.set_target(
                1.0,
                now,
                self.enter_duration.as_millis() as f64,
                Curve::ease_out(),
            );
            element.anim_phase = TooltipAnimPhase::Entering;
            element.apply_animated_effects(ctx, self.animation, element.anim_progress.get());
            ctx.request_frame();
        }
    }

    fn start_exit<E>(&self, element: &mut TooltipElement<E>, ctx: &mut Context) {
        if !element.is_attached || element.anim_phase == TooltipAnimPhase::Hidden {
            return;
        }

        if self.animation == TooltipAnimation::None || self.exit_duration.is_zero() {
            element.tooltip_node.remove(ctx);
            element.is_attached = false;
            element.anim_phase = TooltipAnimPhase::Hidden;
            element.anim_progress.snap_to(0.0);
            ctx.request_frame();
        } else {
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            element.anim_progress.set_target(
                0.0,
                now,
                self.exit_duration.as_millis() as f64,
                Curve::ease_in(),
            );
            element.anim_phase = TooltipAnimPhase::Exiting;
            ctx.request_frame();
        }
    }

    fn handle_tick<E>(
        &self,
        element: &mut TooltipElement<E>,
        ctx: &mut Context,
        node: Node,
    ) -> bool {
        let mut changed = false;

        if element.is_hovered && element.anim_phase == TooltipAnimPhase::Hidden {
            let elapsed = element.hover_start.map(|s| s.elapsed()).unwrap_or_default();
            if elapsed >= self.delay {
                self.start_entrance(element, ctx, node);
                changed = true;
            } else {
                ctx.request_frame();
            }
        }

        if element.is_attached
            && (element.anim_phase == TooltipAnimPhase::Entering
                || element.anim_phase == TooltipAnimPhase::Exiting)
        {
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            element.anim_progress.tick(now);
            let progress = element.anim_progress.get();
            let animating = element.anim_progress.is_animating();

            element.apply_animated_effects(ctx, self.animation, progress);
            changed = true;

            if !animating {
                if element.anim_phase == TooltipAnimPhase::Entering {
                    element.anim_phase = TooltipAnimPhase::Visible;
                } else if element.anim_phase == TooltipAnimPhase::Exiting {
                    element.tooltip_node.remove(ctx);
                    element.is_attached = false;
                    element.anim_phase = TooltipAnimPhase::Hidden;
                }
            } else {
                ctx.request_frame();
            }
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::button;

    #[test]
    fn test_tooltip_defaults() {
        let t = tooltip(button::<_, ()>("Save"), "Save file");
        assert_eq!(t.text, "Save file");
        assert_eq!(t.placement, TooltipPlacement::Top);
        assert_eq!(t.offset, 6.0);
        assert_eq!(t.delay, Duration::ZERO);
        assert_eq!(t.enter_duration, Duration::from_millis(150));
        assert_eq!(t.exit_duration, Duration::from_millis(150));
        assert_eq!(t.animation, TooltipAnimation::FadeAndScale);
        assert!(t.custom_style.is_none());
        assert!(t.override_style.is_none());
    }

    #[test]
    fn test_tooltip_builder_chaining() {
        let t = tooltip(button::<_, ()>("Delete"), "Delete item")
            .placement(TooltipPlacement::BottomEnd)
            .offset(10.0)
            .delay_ms(300)
            .enter_duration_ms(200)
            .exit_duration_ms(100)
            .animation(TooltipAnimation::Fade)
            .style(Style::new().bg_color(rgb!(220, 38, 38)));

        assert_eq!(t.placement, TooltipPlacement::BottomEnd);
        assert_eq!(t.offset, 10.0);
        assert_eq!(t.delay, Duration::from_millis(300));
        assert_eq!(t.enter_duration, Duration::from_millis(200));
        assert_eq!(t.exit_duration, Duration::from_millis(100));
        assert_eq!(t.animation, TooltipAnimation::Fade);
        assert!(t.custom_style.is_some());
    }

    #[test]
    fn test_tooltip_style_merge() {
        let custom = Style::new().bg_color(rgb!(255, 0, 0)).corner_radius(8.0);
        let t = tooltip(button::<_, ()>("Action"), "Hint").style(custom);
        let resolved = t.resolve_style();

        // Overridden properties
        assert_eq!(resolved.base_effects.background_color, rgb!(255, 0, 0));
        assert_eq!(resolved.base_effects.border.radius.tl, 8.0);

        // Preserved defaults
        assert_eq!(resolved.base_constraints.padding.left, 8.0);
        assert_eq!(resolved.base_constraints.padding.top, 4.0);
        assert_eq!(resolved.base_constraints.z_index, 3000);
        assert_eq!(resolved.base_text_style.color, clr!(white));
        assert_eq!(resolved.base_text_style.font_size, 11.0);
    }

    #[test]
    fn test_tooltip_style_override() {
        let overhaul = Style::new().bg_color(clr!(black)).padding(16.0);
        let t = tooltip(button::<_, ()>("Action"), "Hint").override_style(overhaul);
        let resolved = t.resolve_style();

        assert_eq!(resolved.base_effects.background_color, clr!(black));
        assert_eq!(resolved.base_constraints.padding.left, 16.0);
        assert_eq!(resolved.base_constraints.z_index, 0);
        assert_eq!(resolved.base_constraints.border.top, 0.0);
    }

    #[test]
    fn test_tooltip_compute_positions() {
        let pw = 100.0;
        let ph = 40.0;
        let tw = 60.0;
        let th = 20.0;
        let offset = 8.0;

        // Top
        let pos_top = compute_absolute_position(TooltipPlacement::Top, offset, pw, ph, tw, th);
        if let PositionStrategy::Absolute { left, top, .. } = pos_top {
            assert_eq!(left, (100.0 - 60.0) / 2.0);
            assert_eq!(top, -(20.0 + 8.0));
        } else {
            panic!("Expected absolute position");
        }

        // Bottom
        let pos_bot = compute_absolute_position(TooltipPlacement::Bottom, offset, pw, ph, tw, th);
        if let PositionStrategy::Absolute { left, top, .. } = pos_bot {
            assert_eq!(left, 20.0);
            assert_eq!(top, 48.0);
        } else {
            panic!("Expected absolute position");
        }

        // Left
        let pos_left = compute_absolute_position(TooltipPlacement::Left, offset, pw, ph, tw, th);
        if let PositionStrategy::Absolute { top, left, .. } = pos_left {
            assert_eq!(top, 10.0);
            assert_eq!(left, -(60.0 + 8.0));
        } else {
            panic!("Expected absolute position");
        }

        // Right
        let pos_right = compute_absolute_position(TooltipPlacement::Right, offset, pw, ph, tw, th);
        if let PositionStrategy::Absolute { top, left, .. } = pos_right {
            assert_eq!(top, 10.0);
            assert_eq!(left, 108.0);
        } else {
            panic!("Expected absolute position");
        }
    }

    #[test]
    fn test_tooltip_lifecycle_hover_and_dismiss() {
        let mut ctx = Context::new();
        let btn = button::<_, ()>("Click").on_click(());
        let t = tooltip(btn, "Helpful Tip")
            .delay(Duration::ZERO)
            .animation(TooltipAnimation::None);

        let mut el = View::<()>::build(&t, &mut ctx);
        let node = View::<()>::get_node(&t, &el);

        assert!(!el.is_attached);

        // Hover over node
        let _ = View::<()>::handle_event(
            &t,
            &mut el,
            &(),
            Event::CursorMoved {
                x: 10.0,
                y: 10.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );

        assert!(el.is_hovered);
        assert!(el.is_attached);
        assert_eq!(el.anim_progress.get(), 1.0);

        // Mouse click dismisses immediately
        let _ = View::<()>::handle_event(
            &t,
            &mut el,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 10.0,
                y: 10.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );

        assert!(!el.is_hovered);
        assert!(!el.is_attached);

        View::<()>::teardown(&t, &mut ctx, &mut el);
    }

    #[test]
    fn test_tooltip_fade_and_scale_animation_lifecycle() {
        let mut ctx = Context::new();
        let btn = button::<_, ()>("Click").on_click(());
        let t = tooltip(btn, "Helpful Tip")
            .delay(Duration::ZERO)
            .enter_duration_ms(100)
            .exit_duration_ms(100)
            .animation(TooltipAnimation::FadeAndScale);

        let mut el = View::<()>::build(&t, &mut ctx);
        let node = View::<()>::get_node(&t, &el);

        assert_eq!(el.anim_phase, TooltipAnimPhase::Hidden);
        assert!(!el.is_attached);

        // 1. Mouse hover enters
        let _ = View::<()>::handle_event(
            &t,
            &mut el,
            &(),
            Event::CursorMoved {
                x: 10.0,
                y: 10.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );

        assert!(el.is_hovered);
        assert!(el.is_attached);
        assert_eq!(el.anim_phase, TooltipAnimPhase::Entering);

        // 2. Tick while entering
        el.anim_start = Instant::now() - Duration::from_millis(50);
        let _ = View::<()>::handle_event(&t, &mut el, &(), Event::Tick { dt: 0.05 }, &mut ctx);

        assert_eq!(el.anim_phase, TooltipAnimPhase::Entering);
        let mid_progress = el.anim_progress.get();
        assert!(mid_progress > 0.1 && mid_progress < 0.99);

        // 3. Tick after enter duration completes
        el.anim_start = Instant::now() - Duration::from_millis(150);
        let _ = View::<()>::handle_event(&t, &mut el, &(), Event::Tick { dt: 0.1 }, &mut ctx);

        assert_eq!(el.anim_phase, TooltipAnimPhase::Visible);
        assert_eq!(el.anim_progress.get(), 1.0);

        // 4. Mouse leaves anchor node
        let _ = View::<()>::handle_event(
            &t,
            &mut el,
            &(),
            Event::CursorMoved {
                x: 999.0,
                y: 999.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: vec![],
            },
            &mut ctx,
        );

        assert!(!el.is_hovered);
        assert!(el.is_attached);
        assert_eq!(el.anim_phase, TooltipAnimPhase::Exiting);

        // 5. Tick after exit duration completes
        el.anim_start = Instant::now() - Duration::from_millis(300);
        let _ = View::<()>::handle_event(&t, &mut el, &(), Event::Tick { dt: 0.15 }, &mut ctx);

        assert_eq!(el.anim_phase, TooltipAnimPhase::Hidden);
        assert!(!el.is_attached);

        View::<()>::teardown(&t, &mut ctx, &mut el);
    }

    #[test]
    fn test_tooltip_delayed_hover_lifecycle() {
        let mut ctx = Context::new();
        let btn = button::<_, ()>("Click").on_click(());
        let t = tooltip(btn, "Delayed Hint")
            .delay_ms(200)
            .enter_duration_ms(100)
            .animation(TooltipAnimation::Fade);

        let mut el = View::<()>::build(&t, &mut ctx);
        let node = View::<()>::get_node(&t, &el);

        // Hover over node at t = 0
        let _ = View::<()>::handle_event(
            &t,
            &mut el,
            &(),
            Event::CursorMoved {
                x: 10.0,
                y: 10.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );

        assert!(el.is_hovered);
        assert!(!el.is_attached);
        assert_eq!(el.anim_phase, TooltipAnimPhase::Hidden);

        // Tick before delay has passed (100ms < 200ms)
        el.hover_start = Some(Instant::now() - Duration::from_millis(100));
        let _ = View::<()>::handle_event(&t, &mut el, &(), Event::Tick { dt: 0.016 }, &mut ctx);

        assert!(!el.is_attached);
        assert_eq!(el.anim_phase, TooltipAnimPhase::Hidden);

        // Tick after delay has passed (250ms >= 200ms)
        el.hover_start = Some(Instant::now() - Duration::from_millis(250));
        let _ = View::<()>::handle_event(&t, &mut el, &(), Event::Tick { dt: 0.016 }, &mut ctx);

        assert!(el.is_attached);
        assert_eq!(el.anim_phase, TooltipAnimPhase::Entering);

        View::<()>::teardown(&t, &mut ctx, &mut el);
    }
}
