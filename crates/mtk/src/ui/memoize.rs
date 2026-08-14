//! View memoization utilities for caching layout subtrees based on value equality.
//!
//! Memoization prevents unnecessary view rebuilds and layout tree diffing when input parameters
//! (`data`) remain unchanged between render passes.

use crate::{
    Context,
    ui::{Event, View, event::EventResult},
};

/// A memoized view wrapper that caches sub-view layout trees based on key equality (`T: PartialEq`).
pub struct Memoize<T, F> {
    pub(crate) data: T,
    pub(crate) builder: F,
}

/// Memoizes a view construction function.
///
/// The returned [`Memoize`] component will only rebuild its inner view if `data` compares unequal (`!=`)
/// to the key stored during the previous frame.
///
/// # Requirements
/// - `T` must implement [`PartialEq`] and [`Clone`].
///
/// # Examples
/// ```rust,ignore
/// use mtk::ui::memoize::memoize;
/// use mtk::ui::widgets::text;
///
/// let name = "Alice".to_string();
/// let view = memoize(name, |data| text(format!("Hello, {data}")));
/// ```
pub fn memoize<T, V, F>(data: T, builder: F) -> Memoize<T, F>
where
    T: PartialEq + Clone,
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
    /// Element state tuple holding:
    /// 1 - The cached key `T` diffed against new frames.
    /// 2 - The constructed view `V`.
    /// 3 - The persistent element node state `V::Element`.
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
