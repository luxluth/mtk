use crate::accessibility::AccessibleInfo;
use crate::ui::View;
use crate::{Context, Node};
use accesskit::Role;

/// Declarative extension trait for attaching accessibility metadata to any view.
pub trait AccessibleViewExt<State>: View<State> + Sized {
    /// Attaches accessibility metadata to this view's underlying layout node.
    fn accessible(self, info: AccessibleInfo) -> AccessibleView<State, Self> {
        AccessibleView {
            inner: self,
            info,
            _marker: std::marker::PhantomData,
        }
    }

    /// Shortcut to set the accessible role for this view.
    fn accessible_role(self, role: Role) -> AccessibleView<State, Self> {
        self.accessible(AccessibleInfo::new(role))
    }

    /// Shortcut to set the accessible label for this view.
    fn accessible_label(self, label: impl Into<String>) -> AccessibleView<State, Self> {
        self.accessible(AccessibleInfo::new(Role::GenericContainer).with_label(label))
    }

    /// Shortcut to set the accessible description for this view.
    fn accessible_description(self, description: impl Into<String>) -> AccessibleView<State, Self> {
        self.accessible(AccessibleInfo::new(Role::GenericContainer).with_description(description))
    }
}

impl<State, V: View<State>> AccessibleViewExt<State> for V {}

/// A wrapper view that attaches semantic accessibility metadata to its child view.
pub struct AccessibleView<State, V> {
    inner: V,
    info: AccessibleInfo,
    _marker: std::marker::PhantomData<State>,
}

impl<State, V> AccessibleView<State, V> {
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.info = self.info.with_label(label);
        self
    }

    pub fn accessible_description(mut self, description: impl Into<String>) -> Self {
        self.info = self.info.with_description(description);
        self
    }

    pub fn accessible_role(mut self, role: Role) -> Self {
        self.info.role = role;
        self
    }
}

impl<State, V: View<State>> View<State> for AccessibleView<State, V> {
    type Element = V::Element;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let element = self.inner.build(ctx);
        let node = self.inner.get_node(&element);
        node.set_accessible(ctx, self.info.clone());
        element
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.rebuild(&prev.inner, ctx, element);
        let node = self.inner.get_node(element);
        if self.info != prev.info {
            node.set_accessible(ctx, self.info.clone());
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        let node = self.inner.get_node(element);
        node.remove_accessible(ctx);
        self.inner.teardown(ctx, element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(element)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: crate::ui::Event,
        ctx: &mut Context,
    ) -> (crate::ui::event::EventResult, Option<Self::Message>) {
        self.inner.handle_event(element, state, event, ctx)
    }
}
