//! Focus management wrappers and declarative focus extension traits.
//!
//! Provides [`FocusableExt`] and [`Focusable`] for enabling keyboard and click focus
//! on arbitrary views and container layouts.

use crate::{
    Context, Node,
    ui::{Event, EventResult, View},
};

/// Extension trait for making any view focusable via Tab navigation and mouse clicks.
pub trait FocusableExt: Sized {
    /// Registers this view as a focusable element that gains focus on mouse click and Tab navigation.
    fn focusable(self) -> Focusable<Self> {
        Focusable { inner: self }
    }
}

impl<V> FocusableExt for V {}

/// A wrapper view that registers an underlying node as focusable and requests focus on click.
pub struct Focusable<V> {
    pub(crate) inner: V,
}

impl<State, V: View<State>> View<State> for Focusable<V> {
    type Element = V::Element;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let element = self.inner.build(ctx);
        let node = self.inner.get_node(&element);
        ctx.register_focusable(node);
        element
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner
            .rebuild_with_parent(&prev.inner, ctx, element, parent, next_sibling);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        let node = self.inner.get_node(element);
        ctx.unregister_focusable(node);
        self.inner.teardown(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let node = self.inner.get_node(element);
        let (inner_res, inner_msg) = self.inner.handle_event(element, state, event.clone(), ctx);
        if inner_msg.is_some() {
            return (inner_res, inner_msg);
        }

        let mut handled = EventResult::Ignored;

        if let Event::MouseInput {
            button: winit::event::MouseButton::Left,
            pressed: true,
            hit_nodes,
            ..
        } = &event
        {
            if hit_nodes.contains(&node) {
                ctx.request_focus(node);
                handled = EventResult::Handled;
            }
        }

        (handled.or(inner_res), inner_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::text;

    #[test]
    fn test_focusable_view_lifecycle_and_click_focus() {
        let mut ctx = Context::new();
        let view = text::<_, ()>("Focus Me").focusable();

        let mut el = View::<()>::build(&view, &mut ctx);
        let node = View::<()>::get_node(&view, &el);

        // 1. Registered in active focusable nodes
        assert!(ctx.active_focusable_nodes().contains(&node));
        assert_eq!(ctx.focused_node(), None);

        // 2. Tab cycling gains focus
        ctx.focus_next();
        assert_eq!(ctx.focused_node(), Some(node));

        ctx.clear_focus();
        assert_eq!(ctx.focused_node(), None);

        // 3. Mouse click gains focus
        let (res, _) = View::<()>::handle_event(
            &view,
            &mut el,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 0.0,
                y: 0.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);
        assert_eq!(ctx.focused_node(), Some(node));

        // 4. Teardown unregisters
        View::<()>::teardown(&view, &mut ctx, &mut el);
        assert!(!ctx.active_focusable_nodes().contains(&node));
    }

    #[test]
    fn test_check_click_focus_blur() {
        let mut ctx = Context::new();
        let parent = ctx.create_node();
        let child = ctx.create_node();
        let outside = ctx.create_node();
        parent.append(&mut ctx, child);

        // 1. When nothing is focused, clicking returns None
        assert_eq!(ctx.check_click_focus_blur(&[outside]), None);
        assert_eq!(ctx.focused_node(), None);

        // 2. Focus parent, clicking parent preserves focus
        ctx.request_focus(parent);
        assert_eq!(ctx.check_click_focus_blur(&[parent]), None);
        assert_eq!(ctx.focused_node(), Some(parent));

        // 3. Clicking descendant child of focused node preserves focus
        assert_eq!(ctx.check_click_focus_blur(&[child]), None);
        assert_eq!(ctx.focused_node(), Some(parent));

        // 4. Clicking outside blurs focus and returns the blurred node
        assert_eq!(ctx.check_click_focus_blur(&[outside]), Some(parent));
        assert_eq!(ctx.focused_node(), None);

        // 5. Clicking empty space blurs focus
        ctx.request_focus(parent);
        assert_eq!(ctx.check_click_focus_blur(&[]), Some(parent));
        assert_eq!(ctx.focused_node(), None);
    }

    #[test]
    fn test_focus_lost_event_and_on_blur_handler() {
        use crate::ui::event::ViewEventExt;

        let mut ctx = Context::new();
        let view = text::<_, &'static str>("Blur Me")
            .focusable()
            .on_blur(|_| Some("blurred"));

        let mut el = View::<()>::build(&view, &mut ctx);
        let node = View::<()>::get_node(&view, &el);
        let other_node = ctx.create_node();

        // 1. Event targeting other node is ignored
        let (res_other, msg_other) = View::<()>::handle_event(
            &view,
            &mut el,
            &(),
            Event::FocusLost { node: other_node },
            &mut ctx,
        );
        assert_eq!(res_other, EventResult::Ignored);
        assert_eq!(msg_other, None);

        // 2. Event targeting this node triggers the on_blur handler
        let (res_self, msg_self) =
            View::<()>::handle_event(&view, &mut el, &(), Event::FocusLost { node }, &mut ctx);
        assert_eq!(res_self, EventResult::Handled);
        assert_eq!(msg_self, Some("blurred"));
    }

    #[test]
    fn test_focus_transition_without_disruption() {
        use crate::ui::event::ViewEventExt;

        let mut ctx = Context::new();

        // Two focusable views: A and B
        let view_a = text::<_, &'static str>("View A")
            .focusable()
            .on_blur(|_| Some("a_lost_focus"));
        let view_b = text::<_, &'static str>("View B").focusable();

        let mut el_a = View::<()>::build(&view_a, &mut ctx);
        let node_a = View::<()>::get_node(&view_a, &el_a);

        let mut el_b = View::<()>::build(&view_b, &mut ctx);
        let node_b = View::<()>::get_node(&view_b, &el_b);

        // Start with Node A focused
        ctx.request_focus(node_a);
        assert_eq!(ctx.focused_node(), Some(node_a));

        // Simulate user clicking on Node B:
        // 1. Central blur check detects click is outside Node A
        let blurred = ctx.check_click_focus_blur(&[node_b]);
        assert_eq!(blurred, Some(node_a));
        assert_eq!(ctx.focused_node(), None);

        // 2. Dispatch FocusLost to View A
        let (blur_res, blur_msg) = View::<()>::handle_event(
            &view_a,
            &mut el_a,
            &(),
            Event::FocusLost { node: node_a },
            &mut ctx,
        );
        assert_eq!(blur_res, EventResult::Handled);
        assert_eq!(blur_msg, Some("a_lost_focus"));

        // 3. Dispatch MouseInput to View B without disruption
        let (click_res, _) = View::<()>::handle_event(
            &view_b,
            &mut el_b,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 0.0,
                y: 0.0,
                hit_nodes: vec![node_b],
            },
            &mut ctx,
        );
        assert_eq!(click_res, EventResult::Handled);

        // 4. Node B cleanly gains focus
        assert_eq!(ctx.focused_node(), Some(node_b));
    }

    #[test]
    fn test_input_text_focus_lost_clears_selection() {
        use crate::ui::widgets::input_text;

        let mut ctx = Context::new();
        let view = input_text();
        let text_val = "Hello World".to_string();

        let mut el = View::<String>::build(&view, &mut ctx);
        let node = View::<String>::get_node(&view, &el);

        // Focus and select text
        ctx.request_focus(node);
        let (res, _) = View::<String>::handle_event(
            &view,
            &mut el,
            &text_val,
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 10.0,
                y: 10.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );
        assert_eq!(res, EventResult::Handled);

        // 1. Simulate outside click blur
        let blurred = ctx.check_click_focus_blur(&[]);
        assert_eq!(blurred, Some(node));
        assert_eq!(ctx.focused_node(), None);

        // 2. Dispatch FocusLost
        let (res_lost, _) = View::<String>::handle_event(
            &view,
            &mut el,
            &text_val,
            Event::FocusLost { node },
            &mut ctx,
        );
        assert_eq!(res_lost, EventResult::Handled);

        // 3. Selection and cursor are cleared
        let render_info = node
            .get_text_userdata::<crate::text::TextRenderInfo>(&ctx)
            .unwrap();
        assert_eq!(render_info.selection, None);
        assert_eq!(render_info.cursor, None);
    }
}
