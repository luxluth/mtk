use std::marker::PhantomData;
use winit::keyboard::{Key, NamedKey};

use crate::animation::Curve;
use crate::style::{AlignItems, FlexDirection, JustifyContent, Size, Style, TextStyle};
use crate::text_property::FontWeight;
use crate::ui::event::EventResult;
use crate::ui::style::ViewStyleExt;
use crate::ui::widgets::column;
use crate::ui::{Event, View};
use crate::{Context, Node, clr, rgb};

/// An accessible circular radio button widget.
pub struct Radio<Msg, F = fn() -> Msg> {
    pub(crate) is_selected: bool,
    pub(crate) label: Option<String>,
    pub(crate) on_select: Option<F>,
    pub(crate) disabled: bool,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Radio` button widget.
///
/// # Examples
/// ```rust,ignore
/// radio(state.frequency == Frequency::Daily)
///     .label("Daily Summary")
///     .on_select(AppMsg::SetDaily)
/// ```
pub fn radio<Msg>(is_selected: bool) -> Radio<Msg, fn() -> Msg> {
    Radio {
        is_selected,
        label: None,
        on_select: None,
        disabled: false,
        _marker: PhantomData,
    }
}

impl<Msg, F> Radio<Msg, F> {
    /// Attaches an adjacent text label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the callback invoked when this radio button is selected.
    pub fn on_select<NewF: Fn() -> Msg>(self, on_select: NewF) -> Radio<Msg, NewF> {
        Radio {
            is_selected: self.is_selected,
            label: self.label,
            on_select: Some(on_select),
            disabled: self.disabled,
            _marker: PhantomData,
        }
    }

    /// Disables or enables the radio button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct RadioElement {
    container_node: Node,
    outer_circle: Node,
    inner_dot: Node,
    label_node: Option<Node>,
    is_pressed: bool,
}

impl<State, Msg, F> View<State> for Radio<Msg, F>
where
    F: Fn() -> Msg,
{
    type Element = RadioElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        let outer_circle = ctx.create_node();
        let inner_dot = ctx.create_node();

        Style::new()
            .flex_direction(FlexDirection::Row)
            .align_items(AlignItems::Center)
            .gap(8.0)
            .apply_to_node(ctx, container_node);

        let border_color = if self.disabled {
            rgb!(203, 213, 225)
        } else if self.is_selected {
            rgb!(37, 99, 235)
        } else {
            rgb!(148, 163, 184)
        };

        Style::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(20))
            .corner_radius(10.0)
            .border(2.0, border_color)
            .bg_color(clr!(white))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .transition_all(120.0, Curve::ease_out())
            .apply_to_node(ctx, outer_circle);

        let dot_size = if self.is_selected { 10 } else { 0 };
        Style::new()
            .width(Size::Fixed(dot_size))
            .height(Size::Fixed(dot_size))
            .corner_radius(5.0)
            .bg_color(if self.disabled {
                rgb!(203, 213, 225)
            } else {
                rgb!(37, 99, 235)
            })
            .transition_all(120.0, Curve::ease_out())
            .apply_to_node(ctx, inner_dot);

        outer_circle.append(ctx, inner_dot);
        container_node.append(ctx, outer_circle);

        let label_node = if let Some(ref text_str) = self.label {
            let l_node = ctx.create_node();
            l_node.set_text_with_userdata(
                ctx,
                text_str,
                TextStyle {
                    font_size: 14.0,
                    font_weight: FontWeight::MEDIUM,
                    color: if self.disabled {
                        rgb!(148, 163, 184)
                    } else {
                        rgb!(15, 23, 42)
                    },
                    wrap: false,
                    ..Default::default()
                },
            );
            container_node.append(ctx, l_node);
            Some(l_node)
        } else {
            None
        };

        if !self.disabled {
            ctx.register_focusable(outer_circle);
        }

        RadioElement {
            container_node,
            outer_circle,
            inner_dot,
            label_node,
            is_pressed: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.is_selected != prev.is_selected || self.disabled != prev.disabled {
            let border_color = if self.disabled {
                rgb!(203, 213, 225)
            } else if self.is_selected {
                rgb!(37, 99, 235)
            } else {
                rgb!(148, 163, 184)
            };

            element.outer_circle.update_effects(ctx, |e| {
                e.border.color = border_color;
            });

            let dot_size = if self.is_selected { 10 } else { 0 };
            element.inner_dot.update_constraints(ctx, |c| {
                c.width = Size::Fixed(dot_size);
                c.height = Size::Fixed(dot_size);
            });
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_focusable(element.outer_circle);
        if let Some(l_node) = element.label_node {
            l_node.remove(ctx);
            ctx.destroy_node(l_node);
        }
        element.inner_dot.remove(ctx);
        ctx.destroy_node(element.inner_dot);
        element.outer_circle.remove(ctx);
        ctx.destroy_node(element.outer_circle);
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
                pressed, hit_nodes, ..
            } => {
                let is_hit = hit_nodes.contains(&element.container_node)
                    || hit_nodes.contains(&element.outer_circle)
                    || hit_nodes.contains(&element.inner_dot)
                    || element
                        .label_node
                        .map(|l| hit_nodes.contains(&l))
                        .unwrap_or(false);

                if is_hit && pressed {
                    element.is_pressed = true;
                    ctx.request_focus(element.outer_circle);
                    (EventResult::Handled, None)
                } else if !pressed && element.is_pressed {
                    element.is_pressed = false;
                    if is_hit {
                        let msg = self.on_select.as_ref().map(|f| f());
                        (EventResult::Handled, msg)
                    } else {
                        (EventResult::Handled, None)
                    }
                } else {
                    (EventResult::Ignored, None)
                }
            }
            Event::KeyboardInput { event: k_event, .. } => {
                if Some(element.outer_circle) == ctx.focused_node() && k_event.state.is_pressed() {
                    match k_event.logical_key {
                        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => {
                            let msg = self.on_select.as_ref().map(|f| f());
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

/// Creates a group of selectable radio buttons.
pub fn radio_group<State: 'static, Msg: 'static>(
    selected_index: usize,
    options: Vec<String>,
    on_change: impl Fn(usize) -> Msg + Clone + 'static,
) -> impl View<State, Message = Msg> {
    let views: Vec<_> = options
        .into_iter()
        .enumerate()
        .map(move |(i, label)| {
            let on_change = on_change.clone();
            radio(i == selected_index)
                .label(label)
                .on_select(move || on_change(i))
        })
        .collect();

    column(views).style(Style::new().gap(8.0))
}
