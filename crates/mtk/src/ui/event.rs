//! Event handling wrappers, event consumption states, and view extension traits.
//!
//! This module provides declarative event listener wrappers ([`EventHandler`]), event results
//! ([`EventResult`]), interaction kinds ([`EventKind`]), and the [`ViewEventExt`] extension trait
//! for attaching click, hover, press, and release handlers to views.

use super::{Event, View};
use crate::{Context, Node};
use std::rc::Rc;

/// Categorizes high-level user interaction gesture triggers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Triggered on mouse button release while the cursor is over the view.
    Click,
    /// Triggered when the mouse cursor enters the view's layout bounds.
    HoverIn,
    /// Triggered when the mouse cursor exits the view's layout bounds.
    HoverOut,
    /// Triggered when a mouse button is pressed down over the view.
    Press,
    /// Triggered when a mouse button is released over the view.
    Release,
    /// Triggered when the user submits input (e.g., pressing Enter in a focused input field).
    Submit,
}

/// Indicates whether a view successfully processed or ignored an incoming event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// The event was processed by the view.
    Handled,
    /// The event was not consumed by the view and may propagate further.
    Ignored,
}

impl EventResult {
    /// Combines two [`EventResult`] values. Returns [`EventResult::Handled`] if either result is handled.
    pub fn or(self, other: EventResult) -> EventResult {
        match (self, other) {
            (EventResult::Handled, _) | (_, EventResult::Handled) => EventResult::Handled,
            _ => EventResult::Ignored,
        }
    }
}

/// A wrapper view that attaches an event listener closure to an inner view.
///
/// Created via [`ViewEventExt::on_event`].
pub struct EventHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) kind: EventKind,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

/// Persistent element state for an [`EventHandler`], tracking current hover state and inner element state.
pub struct EventElement<VEl> {
    pub(crate) inner_element: VEl,
    pub(crate) is_hovered: bool,
    pub(crate) is_pressed: bool,
}

impl<State, V: View<State>, F> View<State> for EventHandler<State, V, F>
where
    F: Fn(&State) -> Option<V::Message> + 'static,
{
    type Element = EventElement<V::Element>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        EventElement {
            inner_element: self.inner.build(ctx),
            is_hovered: false,
            is_pressed: false,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner
            .rebuild(&prev.inner, ctx, &mut element.inner_element);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.inner_element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.inner_element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let self_node = self.get_node(element);

        // Pre-track is_pressed on mouse-down for this node regardless of whether inner handles it
        if let Event::MouseInput {
            pressed, hit_nodes, ..
        } = &event
        {
            if *pressed && hit_nodes.contains(&self_node) {
                element.is_pressed = true;
            }
        }

        let (inner_res, inner_msg) =
            self.inner
                .handle_event(&mut element.inner_element, state, event.clone(), ctx);

        // If an inner child already produced a message, prioritize child and avoid duplicate parent actions
        if inner_msg.is_some() {
            if let Event::MouseInput { pressed, .. } = &event {
                if !*pressed {
                    element.is_pressed = false;
                }
            }
            return (inner_res, inner_msg);
        }

        // If inner handled the event, check if this handler should still inspect and process it:
        // 1. Submit on KeyboardInput when this node is focused
        // 2. Release / Click on MouseInput release when this node was previously pressed
        if inner_res == EventResult::Handled {
            let allow_outer_processing = match &event {
                Event::KeyboardInput { .. } => self.kind == EventKind::Submit,
                Event::MouseInput { pressed: false, .. } => {
                    element.is_pressed
                        && (self.kind == EventKind::Release || self.kind == EventKind::Click)
                }
                _ => false,
            };

            if !allow_outer_processing {
                if let Event::MouseInput { pressed, .. } = &event {
                    if !*pressed {
                        element.is_pressed = false;
                    }
                }
                return (inner_res, inner_msg);
            }
        }

        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        match &event {
            Event::CursorMoved { hit_nodes, .. } => {
                let newly_hovered = hit_nodes.contains(&self_node);

                if newly_hovered != element.is_hovered {
                    element.is_hovered = newly_hovered;
                    if newly_hovered && self.kind == EventKind::HoverIn {
                        emitted_msg = (self.handler)(state);
                        handled = EventResult::Handled;
                    } else if !newly_hovered && self.kind == EventKind::HoverOut {
                        emitted_msg = (self.handler)(state);
                        handled = EventResult::Handled;
                    }
                }
            }
            Event::MouseInput {
                pressed, hit_nodes, ..
            } => {
                let is_hit = hit_nodes.contains(&self_node);
                if *pressed {
                    if is_hit {
                        element.is_pressed = true;
                        if self.kind == EventKind::Press {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    }
                } else if element.is_pressed {
                    element.is_pressed = false;
                    if is_hit {
                        if self.kind == EventKind::Click || self.kind == EventKind::Release {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    } else if self.kind == EventKind::Release {
                        emitted_msg = (self.handler)(state);
                        handled = EventResult::Handled;
                    }
                }
            }
            Event::KeyboardInput {
                event: key_event, ..
            } => {
                if self.kind == EventKind::Submit && key_event.state.is_pressed() {
                    if Some(self_node) == ctx.focused_node() {
                        let is_enter = match key_event.logical_key.as_ref() {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => true,
                            winit::keyboard::Key::Character(s) => s == "\r" || s == "\n",
                            _ => false,
                        };
                        if is_enter {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    }
                }
            }
            _ => {}
        }

        (handled.or(inner_res), inner_msg.or(emitted_msg))
    }
}

/// Extension trait for [`View`] providing event handling combinators.
pub trait ViewEventExt<State>: View<State> + Sized {
    /// Attaches an event listener closure that runs when `event` occurs on this view.
    ///
    /// # Parameters
    /// - `event`: The [`EventKind`] trigger (e.g. `EventKind::Click`, `EventKind::HoverIn`).
    /// - `handler`: A closure evaluating current application state and optionally returning a message.
    fn on_event<F>(self, event: EventKind, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static;
}

impl<State, V: View<State>> ViewEventExt<State> for V {
    fn on_event<F>(self, event: EventKind, handler: F) -> EventHandler<State, Self, F>
    where
        F: Fn(&State) -> Option<Self::Message> + 'static,
    {
        EventHandler {
            inner: self,
            kind: event,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::{row, text};

    #[derive(Clone, Debug, PartialEq)]
    enum TestMsg {
        ParentClick,
        ChildClick,
    }

    #[test]
    fn test_nested_event_handler_child_priority() {
        let mut ctx = Context::new();

        let child_view = row((text::<_, TestMsg>("Child"),))
            .on_event(EventKind::Click, |_| Some(TestMsg::ChildClick));

        let parent_view = row((child_view, text::<_, TestMsg>("Parent Text")))
            .on_event(EventKind::Click, |_| Some(TestMsg::ParentClick));

        let mut element = View::<()>::build(&parent_view, &mut ctx);
        let parent_node = View::<()>::get_node(&parent_view, &element);
        let child_node =
            View::<()>::get_node(&parent_view.inner.children.0, &element.inner_element.1.0);

        // 1. Click child node: both child and parent are in hit_nodes
        // Press on child
        let (res_down, msg_down) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![child_node, parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res_down, EventResult::Ignored);
        assert_eq!(msg_down, None);

        // Release on child: child handler must fire, parent handler must NOT fire
        let (res_up, msg_up) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: false,
                hit_nodes: vec![child_node, parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(res_up, EventResult::Handled);
        assert_eq!(msg_up, Some(TestMsg::ChildClick));

        // 2. Click parent node directly (child not hit)
        let (p_down_res, p_down_msg) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(p_down_res, EventResult::Ignored);
        assert_eq!(p_down_msg, None);

        let (p_up_res, p_up_msg) = View::<()>::handle_event(
            &parent_view,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: false,
                hit_nodes: vec![parent_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(p_up_res, EventResult::Handled);
        assert_eq!(p_up_msg, Some(TestMsg::ParentClick));
    }

    #[test]
    fn test_chained_press_and_release() {
        #[derive(Clone, Debug, PartialEq)]
        enum BtnMsg {
            Press,
            Release,
        }

        let mut ctx = Context::new();
        let btn_view = text::<_, BtnMsg>("7")
            .on_event(EventKind::Press, |_| Some(BtnMsg::Press))
            .on_event(EventKind::Release, |_| Some(BtnMsg::Release));

        let mut element = View::<()>::build(&btn_view, &mut ctx);
        let btn_node = View::<()>::get_node(&btn_view, &element);

        // 1. Mouse down on button
        let (down_res, down_msg) = View::<()>::handle_event(
            &btn_view,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: true,
                hit_nodes: vec![btn_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(down_res, EventResult::Handled);
        assert_eq!(down_msg, Some(BtnMsg::Press));

        // 2. Mouse up on button -> Release MUST fire
        let (up_res, up_msg) = View::<()>::handle_event(
            &btn_view,
            &mut element,
            &(),
            Event::MouseInput {
                pressed: false,
                hit_nodes: vec![btn_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(up_res, EventResult::Handled);
        assert_eq!(up_msg, Some(BtnMsg::Release));
    }
}
