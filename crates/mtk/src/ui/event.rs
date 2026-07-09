use super::{Event, View};
use crate::{Context, Node};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Click,
    HoverIn,
    HoverOut,
    Press,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Handled,
    Ignored,
}

impl EventResult {
    pub fn or(self, other: EventResult) -> EventResult {
        match (self, other) {
            (EventResult::Handled, _) | (_, EventResult::Handled) => EventResult::Handled,
            _ => EventResult::Ignored,
        }
    }
}

pub struct EventHandler<State, V, F> {
    pub(crate) inner: V,
    pub(crate) kind: EventKind,
    pub(crate) handler: Rc<F>,
    pub(crate) _marker: std::marker::PhantomData<State>,
}

pub struct EventElement<VEl> {
    pub(crate) inner_element: VEl,
    pub(crate) is_hovered: bool,
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
            Event::MouseInput { pressed, .. } => {
                if element.is_hovered {
                    if *pressed {
                        if self.kind == EventKind::Press {
                            emitted_msg = (self.handler)(state);
                            handled = EventResult::Handled;
                        }
                    } else {
                        if self.kind == EventKind::Release || self.kind == EventKind::Click {
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

pub trait ViewEventExt<State>: View<State> + Sized {
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
