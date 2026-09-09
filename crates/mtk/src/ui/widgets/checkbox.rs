use std::marker::PhantomData;
use winit::keyboard::{Key, NamedKey};

use crate::animation::Curve;
use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::style::{
    AlignItems, FlexDirection, JustifyContent, Size, Style, TextStyle, VerticalAlignment,
};
use crate::text_property::{Alignment, FontWeight};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{AccessibleInfo, Context, Node, clr, rgb};

/// Visual styling configuration for a [`Checkbox`] widget.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckboxStyle {
    pub checked_bg: Color,
    pub unchecked_bg: Color,
    pub checked_border: Color,
    pub unchecked_border: Color,
    pub disabled_bg: Color,
    pub disabled_border: Color,
    pub check_color: Color,
    pub label_style: Option<TextStyle>,
    pub gap: f32,
}

impl Default for CheckboxStyle {
    fn default() -> Self {
        Self {
            checked_bg: rgb!(59, 130, 246),
            unchecked_bg: rgb!(255, 255, 255),
            checked_border: rgb!(37, 99, 235),
            unchecked_border: rgb!(203, 213, 225),
            disabled_bg: rgb!(226, 232, 240),
            disabled_border: rgb!(203, 213, 225),
            check_color: clr!(white),
            label_style: None,
            gap: 8.0,
        }
    }
}

impl CheckboxStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_color(mut self, color: Color) -> Self {
        self.checked_bg = color;
        self.checked_border = color;
        self
    }

    pub fn checked_bg(mut self, color: Color) -> Self {
        self.checked_bg = color;
        self
    }

    pub fn unchecked_bg(mut self, color: Color) -> Self {
        self.unchecked_bg = color;
        self
    }

    pub fn checked_border(mut self, color: Color) -> Self {
        self.checked_border = color;
        self
    }

    pub fn unchecked_border(mut self, color: Color) -> Self {
        self.unchecked_border = color;
        self
    }

    pub fn check_color(mut self, color: Color) -> Self {
        self.check_color = color;
        self
    }

    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.label_style = Some(style);
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        let mut s = self.label_style.unwrap_or_default();
        s.color = color;
        self.label_style = Some(s);
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

/// An accessible checkbox toggle widget.
pub struct Checkbox<Msg, F = fn(bool) -> Msg> {
    pub(crate) checked: bool,
    pub(crate) label: Option<String>,
    pub(crate) on_toggle: Option<F>,
    pub(crate) disabled: bool,
    pub(crate) style: CheckboxStyle,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Checkbox` widget with the given checked state.
///
/// # Examples
/// ```rust,ignore
/// checkbox(state.is_completed).on_toggle(|checked| AppMsg::ToggleDone(checked))
/// ```
#[track_caller]
pub fn checkbox<Msg>(checked: bool) -> Checkbox<Msg, fn(bool) -> Msg> {
    Checkbox {
        checked,
        label: None,
        on_toggle: None,
        disabled: false,
        style: CheckboxStyle::default(),
        source_loc: Some(SourceLocation::here("Checkbox")),
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
            style: self.style,
            source_loc: self.source_loc,
            _marker: PhantomData,
        }
    }

    /// Disables or enables the checkbox.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the full visual style configuration for the checkbox.
    pub fn style(mut self, style: CheckboxStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the active/checked color for the checkbox background and border.
    pub fn active_color(mut self, color: Color) -> Self {
        self.style = self.style.active_color(color);
        self
    }

    /// Sets the checkmark glyph color.
    pub fn check_color(mut self, color: Color) -> Self {
        self.style = self.style.check_color(color);
        self
    }

    /// Sets custom styling for the label text.
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.style = self.style.label_style(style);
        self
    }

    /// Sets the color of the label text.
    pub fn label_color(mut self, color: Color) -> Self {
        self.style = self.style.label_color(color);
        self
    }

    /// Sets the gap spacing between the checkbox and its label.
    pub fn gap(mut self, gap: f32) -> Self {
        self.style = self.style.gap(gap);
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
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }
        let box_node = ctx.create_node();

        let box_bg = if self.disabled {
            self.style.disabled_bg
        } else if self.checked {
            self.style.checked_bg
        } else {
            self.style.unchecked_bg
        };

        let box_border = if self.disabled {
            self.style.disabled_border
        } else if self.checked {
            self.style.checked_border
        } else {
            self.style.unchecked_border
        };

        let check_color = if self.disabled {
            self.style.disabled_border
        } else {
            self.style.check_color
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
                color: check_color,
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
                color: check_color,
                wrap: false,
                ..Default::default()
            },
        );

        container_node.append(ctx, box_node);

        let label_node = if let Some(ref text_label) = self.label {
            let lbl = ctx.create_node();
            let mut text_style = self.style.label_style.clone().unwrap_or_else(|| TextStyle {
                font_size: 14.0,
                vertical_alignment: VerticalAlignment::Center,
                color: rgb!(15, 23, 42),
                wrap: false,
                ..Default::default()
            });
            if self.disabled {
                text_style.color = rgb!(148, 163, 184);
            }
            lbl.set_text_with_userdata(ctx, text_label, text_style);
            container_node.append(ctx, lbl);
            Some(lbl)
        } else {
            None
        };

        Style::new()
            .flex_direction(FlexDirection::Row)
            .gap(self.style.gap)
            .align_items(AlignItems::Center)
            .apply_to_node(ctx, container_node);

        if !self.disabled {
            ctx.register_focusable(container_node);
        }

        let mut a11y_info = AccessibleInfo::new(accesskit::Role::CheckBox)
            .with_toggled(self.checked)
            .with_disabled(self.disabled)
            .with_action(accesskit::Action::Click)
            .with_action(accesskit::Action::Focus);
        if let Some(ref l) = self.label {
            a11y_info = a11y_info.with_label(l);
        }
        ctx.set_accessible(container_node, a11y_info);

        CheckboxElement {
            container_node,
            box_node,
            label_node,
            is_pressed: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.checked != prev.checked
            || self.disabled != prev.disabled
            || self.style != prev.style
        {
            let box_bg = if self.disabled {
                self.style.disabled_bg
            } else if self.checked {
                self.style.checked_bg
            } else {
                self.style.unchecked_bg
            };

            let box_border = if self.disabled {
                self.style.disabled_border
            } else if self.checked {
                self.style.checked_border
            } else {
                self.style.unchecked_border
            };

            let check_color = if self.disabled {
                self.style.disabled_border
            } else {
                self.style.check_color
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
                    color: check_color,
                    wrap: false,
                    ..Default::default()
                },
            );
        }

        if self.style.gap != prev.style.gap {
            element.container_node.update_constraints(ctx, |c| {
                c.gap = self.style.gap;
            });
        }

        if self.label != prev.label
            || self.style.label_style != prev.style.label_style
            || self.disabled != prev.disabled
        {
            if let (Some(lbl_node), Some(new_label)) = (element.label_node, &self.label) {
                let mut text_style = self.style.label_style.clone().unwrap_or_else(|| TextStyle {
                    font_size: 14.0,
                    vertical_alignment: VerticalAlignment::Center,
                    color: rgb!(15, 23, 42),
                    wrap: false,
                    ..Default::default()
                });
                if self.disabled {
                    text_style.color = rgb!(148, 163, 184);
                }
                lbl_node.set_text_with_userdata(ctx, new_label, text_style);
            }
        }

        if self.checked != prev.checked
            || self.disabled != prev.disabled
            || self.label != prev.label
        {
            let mut a11y_info = AccessibleInfo::new(accesskit::Role::CheckBox)
                .with_toggled(self.checked)
                .with_disabled(self.disabled)
                .with_action(accesskit::Action::Click)
                .with_action(accesskit::Action::Focus);
            if let Some(ref l) = self.label {
                a11y_info = a11y_info.with_label(l);
            }
            ctx.set_accessible(element.container_node, a11y_info);
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.remove_accessible(element.container_node);
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
                        Key::Named(NamedKey::Enter) => {
                            let new_val = !self.checked;
                            let msg = self.on_toggle.as_ref().map(|f| f(new_val));
                            (EventResult::Handled, msg)
                        }
                        Key::Character(ref s) if s == " " => {
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
            Event::Action { node, action, .. } => {
                if node == element.container_node {
                    match action {
                        accesskit::Action::Click => {
                            let new_val = !self.checked;
                            let msg = self.on_toggle.as_ref().map(|f| f(new_val));
                            (EventResult::Handled, msg)
                        }
                        accesskit::Action::Focus => {
                            ctx.request_focus(element.container_node);
                            (EventResult::Handled, None)
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
