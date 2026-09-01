use std::marker::PhantomData;
use winit::keyboard::{Key, NamedKey};

use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::style::{
    AlignItems, FlexDirection, JustifyContent, PositionStrategy, Size, Style, TextStyle,
};
use crate::text_property::FontWeight;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, clr, rgb, rgba};

/// A custom dropdown select menu widget with full keyboard navigation and focus support.
pub struct Dropdown<Msg, F = fn(usize) -> Msg> {
    pub(crate) selected_index: Option<usize>,
    pub(crate) options: Vec<String>,
    pub(crate) placeholder: String,
    pub(crate) on_select: Option<F>,
    pub(crate) disabled: bool,
    pub(crate) bg_color: Color,
    pub(crate) text_color: Color,
    pub(crate) border_color: Color,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Dropdown` select widget.
///
/// # Examples
/// ```rust,ignore
/// dropdown(Some(state.theme_index), vec!["Light".into(), "Dark".into(), "System".into()])
///     .on_select(|idx| AppMsg::SetTheme(idx))
/// ```
#[track_caller]
pub fn dropdown<Msg>(
    selected_index: Option<usize>,
    options: Vec<String>,
) -> Dropdown<Msg, fn(usize) -> Msg> {
    Dropdown {
        selected_index,
        options,
        placeholder: "Select option...".to_string(),
        on_select: None,
        disabled: false,
        bg_color: clr!(white),
        text_color: rgb!(15, 23, 42),
        border_color: rgb!(203, 213, 225),
        source_loc: Some(SourceLocation::here("Dropdown")),
        _marker: PhantomData,
    }
}

impl<Msg, F> Dropdown<Msg, F> {
    /// Sets the placeholder text shown when no option is selected.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Sets the callback triggered when an option is chosen.
    pub fn on_select<NewF: Fn(usize) -> Msg>(self, on_select: NewF) -> Dropdown<Msg, NewF> {
        Dropdown {
            selected_index: self.selected_index,
            options: self.options,
            placeholder: self.placeholder,
            on_select: Some(on_select),
            disabled: self.disabled,
            bg_color: self.bg_color,
            text_color: self.text_color,
            border_color: self.border_color,
            source_loc: self.source_loc,
            _marker: PhantomData,
        }
    }

    /// Sets custom background color.
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// Sets custom text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Sets custom border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Disables or enables the dropdown.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct DropdownElement {
    pub(crate) container_node: Node,
    pub(crate) trigger_node: Node,
    pub(crate) label_node: Node,
    pub(crate) icon_node: Node,
    pub(crate) menu_node: Node,
    pub(crate) item_nodes: Vec<Node>,
    pub(crate) is_open: bool,
}

impl<State, Msg, F> View<State> for Dropdown<Msg, F>
where
    F: Fn(usize) -> Msg,
{
    type Element = DropdownElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }
        let trigger_node = ctx.create_node();
        let label_node = ctx.create_node();
        let icon_node = ctx.create_node();
        let menu_node = ctx.create_node();

        Style::new()
            .width(Size::Percent(1.0))
            .min_width(220.0)
            .apply_to_node(ctx, container_node);

        let selected_text = self
            .selected_index
            .and_then(|i| self.options.get(i))
            .map(|s| s.as_str())
            .unwrap_or(&self.placeholder);

        label_node.set_text_with_userdata(
            ctx,
            selected_text,
            TextStyle {
                font_size: 14.0,
                font_weight: FontWeight::MEDIUM,
                color: self.text_color,
                wrap: false,
                ..Default::default()
            },
        );

        icon_node.set_text_with_userdata(
            ctx,
            "▾",
            TextStyle {
                font_size: 14.0,
                color: rgb!(100, 116, 139),
                wrap: false,
                ..Default::default()
            },
        );

        Style::new()
            .flex_direction(FlexDirection::Row)
            .width(Size::Percent(1.0))
            .height(Size::Fixed(38))
            .padding_xy(12.0, 8.0)
            .justify_content(JustifyContent::SpaceBetween)
            .align_items(AlignItems::Center)
            .bg_color(self.bg_color)
            .border(1.0, self.border_color)
            .corner_radius(6.0)
            .apply_to_node(ctx, trigger_node);

        trigger_node.append(ctx, label_node);
        trigger_node.append(ctx, icon_node);
        container_node.append(ctx, trigger_node);

        if !self.disabled {
            ctx.register_focusable(trigger_node);
        }

        // Build floating menu items
        Style::new()
            .position(PositionStrategy::Absolute {
                top: 42.0,
                left: 0.0,
                bottom: f32::NAN,
                right: 0.0,
            })
            .width(Size::Percent(1.0))
            .padding(4.0)
            .gap(2.0)
            .bg_color(self.bg_color)
            .border(1.0, self.border_color)
            .corner_radius(8.0)
            .shadow(rgba!(0, 0, 0, 20), 12.0, 0.4)
            .z_index(2000)
            .apply_to_node(ctx, menu_node);

        let is_dark = self.bg_color.r < 100 && self.bg_color.g < 100 && self.bg_color.b < 100;
        let sel_bg = if is_dark {
            rgba!(59, 130, 246, 60)
        } else {
            rgb!(239, 246, 255)
        };
        let sel_color = if is_dark {
            rgb!(96, 165, 250)
        } else {
            rgb!(37, 99, 235)
        };

        let mut item_nodes = Vec::new();
        for (i, opt) in self.options.iter().enumerate() {
            let item_node = ctx.create_node();
            let is_sel = Some(i) == self.selected_index;

            item_node.set_text_with_userdata(
                ctx,
                opt,
                TextStyle {
                    font_size: 13.0,
                    font_weight: if is_sel {
                        FontWeight::SEMI_BOLD
                    } else {
                        FontWeight::NORMAL
                    },
                    color: if is_sel { sel_color } else { self.text_color },
                    wrap: false,
                    ..Default::default()
                },
            );

            Style::new()
                .width(Size::Percent(1.0))
                .padding_xy(10.0, 7.0)
                .corner_radius(4.0)
                .bg_color(if is_sel { sel_bg } else { clr!(transparent) })
                .apply_to_node(ctx, item_node);

            menu_node.append(ctx, item_node);
            item_nodes.push(item_node);
        }

        DropdownElement {
            container_node,
            trigger_node,
            label_node,
            icon_node,
            menu_node,
            item_nodes,
            is_open: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.bg_color != prev.bg_color || self.border_color != prev.border_color {
            element.trigger_node.update_effects(ctx, |e| {
                e.background_color = self.bg_color;
                e.border.color = self.border_color;
            });
            element.menu_node.update_effects(ctx, |e| {
                e.background_color = self.bg_color;
                e.border.color = self.border_color;
            });
        }

        if self.selected_index != prev.selected_index
            || self.options != prev.options
            || self.text_color != prev.text_color
        {
            let selected_text = self
                .selected_index
                .and_then(|i| self.options.get(i))
                .map(|s| s.as_str())
                .unwrap_or(&self.placeholder);

            element.label_node.set_text_with_userdata(
                ctx,
                selected_text,
                TextStyle {
                    font_size: 14.0,
                    font_weight: FontWeight::MEDIUM,
                    color: self.text_color,
                    wrap: false,
                    ..Default::default()
                },
            );

            let is_dark = self.bg_color.r < 100 && self.bg_color.g < 100 && self.bg_color.b < 100;
            let sel_bg = if is_dark {
                rgba!(59, 130, 246, 60)
            } else {
                rgb!(239, 246, 255)
            };
            let sel_color = if is_dark {
                rgb!(96, 165, 250)
            } else {
                rgb!(37, 99, 235)
            };

            for (i, node) in element.item_nodes.iter().enumerate() {
                let is_sel = Some(i) == self.selected_index;
                let opt_text = self.options.get(i).map(|s| s.as_str()).unwrap_or("");

                node.set_text_with_userdata(
                    ctx,
                    opt_text,
                    TextStyle {
                        font_size: 13.0,
                        font_weight: if is_sel {
                            FontWeight::SEMI_BOLD
                        } else {
                            FontWeight::NORMAL
                        },
                        color: if is_sel { sel_color } else { self.text_color },
                        wrap: false,
                        ..Default::default()
                    },
                );

                node.update_effects(ctx, |e| {
                    e.background_color = if is_sel { sel_bg } else { clr!(transparent) };
                });
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_focusable(element.trigger_node);
        if element.is_open {
            element.menu_node.remove(ctx);
        }
        for item in &mut element.item_nodes {
            item.remove(ctx);
            ctx.destroy_node(*item);
        }
        element.menu_node.remove(ctx);
        ctx.destroy_node(element.menu_node);
        element.icon_node.remove(ctx);
        ctx.destroy_node(element.icon_node);
        element.label_node.remove(ctx);
        ctx.destroy_node(element.label_node);
        element.trigger_node.remove(ctx);
        ctx.destroy_node(element.trigger_node);
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
            Event::CursorMoved { ref hit_nodes, .. } => {
                if element.is_open {
                    let is_dark =
                        self.bg_color.r < 100 && self.bg_color.g < 100 && self.bg_color.b < 100;
                    let sel_bg = if is_dark {
                        rgba!(59, 130, 246, 60)
                    } else {
                        rgb!(239, 246, 255)
                    };
                    let hov_bg = if is_dark {
                        rgba!(255, 255, 255, 25)
                    } else {
                        rgb!(241, 245, 249)
                    };

                    for (i, node) in element.item_nodes.iter().enumerate() {
                        let is_sel = Some(i) == self.selected_index;
                        let is_hov = hit_nodes.contains(node);
                        let bg = if is_sel {
                            sel_bg
                        } else if is_hov {
                            hov_bg
                        } else {
                            clr!(transparent)
                        };
                        node.update_effects(ctx, |e| e.background_color = bg);
                    }
                }
                (EventResult::Ignored, None)
            }
            Event::MouseInput {
                pressed: true,
                hit_nodes,
                ..
            } => {
                let hit_trigger = hit_nodes.contains(&element.trigger_node)
                    || hit_nodes.contains(&element.label_node)
                    || hit_nodes.contains(&element.icon_node);

                if hit_trigger {
                    element.is_open = !element.is_open;
                    ctx.request_focus(element.trigger_node);
                    if element.is_open {
                        element.container_node.append(ctx, element.menu_node);
                        element.icon_node.set_text(ctx, "▴");
                    } else {
                        element.menu_node.remove(ctx);
                        element.icon_node.set_text(ctx, "▾");
                    }
                    return (EventResult::Handled, None);
                }

                if element.is_open {
                    for (i, item_node) in element.item_nodes.iter().enumerate() {
                        if hit_nodes.contains(item_node) {
                            element.is_open = false;
                            element.menu_node.remove(ctx);
                            element.icon_node.set_text(ctx, "▾");
                            let msg = self.on_select.as_ref().map(|f| f(i));
                            return (EventResult::Handled, msg);
                        }
                    }

                    // Click outside closes menu
                    element.is_open = false;
                    element.menu_node.remove(ctx);
                    element.icon_node.set_text(ctx, "▾");
                    return (EventResult::Handled, None);
                }

                (EventResult::Ignored, None)
            }
            Event::KeyboardInput {
                event: ref k_event, ..
            } => {
                if Some(element.trigger_node) == ctx.focused_node() && k_event.state.is_pressed() {
                    match k_event.logical_key {
                        Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter) => {
                            element.is_open = !element.is_open;
                            if element.is_open {
                                element.container_node.append(ctx, element.menu_node);
                                element.icon_node.set_text(ctx, "▴");
                            } else {
                                element.menu_node.remove(ctx);
                                element.icon_node.set_text(ctx, "▾");
                            }
                            return (EventResult::Handled, None);
                        }
                        Key::Named(NamedKey::Escape) if element.is_open => {
                            element.is_open = false;
                            element.menu_node.remove(ctx);
                            element.icon_node.set_text(ctx, "▾");
                            return (EventResult::Handled, None);
                        }
                        Key::Named(NamedKey::ArrowDown) if element.is_open => {
                            let next = match self.selected_index {
                                Some(i) if i + 1 < self.options.len() => i + 1,
                                None if !self.options.is_empty() => 0,
                                _ => return (EventResult::Handled, None),
                            };
                            let msg = self.on_select.as_ref().map(|f| f(next));
                            return (EventResult::Handled, msg);
                        }
                        Key::Named(NamedKey::ArrowUp) if element.is_open => {
                            let prev = match self.selected_index {
                                Some(i) if i > 0 => i - 1,
                                _ => return (EventResult::Handled, None),
                            };
                            let msg = self.on_select.as_ref().map(|f| f(prev));
                            return (EventResult::Handled, msg);
                        }
                        _ => {}
                    }
                }
                (EventResult::Ignored, None)
            }
            _ => (EventResult::Ignored, None),
        }
    }
}
