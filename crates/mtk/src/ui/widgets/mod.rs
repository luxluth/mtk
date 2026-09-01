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

pub use crate::ui::router::router;
pub use async_image::*;
pub use badge::*;
pub use button::*;
pub use canvas::*;
pub use checkbox::*;
pub use divider::*;
pub use dropdown::*;
pub use editor::Editor;
pub use image::*;
pub use input_text::*;
pub use modal::*;
pub use progress_bar::*;
pub use radio::*;
pub use rich_text::*;
pub use scroll_view::*;
pub use slider::*;
pub use svg::*;
pub use switch::*;
pub use textarea::*;
pub use tooltip::*;
pub use virtual_list::*;

mod async_image;
mod badge;
mod button;
pub mod canvas;
mod checkbox;
mod divider;
mod dropdown;
pub mod editor;
mod image;
mod input_text;
mod modal;
mod progress_bar;
mod radio;
mod rich_text;
mod scroll_view;
mod slider;
mod svg;
mod switch;
mod textarea;
mod tooltip;
mod virtual_list;

use crate::debugger::SourceLocation;

/// A simple widget that displays a string of text.
///
/// The `Text` widget takes a string-like value and creates a node with that text.
/// If the text changes during a rebuild, the underlying node's text is automatically updated.
pub struct Text<Msg> {
    pub(crate) label: String,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `Text` widget displaying the provided label.
///
/// # Examples
/// ```rust,ignore
/// text("Hello, World!")
/// text(format!("Counter: {}", state.count))
/// ```
#[track_caller]
pub fn text<S: ToString, Msg>(label: S) -> Text<Msg> {
    Text {
        label: label.to_string(),
        source_loc: Some(SourceLocation::here("Text")),
        _marker: PhantomData,
    }
}

impl<State, Msg> View<State> for Text<Msg> {
    type Element = Node;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(node, loc);
        }
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
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Creates a new `Container` widget with the given children and no default flex direction.
#[track_caller]
pub fn container<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: None,
        source_loc: Some(SourceLocation::here("Container")),
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
#[track_caller]
pub fn column<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: Some(FlexDirection::Column),
        source_loc: Some(SourceLocation::here("Column")),
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
#[track_caller]
pub fn row<Children>(children: Children) -> Container<Children> {
    Container {
        children,
        direction: Some(FlexDirection::Row),
        source_loc: Some(SourceLocation::here("Row")),
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
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(parent, loc);
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum TestMsg {
        Clicked,
        Toggled(bool),
        SliderChanged(f32),
        OptionSelected(usize),
        Closed,
        Deleted,
    }

    #[test]
    fn test_button_lifecycle_and_events() {
        let mut ctx = Context::new();
        let btn = button("Click Me").on_click(TestMsg::Clicked);

        let mut element = View::<()>::build(&btn, &mut ctx);
        let node = View::<()>::get_node(&btn, &element);
        assert!(node.get_computed(&ctx).is_some() || node.get_constraints(&ctx).is_some());

        // Press
        let (res, msg) = View::<()>::handle_event(
            &btn,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, None);

        // Release
        let (res, msg) = View::<()>::handle_event(
            &btn,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: false,
                hit_nodes: vec![node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(TestMsg::Clicked));

        // Rebuild with new label
        let btn2 = button("Updated Label").on_click(TestMsg::Clicked);
        View::<()>::rebuild(&btn2, &btn, &mut ctx, &mut element);
        View::<()>::teardown(&btn2, &mut ctx, &mut element);
    }

    #[test]
    fn test_checkbox_lifecycle_and_events() {
        let mut ctx = Context::new();
        let chk = checkbox(false)
            .label("Accept Terms")
            .on_toggle(TestMsg::Toggled);

        let mut element = View::<()>::build(&chk, &mut ctx);
        let node = View::<()>::get_node(&chk, &element);

        // Click to toggle
        let _ = View::<()>::handle_event(
            &chk,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        let (res, msg) = View::<()>::handle_event(
            &chk,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: false,
                hit_nodes: vec![node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(TestMsg::Toggled(true)));

        // Rebuild checked
        let chk2 = checkbox(true)
            .label("Accept Terms")
            .on_toggle(TestMsg::Toggled);
        View::<()>::rebuild(&chk2, &chk, &mut ctx, &mut element);
        View::<()>::teardown(&chk2, &mut ctx, &mut element);
    }

    #[test]
    fn test_switch_lifecycle_and_events() {
        let mut ctx = Context::new();
        let sw = switch(false).on_toggle(TestMsg::Toggled);

        let mut element = View::<()>::build(&sw, &mut ctx);
        let node = View::<()>::get_node(&sw, &element);

        // Click to toggle
        let _ = View::<()>::handle_event(
            &sw,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        let (res, msg) = View::<()>::handle_event(
            &sw,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: false,
                hit_nodes: vec![node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(TestMsg::Toggled(true)));

        let sw2 = switch(true).on_toggle(TestMsg::Toggled);
        View::<()>::rebuild(&sw2, &sw, &mut ctx, &mut element);
        View::<()>::teardown(&sw2, &mut ctx, &mut element);
    }

    #[test]
    fn test_slider_lifecycle_and_events() {
        let mut ctx = Context::new();
        let sld = slider(50.0, 0.0, 100.0).on_change(TestMsg::SliderChanged);

        let mut element = View::<()>::build(&sld, &mut ctx);
        let node = View::<()>::get_node(&sld, &element);
        ctx.root_attach(node);
        ctx.compute_layout(200.0, 50.0);

        // Click halfway
        let (res, msg) = View::<()>::handle_event(
            &sld,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![node],
                x: 100.0,
                y: 10.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert!(msg.is_some());

        let sld2 = slider(75.0, 0.0, 100.0).on_change(TestMsg::SliderChanged);
        View::<()>::rebuild(&sld2, &sld, &mut ctx, &mut element);
        View::<()>::teardown(&sld2, &mut ctx, &mut element);
    }

    #[test]
    fn test_modal_lifecycle_and_events() {
        fn close_cb() -> TestMsg {
            TestMsg::Closed
        }

        let mut ctx = Context::new();
        let m = modal(true, text("Main View"), button("Dialog Button")).on_close(close_cb);

        let mut element = View::<()>::build(&m, &mut ctx);
        let node = View::<()>::get_node(&m, &element);
        ctx.root_attach(node);
        ctx.compute_layout(400.0, 400.0);

        // Clicking scrim (outside dialog) triggers on_close
        let scrim = element.scrim_node;
        let (res, msg) = View::<()>::handle_event(
            &m,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![scrim],
                x: 10.0,
                y: 10.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(TestMsg::Closed));

        let m_closed = modal(false, text("Main View"), button("Dialog Button")).on_close(close_cb);
        View::<()>::rebuild(&m_closed, &m, &mut ctx, &mut element);
        View::<()>::teardown(&m_closed, &mut ctx, &mut element);
    }

    #[test]
    fn test_dropdown_lifecycle_and_events() {
        let mut ctx = Context::new();
        let opts = vec!["Option 1".to_string(), "Option 2".to_string()];
        let dd = dropdown(Some(0), opts).on_select(TestMsg::OptionSelected);

        let mut element = View::<()>::build(&dd, &mut ctx);
        let trig = element.trigger_node;
        ctx.root_attach(View::<()>::get_node(&dd, &element));
        ctx.compute_layout(200.0, 200.0);

        // Click trigger opens dropdown
        let (res, _) = View::<()>::handle_event(
            &dd,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![trig],
                x: 10.0,
                y: 10.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert!(element.is_open);

        // Click option 1 selects it
        let opt1 = element.item_nodes[1];
        let (res, msg) = View::<()>::handle_event(
            &dd,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![opt1],
                x: 10.0,
                y: 10.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(TestMsg::OptionSelected(1)));
        assert!(!element.is_open);

        View::<()>::teardown(&dd, &mut ctx, &mut element);
    }

    #[test]
    fn test_radio_and_progress_bar() {
        let mut ctx = Context::new();
        let r = radio(false).label("Radio 1").on_select(|| TestMsg::Clicked);
        let mut r_el = View::<()>::build(&r, &mut ctx);
        let r_node = View::<()>::get_node(&r, &r_el);

        let (res, _) = View::<()>::handle_event(
            &r,
            &mut r_el,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![r_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        let (res, msg) = View::<()>::handle_event(
            &r,
            &mut r_el,
            &(),
            Event::MouseInput {
                pressed: false,
                hit_nodes: vec![r_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(TestMsg::Clicked));
        View::<()>::teardown(&r, &mut ctx, &mut r_el);

        // ProgressBar
        let pb: ProgressBar<TestMsg> = progress_bar(0.6).indeterminate(true);
        let mut pb_el = View::<()>::build(&pb, &mut ctx);
        let (res, _) =
            View::<()>::handle_event(&pb, &mut pb_el, &(), Event::Tick { dt: 0.016 }, &mut ctx);
        assert_eq!(res, EventResult::Handled);
        View::<()>::teardown(&pb, &mut ctx, &mut pb_el);
    }

    #[test]
    fn test_badge_divider_and_chip() {
        let mut ctx = Context::new();
        let b: Badge<TestMsg> = badge("Active").success();
        let mut b_el = View::<()>::build(&b, &mut ctx);
        View::<()>::teardown(&b, &mut ctx, &mut b_el);

        let d: Divider<TestMsg> = divider();
        let mut d_el = View::<()>::build(&d, &mut ctx);
        View::<()>::teardown(&d, &mut ctx, &mut d_el);

        let sp: Spacer<TestMsg> = spacer();
        let mut sp_el = View::<()>::build(&sp, &mut ctx);
        View::<()>::teardown(&sp, &mut ctx, &mut sp_el);

        let ch = chip("Rust").on_delete(|| TestMsg::Deleted);
        let mut ch_el = View::<()>::build(&ch, &mut ctx);
        let del_node = ch_el.delete_node.unwrap();
        let (res, msg) = View::<()>::handle_event(
            &ch,
            &mut ch_el,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![del_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(msg, Some(TestMsg::Deleted));
        View::<()>::teardown(&ch, &mut ctx, &mut ch_el);
    }

    #[test]
    fn test_layer_scoped_focus_traversal() {
        let mut ctx = Context::new();

        // 1. Create base layer nodes
        let base_btn1 = ctx.create_node();
        let base_btn2 = ctx.create_node();
        ctx.register_focusable(base_btn1);
        ctx.register_focusable(base_btn2);

        // When on base layer, tab cycles between base nodes
        assert_eq!(ctx.active_focusable_nodes(), vec![base_btn1, base_btn2]);
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(base_btn1));
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(base_btn2));
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(base_btn1));

        // 2. Open a modal with dialog buttons
        let modal_root = ctx.create_node();
        let dlg_btn1 = ctx.create_node();
        let dlg_btn2 = ctx.create_node();
        modal_root.append(&mut ctx, dlg_btn1);
        modal_root.append(&mut ctx, dlg_btn2);
        ctx.register_focusable(dlg_btn1);
        ctx.register_focusable(dlg_btn2);

        ctx.modal_layer.state.root_node = Some(modal_root);
        ctx.modal_layer.state.visible = true;

        // While modal is active, active_focusable_nodes only includes modal buttons!
        assert_eq!(ctx.active_focusable_nodes(), vec![dlg_btn1, dlg_btn2]);

        // Focus next immediately jumps to the active modal buttons and traps focus within the modal
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(dlg_btn1));
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(dlg_btn2));
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(dlg_btn1));
        ctx.focus_prev();
        assert_eq!(ctx.focused_node(), Some(dlg_btn2));

        // 3. Close the modal
        ctx.modal_layer.state.visible = false;
        assert_eq!(ctx.active_focusable_nodes(), vec![base_btn1, base_btn2]);
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(base_btn1));
    }
}
