//! Declarative styling wrappers, animated property transitions, and view extension traits.
//!
//! This module provides the [`StyledView`] component, persistent [`StyledViewState`] animation
//! tracking, [`KeyframedView`], and the [`ViewStyleExt`] extension trait for attaching [`Style`]
//! declarations and keyframe animations to any view.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::animation::{AnimatedValue, Curve, Keyframes};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, Overflow, Style, TextRenderInfo, TextStyle, TransitionProperty};

/// Extension trait for [`View`] that enables fluid `.style(...)` and `.animate_keyframes(...)` method chaining.
pub trait ViewStyleExt: Sized {
    /// Attaches a [`Style`] configuration to this view.
    ///
    /// # Examples
    /// ```rust,ignore
    /// use mtk::style::Style;
    /// use mtk::ui::ViewStyleExt;
    /// use mtk::ui::widgets::text;
    ///
    /// let styled_text = text("Hello").style(Style::new().padding(10.0));
    /// ```
    fn style(self, style: Style) -> StyledView<Self>;

    /// Attaches a [`Keyframes`] timeline animation sequence to this view.
    fn animate_keyframes(self, keyframes: Keyframes<Style>) -> KeyframedView<Self>;

    /// Wraps this view with a hover [`Tooltip`].
    fn tooltip(self, text: impl Into<String>) -> crate::ui::widgets::Tooltip<Self> {
        crate::ui::widgets::tooltip(self, text)
    }
}

impl<V> ViewStyleExt for V {
    fn style(self, style: Style) -> StyledView<Self> {
        StyledView { inner: self, style }
    }

    fn animate_keyframes(self, keyframes: Keyframes<Style>) -> KeyframedView<Self> {
        KeyframedView {
            inner: self,
            keyframes,
        }
    }
}

/// Persistent element state for a [`StyledView`], tracking hover, active, focus states and active property transition animations.
#[derive(Default)]
pub struct StyledViewState {
    /// `true` if the mouse cursor is currently hovering over the view's layout node.
    pub is_hovered: bool,
    /// `true` if the mouse button is pressed over the view's layout node (active state).
    pub is_active: bool,
    /// `true` if the layout node has input focus.
    pub is_focused: bool,
    /// `true` if the element is disabled.
    pub is_disabled: bool,
    /// `true` if any property transition animation is currently progressing.
    pub is_animating: bool,
    /// Active animation state interpolating the computed style.
    pub style_anim: Option<AnimatedValue<Style>>,
}

impl StyledViewState {
    /// Creates a new unhovered, inactive [`StyledViewState`].
    pub fn new() -> Self {
        Self {
            is_hovered: false,
            is_active: false,
            is_focused: false,
            is_disabled: false,
            is_animating: false,
            style_anim: None,
        }
    }
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * 1000.0
}

/// A wrapper component that applies declarative [`Style`] properties, pseudo-classes, and smooth property transitions to an inner view `V`.
pub struct StyledView<V> {
    pub(crate) inner: V,
    pub(crate) style: Style,
}

impl<State, V: View<State>> View<State> for StyledView<V> {
    type Element = (V::Element, StyledViewState);
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let child_el = self.inner.build(ctx);
        let node = self.inner.get_node(&child_el);

        node.update_constraints(ctx, |c| {
            let overflow = c.overflow;
            let scroll = c.scroll;
            let flex_dir = self.style.flex_direction.unwrap_or(c.flex_direction);
            *c = self.style.base_constraints;
            c.flex_direction = flex_dir;

            if self.style.base_constraints.overflow == Overflow::Visible
                && overflow != Overflow::Visible
            {
                c.overflow = overflow;
            }
            c.scroll = scroll;
        });

        node.set_effects(ctx, self.style.base_effects.clone());

        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            if self.style.base_text_style != TextStyle::default() {
                if let Some(mut info) = node.get_text_userdata::<TextRenderInfo>(ctx).cloned() {
                    info.style = self.style.base_text_style.clone();
                    node.set_text_with_userdata(ctx, &text_owned, info);
                } else {
                    node.set_text_with_userdata(
                        ctx,
                        &text_owned,
                        self.style.base_text_style.clone(),
                    );
                }
            }
        }

        let mut view_state = StyledViewState::new();
        view_state.is_focused = Some(node) == ctx.focused_node();
        if !self.style.transitions.is_empty() {
            view_state.style_anim = Some(AnimatedValue::new(self.style.clone()));
        }

        (child_el, view_state)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, &mut element.0);
        let node = self.inner.get_node(&element.0);
        element.1.is_focused = Some(node) == ctx.focused_node();
        element.1.is_animating = self.apply_style(ctx, &mut element.1, node);

        if element.1.is_animating {
            ctx.request_frame();
        }
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
    ) {
        self.inner
            .rebuild_with_parent(&prev.inner, ctx, &mut element.0, parent);
        let node = self.inner.get_node(&element.0);
        element.1.is_focused = Some(node) == ctx.focused_node();
        element.1.is_animating = self.apply_style(ctx, &mut element.1, node);

        if element.1.is_animating {
            ctx.request_frame();
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.0);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.0)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let mut state_changed = false;
        let node = self.inner.get_node(&element.0);

        let newly_focused = Some(node) == ctx.focused_node();
        if element.1.is_focused != newly_focused {
            element.1.is_focused = newly_focused;
            state_changed = true;
        }

        match &event {
            Event::CursorMoved { hit_nodes, .. } => {
                let newly_hovered = hit_nodes.contains(&node);
                if element.1.is_hovered != newly_hovered {
                    element.1.is_hovered = newly_hovered;
                    state_changed = true;
                }
            }
            Event::MouseInput {
                pressed, hit_nodes, ..
            } => {
                let is_hit = hit_nodes.contains(&node);
                let new_active = *pressed && is_hit;
                if element.1.is_active != new_active {
                    element.1.is_active = new_active;
                    state_changed = true;
                }
            }
            _ => {}
        }

        let is_tick = matches!(event, Event::Tick { .. });

        if state_changed || (is_tick && element.1.is_animating) {
            element.1.is_animating = self.apply_style(ctx, &mut element.1, node);
            if state_changed {
                ctx.request_frame();
            }
        }

        let res = self.inner.handle_event(&mut element.0, state, event, ctx);

        let after_focused = Some(node) == ctx.focused_node();
        if element.1.is_focused != after_focused {
            element.1.is_focused = after_focused;
            element.1.is_animating = self.apply_style(ctx, &mut element.1, node);
            ctx.request_frame();
        }

        res
    }
}

impl<V> StyledView<V> {
    fn compute_target_style(&self, view_state: &StyledViewState) -> Style {
        let mut target = self.style.clone();

        if view_state.is_disabled {
            if let Some(disabled) = &self.style.disabled {
                target = target.merge((**disabled).clone());
            }
        }

        if view_state.is_focused {
            if let Some(focus) = &self.style.focus {
                target = target.merge((**focus).clone());
            }
        }

        if view_state.is_hovered {
            if let Some(hover) = &self.style.hover {
                target = target.merge((**hover).clone());
            }
        }

        if view_state.is_active {
            if let Some(active) = &self.style.active {
                target = target.merge((**active).clone());
            }
        }

        target
    }

    fn apply_style(&self, ctx: &mut Context, view_state: &mut StyledViewState, node: Node) -> bool {
        let target_style = self.compute_target_style(view_state);
        let mut is_animating = false;

        let active_style = if !self.style.transitions.is_empty() {
            let mut transition_duration = 200.0;
            let mut transition_curve = Curve::ease_out();

            for t in &self.style.transitions {
                if t.property == TransitionProperty::All || t.duration_ms > transition_duration {
                    transition_duration = t.duration_ms;
                    transition_curve = t.curve;
                }
            }

            if view_state.style_anim.is_none() {
                view_state.style_anim = Some(AnimatedValue::new(target_style.clone()));
            }

            let anim = view_state.style_anim.as_mut().unwrap();
            anim.set_target(
                target_style.clone(),
                now_ms(),
                transition_duration,
                transition_curve,
            );

            if anim.tick(now_ms()) {
                is_animating = true;
                anim.current.clone()
            } else {
                target_style.clone()
            }
        } else {
            target_style.clone()
        };

        // Apply constraints
        node.update_constraints(ctx, |c| {
            let overflow = c.overflow;
            let scroll = c.scroll;
            let flex_dir = active_style.flex_direction.unwrap_or(c.flex_direction);
            *c = active_style.base_constraints;
            c.flex_direction = flex_dir;

            if active_style.base_constraints.overflow == Overflow::Visible
                && overflow != Overflow::Visible
            {
                c.overflow = overflow;
            }
            c.scroll = scroll;
        });

        // Apply effects
        node.set_effects(ctx, active_style.base_effects.clone());

        // Apply text style
        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            if active_style.base_text_style != TextStyle::default() {
                if node.get_text_userdata::<TextRenderInfo>(ctx).is_none() {
                    node.set_text_with_userdata(
                        ctx,
                        &text_owned,
                        active_style.base_text_style.clone(),
                    );
                }
            }
        }

        if is_animating {
            ctx.request_frame();
        }

        is_animating
    }
}

/// A wrapper component that applies a [`Keyframes`] timeline animation sequence to an inner view `V`.
pub struct KeyframedView<V> {
    pub(crate) inner: V,
    pub(crate) keyframes: Keyframes<Style>,
}

#[derive(Default)]
pub struct KeyframedViewState {
    pub start_time: f64,
    pub is_active: bool,
}

impl<State, V: View<State>> View<State> for KeyframedView<V> {
    type Element = (V::Element, KeyframedViewState);
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let child_el = self.inner.build(ctx);
        let node = self.inner.get_node(&child_el);
        let start_time = now_ms();

        let (style, is_active) = self.keyframes.evaluate(0.0);
        self.apply_evaluated_style(ctx, node, &style);

        if is_active {
            ctx.request_frame();
        }

        (
            child_el,
            KeyframedViewState {
                start_time,
                is_active,
            },
        )
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, &mut element.0);
        let node = self.inner.get_node(&element.0);
        let elapsed = now_ms() - element.1.start_time;
        let (style, is_active) = self.keyframes.evaluate(elapsed);
        element.1.is_active = is_active;
        self.apply_evaluated_style(ctx, node, &style);

        if is_active {
            ctx.request_frame();
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.0);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.0)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if matches!(event, Event::Tick { .. }) && element.1.is_active {
            let node = self.inner.get_node(&element.0);
            let elapsed = now_ms() - element.1.start_time;
            let (style, is_active) = self.keyframes.evaluate(elapsed);
            element.1.is_active = is_active;
            self.apply_evaluated_style(ctx, node, &style);

            if is_active {
                ctx.request_frame();
            }
        }

        self.inner.handle_event(&mut element.0, state, event, ctx)
    }
}

impl<V> KeyframedView<V> {
    fn apply_evaluated_style(&self, ctx: &mut Context, node: Node, style: &Style) {
        node.update_constraints(ctx, |c| {
            let overflow = c.overflow;
            let scroll = c.scroll;
            let flex_dir = style.flex_direction.unwrap_or(c.flex_direction);
            *c = style.base_constraints;
            c.flex_direction = flex_dir;

            if style.base_constraints.overflow == Overflow::Visible && overflow != Overflow::Visible
            {
                c.overflow = overflow;
            }
            c.scroll = scroll;
        });

        node.set_effects(ctx, style.base_effects.clone());

        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            if node.get_text_userdata::<TextRenderInfo>(ctx).is_none() {
                node.set_text_with_userdata(ctx, &text_owned, style.base_text_style.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Edges;
    use crate::FlexDirection;
    use crate::ui::widgets::{column, row, text};

    #[test]
    fn test_row_flex_direction_preserved_when_styled() {
        let mut ctx = Context::new();
        let row_view =
            row((text::<_, ()>("Left"), text::<_, ()>("Right"))).style(Style::new().padding(10.0));

        let element = View::<()>::build(&row_view, &mut ctx);
        let node = View::<()>::get_node(&row_view, &element);
        let constraints = node.get_constraints(&ctx).unwrap();

        assert_eq!(constraints.flex_direction, FlexDirection::Row);
    }

    #[test]
    fn test_column_flex_direction_preserved_when_styled() {
        let mut ctx = Context::new();
        let col_view = column((text::<_, ()>("Top"), text::<_, ()>("Bottom")))
            .style(Style::new().padding(10.0));

        let element = View::<()>::build(&col_view, &mut ctx);
        let node = View::<()>::get_node(&col_view, &element);
        let constraints = node.get_constraints(&ctx).unwrap();

        assert_eq!(constraints.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn test_row_flex_direction_explicit_override() {
        let mut ctx = Context::new();
        let row_view = row((text::<_, ()>("A"), text::<_, ()>("B")))
            .style(Style::new().flex_direction(FlexDirection::Column));

        let element = View::<()>::build(&row_view, &mut ctx);
        let node = View::<()>::get_node(&row_view, &element);
        let constraints = node.get_constraints(&ctx).unwrap();

        assert_eq!(constraints.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn test_style_merge_and_mixins() {
        let base = Style::new().padding(10.0).corner_radius(12.0);
        let mixin = |s: Style| s.scale(1.2).opacity(0.8);
        let combined = base.apply(mixin).when(true, |s| s.gap(8.0));

        assert_eq!(combined.base_constraints.padding, Edges::all(10.0));
        assert_eq!(combined.base_effects.scale, 1.2);
        assert_eq!(combined.base_effects.opacity, 0.8);
        assert_eq!(combined.base_constraints.gap, 8.0);
    }

    #[test]
    fn test_frosted_glass_layout() {
        use crate::rgb;
        use crate::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
        use crate::text_property::FontWeight;
        use crate::ui::ViewStyleExt;
        use crate::ui::widgets::{column, row, text};

        let mut ctx = Context::new();
        ctx.set_text_sizing_func(move |ctx, _node, text, userdata, avail_w, avail_h| {
            let default_style = TextStyle::default();
            let style = if let Some(info) =
                userdata.and_then(|u| u.downcast_ref::<crate::TextRenderInfo>())
            {
                &info.style
            } else if let Some(style) = userdata.and_then(|u| u.downcast_ref::<TextStyle>()) {
                style
            } else {
                &default_style
            };

            let text_ctx = ctx.text_context.clone();
            crate::text::measure_text(text, style, avail_w, avail_h, &text_ctx)
        });

        let view = row((column((
            // Window Top Title Bar
            row((
                text::<_, ()>("Frosted Acrylic Glass Inspector").style(
                    Style::new().set_text_style(TextStyle {
                        font_size: 16.0,
                        color: rgb!(15, 23, 42),
                        font_weight: FontWeight::BOLD,
                        ..Default::default()
                    }),
                ),
                text::<_, ()>("Blur Active").style(
                    Style::new()
                        .padding_xy(10.0, 4.0)
                        .set_text_style(TextStyle {
                            font_size: 11.5,
                            color: rgb!(5, 150, 105),
                            ..Default::default()
                        }),
                ),
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::SpaceBetween),
            ),
        ))
        .style(
            Style::new()
                .width(Size::Fixed(720))
                .padding(22.0)
                .blur(0.65),
        ),));

        let element = View::<()>::build(&view, &mut ctx);
        let root_node = View::<()>::get_node(&view, &element);
        ctx.root_attach(root_node);
        ctx.compute_layout(900.0, 720.0);
        ctx.build_render_list(crate::style::Rect {
            x: 0.0,
            y: 0.0,
            w: 900.0,
            h: 720.0,
        });

        for (idx, cmd) in ctx.render_list().enumerate() {
            println!(
                "CMD {}: kind={:?}, node={:?}, computed={:?}, clip={:?}",
                idx,
                cmd.kind(),
                cmd.node(),
                cmd.computed(),
                cmd.clip()
            );
            if let Some(txt) = cmd.node().get_text(&ctx) {
                println!("   -> text: {:?}", txt);
            }
        }

        println!(
            "Node 3 text: {:?}",
            crate::Node(crate::sys::muId {
                numeral: 3,
                generation: 0
            })
            .get_text(&ctx)
        );
        println!(
            "Node 3 computed: {:?}",
            crate::Node(crate::sys::muId {
                numeral: 3,
                generation: 0
            })
            .get_computed(&ctx)
        );
    }

    #[test]
    fn test_styled_view_focus_ring() {
        let mut ctx = Context::new();
        let view = crate::ui::widgets::input_text().style(
            Style::new()
                .border(1.0, crate::rgb!(100, 100, 100))
                .on_focus(|s| s.border(3.0, crate::rgb!(0, 120, 255))),
        );

        let mut el = view.build(&mut ctx);
        let node = view.get_node(&el);

        // Before focus: border is 1.0
        let cons_unfocused = node.get_constraints(&ctx).unwrap();
        assert_eq!(cons_unfocused.border.top, 1.0);

        // Request focus
        ctx.request_focus(node);
        view.rebuild(&view, &mut ctx, &mut el);

        // After focus: on_focus applied, border becomes 3.0
        let cons_focused = node.get_constraints(&ctx).unwrap();
        assert_eq!(cons_focused.border.top, 3.0);
    }
}
