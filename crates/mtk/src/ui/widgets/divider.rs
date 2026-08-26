use std::marker::PhantomData;

use crate::colors::Color;
use crate::style::{Size, Style};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, rgb};

/// A visual separator line.
pub struct Divider<Msg> {
    pub(crate) is_vertical: bool,
    pub(crate) thickness: f32,
    pub(crate) color: Color,
    _marker: PhantomData<Msg>,
}

/// Creates a horizontal 1px divider.
pub fn divider<Msg>() -> Divider<Msg> {
    Divider {
        is_vertical: false,
        thickness: 1.0,
        color: rgb!(226, 232, 240),
        _marker: PhantomData,
    }
}

/// Creates a vertical 1px divider.
pub fn v_divider<Msg>() -> Divider<Msg> {
    Divider {
        is_vertical: true,
        thickness: 1.0,
        color: rgb!(226, 232, 240),
        _marker: PhantomData,
    }
}

impl<Msg> Divider<Msg> {
    /// Sets custom thickness.
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    /// Sets custom divider color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

pub struct DividerElement {
    node: Node,
}

impl<State, Msg> View<State> for Divider<Msg> {
    type Element = DividerElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();

        let mut style = Style::new().bg_color(self.color);
        if self.is_vertical {
            style = style
                .width(Size::Fixed(self.thickness as u32))
                .height(Size::Percent(1.0));
        } else {
            style = style
                .width(Size::Percent(1.0))
                .height(Size::Fixed(self.thickness as u32));
        }
        style.apply_to_node(ctx, node);

        DividerElement { node }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.color != prev.color {
            element.node.update_effects(ctx, |e| {
                e.background_color = self.color;
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

/// A flexible layout spacer that expands to fill available space (`flex_grow: 1.0`).
pub struct Spacer<Msg> {
    _marker: PhantomData<Msg>,
}

/// Creates a flexible layout spacer.
pub fn spacer<Msg>() -> Spacer<Msg> {
    Spacer {
        _marker: PhantomData,
    }
}

pub struct SpacerElement {
    node: Node,
}

impl<State, Msg> View<State> for Spacer<Msg> {
    type Element = SpacerElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        Style::new().flex_grow(1.0).apply_to_node(ctx, node);
        SpacerElement { node }
    }

    fn rebuild(&self, _prev: &Self, _ctx: &mut Context, _element: &mut Self::Element) {}

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
