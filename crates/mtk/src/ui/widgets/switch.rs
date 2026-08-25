use std::marker::PhantomData;
use std::time::Instant;
use winit::keyboard::{Key, NamedKey};

use crate::animation::{Animatable, AnimatedValue, Curve};
use crate::colors::Color;
use crate::style::{AlignItems, FlexDirection, JustifyContent, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, clr, rgb, rgba};

/// A smooth pill-shaped toggle switch widget with fluid animation.
pub struct Switch<Msg, F = fn(bool) -> Msg> {
    pub(crate) is_on: bool,
    pub(crate) on_toggle: Option<F>,
    pub(crate) disabled: bool,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Switch` widget with the given boolean toggle state.
///
/// # Examples
/// ```rust,ignore
/// switch(state.notifications_enabled).on_toggle(|on| AppMsg::SetNotifications(on))
/// ```
pub fn switch<Msg>(is_on: bool) -> Switch<Msg, fn(bool) -> Msg> {
    Switch {
        is_on,
        on_toggle: None,
        disabled: false,
        _marker: PhantomData,
    }
}

impl<Msg, F> Switch<Msg, F> {
    /// Sets the callback invoked when the switch is toggled.
    pub fn on_toggle<NewF: Fn(bool) -> Msg>(self, on_toggle: NewF) -> Switch<Msg, NewF> {
        Switch {
            is_on: self.is_on,
            on_toggle: Some(on_toggle),
            disabled: self.disabled,
            _marker: PhantomData,
        }
    }

    /// Disables or enables the switch.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct SwitchElement {
    track_node: Node,
    knob_node: Node,
    is_pressed: bool,
    anim_progress: AnimatedValue<f32>,
    anim_start: Instant,
}

impl<State, Msg, F> View<State> for Switch<Msg, F>
where
    F: Fn(bool) -> Msg,
{
    type Element = SwitchElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let track_node = ctx.create_node();
        let knob_node = ctx.create_node();

        let initial_progress = if self.is_on { 1.0f32 } else { 0.0f32 };
        let anim_progress = AnimatedValue::new(initial_progress);
        let anim_start = Instant::now();

        let off_bg = rgb!(226, 232, 240);
        let on_bg = rgb!(59, 130, 246);
        let disabled_bg = rgb!(203, 213, 225);

        let track_bg = if self.disabled {
            disabled_bg
        } else {
            Color::interpolate(&off_bg, &on_bg, initial_progress as f64)
        };

        let initial_pad_left = 2.0 + initial_progress * 20.0;

        Style::new()
            .flex_direction(FlexDirection::Row)
            .width(Size::Fixed(44))
            .height(Size::Fixed(24))
            .corner_radius(12.0)
            .bg_color(track_bg)
            .padding_xy(2.0, 2.0)
            .justify_content(JustifyContent::Start)
            .align_items(AlignItems::Center)
            .apply_to_node(ctx, track_node);

        track_node.update_constraints(ctx, |c| {
            c.padding.left = initial_pad_left;
        });

        Style::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(20))
            .corner_radius(10.0)
            .bg_color(clr!(white))
            .shadow(rgba!(0, 0, 0, 40), 4.0, 0.2)
            .apply_to_node(ctx, knob_node);

        track_node.append(ctx, knob_node);

        if !self.disabled {
            ctx.register_focusable(track_node);
        }

        SwitchElement {
            track_node,
            knob_node,
            is_pressed: false,
            anim_progress,
            anim_start,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.is_on != prev.is_on {
            let target = if self.is_on { 1.0f32 } else { 0.0f32 };
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            element
                .anim_progress
                .set_target(target, now, 160.0, Curve::ease_out());
            ctx.request_frame();
        }

        if self.disabled != prev.disabled {
            let off_bg = rgb!(226, 232, 240);
            let on_bg = rgb!(59, 130, 246);
            let disabled_bg = rgb!(203, 213, 225);

            let track_bg = if self.disabled {
                disabled_bg
            } else {
                Color::interpolate(&off_bg, &on_bg, element.anim_progress.get() as f64)
            };

            element.track_node.update_effects(ctx, |e| {
                e.background_color = track_bg;
            });
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_focusable(element.track_node);
        element.knob_node.remove(ctx);
        ctx.destroy_node(element.knob_node);
        element.track_node.remove(ctx);
        ctx.destroy_node(element.track_node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.track_node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        _state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if let Event::Tick { .. } = event {
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            if element.anim_progress.tick(now) || element.anim_progress.is_animating() {
                let progress = element.anim_progress.get();
                let pad_left = 2.0 + progress * 20.0;

                element.track_node.update_constraints(ctx, |c| {
                    c.padding.left = pad_left;
                });

                if !self.disabled {
                    let off_bg = rgb!(226, 232, 240);
                    let on_bg = rgb!(59, 130, 246);
                    let bg = Color::interpolate(&off_bg, &on_bg, progress as f64);
                    element.track_node.update_effects(ctx, |e| {
                        e.background_color = bg;
                    });
                }

                ctx.request_frame();
                return (EventResult::Handled, None);
            }
        }

        if self.disabled {
            return (EventResult::Ignored, None);
        }

        match event {
            Event::MouseInput {
                pressed, hit_nodes, ..
            } => {
                let is_hit = hit_nodes.contains(&element.track_node)
                    || hit_nodes.contains(&element.knob_node);

                if is_hit && pressed {
                    element.is_pressed = true;
                    ctx.request_focus(element.track_node);
                    (EventResult::Handled, None)
                } else if !pressed && element.is_pressed {
                    element.is_pressed = false;
                    if is_hit {
                        let new_val = !self.is_on;
                        let msg = self.on_toggle.as_ref().map(|f| f(new_val));
                        (EventResult::Handled, msg)
                    } else {
                        (EventResult::Handled, None)
                    }
                } else {
                    (EventResult::Ignored, None)
                }
            }
            Event::KeyboardInput { event: k_event, .. } => {
                if Some(element.track_node) == ctx.focused_node() && k_event.state.is_pressed() {
                    match k_event.logical_key {
                        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => {
                            let new_val = !self.is_on;
                            let msg = self.on_toggle.as_ref().map(|f| f(new_val));
                            (EventResult::Handled, msg)
                        }
                        _ => (EventResult::Ignored, None),
                    }
                } else {
                    (EventResult::Ignored, None)
                }
            }
            _ => (EventResult::Ignored, None),
        }
    }
}
