//! Shared element morph animation primitives for smooth transitions across router pages.
//!
//! Provides [`morphable`], [`MorphId`], [`MorphTransition`], and [`MorphViewExt`].
//! These allow matching elements across different views or pages to smoothly animate
//! their position, size, and visual effects (such as corner radius and box shadows)
//! using spring physics and organic cross-fading.

use smol_str::SmolStr;
use std::marker::PhantomData;

use crate::animation::{Curve, Spring};
use crate::debugger::SourceLocation;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A unique identifier for a morphable element shared across pages.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MorphId(pub SmolStr);

impl MorphId {
    /// Creates a new [`MorphId`].
    pub fn new(id: impl Into<SmolStr>) -> Self {
        Self(id.into())
    }
}

impl From<&'static str> for MorphId {
    fn from(s: &'static str) -> Self {
        Self(SmolStr::new_static(s))
    }
}

impl From<String> for MorphId {
    fn from(s: String) -> Self {
        Self(SmolStr::new(s))
    }
}

impl From<SmolStr> for MorphId {
    fn from(s: SmolStr) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for MorphId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Configuration settings for a shared element morph animation.
#[derive(Clone, Debug, PartialEq)]
pub struct MorphTransition {
    /// Trajectory curve (analytical spring or easing curve) guiding the flight.
    pub curve: Curve,
    /// Normalized progress `[0.0, 1.0]` where content cross-fading begins.
    pub cross_fade_start: f32,
    /// Normalized progress `[0.0, 1.0]` where content cross-fading completes.
    pub cross_fade_end: f32,
    /// Whether visual effects (corner radius, box shadows, borders, bg color) are interpolated.
    pub morph_effects: bool,
}

impl Default for MorphTransition {
    fn default() -> Self {
        Self {
            curve: Curve::spring(Spring::gentle()),
            cross_fade_start: 0.35,
            cross_fade_end: 0.65,
            morph_effects: true,
        }
    }
}

impl MorphTransition {
    /// Creates a new `MorphTransition` with default gentle spring physics and organic cross-fading.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the trajectory with an analytical spring physics solver.
    pub fn spring(mut self, spring: Spring) -> Self {
        self.curve = Curve::spring(spring);
        self
    }

    /// Configures the trajectory with an explicit [`Curve`].
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Sets the normalized start and end progress window for the content cross-fade.
    pub fn cross_fade_window(mut self, start: f32, end: f32) -> Self {
        self.cross_fade_start = start.clamp(0.0, 1.0);
        self.cross_fade_end = end.clamp(self.cross_fade_start, 1.0);
        self
    }

    /// Controls whether visual effects (borders, corner radius, box shadows) morph during flight.
    pub fn morph_effects(mut self, morph: bool) -> Self {
        self.morph_effects = morph;
        self
    }
}

/// Metadata stored in [`Context`] for a registered morphable element.
#[derive(Clone, Debug)]
pub struct MorphInfo {
    pub node: Node,
    pub id: MorphId,
    pub transition: MorphTransition,
}

/// A matched pair of morphable elements identified between outgoing and incoming router views.
#[derive(Clone, Debug)]
pub struct MorphPair {
    pub id: MorphId,
    pub source: MorphInfo,
    pub target: MorphInfo,
}

/// A view wrapper that marks an element as eligible for shared element morphing.
pub struct Morphable<Id, V> {
    pub(crate) id: Id,
    pub(crate) view: V,
    pub(crate) transition: MorphTransition,
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Wraps a view to mark it as a shared morphable element across page transitions.
///
/// # Examples
/// ```rust,ignore
/// morphable("avatar", image("profile.png"))
///     .spring(Spring::bouncy())
/// ```
#[track_caller]
pub fn morphable<Id: Into<MorphId>, V>(id: Id, view: V) -> Morphable<MorphId, V> {
    Morphable {
        id: id.into(),
        view,
        transition: MorphTransition::default(),
        source_loc: Some(SourceLocation::here("Morphable")),
    }
}

impl<Id, V> Morphable<Id, V> {
    /// Configures the morph trajectory with an analytical spring.
    pub fn spring(mut self, spring: Spring) -> Self {
        self.transition.curve = Curve::spring(spring);
        self
    }

    /// Configures the morph trajectory with an explicit curve.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.transition.curve = curve;
        self
    }

    /// Configures the organic cross-fade window.
    pub fn cross_fade_window(mut self, start: f32, end: f32) -> Self {
        self.transition = self.transition.cross_fade_window(start, end);
        self
    }

    /// Overwrites the full transition configuration.
    pub fn transition(mut self, transition: MorphTransition) -> Self {
        self.transition = transition;
        self
    }
}

/// Persistent element state for a [`Morphable`] widget.
pub struct MorphElement<V: View<State>, State> {
    pub(crate) node: Node,
    pub(crate) inner_el: V::Element,
    pub(crate) id: MorphId,
    _marker: PhantomData<State>,
}

impl<State, V: View<State>> View<State> for Morphable<MorphId, V> {
    type Element = MorphElement<V, State>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let inner_el = self.view.build(ctx);
        let node = self.view.get_node(&inner_el);
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(node, loc);
        }

        ctx.register_morph_node(node, self.id.clone(), self.transition.clone());

        MorphElement {
            node,
            inner_el,
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.view.rebuild(&prev.view, ctx, &mut element.inner_el);
        let updated_node = self.view.get_node(&element.inner_el);

        if self.id != element.id || updated_node != element.node {
            ctx.unregister_morph_node(element.node);
            element.id = self.id.clone();
            element.node = updated_node;
            ctx.register_morph_node(element.node, self.id.clone(), self.transition.clone());
        }
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.view
            .rebuild_with_parent(&prev.view, ctx, &mut element.inner_el, parent, next_sibling);
        let updated_node = self.view.get_node(&element.inner_el);

        if self.id != element.id || updated_node != element.node {
            ctx.unregister_morph_node(element.node);
            element.id = self.id.clone();
            element.node = updated_node;
            ctx.register_morph_node(element.node, self.id.clone(), self.transition.clone());
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.unregister_morph_node(element.node);
        self.view.teardown(ctx, &mut element.inner_el);
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
        self.view
            .handle_event(&mut element.inner_el, state, event, ctx)
    }
}

/// Extension trait enabling `.morph(id)` method chaining on any view.
pub trait MorphViewExt: Sized {
    /// Marks this view as a shared morphable element across page transitions.
    fn morph<Id: Into<MorphId>>(self, id: Id) -> Morphable<MorphId, Self> {
        morphable(id, self)
    }
}

impl<V> MorphViewExt for V {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Spring;
    use crate::ui::widgets::text;

    #[test]
    fn test_morph_id_and_transition_builder() {
        let id1: MorphId = "hero-image".into();
        let id2: MorphId = String::from("hero-image").into();
        let id3: MorphId = SmolStr::new("hero-image").into();
        assert_eq!(id1, id2);
        assert_eq!(id2, id3);

        let trans = MorphTransition::default()
            .spring(Spring::bouncy())
            .cross_fade_window(0.2, 0.8)
            .morph_effects(false);

        assert_eq!(trans.cross_fade_start, 0.2);
        assert_eq!(trans.cross_fade_end, 0.8);
        assert!(!trans.morph_effects);
    }

    #[test]
    fn test_morphable_registration_and_pair_discovery() {
        let mut ctx = Context::new();

        let v1 = text::<_, ()>("Card").morph("shared-card");
        let v2 = text::<_, ()>("Other").morph("unique-1");
        let v3 = text::<_, ()>("Card Expanded").morph("shared-card");
        let v4 = text::<_, ()>("Another").morph("unique-2");

        let mut el1 = View::<()>::build(&v1, &mut ctx);
        let mut el2 = View::<()>::build(&v2, &mut ctx);
        let mut el3 = View::<()>::build(&v3, &mut ctx);
        let mut el4 = View::<()>::build(&v4, &mut ctx);

        let root_a = ctx.create_node();
        root_a.append(&mut ctx, el1.node);
        root_a.append(&mut ctx, el2.node);

        let root_b = ctx.create_node();
        root_b.append(&mut ctx, el3.node);
        root_b.append(&mut ctx, el4.node);

        let pairs = ctx.find_morph_pairs(root_a, root_b);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].id, MorphId::new("shared-card"));
        assert_eq!(pairs[0].source.node, el1.node);
        assert_eq!(pairs[0].target.node, el3.node);

        View::<()>::teardown(&v1, &mut ctx, &mut el1);
        View::<()>::teardown(&v2, &mut ctx, &mut el2);
        View::<()>::teardown(&v3, &mut ctx, &mut el3);
        View::<()>::teardown(&v4, &mut ctx, &mut el4);

        let pairs_after = ctx.find_morph_pairs(root_a, root_b);
        assert_eq!(pairs_after.len(), 0);
    }

    #[test]
    fn test_morphable_zero_overhead_and_constraint_preservation() {
        use crate::ui::style::ViewStyleExt;
        use crate::{Size, Style};

        let mut ctx = Context::new();

        let styled_box = text::<_, ()>("Album Cover")
            .style(
                Style::new()
                    .width(Size::Fill)
                    .aspect_ratio(1.0)
                    .bg_color(crate::rgb!(255, 0, 0)),
            )
            .morph("album-cover");

        let el = View::<()>::build(&styled_box, &mut ctx);
        let node = View::<()>::get_node(&styled_box, &el);

        // Morphable must return the inner node directly with its constraints and effects preserved.
        let cons = node.get_constraints(&ctx).unwrap();
        assert_eq!(cons.width, Size::Fill);
        assert_eq!(cons.aspect_ratio, 1.0);

        let eff = ctx.effects.get(&node).unwrap();
        assert_eq!(eff.background_color, crate::rgb!(255, 0, 0));
    }
}
