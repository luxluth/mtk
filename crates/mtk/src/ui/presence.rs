//! Conditional presence container with animated enter and exit motions.
//!
//! Provides [`presence`] and [`PresenceViewExt`], which mount and animate any widget
//! into or out of view when a boolean condition changes.
//!
//! Unlike a container widget, [`Presence`] is an abstract view wrapper: it does not create
//! an intermediate container layout node, does not force layout dimensions (such as `Size::Fit`),
//! and does not remain in the layout tree when hidden.

use std::marker::PhantomData;
use std::time::Instant;

use crate::animation::{AnimatedValue, Curve};
use crate::debugger::SourceLocation;
use crate::style::PositionStrategy;
use crate::ui::event::EventResult;
use crate::ui::transition::Motion;
use crate::ui::{Event, View, ViewSequence};
use crate::{Context, Node};

/// An abstract view wrapper that animates a child view when it enters or exits the hierarchy.
///
/// Unlike a container widget, [`Presence`] does not introduce an intermediate layout node,
/// does not force its own dimensions onto the child, and leaves no layout node in the tree when hidden.
pub struct Presence<V, Msg> {
    pub(crate) is_visible: bool,
    pub(crate) view: V,
    pub(crate) enter_motion: Motion,
    pub(crate) exit_motion: Motion,
    pub(crate) duration_ms: f32,
    pub(crate) curve: Curve,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new [`Presence`] wrapper displaying `view` when `is_visible` is `true`,
/// smoothly playing enter and exit animations when visibility changes.
///
/// # Examples
/// ```rust,ignore
/// presence(state.show_banner, banner_view)
///     .enter(Motion::slide_in_top().combined(Motion::fade_in()))
///     .exit(Motion::slide_out_top().combined(Motion::fade_out()))
/// ```
#[track_caller]
pub fn presence<V, Msg>(is_visible: bool, view: V) -> Presence<V, Msg> {
    Presence {
        is_visible,
        view,
        enter_motion: Motion::fade_in(),
        exit_motion: Motion::fade_out(),
        duration_ms: 200.0,
        curve: Curve::ease_out(),
        source_loc: Some(SourceLocation::here("Presence")),
        _marker: PhantomData,
    }
}

impl<V, Msg> Presence<V, Msg> {
    /// Sets the motion played when the view appears into the hierarchy.
    pub fn enter(mut self, motion: Motion) -> Self {
        self.enter_motion = motion;
        self
    }

    /// Sets the motion played when the view exits the hierarchy.
    pub fn exit(mut self, motion: Motion) -> Self {
        self.exit_motion = motion;
        self
    }

    /// Sets transition duration in milliseconds.
    pub fn duration_ms(mut self, duration_ms: f32) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Sets transition easing curve.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }
}

/// Extension trait for [`View`] that enables fluid `.presence(...)` method chaining.
pub trait PresenceViewExt: Sized {
    /// Wraps this view in a [`Presence`] element animated by enter and exit motions.
    fn presence(self, is_visible: bool) -> Presence<Self, ()>;
}

impl<V> PresenceViewExt for V {
    fn presence(self, is_visible: bool) -> Presence<Self, ()> {
        presence(is_visible, self)
    }
}

/// Persistent element state for an abstract [`Presence`] widget.
pub struct PresenceElement<V: View<State>, State> {
    pub(crate) is_visible: bool,
    pub(crate) child: Option<(Node, V::Element)>,
    pub(crate) outgoing: Option<(Node, V::Element)>,
    pub(crate) anim_progress: AnimatedValue<f32>,
    pub(crate) anim_start: Instant,
    pub(crate) is_entering: bool,
    pub(crate) child_orig_positioning: PositionStrategy,
    pub(crate) parent_node: Option<Node>,
    pub(crate) inflow_pos: Option<(f32, f32)>,
    _marker: PhantomData<State>,
}

fn resolve_node_bounds(ctx: &Context, node: Node) -> (f32, f32) {
    if let Some(comp) = node.get_computed(ctx) {
        if comp.w > 0.0 || comp.h > 0.0 {
            return (comp.w.max(1.0), comp.h.max(1.0));
        }
    }
    if let Some(parent) = node.parent(ctx) {
        if let Some(p_comp) = parent.get_computed(ctx) {
            if p_comp.w > 0.0 || p_comp.h > 0.0 {
                return (p_comp.w.max(1.0), p_comp.h.max(1.0));
            }
        }
    }
    (100.0, 100.0)
}

fn apply_motion_step(
    ctx: &mut Context,
    motion: &Motion,
    progress: f32,
    node: Node,
    width: f32,
    height: f32,
    orig_positioning: PositionStrategy,
    inflow_pos: Option<(f32, f32)>,
) {
    let offset_x = motion.resolve_offset_x(progress, width);
    let offset_y = motion.resolve_offset_y(progress, height);
    let opacity = motion.resolve_opacity(progress);
    let scale = motion.resolve_scale(progress);

    let has_offset = offset_x.abs() > 1e-3 || offset_y.abs() > 1e-3;
    if has_offset {
        let (base_x, base_y) = match orig_positioning {
            PositionStrategy::Absolute { top, left, .. } => (
                if left.is_finite() { left } else { 0.0 },
                if top.is_finite() { top } else { 0.0 },
            ),
            PositionStrategy::Inflow => inflow_pos.unwrap_or((0.0, 0.0)),
        };

        node.update_constraints(ctx, |c| {
            c.positioning = PositionStrategy::Absolute {
                top: base_y + offset_y,
                left: base_x + offset_x,
                bottom: f32::NAN,
                right: f32::NAN,
            };
        });
    }

    node.update_effects(ctx, |eff| {
        eff.opacity = opacity.clamp(0.0, 1.0);
        eff.scale = scale;
    });
}

fn rebuild_presence_internal<V, State, Msg>(
    view: &Presence<V, Msg>,
    prev: &Presence<V, Msg>,
    ctx: &mut Context,
    element: &mut PresenceElement<V, State>,
    parent_info: Option<(Node, Option<Node>)>,
) where
    V: View<State, Message = Msg>,
{
    if let Some((p, _)) = parent_info {
        element.parent_node = Some(p);
    } else if element.parent_node.is_none() {
        element.parent_node = element
            .child
            .as_ref()
            .and_then(|(n, _)| n.parent(ctx))
            .or_else(|| element.outgoing.as_ref().and_then(|(n, _)| n.parent(ctx)));
    }

    match (view.is_visible, element.is_visible) {
        (true, true) => {
            if let Some((ref mut node, ref mut el)) = element.child {
                if let Some((parent, next_sibling)) = parent_info {
                    view.view
                        .rebuild_with_parent(&prev.view, ctx, el, parent, next_sibling);
                } else {
                    view.view.rebuild(&prev.view, ctx, el);
                }
                *node = view.view.get_node(el);
                if !element.anim_progress.is_animating() {
                    element.child_orig_positioning = node
                        .get_constraints(ctx)
                        .map(|c| c.positioning)
                        .unwrap_or(PositionStrategy::Inflow);
                }
            }
        }
        (false, false) => {}
        (true, false) => {
            // Enter transition: clean up any pending outgoing node first
            if let Some((out_node, mut out_el)) = element.outgoing.take() {
                view.view.teardown(ctx, &mut out_el);
                if out_node.is_valid() {
                    out_node.remove(ctx);
                    ctx.destroy_node(out_node);
                }
            }

            let el = view.view.build(ctx);
            let node = view.view.get_node(&el);
            if let Some(loc) = view.source_loc {
                if node.is_valid() {
                    ctx.set_node_source(node, loc);
                }
            }
            let orig_pos = node
                .get_constraints(ctx)
                .map(|c| c.positioning)
                .unwrap_or(PositionStrategy::Inflow);
            element.child_orig_positioning = orig_pos;

            if node.is_valid() {
                if let Some((parent, next_sibling)) = parent_info {
                    if let Some(sibling) = next_sibling {
                        node.put_before(ctx, sibling);
                    } else {
                        parent.append(ctx, node);
                    }
                } else if let Some(parent) = element.parent_node {
                    parent.append(ctx, node);
                } else if ctx.layout.root.is_none() {
                    ctx.root_attach(node);
                }
            }

            element.child = Some((node, el));
            element.is_visible = true;
            element.is_entering = true;
            element.inflow_pos = None;

            if view.duration_ms > 0.0 && node.is_valid() {
                element.anim_progress = AnimatedValue::new(0.0);
                element.anim_start = Instant::now();
                element
                    .anim_progress
                    .set_target(1.0, 0.0, view.duration_ms as f64, view.curve);

                let (w, h) = resolve_node_bounds(ctx, node);
                apply_motion_step(ctx, &view.enter_motion, 0.0, node, w, h, orig_pos, None);
                ctx.request_frame();
            } else if node.is_valid() {
                let final_opacity = view.enter_motion.resolve_opacity(1.0);
                let final_scale = view.enter_motion.resolve_scale(1.0);
                node.update_effects(ctx, |eff| {
                    eff.opacity = final_opacity.clamp(0.0, 1.0);
                    eff.scale = final_scale;
                });
            }
        }
        (false, true) => {
            // Exit transition: move current child to outgoing
            if let Some((node, mut el)) = element.child.take() {
                if node.is_valid() {
                    if let Some(parent) = node.parent(ctx) {
                        element.parent_node = Some(parent);
                        if let (Some(c), Some(p)) =
                            (node.get_computed(ctx), parent.get_computed(ctx))
                        {
                            if c.w > 0.0 || c.h > 0.0 {
                                element.inflow_pos = Some((c.x - p.x, c.y - p.y));
                            }
                        }
                    }
                }

                if view.duration_ms > 0.0 && node.is_valid() {
                    element.outgoing = Some((node, el));
                    element.is_visible = false;
                    element.is_entering = false;

                    element.anim_progress = AnimatedValue::new(0.0);
                    element.anim_start = Instant::now();
                    element
                        .anim_progress
                        .set_target(1.0, 0.0, view.duration_ms as f64, view.curve);

                    let (w, h) = resolve_node_bounds(ctx, node);
                    apply_motion_step(
                        ctx,
                        &view.exit_motion,
                        0.0,
                        node,
                        w,
                        h,
                        element.child_orig_positioning,
                        element.inflow_pos,
                    );
                    ctx.request_frame();
                } else {
                    element.is_visible = false;
                    element.is_entering = false;
                    element.outgoing = None;
                    view.view.teardown(ctx, &mut el);
                    if node.is_valid() {
                        node.remove(ctx);
                        ctx.destroy_node(node);
                    }
                }
            }
        }
    }
}

impl<V, State, Msg> View<State> for Presence<V, Msg>
where
    V: View<State, Message = Msg>,
{
    type Element = PresenceElement<V, State>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let (child, child_orig_positioning) = if self.is_visible {
            let el = self.view.build(ctx);
            let node = self.view.get_node(&el);
            if let Some(loc) = self.source_loc {
                if node.is_valid() {
                    ctx.set_node_source(node, loc);
                }
            }
            let orig_pos = node
                .get_constraints(ctx)
                .map(|c| c.positioning)
                .unwrap_or(PositionStrategy::Inflow);
            (Some((node, el)), orig_pos)
        } else {
            (None, PositionStrategy::Inflow)
        };

        PresenceElement {
            is_visible: self.is_visible,
            child,
            outgoing: None,
            anim_progress: AnimatedValue::new(1.0),
            anim_start: Instant::now(),
            is_entering: false,
            child_orig_positioning,
            parent_node: None,
            inflow_pos: None,
            _marker: PhantomData,
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
        rebuild_presence_internal(self, prev, ctx, element, Some((parent, next_sibling)));
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        rebuild_presence_internal(self, prev, ctx, element, None);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some((out_node, mut out_el)) = element.outgoing.take() {
            self.view.teardown(ctx, &mut out_el);
            if out_node.is_valid() {
                out_node.remove(ctx);
                ctx.destroy_node(out_node);
            }
        }
        if let Some((node, mut el)) = element.child.take() {
            self.view.teardown(ctx, &mut el);
            if node.is_valid() {
                node.remove(ctx);
                ctx.destroy_node(node);
            }
        }
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        if let Some((node, _)) = &element.child {
            *node
        } else if let Some((node, _)) = &element.outgoing {
            *node
        } else {
            Node::get_invalid()
        }
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if let Event::Tick { .. } = event {
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            if element.anim_progress.is_animating() {
                element.anim_progress.tick(now);
                let progress = element.anim_progress.get();
                let animating = element.anim_progress.is_animating();

                if element.is_entering {
                    if let Some((node, _)) = &element.child {
                        let node = *node;
                        if ctx.is_morph_active_for(node) {
                            if !animating || progress >= 0.999 {
                                element.is_entering = false;
                            } else {
                                ctx.request_frame();
                            }
                        } else {
                            let (w, h) = resolve_node_bounds(ctx, node);
                            apply_motion_step(
                                ctx,
                                &self.enter_motion,
                                progress,
                                node,
                                w,
                                h,
                                element.child_orig_positioning,
                                element.inflow_pos,
                            );
                            if !animating || progress >= 0.999 {
                                node.update_constraints(ctx, |c| {
                                    c.positioning = element.child_orig_positioning;
                                });
                                let final_opacity = self.enter_motion.resolve_opacity(1.0);
                                let final_scale = self.enter_motion.resolve_scale(1.0);
                                node.update_effects(ctx, |eff| {
                                    eff.opacity = final_opacity.clamp(0.0, 1.0);
                                    eff.scale = final_scale;
                                });
                            } else {
                                ctx.request_frame();
                            }
                        }
                    }
                } else if let Some((out_node, _)) = &element.outgoing {
                    let out_node = *out_node;
                    if ctx.is_morph_active_for(out_node) {
                        ctx.request_frame();
                    } else {
                        let (w, h) = resolve_node_bounds(ctx, out_node);
                        apply_motion_step(
                            ctx,
                            &self.exit_motion,
                            progress,
                            out_node,
                            w,
                            h,
                            element.child_orig_positioning,
                            element.inflow_pos,
                        );
                        if !animating || progress >= 0.999 {
                            if let Some((node, mut el)) = element.outgoing.take() {
                                self.view.teardown(ctx, &mut el);
                                if node.is_valid() {
                                    node.remove(ctx);
                                    ctx.destroy_node(node);
                                }
                            }
                        } else {
                            ctx.request_frame();
                        }
                    }
                }
            } else if let Some((out_node, _)) = &element.outgoing {
                let out_node = *out_node;
                if ctx.is_morph_active_for(out_node) {
                    ctx.request_frame();
                } else if let Some((node, mut el)) = element.outgoing.take() {
                    self.view.teardown(ctx, &mut el);
                    if node.is_valid() {
                        node.remove(ctx);
                        ctx.destroy_node(node);
                    }
                }
            }
        }

        if let Some((_, ref mut el)) = element.child {
            self.view.handle_event(el, state, event, ctx)
        } else {
            (EventResult::Ignored, None)
        }
    }

    fn on_layout(
        &self,
        element: &mut Self::Element,
        computed: crate::layout::Computed,
        ctx: &mut Context,
    ) {
        if let Some((_, ref mut el)) = element.child {
            self.view.on_layout(el, computed, ctx);
        }
    }
}

impl<V, State, Msg> ViewSequence<State> for Presence<V, Msg>
where
    V: View<State, Message = Msg>,
{
    type Elements = PresenceElement<V, State>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context, parent: Node) -> Self::Elements {
        let mut el = View::<State>::build(self, ctx);
        el.parent_node = Some(parent);
        if let Some((node, _)) = &el.child {
            if node.is_valid() {
                parent.append(ctx, *node);
            }
        }
        el
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, elements: &mut Self::Elements, parent: Node) {
        self.rebuild_with_parent(prev, ctx, elements, parent, None);
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

    fn on_layout(&self, elements: &mut Self::Elements, ctx: &mut Context) {
        View::<State>::on_layout(
            self,
            elements,
            elements
                .child
                .as_ref()
                .map(|(n, _)| *n)
                .and_then(|n| n.get_computed(ctx))
                .unwrap_or_default(),
            ctx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Size, Style};
    use crate::ui::style::ViewStyleExt;
    use crate::ui::widgets::text;

    #[test]
    fn test_presence_initial_hidden_and_enter() {
        let mut ctx = Context::new();
        let p_hidden = presence(false, text::<_, ()>("Content"))
            .enter(Motion::fade_in())
            .duration_ms(200.0);
        let p_visible = presence(true, text::<_, ()>("Content"))
            .enter(Motion::fade_in())
            .duration_ms(200.0);

        let mut el = View::<()>::build(&p_hidden, &mut ctx);
        assert!(el.child.is_none());
        assert!(!View::<()>::get_node(&p_hidden, &el).is_valid());

        View::<()>::rebuild(&p_visible, &p_hidden, &mut ctx, &mut el);
        assert!(el.child.is_some());
        assert!(el.is_entering);
        assert!(View::<()>::get_node(&p_visible, &el).is_valid());

        let node = el.child.as_ref().unwrap().0;
        let eff = node.get_effects(&ctx).unwrap();
        assert_eq!(eff.opacity, 0.0);

        // Advance past completion
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&p_visible, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);

        let eff_after = node.get_effects(&ctx).unwrap();
        assert_eq!(eff_after.opacity, 1.0);
        assert_eq!(eff_after.scale, 1.0);
    }

    #[test]
    fn test_presence_exit_and_deferred_unmount() {
        let mut ctx = Context::new();
        let p_visible = presence(true, text::<_, ()>("Goodbye"))
            .exit(Motion::fade_out())
            .duration_ms(200.0);
        let p_hidden = presence(false, text::<_, ()>("Goodbye"))
            .exit(Motion::fade_out())
            .duration_ms(200.0);

        let mut el = View::<()>::build(&p_visible, &mut ctx);
        assert!(el.child.is_some());
        assert!(View::<()>::get_node(&p_visible, &el).is_valid());

        // When switching to hidden, node moves to outgoing and stays alive
        View::<()>::rebuild(&p_hidden, &p_visible, &mut ctx, &mut el);
        assert!(el.child.is_none());
        assert!(el.outgoing.is_some());
        assert!(View::<()>::get_node(&p_hidden, &el).is_valid());

        let out_node = el.outgoing.as_ref().unwrap().0;
        let eff = out_node.get_effects(&ctx).unwrap();
        assert_eq!(eff.opacity, 1.0);

        // Advance past duration
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&p_hidden, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);

        // Deferred unmount now tears down and destroys outgoing node
        assert!(el.outgoing.is_none());
        assert!(!View::<()>::get_node(&p_hidden, &el).is_valid());
    }

    #[test]
    fn test_presence_preserves_child_dimensions_and_no_container() {
        let mut ctx = Context::new();
        let styled_text = text::<_, ()>("Fill Item").style(
            Style::new()
                .width(Size::Fill)
                .height(Size::Fixed(40))
                .flex_grow(2.0),
        );
        let p = presence(true, styled_text);
        let el = View::<()>::build(&p, &mut ctx);
        let node = View::<()>::get_node(&p, &el);

        // Verify the node returned is the child node directly with its own constraints preserved
        let cons = node.get_constraints(&ctx).unwrap();
        assert_eq!(cons.width, Size::Fill);
        assert_eq!(cons.height, Size::Fixed(40));
        assert_eq!(cons.flex_grow, 2.0);
    }

    #[test]
    fn test_presence_in_container_does_not_stay_in_tree_when_hidden() {
        use crate::ui::widgets::column;

        let mut ctx = Context::new();
        let v1 = column((
            text::<_, ()>("Header"),
            presence(false, text::<_, ()>("Hidden Banner")),
            text::<_, ()>("Footer"),
        ));

        let mut el = View::<()>::build(&v1, &mut ctx);
        let parent = View::<()>::get_node(&v1, &el);

        // Presence is hidden: only Header and Footer must be in the layout tree!
        let children1 = parent.children(&ctx);
        assert_eq!(children1.len(), 2);
        assert_eq!(children1[0].get_text(&ctx), Some("Header"));
        assert_eq!(children1[1].get_text(&ctx), Some("Footer"));

        // Rebuild: show the banner with instant duration
        let v2 = column((
            text::<_, ()>("Header"),
            presence(true, text::<_, ()>("Hidden Banner")).duration_ms(0.0),
            text::<_, ()>("Footer"),
        ));
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);

        let children2 = parent.children(&ctx);
        assert_eq!(children2.len(), 3);
        assert_eq!(children2[0].get_text(&ctx), Some("Header"));
        assert_eq!(children2[1].get_text(&ctx), Some("Hidden Banner"));
        assert_eq!(children2[2].get_text(&ctx), Some("Footer"));

        // Rebuild: hide again with instant duration
        let v3 = column((
            text::<_, ()>("Header"),
            presence(false, text::<_, ()>("Hidden Banner")).duration_ms(0.0),
            text::<_, ()>("Footer"),
        ));
        View::<()>::rebuild(&v3, &v2, &mut ctx, &mut el);

        let children3 = parent.children(&ctx);
        assert_eq!(children3.len(), 2);
        assert_eq!(children3[0].get_text(&ctx), Some("Header"));
        assert_eq!(children3[1].get_text(&ctx), Some("Footer"));

        View::<()>::teardown(&v3, &mut ctx, &mut el);
    }

    #[test]
    fn test_presence_animated_exit_unmount_in_container() {
        use crate::ui::widgets::column;

        let mut ctx = Context::new();
        let v1 = column((
            text::<_, ()>("Header"),
            presence(true, text::<_, ()>("Animated Banner"))
                .exit(Motion::fade_out())
                .duration_ms(200.0),
            text::<_, ()>("Footer"),
        ));

        let mut el = View::<()>::build(&v1, &mut ctx);
        let parent = View::<()>::get_node(&v1, &el);
        assert_eq!(parent.children(&ctx).len(), 3);

        // Rebuild to hidden: banner is in exit animation and stays during animation
        let v2 = column((
            text::<_, ()>("Header"),
            presence(false, text::<_, ()>("Animated Banner"))
                .exit(Motion::fade_out())
                .duration_ms(200.0),
            text::<_, ()>("Footer"),
        ));
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        assert_eq!(parent.children(&ctx).len(), 3);

        // Advance past duration and dispatch Tick
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&v2, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);

        // After unmount, banner node is removed from parent
        let children = parent.children(&ctx);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].get_text(&ctx), Some("Header"));
        assert_eq!(children[1].get_text(&ctx), Some("Footer"));

        View::<()>::teardown(&v2, &mut ctx, &mut el);
    }

    #[test]
    fn test_presence_direct_view_sequence() {
        use crate::ui::widgets::column;

        let mut ctx = Context::new();
        let v1 = column(presence(true, text::<_, ()>("Only Presence")));
        let mut el = View::<()>::build(&v1, &mut ctx);
        let parent = View::<()>::get_node(&v1, &el);
        assert_eq!(parent.children(&ctx).len(), 1);

        let v2 = column(presence(false, text::<_, ()>("Only Presence")).duration_ms(0.0));
        View::<()>::rebuild(&v2, &v1, &mut ctx, &mut el);
        assert_eq!(parent.children(&ctx).len(), 0);

        View::<()>::teardown(&v2, &mut ctx, &mut el);
    }

    #[test]
    fn test_presence_extension_trait() {
        let mut ctx = Context::new();
        let view = text::<_, ()>("Hello").presence(true);
        let el = View::<()>::build(&view, &mut ctx);
        assert!(el.child.is_some());
    }
}
