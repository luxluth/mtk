//! OS window drag gesture wrapper widget.
//!
//! Provides [`os_draggable`], [`OsDraggable`], and [`OsDraggableExt`] for handing window
//! movement directly over to the operating system window manager or compositor.

use std::marker::PhantomData;

use crate::debugger::SourceLocation;
use crate::layout::Computed;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A wrapper view that initiates an OS-level window drag when pressed with the primary mouse button.
///
/// This hands control of interactive window movement directly to the operating system,
/// desktop window manager, or compositor.
pub struct OsDraggable<State, V> {
    pub(crate) inner: V,
    pub(crate) enabled: bool,
    pub(crate) source_loc: Option<SourceLocation>,
    pub(crate) _marker: PhantomData<State>,
}

/// Persistent DOM element state for an [`OsDraggable`] widget.
pub struct OsDraggableElement<VEl> {
    pub(crate) inner_element: VEl,
}

/// Creates a new `OsDraggable` widget wrapping `inner`.
///
/// Pressing down with the primary (left) mouse button on this view or any of its non-interactive
/// child elements initiates an OS-level window drag.
///
/// # Examples
/// ```rust,ignore
/// use mtk::ui::widgets::{os_draggable, row, text};
///
/// os_draggable(row((
///     text("Custom Window Titlebar"),
/// )))
/// ```
#[track_caller]
pub fn os_draggable<State, V: View<State>>(inner: V) -> OsDraggable<State, V> {
    OsDraggable {
        inner,
        enabled: true,
        source_loc: Some(SourceLocation::here("OsDraggable")),
        _marker: PhantomData,
    }
}

impl<State, V: View<State>> OsDraggable<State, V> {
    /// Enables or disables OS window dragging.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Extension trait providing `.os_draggable()` on any view.
pub trait OsDraggableExt<State>: View<State> + Sized {
    /// Wraps this view in [`OsDraggable`] to initiate an OS window drag when pressed.
    fn os_draggable(self) -> OsDraggable<State, Self> {
        os_draggable(self)
    }
}

impl<State, V: View<State>> OsDraggableExt<State> for V {}

impl<State, V: View<State>> View<State> for OsDraggable<State, V> {
    type Element = OsDraggableElement<V::Element>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let inner_element = self.inner.build(ctx);
        if let Some(loc) = self.source_loc {
            let node = self.inner.get_node(&inner_element);
            ctx.set_node_source(node, loc);
        }
        OsDraggableElement { inner_element }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner
            .rebuild(&prev.inner, ctx, &mut element.inner_element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner.rebuild_with_parent(
            &prev.inner,
            ctx,
            &mut element.inner_element,
            parent,
            next_sibling,
        );
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.inner_element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.inner_element)
    }

    fn on_layout(&self, element: &mut Self::Element, computed: Computed, ctx: &mut Context) {
        self.inner
            .on_layout(&mut element.inner_element, computed, ctx);
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let self_node = self.inner.get_node(&element.inner_element);

        let (inner_res, inner_msg) =
            self.inner
                .handle_event(&mut element.inner_element, state, event.clone(), ctx);

        if inner_res == EventResult::Handled || inner_msg.is_some() {
            return (inner_res, inner_msg);
        }

        if self.enabled {
            if let Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                hit_nodes,
                ..
            } = &event
            {
                if hit_nodes.contains(&self_node) {
                    let _ = ctx.drag_window();
                    return (EventResult::Handled, None);
                }
            }
        }

        (inner_res, inner_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::{button, text};

    #[test]
    fn test_os_draggable_handles_mouse_press() {
        let mut ctx = Context::new();
        let view = os_draggable::<(), _>(text::<&str, ()>("Title Bar"));
        let mut element = view.build(&mut ctx);
        let node = view.get_node(&element);

        let (res, msg) = view.handle_event(
            &mut element,
            &(),
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
        assert_eq!(msg, None);

        // Outside click should be ignored
        let (res_outside, _) = view.handle_event(
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 100.0,
                y: 100.0,
                hit_nodes: vec![],
            },
            &mut ctx,
        );
        assert_eq!(res_outside, EventResult::Ignored);

        view.teardown(&mut ctx, &mut element);
    }

    #[test]
    fn test_os_draggable_disabled() {
        let mut ctx = Context::new();
        let view = os_draggable::<(), _>(text::<&str, ()>("Title Bar")).enabled(false);
        let mut element = view.build(&mut ctx);
        let node = view.get_node(&element);

        let (res, _) = view.handle_event(
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 10.0,
                y: 10.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );

        assert_eq!(res, EventResult::Ignored);
        view.teardown(&mut ctx, &mut element);
    }

    #[test]
    fn test_os_draggable_inner_interactive_priority() {
        #[derive(Clone, Debug, PartialEq)]
        enum Msg {
            Close,
        }

        let mut ctx = Context::new();
        let close_btn = button("X").on_click(Msg::Close);
        let view = os_draggable::<(), _>(close_btn);
        let mut element = view.build(&mut ctx);
        let node = view.get_node(&element);

        // Mouse down on the button: button handles it (pressed state)
        let (res_down, msg_down) = view.handle_event(
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 10.0,
                y: 10.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );

        assert_eq!(res_down, EventResult::Handled);
        assert_eq!(msg_down, None);

        // Mouse up on the button: button emits Close message
        let (res_up, msg_up) = view.handle_event(
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                x: 10.0,
                y: 10.0,
                hit_nodes: vec![node],
            },
            &mut ctx,
        );

        assert_eq!(res_up, EventResult::Handled);
        assert_eq!(msg_up, Some(Msg::Close));

        view.teardown(&mut ctx, &mut element);
    }

    #[test]
    fn test_os_draggable_extension_trait() {
        let mut ctx = Context::new();
        let view = text::<&str, ()>("Draggable").os_draggable();
        let mut element = view.build(&mut ctx);
        let node = view.get_node(&element);

        let (res, _) = view.handle_event(
            &mut element,
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
        view.teardown(&mut ctx, &mut element);
    }
}
