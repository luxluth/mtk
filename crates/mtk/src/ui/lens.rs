use crate::ui::{Context, Event, View};

/// A Lens allows focusing on a specific part (`Inner`) of a larger state (`Outer`).
pub trait Lens<Outer, Inner> {
    fn get<'a>(&self, outer: &'a Outer) -> &'a Inner;
    fn get_mut<'a>(&self, outer: &'a mut Outer) -> &'a mut Inner;
}

/// A wrapper View that maps a parent state `Outer` down to a child view's state `Inner`.
pub struct LensWrap<Outer, Inner, L: Lens<Outer, Inner>, V: View<Inner>> {
    lens: L,
    view: V,
    _marker: std::marker::PhantomData<(Outer, Inner)>,
}

impl<Outer, Inner, L: Lens<Outer, Inner>, V: View<Inner>> LensWrap<Outer, Inner, L, V> {
    pub fn new(view: V, lens: L) -> Self {
        Self {
            lens,
            view,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<Outer, Inner, L, V> View<Outer> for LensWrap<Outer, Inner, L, V>
where
    L: Lens<Outer, Inner>,
    V: View<Inner>,
{
    type Element = V::Element;

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

    fn message(&self, element: &mut Self::Element, state: &mut Outer, event: Event, ctx: &mut Context) {
        self.view.message(element, self.lens.get_mut(state), event, ctx)
    }
}
