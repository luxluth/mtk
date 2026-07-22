//! Built-in UI Widgets in MTK.
//!
//! This module provides a set of reusable UI components (widgets) that can be combined
//! to build complex user interfaces. Each widget implements the [View] trait, meaning
//! it can be built into a renderable node, react to state changes, and handle user events.
//!
//! # Available Widgets
//! - **Layouts**: [container], [column()], [row()] - Basic building blocks for structuring your UI.
//! - **Display**: [text()] - Renders static text strings.
//! - **Input**: [input_text()] - A single-line text input field with full cursor and selection support.
//! - **Containers**: [scroll_view()] - A scrollable container for displaying overflowing content.
//!
//! # Usage
//! Widgets are typically constructed in your application's `app` function and often wrapped
//! in `adapt` if they need to map local widget states to global application states.
//!
//! ```rust,ignore
//! fn app(state: &AppState) -> impl View<AppState, Message = AppMsg> + use<> {
//!     column((
//!         text("Welcome to MTK!"),
//!         row((
//!             text("Username:"),
//!             adapt(input_text(), AppState::username, AppMsg::UpdateUsername)
//!         ))
//!     ))
//! }
//! ```

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

/// A simple widget that displays a string of text.
///
/// The `Text` widget takes a string-like value and creates a node with that text.
/// If the text changes during a rebuild, the underlying node's text is automatically updated.
pub struct Text<Msg> {
    pub(crate) label: String,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Text` widget displaying the provided label.
///
/// # Examples
/// ```rust,ignore
/// text("Hello, World!")
/// text(format!("Counter: {}", state.count))
/// ```
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
        // Obscure code note: We only update the text if the label actually changed
        // between the previous render tree and the new one. This prevents unnecessary
        // dirtying of the layout and rendering pipeline.
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

/// A generic layout container that groups a sequence of children widgets.
///
/// `Container` is the base widget for creating structured layouts. By default, it has no
/// flex direction (stacking or absolute layout depending on style), but can be configured
/// to behave as a `column` or `row`.
pub struct Container<Children> {
    pub(crate) children: Children,
    pub(crate) direction: Option<FlexDirection>,
}

/// Creates a new `Container` widget with the given children and no default flex direction.
pub fn container<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: None,
    }
}

/// Creates a vertical layout container (`FlexDirection::Column`).
///
/// Children provided to `column` will be stacked vertically from top to bottom.
///
/// # Examples
/// ```rust,ignore
/// column((
///     text("Top"),
///     text("Bottom"),
/// ))
/// ```
pub fn column<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: Some(FlexDirection::Column),
    }
}

/// Creates a horizontal layout container (`FlexDirection::Row`).
///
/// Children provided to `row` will be stacked horizontally from left to right.
///
/// # Examples
/// ```rust,ignore
/// row((
///     text("Left"),
///     text("Right"),
/// ))
/// ```
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
