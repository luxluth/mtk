use crate::{
    Context, FlexDirection, Node,
    ui::{Event, View, ViewSequence},
};

pub use text_input::*;

mod editor;
mod text_input;

pub struct Text {
    pub(crate) label: String,
}

pub fn text<S: ToString>(label: S) -> Text {
    Text {
        label: label.to_string(),
    }
}

impl<State> View<State> for Text {
    type Element = Node;

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

    fn message(
        &self,
        _element: &mut Self::Element,
        _state: &mut State,
        _event: Event,
        _ctx: &mut Context,
    ) {
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
        self.children.rebuild(&prev.children, ctx, &mut element.1);
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

    fn message(
        &self,
        element: &mut Self::Element,
        state: &mut State,
        event: Event,
        ctx: &mut Context,
    ) {
        self.children.message(&mut element.1, state, event, ctx);
    }
}
