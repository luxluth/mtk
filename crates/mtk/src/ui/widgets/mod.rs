use std::marker::PhantomData;

use crate::{
    Context, FlexDirection, Node,
    ui::{Event, View, ViewSequence, event::EventResult},
};

pub use input_text::*;
pub use scroll_view::*;
pub use textarea::*;

mod editor;
mod input_text;
mod scroll_view;
mod textarea;

pub struct Text<Msg> {
    pub(crate) label: String,
    _marker: PhantomData<Msg>,
}

pub fn text<S: ToString, Msg>(label: S) -> Text<Msg> {
    Text {
        label: label.to_string(),
        _marker: PhantomData,
    }
}

impl<State, Msg> View<State> for Text<Msg> {
    type Element = Node;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        node.set_text(ctx, &self.label);

        node
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.label != prev.label {
            element.set_text(ctx, &self.label);
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        element.remove(ctx);
        ctx.destroy_node(*element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        *element
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

pub struct Container<Children> {
    pub(crate) children: Children,
    pub(crate) direction: Option<FlexDirection>,
}

pub fn container<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: None,
    }
}

pub fn column<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: Some(FlexDirection::Column),
    }
}

pub fn row<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: Some(FlexDirection::Row),
    }
}

impl<State, Children> View<State> for Container<Children>
where
    Children: ViewSequence<State>,
{
    // The Element is a tuple: (The Parent Node, The Elements of the Children)
    type Element = (Node, Children::Elements);
    type Message = Children::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let parent = ctx.create_node();

        if let Some(direction) = self.direction {
            parent.update_constraints(ctx, |c| {
                c.flex_direction = direction;
            });
        }

        // Build the children and append them to `parent`
        let child_elements = self.children.build(ctx, parent);

        (parent, child_elements)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        // Rebuild children
        self.children
            .rebuild(&prev.children, ctx, &mut element.1, element.0);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        // Teardown all children first (gives them a chance to clean up custom states)
        self.children.teardown(ctx, &mut element.1);

        // Then destroy the parent container
        element.0.remove(ctx);
        ctx.destroy_node(element.0);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.0
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        self.children
            .handle_event(&mut element.1, state, event, ctx)
    }
}
