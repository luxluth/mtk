use std::marker::PhantomData;
use winit::keyboard::{Key, NamedKey};

use crate::style::{AlignItems, FlexDirection, JustifyContent, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, rgb};

/// A continuous or stepped horizontal range slider widget.
pub struct Slider<Msg, F = fn(f32) -> Msg> {
    pub(crate) value: f32,
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) step: Option<f32>,
    pub(crate) on_change: Option<F>,
    pub(crate) disabled: bool,
    pub(crate) width: Option<Size>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Slider` widget bound to the current value within the `[min, max]` range.
///
/// # Examples
/// ```rust,ignore
/// slider(state.volume, 0.0, 100.0).on_change(|val| AppMsg::SetVolume(val))
/// ```
pub fn slider<Msg>(value: f32, min: f32, max: f32) -> Slider<Msg, fn(f32) -> Msg> {
    Slider {
        value,
        min,
        max,
        step: None,
        on_change: None,
        disabled: false,
        width: None,
        _marker: PhantomData,
    }
}

impl<Msg, F> Slider<Msg, F> {
    /// Sets a step increment for discrete values (e.g. `1.0` or `5.0`).
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }

    /// Sets the callback invoked when the slider value changes during dragging or clicking.
    pub fn on_change<NewF: Fn(f32) -> Msg>(self, on_change: NewF) -> Slider<Msg, NewF> {
        Slider {
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
            on_change: Some(on_change),
            disabled: self.disabled,
            width: self.width,
            _marker: PhantomData,
        }
    }

    /// Sets an explicit width for the slider widget.
    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    /// Disables or enables the slider.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn calculate_pct(&self) -> f32 {
        if self.max <= self.min {
            0.0
        } else {
            ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
        }
    }
}

pub struct SliderElement {
    container_node: Node,
    track_node: Node,
    fill_node: Node,
    is_dragging: bool,
}

impl<State, Msg, F> View<State> for Slider<Msg, F>
where
    F: Fn(f32) -> Msg,
{
    type Element = SliderElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        let track_node = ctx.create_node();
        let fill_node = ctx.create_node();

        let pct = self.calculate_pct();

        Style::new()
            .width(self.width.unwrap_or(Size::Fixed(160)))
            .height(Size::Fixed(24))
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center)
            .apply_to_node(ctx, container_node);

        Style::new()
            .flex_direction(FlexDirection::Row)
            .width(Size::Percent(1.0))
            .height(Size::Fixed(6))
            .corner_radius(3.0)
            .bg_color(if self.disabled {
                rgb!(226, 232, 240)
            } else {
                rgb!(226, 232, 240)
            })
            .apply_to_node(ctx, track_node);

        Style::new()
            .width(Size::Percent(pct))
            .height(Size::Fixed(6))
            .corner_radius(3.0)
            .bg_color(if self.disabled {
                rgb!(148, 163, 184)
            } else {
                rgb!(59, 130, 246)
            })
            .apply_to_node(ctx, fill_node);

        track_node.append(ctx, fill_node);
        container_node.append(ctx, track_node);

        if !self.disabled {
            ctx.register_focusable(container_node);
        }

        SliderElement {
            container_node,
            track_node,
            fill_node,
            is_dragging: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if (self.value - prev.value).abs() > 0.0001
            || self.min != prev.min
            || self.max != prev.max
            || self.disabled != prev.disabled
        {
            let pct = self.calculate_pct();
            element.fill_node.update_constraints(ctx, |c| {
                c.width = Size::Percent(pct);
            });
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_focusable(element.container_node);
        element.fill_node.remove(ctx);
        ctx.destroy_node(element.fill_node);
        element.track_node.remove(ctx);
        ctx.destroy_node(element.track_node);
        element.container_node.remove(ctx);
        ctx.destroy_node(element.container_node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.container_node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        _state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if self.disabled {
            return (EventResult::Ignored, None);
        }

        match event {
            Event::MouseInput {
                pressed,
                x,
                hit_nodes,
                ..
            } => {
                let is_hit = hit_nodes.contains(&element.container_node)
                    || hit_nodes.contains(&element.track_node)
                    || hit_nodes.contains(&element.fill_node);

                if is_hit && pressed {
                    element.is_dragging = true;
                    ctx.request_focus(element.container_node);
                    if let Some(computed) = element.container_node.get_computed(ctx) {
                        let rel_x = (x - computed.x).clamp(0.0, computed.w);
                        let pct = if computed.w > 0.0 {
                            rel_x / computed.w
                        } else {
                            0.0
                        };
                        let mut val = self.min + pct * (self.max - self.min);
                        if let Some(step) = self.step {
                            if step > 0.0 {
                                val = (val / step).round() * step;
                            }
                        }
                        val = val.clamp(self.min, self.max);
                        let msg = self.on_change.as_ref().map(|f| f(val));
                        return (EventResult::Handled, msg);
                    }
                    (EventResult::Handled, None)
                } else if !pressed && element.is_dragging {
                    element.is_dragging = false;
                    (EventResult::Handled, None)
                } else {
                    (EventResult::Ignored, None)
                }
            }
            Event::CursorMoved { x, .. } => {
                if element.is_dragging {
                    if let Some(computed) = element.container_node.get_computed(ctx) {
                        let rel_x = (x - computed.x).clamp(0.0, computed.w);
                        let pct = if computed.w > 0.0 {
                            rel_x / computed.w
                        } else {
                            0.0
                        };
                        let mut val = self.min + pct * (self.max - self.min);
                        if let Some(step) = self.step {
                            if step > 0.0 {
                                val = (val / step).round() * step;
                            }
                        }
                        val = val.clamp(self.min, self.max);
                        let msg = self.on_change.as_ref().map(|f| f(val));
                        return (EventResult::Handled, msg);
                    }
                }
                (EventResult::Ignored, None)
            }
            Event::KeyboardInput { event: k_event, .. } => {
                if Some(element.container_node) == ctx.focused_node() && k_event.state.is_pressed()
                {
                    let delta = self.step.unwrap_or((self.max - self.min) * 0.05);
                    match k_event.logical_key {
                        Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowDown) => {
                            let val = (self.value - delta).clamp(self.min, self.max);
                            let msg = self.on_change.as_ref().map(|f| f(val));
                            (EventResult::Handled, msg)
                        }
                        Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowUp) => {
                            let val = (self.value + delta).clamp(self.min, self.max);
                            let msg = self.on_change.as_ref().map(|f| f(val));
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
