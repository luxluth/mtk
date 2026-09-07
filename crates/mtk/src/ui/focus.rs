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
}
