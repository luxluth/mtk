use std::marker::PhantomData;
use winit::keyboard::{Key, NamedKey};

use crate::animation::Curve;
use crate::style::{
    AlignItems, FlexDirection, JustifyContent, Size, Style, TextStyle, VerticalAlignment,
};
use crate::text_property::{Alignment, FontWeight};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, clr, rgb};

/// An accessible checkbox toggle widget.
pub struct Checkbox<Msg, F = fn(bool) -> Msg> {
    pub(crate) checked: bool,
    pub(crate) label: Option<String>,
    pub(crate) on_toggle: Option<F>,
    pub(crate) disabled: bool,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Checkbox` widget with the given checked state.
///
/// # Examples
/// ```rust,ignore
/// checkbox(state.is_completed).on_toggle(|checked| AppMsg::ToggleDone(checked))
/// ```
pub fn checkbox<Msg>(checked: bool) -> Checkbox<Msg, fn(bool) -> Msg> {
    Checkbox {
        checked,
        label: None,
        on_toggle: None,
        disabled: false,
        _marker: PhantomData,
    }
}

impl<Msg, F> Checkbox<Msg, F> {
    /// Sets a text label next to the checkbox.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the callback invoked when the checkbox is toggled.
    pub fn on_toggle<NewF: Fn(bool) -> Msg>(self, on_toggle: NewF) -> Checkbox<Msg, NewF> {
        Checkbox {
            checked: self.checked,
            label: self.label,
            on_toggle: Some(on_toggle),
            disabled: self.disabled,
            _marker: PhantomData,
        }
    }

    /// Disables or enables the checkbox.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct CheckboxElement {
    container_node: Node,
    box_node: Node,
    label_node: Option<Node>,
    is_pressed: bool,
}

impl<State, Msg, F> View<State> for Checkbox<Msg, F>
where
    F: Fn(bool) -> Msg,
{
    type Element = CheckboxElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        let box_node = ctx.create_node();

        let box_bg = if self.disabled {
            rgb!(226, 232, 240)
        } else if self.checked {
            rgb!(59, 130, 246)
        } else {
            rgb!(255, 255, 255)
        };

        let box_border = if self.disabled {
            rgb!(203, 213, 225)
        } else if self.checked {
            rgb!(37, 99, 235)
        } else {
            rgb!(203, 213, 225)
        };

        Style::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(20))
            .corner_radius(4.0)
            .border(1.5, box_border)
            .bg_color(box_bg)
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center)
            .set_text_style(TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::BOLD,
                alignment: Alignment::Center,
                vertical_alignment: VerticalAlignment::Center,
                color: clr!(white),
                ..Default::default()
            })
            .transition_all(100.0, Curve::ease_out())
            .apply_to_node(ctx, box_node);

        box_node.set_text_with_userdata(
            ctx,
            if self.checked { "✓" } else { "" },
            TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::BOLD,
                alignment: Alignment::Center,
                vertical_alignment: VerticalAlignment::Center,
                color: clr!(white),
                wrap: false,
                ..Default::default()
            },
        );

        container_node.append(ctx, box_node);

        let label_node = if let Some(ref text_label) = self.label {
            let lbl = ctx.create_node();
            lbl.set_text_with_userdata(
                ctx,
                text_label,
                TextStyle {
                    font_size: 14.0,
                    vertical_alignment: VerticalAlignment::Center,
                    color: if self.disabled {
                        rgb!(148, 163, 184)
                    } else {
                        rgb!(15, 23, 42)
                    },
                    wrap: false,
                    ..Default::default()
                },
            );
            container_node.append(ctx, lbl);
            Some(lbl)
        } else {
            None
        };

        Style::new()
            .flex_direction(FlexDirection::Row)
            .gap(8.0)
            .align_items(AlignItems::Center)
            .apply_to_node(ctx, container_node);

        if !self.disabled {
            ctx.register_focusable(container_node);
        }

        CheckboxElement {
            container_node,
            box_node,
            label_node,
            is_pressed: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.checked != prev.checked || self.disabled != prev.disabled {
            let box_bg = if self.disabled {
                rgb!(226, 232, 240)
            } else if self.checked {
                rgb!(59, 130, 246)
            } else {
                rgb!(255, 255, 255)
            };

            let box_border = if self.disabled {
                rgb!(203, 213, 225)
            } else if self.checked {
                rgb!(37, 99, 235)
            } else {
                rgb!(203, 213, 225)
            };

            element.box_node.update_effects(ctx, |e| {
                e.background_color = box_bg;
                e.border.color = box_border;
            });

            element.box_node.set_text_with_userdata(
                ctx,
                if self.checked { "✓" } else { "" },
                TextStyle {
                    font_size: 13.0,
                    font_weight: FontWeight::BOLD,
                    alignment: Alignment::Center,
                    vertical_alignment: VerticalAlignment::Center,
                    color: clr!(white),
                    wrap: false,
                    ..Default::default()
                },
            );
        }

        if self.label != prev.label {
            if let (Some(lbl_node), Some(new_label)) = (element.label_node, &self.label) {
                lbl_node.set_text(ctx, new_label);
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_focusable(element.container_node);
        element.box_node.remove(ctx);
        ctx.destroy_node(element.box_node);
        if let Some(lbl) = element.label_node {
            lbl.remove(ctx);
            ctx.destroy_node(lbl);
        }
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
                    || hit_nodes.contains(&element.box_node);

                if is_hit && pressed {
                    element.is_pressed = true;
                    ctx.request_focus(element.container_node);
                    (EventResult::Handled, None)
                } else if !pressed && element.is_pressed {
                    element.is_pressed = false;
                    if is_hit {
                        let new_val = !self.checked;
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
                if Some(element.container_node) == ctx.focused_node() && k_event.state.is_pressed()
                {
                    match k_event.logical_key {
                        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => {
                            let new_val = !self.checked;
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
