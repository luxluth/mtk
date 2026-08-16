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
        let mut handled = EventResult::Ignored;
        let mut emitted_msg = None;

        match &event {
            Event::CursorMoved { hit_nodes, .. } => {
                let node = self.get_node(element);
                let newly_hovered = hit_nodes.contains(&node);

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
                let node = self.get_node(element);
                let is_hit = hit_nodes.contains(&node);
                if *pressed {
                    if is_hit {
                        element.is_pressed = true;
                        if self.kind == EventKind::Press {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    }
                } else {
                    if element.is_pressed {
                        element.is_pressed = false;
                        if is_hit {
                            if self.kind == EventKind::Click || self.kind == EventKind::Release {
                                emitted_msg = (self.handler)(state);
                                handled = EventResult::Handled;
                            }
                        } else {
                            if self.kind == EventKind::Release {
                                emitted_msg = (self.handler)(state);
                                handled = EventResult::Handled;
                            }
                        }
                    }
                }
            }
            Event::KeyboardInput {
                event: key_event, ..
            } => {
                if self.kind == EventKind::Submit && key_event.state.is_pressed() {
                    let self_node = self.get_node(element);
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

        let (inner_res, inner_msg) =
            self.inner
                .handle_event(&mut element.inner_element, state, event, ctx);

        (handled.or(inner_res), emitted_msg.or(inner_msg))
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
