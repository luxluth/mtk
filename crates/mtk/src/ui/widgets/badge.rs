use std::marker::PhantomData;

use crate::colors::Color;
use crate::style::{AlignItems, JustifyContent, Style, TextStyle};
use crate::text_property::FontWeight;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, rgb};

/// A compact status indicator or pill badge.
pub struct Badge<Msg> {
    pub(crate) text: String,
    pub(crate) bg_color: Color,
    pub(crate) text_color: Color,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Badge` with the given text.
///
/// # Examples
/// ```rust,ignore
/// badge("Active").success()
/// ```
pub fn badge<Msg>(text: impl Into<String>) -> Badge<Msg> {
    Badge {
        text: text.into(),
        bg_color: rgb!(241, 245, 249),
        text_color: rgb!(71, 85, 105),
        _marker: PhantomData,
    }
}

impl<Msg> Badge<Msg> {
    /// Sets the badge to a success green appearance.
    pub fn success(mut self) -> Self {
        self.bg_color = rgb!(220, 252, 231);
        self.text_color = rgb!(22, 101, 52);
        self
    }

    /// Sets the badge to a warning yellow/orange appearance.
    pub fn warning(mut self) -> Self {
        self.bg_color = rgb!(254, 243, 199);
        self.text_color = rgb!(146, 64, 14);
        self
    }

    /// Sets the badge to an error red appearance.
    pub fn error(mut self) -> Self {
        self.bg_color = rgb!(254, 226, 226);
        self.text_color = rgb!(153, 27, 27);
        self
    }

    /// Sets the badge to an informative blue appearance.
    pub fn info(mut self) -> Self {
        self.bg_color = rgb!(219, 234, 254);
        self.text_color = rgb!(30, 64, 175);
        self
    }

    /// Sets custom background and text colors.
    pub fn custom(mut self, bg: Color, text: Color) -> Self {
        self.bg_color = bg;
        self.text_color = text;
        self
    }
}

pub struct BadgeElement {
    node: Node,
}

impl<State, Msg> View<State> for Badge<Msg> {
    type Element = BadgeElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();

        node.set_text_with_userdata(
            ctx,
            &self.text,
            TextStyle {
                font_size: 11.0,
                font_weight: FontWeight::SEMI_BOLD,
                color: self.text_color,
                wrap: false,
                ..Default::default()
            },
        );

        Style::new()
            .padding_xy(8.0, 3.0)
            .corner_radius(12.0)
            .bg_color(self.bg_color)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .apply_to_node(ctx, node);

        BadgeElement { node }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.text != prev.text || self.text_color != prev.text_color {
            element.node.set_text_with_userdata(
                ctx,
                &self.text,
                TextStyle {
                    font_size: 11.0,
                    font_weight: FontWeight::SEMI_BOLD,
                    color: self.text_color,
                    wrap: false,
                    ..Default::default()
                },
            );
        }
        if self.bg_color != prev.bg_color {
            element.node.update_effects(ctx, |e| {
                e.background_color = self.bg_color;
            });
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        element.node.remove(ctx);
        ctx.destroy_node(element.node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.node
    }

    fn handle_event(
        &self,
        _element: &mut Self::Element,
        _state: &State,
        _event: Event,
        _ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        (EventResult::Ignored, None)
    }
}

/// An interactive tag chip widget with an optional delete action.
pub struct Chip<Msg, F = fn() -> Msg> {
    pub(crate) label: String,
    pub(crate) on_delete: Option<F>,
    pub(crate) bg_color: Color,
    pub(crate) text_color: Color,
    _marker: std::marker::PhantomData<Msg>,
}

/// Creates a new interactive `Chip` tag widget.
pub fn chip<Msg>(label: impl Into<String>) -> Chip<Msg, fn() -> Msg> {
    Chip {
        label: label.into(),
        on_delete: None,
        bg_color: rgb!(241, 245, 249),
        text_color: rgb!(30, 41, 59),
        _marker: std::marker::PhantomData,
    }
}

impl<Msg, F> Chip<Msg, F> {
    /// Sets the callback to emit when the user clicks the delete button.
    pub fn on_delete<NewF: Fn() -> Msg>(self, on_delete: NewF) -> Chip<Msg, NewF> {
        Chip {
            label: self.label,
            on_delete: Some(on_delete),
            bg_color: self.bg_color,
            text_color: self.text_color,
            _marker: std::marker::PhantomData,
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
}

pub struct ChipElement {
    pub(crate) container_node: Node,
    pub(crate) label_node: Node,
    pub(crate) delete_node: Option<Node>,
}

impl<State, Msg, F> View<State> for Chip<Msg, F>
where
    F: Fn() -> Msg,
{
    type Element = ChipElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        let label_node = ctx.create_node();

        label_node.set_text_with_userdata(
            ctx,
            &self.label,
            TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::MEDIUM,
                color: self.text_color,
                wrap: false,
                ..Default::default()
            },
        );

        Style::new()
            .flex_direction(crate::style::FlexDirection::Row)
            .align_items(AlignItems::Center)
            .padding_xy(10.0, 5.0)
            .gap(6.0)
            .corner_radius(14.0)
            .bg_color(self.bg_color)
            .border(1.0, rgb!(203, 213, 225))
            .apply_to_node(ctx, container_node);

        container_node.append(ctx, label_node);

        let delete_node = if self.on_delete.is_some() {
            let del_node = ctx.create_node();
            del_node.set_text_with_userdata(
                ctx,
                "✕",
                TextStyle {
                    font_size: 11.0,
                    font_weight: FontWeight::BOLD,
                    color: self.text_color,
                    wrap: false,
                    ..Default::default()
                },
            );
            container_node.append(ctx, del_node);
            Some(del_node)
        } else {
            None
        };

        ChipElement {
            container_node,
            label_node,
            delete_node,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.label != prev.label || self.text_color != prev.text_color {
            element.label_node.set_text_with_userdata(
                ctx,
                &self.label,
                TextStyle {
                    font_size: 13.0,
                    font_weight: FontWeight::MEDIUM,
                    color: self.text_color,
                    wrap: false,
                    ..Default::default()
                },
            );
            if let Some(del) = element.delete_node {
                del.set_text_with_userdata(
                    ctx,
                    "✕",
                    TextStyle {
                        font_size: 11.0,
                        font_weight: FontWeight::BOLD,
                        color: self.text_color,
                        wrap: false,
                        ..Default::default()
                    },
                );
            }
        }
        if self.bg_color != prev.bg_color {
            element.container_node.update_effects(ctx, |e| {
                e.background_color = self.bg_color;
            });
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some(del) = element.delete_node {
            del.remove(ctx);
            ctx.destroy_node(del);
        }
        element.label_node.remove(ctx);
        ctx.destroy_node(element.label_node);
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
        _ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if let Event::MouseInput {
            pressed: true,
            hit_nodes,
            ..
        } = event
        {
            if let Some(del_node) = element.delete_node {
                if hit_nodes.contains(&del_node) {
                    if let Some(ref on_del) = self.on_delete {
                        return (EventResult::Handled, Some(on_del()));
                    }
                }
            }
        }

        (EventResult::Ignored, None)
    }
}
