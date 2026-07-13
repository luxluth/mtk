use std::marker::PhantomData;

use crate::{
    Context,
    ui::{Event, Lens, View, event::EventResult},
};

/// Adapt maps a parent state `Outer` down to a child's state `Inner`,
/// and maps a child's message `V::Message` up to a parent's message `OuterMsg`.
pub struct Adapt<Outer, Inner, OuterMsg, L, M, V>
where
    L: Lens<Outer, Inner>,
    V: View<Inner>,
    M: Fn(V::Message) -> OuterMsg,
{
    lens: L,
    mapper: M,
    view: V,
    _marker: PhantomData<(Outer, Inner, OuterMsg)>,
}

pub fn adapt<Outer, Inner, OuterMsg, L, M, V>(
    view: V,
    lens: L,
    mapper: M,
) -> Adapt<Outer, Inner, OuterMsg, L, M, V>
where
    L: Lens<Outer, Inner>,
    V: View<Inner>,
    M: Fn(V::Message) -> OuterMsg,
{
    Adapt {
        lens,
        mapper,
        view,
        _marker: PhantomData,
    }
}

impl<Outer, Inner, OuterMsg, L, M, V> View<Outer> for Adapt<Outer, Inner, OuterMsg, L, M, V>
where
    L: Lens<Outer, Inner>,
    V: View<Inner>,
    M: Fn(V::Message) -> OuterMsg,
{
    type Element = V::Element;
    type Message = OuterMsg; // The wrapper emits the Parent's message type

    fn build(&self, ctx: &mut Context) -> Self::Element {
        self.view.build(ctx)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.view.rebuild(&prev.view, ctx, element)
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.view.teardown(ctx, element)
    }

    fn get_node(&self, element: &Self::Element) -> crate::Node {
        self.view.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &Outer,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        // 1 - Map the state down using the Lens
        let inner_state = self.lens.get(state);

        // 2 - Pass the event to the child view
        let (result, inner_msg) = self.view.handle_event(element, inner_state, event, ctx);

        // 3 - Map the message up using the closure (if the child emitted one)
        let outer_msg = inner_msg.map(&self.mapper);

        (result, outer_msg)
    }
}
