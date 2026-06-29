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

impl<State, V: View<State>, F: Fn(&mut State) + 'static> View<State> for EventHandler<State, V, F> {
    type Element = EventElement<V::Element>;

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

    fn message(&self, element: &mut Self::Element, state: &mut State, event: Event) {
        match &event {
            Event::CursorMoved { hit_nodes, .. } => {
                let node = self.get_node(element);
                let newly_hovered = hit_nodes.contains(&node);

                if newly_hovered != element.is_hovered {
                    element.is_hovered = newly_hovered;
                    if newly_hovered && self.kind == EventKind::HoverIn {
                        (self.handler)(state);
                    } else if !newly_hovered && self.kind == EventKind::HoverOut {
                        (self.handler)(state);
                    }
                }
            }
            Event::MouseInput { pressed } => {
                if element.is_hovered {
                    if *pressed {
                        if self.kind == EventKind::Press {
                            (self.handler)(state);
                        }
                    } else {
                        if self.kind == EventKind::Release {
                            (self.handler)(state);
                        }
                        if self.kind == EventKind::Click {
                            (self.handler)(state);
                        }
                    }
                }
            }
            _ => {}
        }
        self.inner.message(&mut element.inner_element, state, event);
    }
}

pub trait ViewEventExt<State>: View<State> + Sized {
    fn on_event<F: Fn(&mut State) + 'static>(
        self,
        event: EventKind,
        handler: F,
    ) -> EventHandler<State, Self, F>;
}

impl<State, V: View<State>> ViewEventExt<State> for V {
    fn on_event<F: Fn(&mut State) + 'static>(
        self,
        event: EventKind,
        handler: F,
    ) -> EventHandler<State, Self, F> {
        EventHandler {
            inner: self,
            kind: event,
            handler: Rc::new(handler),
            _marker: std::marker::PhantomData,
        }
    }
}
