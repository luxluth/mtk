use crate::{
    Context,
    ui::{Event, View, event::EventResult},
};

pub struct Memoize<T, F> {
    pub(crate) data: T,
    pub(crate) builder: F,
}

/// Memoizes a view. The view will only be rebuilt if `data` changes.
/// `T` must implement `PartialEq` and `Clone`.
pub fn memoize<T, V, F>(data: T, builder: F) -> Memoize<T, F>
where
    T: PartialEq + Clone,
    V: View<()>, // Or your global State, depending on your needs
    F: Fn(&T) -> V,
{
    Memoize { data, builder }
}

impl<State, T, V, F> View<State> for Memoize<T, F>
where
    T: PartialEq + Clone,
    V: View<State>,
    F: Fn(&T) -> V,
{
    // The data we diff against (T)
    // The previously generated view (V)
    // The persistent element state (V::Element)
    type Element = (T, V, V::Element);
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let view = (self.builder)(&self.data);
        let element = view.build(ctx);
        (self.data.clone(), view, element)
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.data != element.0 {
            let new_view = (self.builder)(&self.data);
            new_view.rebuild(&element.1, ctx, &mut element.2);
            element.0 = self.data.clone();
            element.1 = new_view;
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        element.1.teardown(ctx, &mut element.2);
    }

    fn get_node(&self, element: &Self::Element) -> crate::Node {
        element.1.get_node(&element.2)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        // routing the event to the cached view
        element.1.handle_event(&mut element.2, state, event, ctx)
    }
}
