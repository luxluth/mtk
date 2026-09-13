//! High-performance page and view router with animated transitions.
//!
//! Provides [`router`], which mounts and animates between different views whenever its
//! route key changes.

use std::marker::PhantomData;
use std::time::Instant;

use crate::animation::AnimatedValue;
use crate::debugger::SourceLocation;
use crate::style::{Overflow, PositionStrategy, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::transition::{PageTransition, TransitionOrder};
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A router widget that smoothly animates between pages/views when its route key changes.
pub struct Router<Key, V, Msg> {
    pub(crate) key: Key,
    pub(crate) view: V,
    pub(crate) transition: PageTransition,
    pub(crate) transition_spec: Option<Box<dyn Fn(&Key, &Key) -> PageTransition>>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new [`Router`] displaying `view` for the given route `key`.
///
/// Whenever `key` changes between rebuilds, the router automatically clears stale focus,
/// animates the outgoing view out, and animates the new view in.
///
/// # Examples
/// ```rust,ignore
/// router(state.current_page, render_page(state.current_page, state))
///     .transition(PageTransition::push())
/// ```
#[track_caller]
pub fn router<Key, V, Msg>(key: Key, view: V) -> Router<Key, V, Msg>
where
    Key: PartialEq + Clone + 'static,
{
    Router {
        key,
        view,
        transition: PageTransition::fade(),
        transition_spec: None,
        source_loc: Some(SourceLocation::here("Router")),
        _marker: PhantomData,
    }
}

impl<Key, V, Msg> Router<Key, V, Msg>
where
    Key: PartialEq + Clone + 'static,
{
    /// Sets the transition animation physics (fade, push, pop, slide, etc.) when switching views.
    ///
    /// Accepts any [`PageTransition`] or preset [`Transition`].
    pub fn transition(mut self, transition: impl Into<PageTransition>) -> Self {
        self.transition = transition.into();
        self
    }

    /// Dynamically specifies transition physics, asymmetry, and layering based on `(from_route, to_route)`.
    pub fn transition_spec(
        mut self,
        spec: impl Fn(&Key, &Key) -> PageTransition + 'static,
    ) -> Self {
        self.transition_spec = Some(Box::new(spec));
        self
    }
}

pub struct RouterElement<Key, V: View<State>, State> {
    container_node: Node,
    active_key: Key,
    current_node: Node,
    current_el: V::Element,
    outgoing: Option<(Node, V::Element)>,
    current_transition: PageTransition,
    anim_progress: AnimatedValue<f32>,
    anim_start: Instant,
    current_orig_positioning: PositionStrategy,
    current_orig_z_index: i32,
    _marker: PhantomData<State>,
}

fn apply_transition_step(
    ctx: &mut Context,
    transition: &PageTransition,
    progress: f32,
    out_node: Node,
    current_node: Node,
    screen_w: f32,
    screen_h: f32,
) {
    let (out_z, cur_z) = match transition.order {
        TransitionOrder::IncomingOnTop => (0, 1),
        TransitionOrder::OutgoingOnTop => (1, 0),
        TransitionOrder::SameLevel => (0, 0),
    };

    let out_offset_x = transition.exit.resolve_offset_x(progress, screen_w);
    let out_offset_y = transition.exit.resolve_offset_y(progress, screen_h);
    let out_opacity = transition.exit.resolve_opacity(progress);
    let out_scale = transition.exit.resolve_scale(progress);

    let cur_offset_x = transition.enter.resolve_offset_x(progress, screen_w);
    let cur_offset_y = transition.enter.resolve_offset_y(progress, screen_h);
    let cur_opacity = transition.enter.resolve_opacity(progress);
    let cur_scale = transition.enter.resolve_scale(progress);

    out_node.update_constraints(ctx, |c| {
        c.positioning = PositionStrategy::Absolute {
            top: out_offset_y,
            left: out_offset_x,
            bottom: f32::NAN,
            right: f32::NAN,
        };
        c.z_index = out_z;
    });

    current_node.update_constraints(ctx, |c| {
        c.positioning = PositionStrategy::Absolute {
            top: cur_offset_y,
            left: cur_offset_x,
            bottom: f32::NAN,
            right: f32::NAN,
        };
        c.z_index = cur_z;
    });

    out_node.update_effects(ctx, |eff| {
        eff.opacity = out_opacity.clamp(0.0, 1.0);
        eff.scale = out_scale;
    });

    current_node.update_effects(ctx, |eff| {
        eff.opacity = cur_opacity.clamp(0.0, 1.0);
        eff.scale = cur_scale;
    });
}

impl<Key, V, State, Msg> View<State> for Router<Key, V, Msg>
where
    Key: PartialEq + Clone + 'static,
    V: View<State, Message = Msg>,
{
    type Element = RouterElement<Key, V, State>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }

        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .overflow(Overflow::Hidden)
            .apply_to_node(ctx, container_node);

        let current_el = self.view.build(ctx);
        let current_node = self.view.get_node(&current_el);
        let current_orig_constraints = current_node.get_constraints(ctx);
        let current_orig_positioning = current_orig_constraints
            .map(|c| c.positioning)
            .unwrap_or(PositionStrategy::Inflow);
        let current_orig_z_index = current_orig_constraints.map(|c| c.z_index).unwrap_or(0);
        container_node.append(ctx, current_node);

        RouterElement {
            container_node,
            active_key: self.key.clone(),
            current_node,
            current_el,
            outgoing: None,
            current_transition: self.transition.clone(),
            anim_progress: AnimatedValue::new(1.0),
            anim_start: Instant::now(),
            current_orig_positioning,
            current_orig_z_index,
            _marker: PhantomData,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if self.key == element.active_key {
            self.view.rebuild(&prev.view, ctx, &mut element.current_el);
        } else {
            // Clear any active focus ring from the previous page
            ctx.blur();

            // Clean up any pending outgoing view before starting a new transition
            if let Some((out_node, _)) = element.outgoing.take() {
                out_node.remove(ctx);
                ctx.destroy_node(out_node);
            }

            // Capture from_key and to_key to determine transition physics
            let from_key = element.active_key.clone();
            let to_key = self.key.clone();
            let transition = if let Some(spec) = &self.transition_spec {
                spec(&from_key, &to_key)
            } else {
                self.transition.clone()
            };
            element.current_transition = transition.clone();

            // Move current view to outgoing slot
            let old_node = element.current_node;
            let old_el = std::mem::replace(&mut element.current_el, self.view.build(ctx));
            element.outgoing = Some((old_node, old_el));

            // Setup new current view
            let new_node = self.view.get_node(&element.current_el);
            let new_orig_constraints = new_node.get_constraints(ctx);
            let new_orig_positioning = new_orig_constraints
                .map(|c| c.positioning)
                .unwrap_or(PositionStrategy::Inflow);
            let new_orig_z_index = new_orig_constraints.map(|c| c.z_index).unwrap_or(0);
            element.current_node = new_node;
            element.current_orig_positioning = new_orig_positioning;
            element.current_orig_z_index = new_orig_z_index;
            element.active_key = to_key;
            element.container_node.append(ctx, new_node);

            let duration = transition.duration_ms;
            let curve = transition.curve;

            if duration > 0.0 {
                element.anim_progress = AnimatedValue::new(0.0);
                element.anim_start = Instant::now();
                element
                    .anim_progress
                    .set_target(1.0, 0.0, duration as f64, curve);

                // Immediately set initial overlapping positions and opacities so layout
                // recomputations do not split space or squash children before the first tick.
                let container_bounds = element.container_node.get_computed(ctx);
                let (screen_w, screen_h) = container_bounds
                    .map(|c| (c.w, c.h))
                    .unwrap_or((800.0, 600.0));
                let screen_w = screen_w.max(1.0);
                let screen_h = screen_h.max(1.0);

                apply_transition_step(
                    ctx,
                    &transition,
                    0.0,
                    old_node,
                    new_node,
                    screen_w,
                    screen_h,
                );

                ctx.request_frame();
            } else {
                if let Some((out_node, _)) = element.outgoing.take() {
                    out_node.remove(ctx);
                    ctx.destroy_node(out_node);
                }
                element.current_node.update_effects(ctx, |eff| {
                    eff.opacity = 1.0;
                    eff.scale = 1.0;
                });
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some((out_node, _)) = element.outgoing.take() {
            out_node.remove(ctx);
            ctx.destroy_node(out_node);
        }
        element.current_node.remove(ctx);
        ctx.destroy_node(element.current_node);
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
        if let Event::Tick { .. } = event {
            if element.outgoing.is_some() {
                let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
                element.anim_progress.tick(now);
                let progress = element.anim_progress.get();
                let animating = element.anim_progress.is_animating();

                let container_bounds = element.container_node.get_computed(ctx);
                let (screen_w, screen_h) = container_bounds
                    .map(|c| (c.w, c.h))
                    .unwrap_or((800.0, 600.0));
                let screen_w = screen_w.max(1.0);
                let screen_h = screen_h.max(1.0);

                if let Some((out_node, _)) = element.outgoing.as_ref() {
                    apply_transition_step(
                        ctx,
                        &element.current_transition,
                        progress,
                        *out_node,
                        element.current_node,
                        screen_w,
                        screen_h,
                    );
                }

                // If transition finished, clean up outgoing view and restore normal flow positioning
                if !animating || progress >= 0.999 {
                    if let Some((out_node, _)) = element.outgoing.take() {
                        out_node.remove(ctx);
                        ctx.destroy_node(out_node);
                    }
                    element.current_node.update_constraints(ctx, |c| {
                        c.positioning = element.current_orig_positioning;
                        c.z_index = element.current_orig_z_index;
                    });
                    element.current_node.update_effects(ctx, |eff| {
                        eff.opacity = 1.0;
                        eff.scale = 1.0;
                    });
                } else {
                    ctx.request_frame();
                }
            }
        }

        // Deliver user events to the active incoming view
        self.view
            .handle_event(&mut element.current_el, state, event, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ViewStyleExt;
    use crate::ui::transition::{Motion, Transition};
    use crate::ui::widgets::{column, text};

    #[test]
    fn test_router_fade_transition_node_overlap_and_no_split() {
        let mut ctx = Context::new();

        let page1 = column((text::<_, ()>("Page 1"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));
        let page2 = column((text::<_, ()>("Page 2"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));

        let router1 = router(1, page1).transition(Transition::fade());
        let router2 = router(2, page2).transition(Transition::fade());

        let mut el = View::<()>::build(&router1, &mut ctx);
        let container = View::<()>::get_node(&router1, &el);
        ctx.root_attach(container);
        ctx.compute_layout(800.0, 600.0);

        let initial_comp = el.current_node.get_computed(&ctx).unwrap();
        assert_eq!(initial_comp.w, 800.0);
        assert_eq!(initial_comp.h, 600.0);
        assert_eq!(initial_comp.x, 0.0);
        assert_eq!(initial_comp.y, 0.0);

        // Switch route to page 2
        View::<()>::rebuild(&router2, &router1, &mut ctx, &mut el);
        assert!(el.outgoing.is_some());

        // Compute layout during transition
        ctx.compute_layout(800.0, 600.0);

        let (out_node, _) = el.outgoing.as_ref().unwrap();
        let out_comp = out_node.get_computed(&ctx).unwrap();
        let new_comp = el.current_node.get_computed(&ctx).unwrap();

        // Crucial test: both views must overlap at full dimensions, NOT split the screen (e.g. h=300)
        assert_eq!(out_comp.w, 800.0);
        assert_eq!(out_comp.h, 600.0);
        assert_eq!(out_comp.x, 0.0);
        assert_eq!(out_comp.y, 0.0);

        assert_eq!(new_comp.w, 800.0);
        assert_eq!(new_comp.h, 600.0);
        assert_eq!(new_comp.x, 0.0);
        assert_eq!(new_comp.y, 0.0);

        // Advance animation past completion
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&router2, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);

        assert!(el.outgoing.is_none());
        assert_eq!(
            el.current_node.get_constraints(&ctx).unwrap().positioning,
            PositionStrategy::Inflow
        );
        let eff = el.current_node.get_effects(&ctx).unwrap();
        assert_eq!(eff.opacity, 1.0);
    }

    #[test]
    fn test_router_slide_right_offset_and_width_preservation() {
        let mut ctx = Context::new();

        let page1 = column((text::<_, ()>("Page 1"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));
        let page2 = column((text::<_, ()>("Page 2"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));

        let router1 = router("a", page1).transition(Transition::slide_right());
        let router2 = router("b", page2).transition(Transition::slide_right());

        let mut el = View::<()>::build(&router1, &mut ctx);
        let container = View::<()>::get_node(&router1, &el);
        ctx.root_attach(container);
        ctx.compute_layout(1000.0, 500.0);

        View::<()>::rebuild(&router2, &router1, &mut ctx, &mut el);
        assert!(el.outgoing.is_some());

        // Immediately in rebuild at progress 0, new_node is placed at left=1000.0 (offscreen right)
        let new_cons = el.current_node.get_constraints(&ctx).unwrap();
        if let PositionStrategy::Absolute { left, right, .. } = new_cons.positioning {
            assert_eq!(left, 1000.0);
            assert!(right.is_nan(), "right must be NaN so width is not squeezed");
        } else {
            panic!("Expected Absolute positioning");
        }

        ctx.compute_layout(1000.0, 500.0);
        let new_comp = el.current_node.get_computed(&ctx).unwrap();
        assert_eq!(
            new_comp.w, 1000.0,
            "Slide transition must preserve full width"
        );
        assert_eq!(new_comp.x, 1000.0);
    }

    #[test]
    fn test_router_asymmetric_page_transition() {
        let mut ctx = Context::new();

        let page1 = column((text::<_, ()>("Page 1"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));
        let page2 = column((text::<_, ()>("Page 2"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));

        let trans = PageTransition::asymmetric(Motion::slide_in_right(), Motion::fade_out())
            .duration_ms(300.0);

        let router1 = router("a", page1).transition(trans.clone());
        let router2 = router("b", page2).transition(trans);

        let mut el = View::<()>::build(&router1, &mut ctx);
        let container = View::<()>::get_node(&router1, &el);
        ctx.root_attach(container);
        ctx.compute_layout(1000.0, 500.0);

        View::<()>::rebuild(&router2, &router1, &mut ctx, &mut el);
        assert!(el.outgoing.is_some());

        // At progress 0:
        // Incoming view is entering with slide_in_right, so left = 1000.0
        let new_cons = el.current_node.get_constraints(&ctx).unwrap();
        if let PositionStrategy::Absolute { left, .. } = new_cons.positioning {
            assert_eq!(left, 1000.0);
        } else {
            panic!("Expected Absolute positioning for incoming slide");
        }

        // Outgoing view is fading out, remaining stationary at top=0, left=0
        let (out_node, _) = el.outgoing.as_ref().unwrap();
        let out_cons = out_node.get_constraints(&ctx).unwrap();
        if let PositionStrategy::Absolute { left, top, .. } = out_cons.positioning {
            assert_eq!(left, 0.0);
            assert_eq!(top, 0.0);
        } else {
            panic!("Expected Absolute positioning for outgoing fade");
        }
    }

    #[test]
    fn test_router_transition_order_z_index() {
        let mut ctx = Context::new();

        let page1 = column((text::<_, ()>("Page 1"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));
        let page2 = column((text::<_, ()>("Page 2"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));

        // Test OutgoingOnTop (e.g. pop)
        let pop_trans = PageTransition::pop().duration_ms(200.0);
        let router1 = router("a", page1).transition(pop_trans.clone());
        let router2 = router("b", page2).transition(pop_trans);

        let mut el = View::<()>::build(&router1, &mut ctx);
        let container = View::<()>::get_node(&router1, &el);
        ctx.root_attach(container);
        ctx.compute_layout(800.0, 600.0);

        View::<()>::rebuild(&router2, &router1, &mut ctx, &mut el);
        assert!(el.outgoing.is_some());

        let (out_node, _) = el.outgoing.as_ref().unwrap();
        // OutgoingOnTop => out_node has z_index = 1, current_node has z_index = 0
        assert_eq!(out_node.get_constraints(&ctx).unwrap().z_index, 1);
        assert_eq!(el.current_node.get_constraints(&ctx).unwrap().z_index, 0);

        // Advance to completion
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&router2, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);
        assert!(el.outgoing.is_none());
        // Upon completion, current_node z_index resets to 0
        assert_eq!(el.current_node.get_constraints(&ctx).unwrap().z_index, 0);
    }

    #[test]
    fn test_router_transition_spec() {
        let mut ctx = Context::new();

        let page1 = column((text::<_, ()>("Page 1"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));
        let page2 = column((text::<_, ()>("Page 2"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));
        let page3 = column((text::<_, ()>("Page 3"),))
            .style(Style::new().width(Size::Fill).height(Size::Fill));

        let make_router = |idx: usize, page| {
            router(idx, page).transition_spec(|from, to| {
                if to > from {
                    PageTransition::push().duration_ms(200.0)
                } else {
                    PageTransition::pop().duration_ms(200.0)
                }
            })
        };

        let r1 = make_router(1, page1);
        let r2 = make_router(2, page2);
        let r3 = make_router(1, page3);

        let mut el = View::<()>::build(&r1, &mut ctx);
        let container = View::<()>::get_node(&r1, &el);
        ctx.root_attach(container);
        ctx.compute_layout(1000.0, 500.0);

        // 1 -> 2 is forward: should select push() (IncomingOnTop, incoming starts at left=1000)
        View::<()>::rebuild(&r2, &r1, &mut ctx, &mut el);
        assert!(el.outgoing.is_some());
        let (out_node, _) = el.outgoing.as_ref().unwrap();
        assert_eq!(el.current_node.get_constraints(&ctx).unwrap().z_index, 1);
        assert_eq!(out_node.get_constraints(&ctx).unwrap().z_index, 0);

        // Complete the forward transition
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&r2, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);
        assert!(el.outgoing.is_none());

        // 2 -> 1 is backward: should select pop() (OutgoingOnTop, out_node has z_index=1)
        View::<()>::rebuild(&r3, &r2, &mut ctx, &mut el);
        assert!(el.outgoing.is_some());
        let (out_node2, _) = el.outgoing.as_ref().unwrap();
        assert_eq!(out_node2.get_constraints(&ctx).unwrap().z_index, 1);
        assert_eq!(el.current_node.get_constraints(&ctx).unwrap().z_index, 0);
    }
}
