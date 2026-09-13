//! Type-erased [`View`] wrapper for dynamic and heterogeneous view composition.
//!
//! Provides [`BoxedView`] and [`BoxedViewExt`], enabling different concrete view types
//! to be unified under a single type (e.g. in `match` branches, routers, or conditional views)
//! without writing custom sum-type macros.

use std::any::{Any, TypeId};
use std::fmt;

use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// Type-erased DOM element maintained between render frames by a [`BoxedView`].
pub struct BoxedElement {
    node: Node,
    inner: Box<dyn Any>,
    type_id: TypeId,
}

impl fmt::Debug for BoxedElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxedElement")
            .field("node", &self.node)
            .field("type_id", &self.type_id)
            .finish()
    }
}

/// Internal object-safe trait implemented for all concrete [`View`] types.
pub trait ErasedView<State, Msg> {
    fn build_erased(&self, ctx: &mut Context) -> BoxedElement;

    fn rebuild_erased(
        &self,
        prev: &dyn ErasedView<State, Msg>,
        ctx: &mut Context,
        element: &mut BoxedElement,
    );

    fn rebuild_with_parent_erased(
        &self,
        prev: &dyn ErasedView<State, Msg>,
        ctx: &mut Context,
        element: &mut BoxedElement,
        parent: Node,
        next_sibling: Option<Node>,
    );

    fn teardown_erased(&self, ctx: &mut Context, element: &mut BoxedElement);

    fn handle_event_erased(
        &self,
        element: &mut BoxedElement,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Msg>);

    fn as_any(&self) -> &dyn Any;

    fn view_type_id(&self) -> TypeId;
}

impl<V, State, Msg> ErasedView<State, Msg> for V
where
    V: View<State, Message = Msg> + 'static,
    V::Element: 'static,
    State: 'static,
    Msg: 'static,
{
    fn build_erased(&self, ctx: &mut Context) -> BoxedElement {
        let el = self.build(ctx);
        let node = self.get_node(&el);
        BoxedElement {
            node,
            inner: Box::new(el),
            type_id: TypeId::of::<V>(),
        }
    }

    fn rebuild_erased(
        &self,
        prev: &dyn ErasedView<State, Msg>,
        ctx: &mut Context,
        element: &mut BoxedElement,
    ) {
        if element.type_id == TypeId::of::<V>() && prev.view_type_id() == TypeId::of::<V>() {
            if let Some(prev_concrete) = prev.as_any().downcast_ref::<V>() {
                if let Some(concrete_el) = element.inner.downcast_mut::<V::Element>() {
                    self.rebuild(prev_concrete, ctx, concrete_el);
                    element.node = self.get_node(concrete_el);
                    return;
                }
            }
        }

        // View type changed: tear down previous element and construct new one
        prev.teardown_erased(ctx, element);
        *element = self.build_erased(ctx);
    }

    fn rebuild_with_parent_erased(
        &self,
        prev: &dyn ErasedView<State, Msg>,
        ctx: &mut Context,
        element: &mut BoxedElement,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        if element.type_id == TypeId::of::<V>() && prev.view_type_id() == TypeId::of::<V>() {
            if let Some(prev_concrete) = prev.as_any().downcast_ref::<V>() {
                if let Some(concrete_el) = element.inner.downcast_mut::<V::Element>() {
                    self.rebuild_with_parent(prev_concrete, ctx, concrete_el, parent, next_sibling);
                    element.node = self.get_node(concrete_el);
                    return;
                }
            }
        }

        // View type changed: detach old layout node, tear down, and attach new node
        prev.teardown_erased(ctx, element);
        let old_node = element.node;
        old_node.remove(ctx);

        *element = self.build_erased(ctx);
        if let Some(sibling) = next_sibling {
            element.node.put_before(ctx, sibling);
        } else {
            parent.append(ctx, element.node);
        }
    }

    fn teardown_erased(&self, ctx: &mut Context, element: &mut BoxedElement) {
        if let Some(concrete_el) = element.inner.downcast_mut::<V::Element>() {
            self.teardown(ctx, concrete_el);
        }
    }

    fn handle_event_erased(
        &self,
        element: &mut BoxedElement,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Msg>) {
        if let Some(concrete_el) = element.inner.downcast_mut::<V::Element>() {
            self.handle_event(concrete_el, state, event, ctx)
        } else {
            (EventResult::Ignored, None)
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn view_type_id(&self) -> TypeId {
        TypeId::of::<V>()
    }
}

/// A heap-allocated, type-erased [`View`] that allows heterogeneous views to share a single type.
///
/// # Examples
///
/// ```rust,ignore
/// fn render_page(state: &AppState) -> BoxedView<AppState, AppMsg> {
///     match state.current_page {
///         Page::Home => home_view(state).boxed(),
///         Page::Settings => settings_view(state).boxed(),
///         Page::Profile => profile_view(state).boxed(),
///     }
/// }
/// ```
pub struct BoxedView<State, Msg = ()> {
    inner: Box<dyn ErasedView<State, Msg>>,
}

impl<State, Msg> BoxedView<State, Msg> {
    /// Creates a new `BoxedView` wrapping `view`.
    pub fn new<V>(view: V) -> Self
    where
        V: View<State, Message = Msg> + 'static,
        V::Element: 'static,
        State: 'static,
        Msg: 'static,
    {
        Self {
            inner: Box::new(view),
        }
    }
}

/// Creates a new [`BoxedView`] wrapping the given `view`.
pub fn boxed<State, Msg, V>(view: V) -> BoxedView<State, Msg>
where
    V: View<State, Message = Msg> + 'static,
    V::Element: 'static,
    State: 'static,
    Msg: 'static,
{
    BoxedView::new(view)
}

/// Extension trait enabling fluid `.boxed()` method chaining on any [`View`].
pub trait BoxedViewExt<State>: View<State> {
    /// Erases the concrete type of this view into a [`BoxedView`].
    fn boxed(self) -> BoxedView<State, Self::Message>
    where
        Self: Sized + 'static,
        Self::Element: 'static,
        Self::Message: 'static,
        State: 'static,
    {
        BoxedView::new(self)
    }
}

impl<State, V: View<State>> BoxedViewExt<State> for V {}

impl<State: 'static, Msg: 'static> View<State> for BoxedView<State, Msg> {
    type Element = BoxedElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        self.inner.build_erased(ctx)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild_erased(prev.inner.as_ref(), ctx, element);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.inner.rebuild_with_parent_erased(
            prev.inner.as_ref(),
            ctx,
            element,
            parent,
            next_sibling,
        );
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown_erased(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        self.inner.handle_event_erased(element, state, event, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Size, Style};
    use crate::ui::ViewStyleExt;
    use crate::ui::router::router;
    use crate::ui::transition::PageTransition;
    use crate::ui::widgets::{button, column, text};

    #[test]
    fn test_boxed_view_build_and_rebuild_same_type() {
        let mut ctx = Context::new();

        let v1 = text::<_, ()>("Hello").boxed();
        let v2 = text::<_, ()>("World").boxed();

        let mut el = View::<()>::build(&v1, &mut ctx);
        let node1 = el.node;
        assert!(node1.is_valid());

        // Rebuild with same concrete view type: node must be preserved
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        assert_eq!(el.node, node1);
    }

    #[test]
    fn test_boxed_view_switch_type_in_parent() {
        let mut ctx = Context::new();
        let parent = ctx.create_node();
        ctx.root_attach(parent);

        let v_text = text::<_, ()>("Initial").boxed();
        let v_button = button("Click Me").boxed();

        let mut el = View::<()>::build(&v_text, &mut ctx);
        parent.append(&mut ctx, el.node);
        let old_node = el.node;

        // Rebuild with a completely different view type
        View::<()>::rebuild_with_parent(&v_button, &v_text, &mut ctx, &mut el, parent, None);

        // A new node was created and attached to parent
        assert_ne!(el.node, old_node);
        assert!(el.node.is_valid());
    }

    #[test]
    fn test_boxed_view_in_router() {
        let mut ctx = Context::new();

        enum Page {
            First,
            Second,
        }

        fn render_page(p: &Page) -> BoxedView<(), ()> {
            match p {
                Page::First => column((text::<_, ()>("Page 1"),)).boxed(),
                Page::Second => column((text::<_, ()>("Page 2"), button("Action")))
                    .style(Style::new().width(Size::Fill))
                    .boxed(),
            }
        }

        let r1 = router(1, render_page(&Page::First)).transition(PageTransition::fade());
        let r2 = router(2, render_page(&Page::Second)).transition(PageTransition::fade());

        let mut el = View::<()>::build(&r1, &mut ctx);
        let container = View::<()>::get_node(&r1, &el);
        ctx.root_attach(container);
        ctx.compute_layout(800.0, 600.0);

        View::<()>::rebuild(&r2, &r1, &mut ctx, &mut el);
        assert!(el.outgoing.is_some());

        // Fast-forward animation to completion
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&r2, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);
        assert!(el.outgoing.is_none());
    }
}
