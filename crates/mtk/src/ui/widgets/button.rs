use std::marker::PhantomData;
use winit::keyboard::{Key, NamedKey};

use crate::animation::Curve;
use crate::style::{Style, TextStyle, VerticalAlignment};
use crate::text_property::{Alignment, FontWeight};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, rgb};

/// A clickable button widget with built-in hover, focus, and press feedback.
pub struct Button<Msg> {
    pub(crate) label: String,
    pub(crate) on_click: Option<Msg>,
    pub(crate) disabled: bool,
    pub(crate) custom_style: Option<Style>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Button` widget with the provided text label.
///
/// # Examples
/// ```rust,ignore
/// button("Submit").on_click(AppMsg::SubmitForm)
/// ```
pub fn button<S: ToString, Msg>(label: S) -> Button<Msg> {
    Button {
        label: label.to_string(),
        on_click: None,
        disabled: false,
        custom_style: None,
        _marker: PhantomData,
    }
}

impl<Msg: Clone> Button<Msg> {
    /// Sets the message to emit when the button is clicked.
    pub fn on_click(mut self, msg: Msg) -> Self {
        self.on_click = Some(msg);
        self
    }

    /// Disables or enables the button. Disabled buttons ignore click events and render with reduced opacity.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Applies custom styling to the button container.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }
}

pub struct ButtonElement {
    node: Node,
    is_pressed: bool,
    is_hovered: bool,
}

impl<State, Msg: Clone> View<State> for Button<Msg> {
    type Element = ButtonElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();

        let default_text_style = TextStyle {
            font_size: 14.0,
            font_weight: FontWeight::SEMI_BOLD,
            alignment: Alignment::Center,
            vertical_alignment: VerticalAlignment::Center,
            color: if self.disabled {
                rgb!(148, 163, 184)
            } else {
                rgb!(255, 255, 255)
            },
            ..Default::default()
        };

        let default_style = Style::new()
            .padding_xy(16.0, 8.0)
            .corner_radius(6.0)
            .bg_color(if self.disabled {
                rgb!(226, 232, 240)
            } else {
                rgb!(59, 130, 246)
            })
            .border(
                1.0,
                if self.disabled {
                    rgb!(203, 213, 225)
                } else {
                    rgb!(37, 99, 235)
                },
            )
            .set_text_style(default_text_style)
            .transition_all(100.0, Curve::ease_out());

        let final_style = if let Some(custom) = &self.custom_style {
            default_style.merge(custom.clone())
        } else {
            default_style
        };

        final_style.apply_to_node(ctx, node);
        node.set_text_with_userdata(ctx, &self.label, final_style.base_text_style.clone());

        if !self.disabled {
            ctx.register_focusable(node);
        }

        ButtonElement {
            node,
            is_pressed: false,
            is_hovered: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.disabled != prev.disabled || self.custom_style != prev.custom_style {
            let default_text_style = TextStyle {
                font_size: 14.0,
                font_weight: FontWeight::SEMI_BOLD,
                alignment: Alignment::Center,
                vertical_alignment: VerticalAlignment::Center,
                color: if self.disabled {
                    rgb!(148, 163, 184)
                } else {
                    rgb!(255, 255, 255)
                },
                ..Default::default()
            };

            let default_style = Style::new()
                .padding_xy(16.0, 8.0)
                .corner_radius(6.0)
                .bg_color(if self.disabled {
                    rgb!(226, 232, 240)
                } else {
                    rgb!(59, 130, 246)
                })
                .border(
                    1.0,
                    if self.disabled {
                        rgb!(203, 213, 225)
                    } else {
                        rgb!(37, 99, 235)
                    },
                )
                .set_text_style(default_text_style)
                .transition_all(100.0, Curve::ease_out());

            let final_style = if let Some(custom) = &self.custom_style {
                default_style.merge(custom.clone())
            } else {
                default_style
            };

            final_style.apply_to_node(ctx, element.node);
            element.node.set_text_with_userdata(
                ctx,
                &self.label,
                final_style.base_text_style.clone(),
            );
        } else if self.label != prev.label {
            if let Some(info) = element.node.get_text_userdata::<TextStyle>(ctx).cloned() {
                element.node.set_text_with_userdata(ctx, &self.label, info);
            } else {
                element.node.set_text(ctx, &self.label);
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_focusable(element.node);
        element.node.remove(ctx);
        ctx.destroy_node(element.node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.node
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
            Event::CursorMoved { hit_nodes, .. } => {
                let is_hit = hit_nodes.contains(&element.node);
                if is_hit != element.is_hovered {
                    element.is_hovered = is_hit;
                    let bg = if element.is_hovered {
                        rgb!(37, 99, 235)
                    } else {
                        rgb!(59, 130, 246)
                    };
                    element.node.update_effects(ctx, |e| {
                        if self.custom_style.is_none() {
                            e.background_color = bg;
                        }
                    });
                    ctx.request_frame();
                }
                (EventResult::Ignored, None)
            }
            Event::MouseInput {
                pressed, hit_nodes, ..
            } => {
                let is_hit = hit_nodes.contains(&element.node);
                if is_hit && pressed {
                    element.is_pressed = true;
                    ctx.request_focus(element.node);
                    element.node.update_effects(ctx, |e| {
                        e.scale = 0.96;
                    });
                    ctx.request_frame();
                    (EventResult::Handled, None)
                } else if !pressed && element.is_pressed {
                    element.is_pressed = false;
                    element.node.update_effects(ctx, |e| {
                        e.scale = 1.0;
                    });
                    ctx.request_frame();
                    if is_hit {
                        (EventResult::Handled, self.on_click.clone())
                    } else {
                        (EventResult::Handled, None)
                    }
                } else {
                    (EventResult::Ignored, None)
                }
            }
            Event::KeyboardInput { event: k_event, .. } => {
                if Some(element.node) == ctx.focused_node() && k_event.state.is_pressed() {
                    match k_event.logical_key {
                        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => {
                            (EventResult::Handled, self.on_click.clone())
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
