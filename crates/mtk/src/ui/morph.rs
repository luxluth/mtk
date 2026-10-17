//! Shared element morph animation primitives for smooth transitions across router pages.
//!
//! Provides [`morphable`], [`MorphId`], [`MorphTransition`], and [`MorphViewExt`].
//! These allow matching elements across different views or pages to smoothly animate
//! their position, size, and visual effects (such as corner radius and box shadows)
//! using spring physics and organic cross-fading.

use smol_str::SmolStr;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::Instant;

use crate::animation::{Animatable, AnimatedValue, Curve, Spring};
use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::effects::{BoxShadow, Effects};
use crate::style::{AlignItems, JustifyContent, Overflow, PositionStrategy, Rect, Size};
use crate::ui::event::EventResult;
use crate::ui::{Event, View, ViewSequence};
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
    /// Duration of the morph transition in milliseconds.
    pub duration_ms: f32,
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
            duration_ms: 350.0,
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

    /// Sets transition duration in milliseconds.
    pub fn duration_ms(mut self, duration_ms: f32) -> Self {
        self.duration_ms = duration_ms;
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

    /// Sets transition duration in milliseconds.
    pub fn duration_ms(mut self, duration_ms: f32) -> Self {
        self.transition = self.transition.duration_ms(duration_ms);
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

/// Tracks active state for an ongoing shared element morph flight.
#[derive(Debug)]
pub struct ActiveMorphFlight {
    #[allow(dead_code)]
    pub id: MorphId,
    pub source_node: Node,
    pub target_node: Node,
    pub source_placeholder: Option<Node>,
    pub target_placeholder: Option<Node>,
    pub transition: MorphTransition,
    pub source_rect: Option<Rect>,
    pub target_rect: Option<Rect>,
    pub source_effects: Option<Effects>,
    pub target_effects: Option<Effects>,
    pub shuttle_node: Option<Node>,
    pub shuttle_source_box: Option<Node>,
    pub shuttle_target_box: Option<Node>,
    pub anim_progress: AnimatedValue<f32>,
    pub anim_start: Instant,
}

pub fn apply_morph_step(
    ctx: &mut Context,
    container: Node,
    morphs: &mut [ActiveMorphFlight],
    external_progress: Option<f32>,
) {
    if morphs.is_empty() {
        return;
    }

    let container_bounds = container.get_computed(ctx).unwrap_or_default();

    for flight in morphs.iter_mut() {
        if flight.shuttle_node.is_none() {
            let s_comp = flight.source_node.get_computed(ctx);
            let t_comp = flight.target_node.get_computed(ctx);

            if let (Some(s), Some(t)) = (s_comp, t_comp) {
                if s.w > 0.0 && s.h > 0.0 && t.w > 0.0 && t.h > 0.0 {
                    let s_rel_x = s.x - container_bounds.x;
                    let s_rel_y = s.y - container_bounds.y;
                    let t_rel_x = t.x - container_bounds.x;
                    let t_rel_y = t.y - container_bounds.y;

                    let s_rect = Rect::new(s_rel_x, s_rel_y, s.w, s.h);
                    let t_rect = Rect::new(t_rel_x, t_rel_y, t.w, t.h);
                    flight.source_rect = Some(s_rect);
                    flight.target_rect = Some(t_rect);

                    let s_eff = flight.source_node.get_effects(ctx).unwrap_or_default();
                    let t_eff = flight.target_node.get_effects(ctx).unwrap_or_default();
                    flight.source_effects = Some(s_eff.clone());
                    flight.target_effects = Some(t_eff.clone());

                    let s_ph = ctx.create_node();
                    s_ph.update_constraints(ctx, |c| {
                        c.width = Size::Fixed(s.w.round().max(1.0) as u32);
                        c.height = Size::Fixed(s.h.round().max(1.0) as u32);
                        c.flex_grow = 0.0;
                        c.flex_shrink = 0.0;
                    });
                    if !s_ph.put_before(ctx, flight.source_node) {
                        if let Some(parent) = flight.source_node.parent(ctx) {
                            parent.append(ctx, s_ph);
                        }
                    }
                    flight.source_placeholder = Some(s_ph);

                    let t_ph = ctx.create_node();
                    t_ph.update_constraints(ctx, |c| {
                        c.width = Size::Fixed(t.w.round().max(1.0) as u32);
                        c.height = Size::Fixed(t.h.round().max(1.0) as u32);
                        c.flex_grow = 0.0;
                        c.flex_shrink = 0.0;
                    });
                    if !t_ph.put_before(ctx, flight.target_node) {
                        if let Some(parent) = flight.target_node.parent(ctx) {
                            parent.append(ctx, t_ph);
                        }
                    }
                    flight.target_placeholder = Some(t_ph);

                    if flight.transition.morph_effects {
                        ctx.morph_suppressed_nodes.insert(flight.source_node);
                        ctx.morph_suppressed_nodes.insert(flight.target_node);

                        let mut clear_s = s_eff.clone();
                        clear_s.background_color = Color::transparent;
                        clear_s.box_shadow = BoxShadow::default();
                        clear_s.additional_shadows.clear();
                        clear_s.border.color = Color::transparent;
                        flight.source_node.set_effects(ctx, clear_s);

                        let mut clear_t = t_eff.clone();
                        clear_t.background_color = Color::transparent;
                        clear_t.box_shadow = BoxShadow::default();
                        clear_t.additional_shadows.clear();
                        clear_t.border.color = Color::transparent;
                        flight.target_node.set_effects(ctx, clear_t);
                    }

                    let shuttle = ctx.create_node();
                    shuttle.update_constraints(ctx, |c| {
                        c.positioning = PositionStrategy::Absolute {
                            left: s_rect.x,
                            top: s_rect.y,
                            right: f32::NAN,
                            bottom: f32::NAN,
                        };
                        c.width = Size::Fixed(s_rect.w.round().max(1.0) as u32);
                        c.height = Size::Fixed(s_rect.h.round().max(1.0) as u32);
                        c.overflow = Overflow::Visible;
                        c.z_index = 100;
                    });
                    shuttle.set_effects(ctx, s_eff);
                    container.append(ctx, shuttle);

                    let source_box = ctx.create_node();
                    source_box.update_constraints(ctx, |c| {
                        c.positioning = PositionStrategy::Absolute {
                            left: 0.0,
                            top: 0.0,
                            right: 0.0,
                            bottom: 0.0,
                        };
                        c.width = Size::Percent(1.0);
                        c.height = Size::Percent(1.0);
                        c.justify_content = JustifyContent::Center;
                        c.align_items = AlignItems::Center;
                    });
                    source_box.update_effects(ctx, |e| e.opacity = 1.0);

                    let target_box = ctx.create_node();
                    target_box.update_constraints(ctx, |c| {
                        c.positioning = PositionStrategy::Absolute {
                            left: 0.0,
                            top: 0.0,
                            right: 0.0,
                            bottom: 0.0,
                        };
                        c.width = Size::Percent(1.0);
                        c.height = Size::Percent(1.0);
                        c.justify_content = JustifyContent::Center;
                        c.align_items = AlignItems::Center;
                    });
                    target_box.update_effects(ctx, |e| e.opacity = 0.0);

                    shuttle.append(ctx, source_box);
                    shuttle.append(ctx, target_box);

                    flight.source_node.remove(ctx);
                    source_box.append(ctx, flight.source_node);

                    flight.target_node.remove(ctx);
                    target_box.append(ctx, flight.target_node);

                    flight.shuttle_node = Some(shuttle);
                    flight.shuttle_source_box = Some(source_box);
                    flight.shuttle_target_box = Some(target_box);
                }
            }
        }

        if let (Some(shuttle), Some(s_rect), Some(t_rect)) =
            (flight.shuttle_node, flight.source_rect, flight.target_rect)
        {
            let progress = external_progress.unwrap_or_else(|| {
                let elapsed = flight.anim_start.elapsed().as_secs_f64() * 1000.0;
                flight.anim_progress.tick(elapsed);
                flight.anim_progress.get()
            });

            let p = flight.transition.curve.eval(progress as f64) as f32;

            let curr_x = if progress >= 0.999 {
                t_rect.x
            } else {
                s_rect.x + (t_rect.x - s_rect.x) * p
            };
            let curr_y = if progress >= 0.999 {
                t_rect.y
            } else {
                s_rect.y + (t_rect.y - s_rect.y) * p
            };
            let curr_w = if progress >= 0.999 {
                t_rect.w
            } else {
                (s_rect.w + (t_rect.w - s_rect.w) * p).max(1.0)
            };
            let curr_h = if progress >= 0.999 {
                t_rect.h
            } else {
                (s_rect.h + (t_rect.h - s_rect.h) * p).max(1.0)
            };

            shuttle.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    left: curr_x,
                    top: curr_y,
                    right: f32::NAN,
                    bottom: f32::NAN,
                };
                c.width = Size::Fixed(curr_w.round().max(1.0) as u32);
                c.height = Size::Fixed(curr_h.round().max(1.0) as u32);
            });

            if flight.transition.morph_effects {
                if let (Some(s_eff), Some(t_eff)) = (&flight.source_effects, &flight.target_effects)
                {
                    let mut interpolated = Effects::interpolate(s_eff, t_eff, p as f64);
                    interpolated.opacity = 1.0;
                    shuttle.set_effects(ctx, interpolated);
                }
            }

            let fade_start = flight.transition.cross_fade_start;
            let fade_end = flight.transition.cross_fade_end;
            let (source_op, target_op) = if p <= fade_start {
                (1.0, 0.0)
            } else if p >= fade_end {
                (0.0, 1.0)
            } else {
                let t = (p - fade_start) / (fade_end - fade_start);
                let smooth_t = t * t * (3.0 - 2.0 * t);
                (1.0 - smooth_t, smooth_t)
            };

            if let Some(source_box) = flight.shuttle_source_box {
                source_box.update_effects(ctx, |e| e.opacity = source_op.clamp(0.0, 1.0));
            }
            if let Some(target_box) = flight.shuttle_target_box {
                target_box.update_effects(ctx, |e| e.opacity = target_op.clamp(0.0, 1.0));
            }
        }
    }
}

pub fn settle_morphs(ctx: &mut Context, morphs: &mut Vec<ActiveMorphFlight>) {
    let had_morphs = !morphs.is_empty();
    for flight in morphs.drain(..) {
        ctx.morph_suppressed_nodes.remove(&flight.source_node);
        ctx.morph_suppressed_nodes.remove(&flight.target_node);

        if let Some(t_ph) = flight.target_placeholder {
            if !flight.target_node.put_before(ctx, t_ph) {
                if let Some(parent) = t_ph.parent(ctx) {
                    parent.append(ctx, flight.target_node);
                }
            }
            t_ph.remove(ctx);
            ctx.destroy_node(t_ph);
        }
        if let Some(s_ph) = flight.source_placeholder {
            s_ph.remove(ctx);
            ctx.destroy_node(s_ph);
        }
        if let Some(shuttle) = flight.shuttle_node {
            shuttle.remove(ctx);
            ctx.destroy_node(shuttle);
        }
        if let Some(source_eff) = flight.source_effects {
            flight.source_node.set_effects(ctx, source_eff);
        }
        if let Some(target_eff) = flight.target_effects {
            flight.target_node.set_effects(ctx, target_eff);
        }
    }
    if had_morphs {
        ctx.request_frame();
    }
}

pub fn settle_finished_morphs(ctx: &mut Context, morphs: &mut Vec<ActiveMorphFlight>) -> bool {
    let mut any_animating = false;
    let mut i = 0;
    while i < morphs.len() {
        let is_done = if morphs[i].shuttle_node.is_some() {
            let elapsed = morphs[i].anim_start.elapsed().as_secs_f64() * 1000.0;
            morphs[i].anim_progress.tick(elapsed);
            !morphs[i].anim_progress.is_animating() || morphs[i].anim_progress.get() >= 0.999
        } else {
            false
        };

        if is_done {
            let mut finished = vec![morphs.remove(i)];
            settle_morphs(ctx, &mut finished);
        } else {
            any_animating = true;
            i += 1;
        }
    }
    any_animating
}

/// A container scope coordinating shared element morph transitions across child views.
pub struct MorphScope<V> {
    pub(crate) view: V,
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Creates a new [`MorphScope`] wrapping `view` to coordinate shared element morph flights.
#[track_caller]
pub fn morph_scope<V>(view: V) -> MorphScope<V> {
    MorphScope {
        view,
        source_loc: Some(SourceLocation::here("MorphScope")),
    }
}

/// Extension trait enabling `.morph_scope()` method chaining on any view.
pub trait MorphScopeExt: Sized {
    /// Wraps this view in a [`MorphScope`] to coordinate shared element transitions.
    fn morph_scope(self) -> MorphScope<Self> {
        morph_scope(self)
    }
}

impl<V> MorphScopeExt for V {}

/// Persistent element state for [`MorphScope`].
pub struct MorphScopeElement<V: View<State>, State> {
    pub(crate) container_node: Node,
    pub(crate) inner_el: V::Element,
    pub(crate) known_nodes: HashMap<MorphId, Node>,
    pub(crate) active_morphs: Vec<ActiveMorphFlight>,
    _marker: PhantomData<State>,
}

impl<V> MorphScope<V> {
    fn rebuild_internal<State, Msg>(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut MorphScopeElement<V, State>,
        parent_info: Option<(Node, Option<Node>)>,
    ) where
        V: View<State, Message = Msg>,
    {
        if let Some((parent, next_sibling)) = parent_info {
            if let Some(sibling) = next_sibling {
                element.container_node.put_before(ctx, sibling);
            } else if !parent.children(ctx).contains(&element.container_node) {
                parent.append(ctx, element.container_node);
            }
        }

        // Settle any active morphs from a previous rapid transition so all nodes are back in place
        settle_morphs(ctx, &mut element.active_morphs);

        let prev_inner = prev.view.get_node(&element.inner_el);
        self.view.rebuild_with_parent(
            &prev.view,
            ctx,
            &mut element.inner_el,
            element.container_node,
            None,
        );
        let new_inner = self.view.get_node(&element.inner_el);
        if new_inner.is_valid()
            && (new_inner != prev_inner
                || !element.container_node.children(ctx).contains(&new_inner))
        {
            if !element.container_node.children(ctx).contains(&new_inner) {
                element.container_node.append(ctx, new_inner);
            }
        }

        let candidate_pairs: Vec<(MorphId, Vec<Node>)> = ctx
            .morph_nodes_by_id
            .iter()
            .map(|(id, nodes)| (id.clone(), nodes.clone()))
            .collect();

        // Discover matching morph pairs in this scope
        for (id, nodes) in candidate_pairs {
            let in_scope: Vec<Node> = nodes
                .into_iter()
                .filter(|n| {
                    n.is_descendant_of(ctx, element.container_node) || *n == element.container_node
                })
                .collect();

            if in_scope.is_empty() {
                element.known_nodes.remove(&id);
                continue;
            }

            let prev_node = element.known_nodes.get(&id).copied();

            if let Some(old_node) = prev_node {
                if let Some(&new_node) = in_scope.iter().find(|&&n| n != old_node) {
                    // Settle any existing flight for this ID first
                    let mut to_settle = Vec::new();
                    element.active_morphs.retain_mut(|flight| {
                        if flight.id == id {
                            to_settle.push(std::mem::replace(
                                flight,
                                ActiveMorphFlight {
                                    id: id.clone(),
                                    source_node: Node::get_invalid(),
                                    target_node: Node::get_invalid(),
                                    source_placeholder: None,
                                    target_placeholder: None,
                                    transition: MorphTransition::default(),
                                    source_rect: None,
                                    target_rect: None,
                                    source_effects: None,
                                    target_effects: None,
                                    shuttle_node: None,
                                    shuttle_source_box: None,
                                    shuttle_target_box: None,
                                    anim_progress: AnimatedValue::new(1.0),
                                    anim_start: Instant::now(),
                                },
                            ));
                            false
                        } else {
                            true
                        }
                    });
                    settle_morphs(ctx, &mut to_settle);

                    let info = ctx
                        .morph_registry
                        .get(&new_node)
                        .cloned()
                        .unwrap_or_else(|| MorphInfo {
                            node: new_node,
                            id: id.clone(),
                            transition: MorphTransition::default(),
                        });

                    let mut anim_progress = AnimatedValue::new(0.0);
                    anim_progress.set_target(
                        1.0,
                        0.0,
                        info.transition.duration_ms as f64,
                        info.transition.curve,
                    );

                    element.active_morphs.push(ActiveMorphFlight {
                        id: id.clone(),
                        source_node: old_node,
                        target_node: new_node,
                        source_placeholder: None,
                        target_placeholder: None,
                        transition: info.transition,
                        source_rect: None,
                        target_rect: None,
                        source_effects: None,
                        target_effects: None,
                        shuttle_node: None,
                        shuttle_source_box: None,
                        shuttle_target_box: None,
                        anim_progress,
                        anim_start: Instant::now(),
                    });

                    element.known_nodes.insert(id.clone(), new_node);
                    ctx.request_frame();
                }
            } else if let Some(&first) = in_scope.first() {
                element.known_nodes.insert(id.clone(), first);
            }
        }
    }
}

impl<State, V: View<State>> View<State> for MorphScope<V> {
    type Element = MorphScopeElement<V, State>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }
        container_node.update_constraints(ctx, |c| {
            c.overflow = Overflow::Visible;
        });

        let inner_el = self.view.build(ctx);
        let inner_node = self.view.get_node(&inner_el);
        if inner_node.is_valid() {
            container_node.append(ctx, inner_node);
        }

        let mut known_nodes = HashMap::new();
        for (id, nodes) in &ctx.morph_nodes_by_id {
            for &node in nodes {
                if node.is_descendant_of(ctx, container_node) || node == container_node {
                    known_nodes.insert(id.clone(), node);
                }
            }
        }

        MorphScopeElement {
            container_node,
            inner_el,
            known_nodes,
            active_morphs: Vec::new(),
            _marker: PhantomData,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.rebuild_internal(prev, ctx, element, None);
    }

    fn rebuild_with_parent(
        &self,
        prev: &Self,
        ctx: &mut Context,
        element: &mut Self::Element,
        parent: Node,
        next_sibling: Option<Node>,
    ) {
        self.rebuild_internal(prev, ctx, element, Some((parent, next_sibling)));
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        settle_morphs(ctx, &mut element.active_morphs);
        self.view.teardown(ctx, &mut element.inner_el);
        element.container_node.remove(ctx);
        ctx.destroy_node(element.container_node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.container_node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let is_tick = matches!(event, Event::Tick { .. });
        let (result, msg) = self
            .view
            .handle_event(&mut element.inner_el, state, event, ctx);

        if is_tick {
            if !element.active_morphs.is_empty() {
                apply_morph_step(
                    ctx,
                    element.container_node,
                    &mut element.active_morphs,
                    None,
                );
                let still_animating = settle_finished_morphs(ctx, &mut element.active_morphs);
                if still_animating || !element.active_morphs.is_empty() {
                    ctx.request_frame();
                }
            }
        }

        (result, msg)
    }
}

impl<V, State, Msg> ViewSequence<State> for MorphScope<V>
where
    V: View<State, Message = Msg>,
{
    type Elements = MorphScopeElement<V, State>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
        let el = View::<State>::build(self, ctx);
        parent.append(ctx, el.container_node);
        el
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, parent: Node) {
        View::<State>::rebuild_with_parent(self, prev, ctx, elements, parent, None);
    }

    fn teardown(&self, ctx: &mut Context, elements: &mut Self::Elements) {
        View::<State>::teardown(self, ctx, elements);
    }

    fn handle_event(
        &self,
        elements: &mut Self::Elements,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        View::<State>::handle_event(self, elements, state, event, ctx)
    }
}

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

    #[test]
    fn test_morph_scope_pairs_dual_presences() {
        use crate::ui::presence::presence;
        use crate::ui::style::ViewStyleExt;
        use crate::ui::widgets::column;
        use crate::{Size, Style};

        let mut ctx = Context::new();

        let v1 = morph_scope(column((
            presence(
                true,
                text::<_, ()>("Compact")
                    .style(Style::new().width(Size::Fixed(100)).height(Size::Fixed(50)))
                    .morph("card"),
            )
            .duration_ms(200.0),
            presence(
                false,
                text::<_, ()>("Expanded")
                    .style(
                        Style::new()
                            .width(Size::Fixed(300))
                            .height(Size::Fixed(150)),
                    )
                    .morph("card"),
            )
            .duration_ms(200.0),
        )));

        let mut el = View::<()>::build(&v1, &mut ctx);
        let root = View::<()>::get_node(&v1, &el);
        ctx.root_attach(root);
        ctx.compute_layout(800.0, 600.0);

        // Initially: presence 1 is visible, presence 2 is hidden.
        assert_eq!(el.active_morphs.len(), 0);

        // Toggle: presence 1 false, presence 2 true
        let v2 = morph_scope(column((
            presence(
                false,
                text::<_, ()>("Compact")
                    .style(Style::new().width(Size::Fixed(100)).height(Size::Fixed(50)))
                    .morph("card"),
            )
            .duration_ms(200.0),
            presence(
                true,
                text::<_, ()>("Expanded")
                    .style(
                        Style::new()
                            .width(Size::Fixed(300))
                            .height(Size::Fixed(150)),
                    )
                    .morph("card"),
            )
            .duration_ms(200.0),
        )));

        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        assert_eq!(el.active_morphs.len(), 1);
        assert_eq!(el.active_morphs[0].id, MorphId::new("card"));

        // Compute layout so target node bounds are resolved
        ctx.compute_layout(800.0, 600.0);

        // Step animation with a tick
        View::<()>::handle_event(&v2, &mut el, &(), Event::Tick { dt: 0.1 }, &mut ctx);
        assert!(el.active_morphs[0].shuttle_node.is_some());
        let shuttle = el.active_morphs[0].shuttle_node.unwrap();
        let shuttle_c = shuttle.get_constraints(&ctx).unwrap();
        assert_eq!(shuttle_c.z_index, 100);

        // Advance time past duration and tick to settle
        std::thread::sleep(std::time::Duration::from_millis(400));
        View::<()>::handle_event(&v2, &mut el, &(), Event::Tick { dt: 0.4 }, &mut ctx);

        assert!(el.active_morphs.is_empty());
        View::<()>::teardown(&v2, &mut ctx, &mut el);
    }

    #[test]
    fn test_morph_scope_rapid_toggle_settles() {
        use crate::ui::presence::presence;
        use crate::ui::widgets::column;

        let mut ctx = Context::new();

        let v1 = morph_scope(column((
            presence(true, text::<_, ()>("Card A").morph("card")).duration_ms(300.0),
            presence(false, text::<_, ()>("Card B").morph("card")).duration_ms(300.0),
        )));

        let mut el = View::<()>::build(&v1, &mut ctx);
        let root = View::<()>::get_node(&v1, &el);
        ctx.root_attach(root);
        ctx.compute_layout(800.0, 600.0);

        // Toggle 1 -> 2
        let v2 = morph_scope(column((
            presence(false, text::<_, ()>("Card A").morph("card")).duration_ms(300.0),
            presence(true, text::<_, ()>("Card B").morph("card")).duration_ms(300.0),
        )));
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        assert_eq!(el.active_morphs.len(), 1);

        ctx.compute_layout(800.0, 600.0);
        View::<()>::handle_event(&v2, &mut el, &(), Event::Tick { dt: 0.05 }, &mut ctx);

        // Toggle rapidly back before flight finishes (2 -> 1)
        let v3 = morph_scope(column((
            presence(true, text::<_, ()>("Card A").morph("card")).duration_ms(300.0),
            presence(false, text::<_, ()>("Card B").morph("card")).duration_ms(300.0),
        )));
        View::<()>::rebuild(&v3, &v2, &mut ctx, &mut el);
        assert_eq!(el.active_morphs.len(), 1);

        ctx.compute_layout(800.0, 600.0);
        View::<()>::handle_event(&v3, &mut el, &(), Event::Tick { dt: 0.05 }, &mut ctx);

        // Settle completely
        std::thread::sleep(std::time::Duration::from_millis(400));
        View::<()>::handle_event(&v3, &mut el, &(), Event::Tick { dt: 0.4 }, &mut ctx);
        assert!(el.active_morphs.is_empty());

        View::<()>::teardown(&v3, &mut ctx, &mut el);
    }

    #[test]
    fn test_morph_scope_view_sequence_compatibility() {
        use crate::ui::presence::presence;
        use crate::ui::widgets::column;

        let mut ctx = Context::new();

        let v = column((
            text::<_, ()>("Header"),
            column((
                presence(true, text::<_, ()>("Item 1").morph("item")),
                presence(false, text::<_, ()>("Item 2").morph("item")),
            ))
            .morph_scope(),
            text::<_, ()>("Footer"),
        ));

        let mut el = View::<()>::build(&v, &mut ctx);
        let parent = View::<()>::get_node(&v, &el);
        assert_eq!(parent.children(&ctx).len(), 3);

        View::<()>::teardown(&v, &mut ctx, &mut el);
    }
}
