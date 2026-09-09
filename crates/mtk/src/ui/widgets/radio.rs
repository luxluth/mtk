use std::marker::PhantomData;
use winit::keyboard::{Key, NamedKey};

use crate::animation::Curve;
use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::style::{AlignItems, FlexDirection, JustifyContent, Size, Style, TextStyle};
use crate::text_property::FontWeight;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{AccessibleInfo, Context, Node, clr, rgb};

/// Visual styling configuration for a [`Radio`] widget.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioStyle {
    pub selected_border: Color,
    pub unselected_border: Color,
    pub selected_dot: Color,
    pub circle_bg: Color,
    pub disabled_border: Color,
    pub disabled_dot: Color,
    pub label_style: Option<TextStyle>,
    pub gap: f32,
}

impl Default for RadioStyle {
    fn default() -> Self {
        Self {
            selected_border: rgb!(37, 99, 235),
            unselected_border: rgb!(148, 163, 184),
            selected_dot: rgb!(37, 99, 235),
            circle_bg: clr!(white),
            disabled_border: rgb!(203, 213, 225),
            disabled_dot: rgb!(203, 213, 225),
            label_style: None,
            gap: 8.0,
        }
    }
}

impl RadioStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_color(mut self, color: Color) -> Self {
        self.selected_border = color;
        self.selected_dot = color;
        self
    }

    pub fn selected_border(mut self, color: Color) -> Self {
        self.selected_border = color;
        self
    }

    pub fn unselected_border(mut self, color: Color) -> Self {
        self.unselected_border = color;
        self
    }

    pub fn selected_dot(mut self, color: Color) -> Self {
        self.selected_dot = color;
        self
    }

    pub fn dot_color(mut self, color: Color) -> Self {
        self.selected_dot = color;
        self
    }

    pub fn circle_bg(mut self, color: Color) -> Self {
        self.circle_bg = color;
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

/// An accessible circular radio button widget.
pub struct Radio<Msg, F = fn() -> Msg> {
    pub(crate) is_selected: bool,
    pub(crate) label: Option<String>,
    pub(crate) on_select: Option<F>,
    pub(crate) disabled: bool,
    pub(crate) style: RadioStyle,
    pub(crate) source_loc: Option<SourceLocation>,
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
#[track_caller]
pub fn radio<Msg>(is_selected: bool) -> Radio<Msg, fn() -> Msg> {
    Radio {
        is_selected,
        label: None,
        on_select: None,
        disabled: false,
        style: RadioStyle::default(),
        source_loc: Some(SourceLocation::here("Radio")),
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
            style: self.style,
            source_loc: self.source_loc,
            _marker: PhantomData,
        }
    }

    /// Disables or enables the radio button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the full visual style configuration for the radio button.
    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the active/selected color for the radio border and inner dot.
    pub fn active_color(mut self, color: Color) -> Self {
        self.style = self.style.active_color(color);
        self
    }

    /// Sets the inner dot color.
    pub fn dot_color(mut self, color: Color) -> Self {
        self.style = self.style.selected_dot(color);
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

    /// Sets the gap spacing between the radio circle and its label.
    pub fn gap(mut self, gap: f32) -> Self {
        self.style = self.style.gap(gap);
        self
    }
}

pub struct RadioElement {
    pub(crate) container_node: Node,
    pub(crate) outer_circle: Node,
    pub(crate) inner_dot: Node,
    pub(crate) label_node: Option<Node>,
    pub(crate) is_pressed: bool,
}

impl<State, Msg, F> View<State> for Radio<Msg, F>
where
    F: Fn() -> Msg,
{
    type Element = RadioElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }
        let outer_circle = ctx.create_node();
        let inner_dot = ctx.create_node();

        let border_color = if self.disabled {
            self.style.disabled_border
        } else if self.is_selected {
            self.style.selected_border
        } else {
            self.style.unselected_border
        };

        let dot_color = if self.disabled {
            self.style.disabled_dot
        } else {
            self.style.selected_dot
        };

        let circle_bg = if self.disabled {
            self.style.disabled_border
        } else {
            self.style.circle_bg
        };

        Style::new()
            .flex_direction(FlexDirection::Row)
            .align_items(AlignItems::Center)
            .gap(self.style.gap)
            .apply_to_node(ctx, container_node);

        Style::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(20))
            .corner_radius(10.0)
            .border(2.0, border_color)
            .bg_color(circle_bg)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .transition_all(120.0, Curve::ease_out())
            .apply_to_node(ctx, outer_circle);

        let dot_size = if self.is_selected { 10 } else { 0 };
        Style::new()
            .width(Size::Fixed(dot_size))
            .height(Size::Fixed(dot_size))
            .corner_radius(5.0)
            .bg_color(dot_color)
            .transition_all(120.0, Curve::ease_out())
            .apply_to_node(ctx, inner_dot);

        outer_circle.append(ctx, inner_dot);
        container_node.append(ctx, outer_circle);

        let label_node = if let Some(ref text_str) = self.label {
            let l_node = ctx.create_node();
            let mut text_style = self.style.label_style.clone().unwrap_or_else(|| TextStyle {
                font_size: 14.0,
                font_weight: FontWeight::MEDIUM,
                color: rgb!(15, 23, 42),
                wrap: false,
                ..Default::default()
            });
            if self.disabled {
                text_style.color = rgb!(148, 163, 184);
            }
            l_node.set_text_with_userdata(ctx, text_str, text_style);
            container_node.append(ctx, l_node);
            Some(l_node)
        } else {
            None
        };

        if !self.disabled {
            ctx.register_focusable(outer_circle);
        }

        let mut a11y_info = AccessibleInfo::new(accesskit::Role::RadioButton)
            .with_toggled(self.is_selected)
            .with_disabled(self.disabled)
            .with_action(accesskit::Action::Click)
            .with_action(accesskit::Action::Focus);
        if let Some(ref l) = self.label {
            a11y_info = a11y_info.with_label(l);
        }
        ctx.set_accessible(outer_circle, a11y_info);

        RadioElement {
            container_node,
            outer_circle,
            inner_dot,
            label_node,
            is_pressed: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.is_selected != prev.is_selected
            || self.disabled != prev.disabled
            || self.style != prev.style
        {
            let border_color = if self.disabled {
                self.style.disabled_border
            } else if self.is_selected {
                self.style.selected_border
            } else {
                self.style.unselected_border
            };

            let dot_color = if self.disabled {
                self.style.disabled_dot
            } else {
                self.style.selected_dot
            };

            let circle_bg = if self.disabled {
                self.style.disabled_border
            } else {
                self.style.circle_bg
            };

            element.outer_circle.update_effects(ctx, |e| {
                e.border.color = border_color;
                e.background_color = circle_bg;
            });

            let dot_size = if self.is_selected { 10 } else { 0 };
            element.inner_dot.update_constraints(ctx, |c| {
                c.width = Size::Fixed(dot_size);
                c.height = Size::Fixed(dot_size);
            });
            element.inner_dot.update_effects(ctx, |e| {
                e.background_color = dot_color;
            });
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
            if let (Some(l_node), Some(text_str)) = (element.label_node, &self.label) {
                let mut text_style = self.style.label_style.clone().unwrap_or_else(|| TextStyle {
                    font_size: 14.0,
                    font_weight: FontWeight::MEDIUM,
                    color: rgb!(15, 23, 42),
                    wrap: false,
                    ..Default::default()
                });
                if self.disabled {
                    text_style.color = rgb!(148, 163, 184);
                }
                l_node.set_text_with_userdata(ctx, text_str, text_style);
            }
        }

        if self.is_selected != prev.is_selected
            || self.disabled != prev.disabled
            || self.label != prev.label
        {
            let mut a11y_info = AccessibleInfo::new(accesskit::Role::RadioButton)
                .with_toggled(self.is_selected)
                .with_disabled(self.disabled)
                .with_action(accesskit::Action::Click)
                .with_action(accesskit::Action::Focus);
            if let Some(ref l) = self.label {
                a11y_info = a11y_info.with_label(l);
            }
            ctx.set_accessible(element.outer_circle, a11y_info);
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.remove_accessible(element.outer_circle);
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
                        Key::Named(NamedKey::Enter) => {
                            let msg = self.on_select.as_ref().map(|f| f());
                            (EventResult::Handled, msg)
                        }
                        Key::Character(ref s) if s == " " => {
                            let msg = self.on_select.as_ref().map(|f| f());
                            (EventResult::Handled, msg)
                        }
                        _ => (EventResult::Ignored, None),
                    }
                } else {
                    (EventResult::Ignored, None)
                }
            }
            Event::Action { node, action, .. } => {
                if node == element.outer_circle {
                    match action {
                        accesskit::Action::Click => {
                            let msg = self.on_select.as_ref().map(|f| f());
                            (EventResult::Handled, msg)
                        }
                        accesskit::Action::Focus => {
                            ctx.request_focus(element.outer_circle);
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

/// A group of selectable radio button options.
pub struct RadioGroup<Msg, F = fn(usize) -> Msg> {
    pub(crate) selected_index: usize,
    pub(crate) options: Vec<String>,
    pub(crate) on_change: Option<F>,
    pub(crate) style: RadioStyle,
    pub(crate) spacing: f32,
    pub(crate) direction: FlexDirection,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

pub struct RadioGroupElement {
    pub(crate) container_node: Node,
    pub(crate) elements: Vec<RadioElement>,
}

/// Creates a group of selectable radio buttons.
///
/// # Examples
/// ```rust,ignore
/// radio_group(state.selected_option, options, AppMsg::SelectOption)
///     .active_color(rgb!(16, 185, 129))
///     .horizontal()
/// ```
#[track_caller]
pub fn radio_group<Msg, F>(
    selected_index: usize,
    options: Vec<String>,
    on_change: F,
) -> RadioGroup<Msg, F>
where
    F: Fn(usize) -> Msg,
{
    RadioGroup {
        selected_index,
        options,
        on_change: Some(on_change),
        style: RadioStyle::default(),
        spacing: 8.0,
        direction: FlexDirection::Column,
        source_loc: Some(SourceLocation::here("RadioGroup")),
        _marker: PhantomData,
    }
}

impl<Msg, F> RadioGroup<Msg, F> {
    /// Sets the visual style configuration for each radio button in the group.
    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the active/selected color for the radio border and inner dot.
    pub fn active_color(mut self, color: Color) -> Self {
        self.style = self.style.active_color(color);
        self
    }

    /// Sets the inner dot color.
    pub fn dot_color(mut self, color: Color) -> Self {
        self.style = self.style.dot_color(color);
        self
    }

    /// Sets custom styling for the label text of each radio option.
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.style = self.style.label_style(style);
        self
    }

    /// Sets the color of the label text of each radio option.
    pub fn label_color(mut self, color: Color) -> Self {
        self.style = self.style.label_color(color);
        self
    }

    /// Sets the gap between each radio circle and its label text.
    pub fn item_gap(mut self, gap: f32) -> Self {
        self.style = self.style.gap(gap);
        self
    }

    /// Sets the spacing between consecutive radio options in the group.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets the spacing between consecutive radio options in the group.
    pub fn gap(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets the layout direction of the radio group.
    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Lays out the radio options horizontally in a row.
    pub fn horizontal(mut self) -> Self {
        self.direction = FlexDirection::Row;
        self
    }

    /// Lays out the radio options vertically in a column.
    pub fn vertical(mut self) -> Self {
        self.direction = FlexDirection::Column;
        self
    }

    fn create_radio(&self, index: usize, label: &str) -> Radio<Msg, fn() -> Msg> {
        radio(index == self.selected_index)
            .label(label)
            .style(self.style.clone())
    }
}

impl<State, Msg, F> View<State> for RadioGroup<Msg, F>
where
    F: Fn(usize) -> Msg,
{
    type Element = RadioGroupElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }
        Style::new()
            .flex_direction(self.direction)
            .gap(self.spacing)
            .apply_to_node(ctx, container_node);

        let mut elements = Vec::with_capacity(self.options.len());
        for (i, label) in self.options.iter().enumerate() {
            let r = self.create_radio(i, label);
            let el = View::<State>::build(&r, ctx);
            container_node.append(ctx, el.container_node);
            elements.push(el);
        }

        RadioGroupElement {
            container_node,
            elements,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.spacing != prev.spacing {
            element.container_node.update_constraints(ctx, |c| {
                c.gap = self.spacing;
            });
        }
        if self.direction != prev.direction {
            element.container_node.update_constraints(ctx, |c| {
                c.flex_direction = self.direction;
            });
        }

        let prev_len = element.elements.len();
        let curr_len = self.options.len();

        if curr_len < prev_len {
            for i in curr_len..prev_len {
                if let Some(mut el) = element.elements.pop() {
                    let r =
                        prev.create_radio(i, prev.options.get(i).map(|s| s.as_str()).unwrap_or(""));
                    View::<State>::teardown(&r, ctx, &mut el);
                }
            }
        }

        for (i, label) in self.options.iter().enumerate() {
            let curr_radio = self.create_radio(i, label);
            if i < element.elements.len() {
                let prev_radio =
                    prev.create_radio(i, prev.options.get(i).map(|s| s.as_str()).unwrap_or(label));
                View::<State>::rebuild(&curr_radio, &prev_radio, ctx, &mut element.elements[i]);
            } else {
                let el = View::<State>::build(&curr_radio, ctx);
                element.container_node.append(ctx, el.container_node);
                element.elements.push(el);
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        for (i, label) in self.options.iter().enumerate() {
            let r = self.create_radio(i, label);
            if let Some(el) = element.elements.get_mut(i) {
                View::<State>::teardown(&r, ctx, el);
            }
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
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        for (i, label) in self.options.iter().enumerate() {
            if let Some(el) = element.elements.get_mut(i) {
                let r = self.create_radio(i, label);
                if let Some(ref on_change) = self.on_change {
                    let r_with_cb = r.on_select(move || on_change(i));
                    let (res, msg) =
                        View::<State>::handle_event(&r_with_cb, el, state, event.clone(), ctx);
                    if res == EventResult::Handled {
                        return (res, msg);
                    }
                } else {
                    let (res, msg) = View::<State>::handle_event(&r, el, state, event.clone(), ctx);
                    if res == EventResult::Handled {
                        return (res, msg);
                    }
                }
            }
        }
        (EventResult::Ignored, None)
    }
}
