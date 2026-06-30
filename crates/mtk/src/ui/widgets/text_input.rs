use crate::ui::{Event, EventKind, Style, View, ViewEventExt, ViewStyleExt};
use crate::{AlignItems, Context, JustifyContent, Lens, Node, Size, clr, rgb};
use std::sync::Mutex;

pub struct TextInput;

pub fn text_input() -> TextInput {
    TextInput
}

impl<S> View<S> for TextInput {
    type Element = (Node, Node, bool); // Node, Caret, Hover

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        let caret = ctx.create_node();
        (node, caret, false)
    }

    fn rebuild(&self, _prev: &Self, _ctx: &mut Context, _element: &mut Self::Element) {
        // TODO:
    }

    fn teardown(&self, _ctx: &mut Context, _element: &mut Self::Element) {}

    fn get_node(&self, element: &Self::Element) -> Node {
        element.0.clone()
    }

    fn message(&self, element: &mut Self::Element, state: &mut S, event: Event, ctx: &mut Context) {
        // TODO:
    }
}
