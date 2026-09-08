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
use crate::ui::transition::Transition;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A router widget that smoothly animates between pages/views when its route key changes.
pub struct Router<Key, V, Msg> {
    pub(crate) key: Key,
    pub(crate) view: V,
    pub(crate) transition: Transition,
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
///     .transition(Transition::fade())
/// ```
#[track_caller]
pub fn router<Key, V, Msg>(key: Key, view: V) -> Router<Key, V, Msg>
where
    Key: PartialEq + Clone + 'static,
{
    Router {
        key,
        view,
        transition: Transition::fade(),
        source_loc: Some(SourceLocation::here("Router")),
        _marker: PhantomData,
    }
}

impl<Key, V, Msg> Router<Key, V, Msg> {
    /// Sets the transition animation physics (fade, slide, etc.) when switching views.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = transition;
        self
    }
}

pub struct RouterElement<Key, V: View<State>, State> {
    container_node: Node,
    active_key: Key,
    current_node: Node,
    current_el: V::Element,
    outgoing: Option<(Node, V::Element)>,
    anim_progress: AnimatedValue<f32>,
    anim_start: Instant,
    current_orig_positioning: PositionStrategy,
    _marker: PhantomData<State>,
}

fn apply_transition_step(
    ctx: &mut Context,
    transition: &Transition,
    progress: f32,
    out_node: Node,
    current_node: Node,
    screen_w: f32,
    screen_h: f32,
) {
    match transition {
        Transition::Fade { .. } => {
            out_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            out_node.update_effects(ctx, |eff| {
                eff.opacity = (1.0 - progress).clamp(0.0, 1.0);
            });
            current_node.update_effects(ctx, |eff| {
                eff.opacity = progress.clamp(0.0, 1.0);
            });
        }
        Transition::SlideRight { .. } => {
            let out_offset = -progress * screen_w;
            let in_offset = (1.0 - progress) * screen_w;
            out_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: out_offset,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: in_offset,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_effects(ctx, |eff| {
                eff.opacity = progress.clamp(0.0, 1.0);
            });
        }
        Transition::SlideLeft { .. } => {
            let out_offset = progress * screen_w;
            let in_offset = -(1.0 - progress) * screen_w;
            out_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: out_offset,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: in_offset,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_effects(ctx, |eff| {
                eff.opacity = progress.clamp(0.0, 1.0);
            });
        }
        Transition::SlideUp { .. } => {
            let in_offset = (1.0 - progress) * screen_h;
            out_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: in_offset,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            out_node.update_effects(ctx, |eff| {
                eff.opacity = (1.0 - progress * 0.5).clamp(0.0, 1.0);
            });
            current_node.update_effects(ctx, |eff| {
                eff.opacity = progress.clamp(0.0, 1.0);
            });
        }
        Transition::SlideDown { .. } => {
            let in_offset = -(1.0 - progress) * screen_h;
            out_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: in_offset,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            out_node.update_effects(ctx, |eff| {
                eff.opacity = (1.0 - progress * 0.5).clamp(0.0, 1.0);
            });
            current_node.update_effects(ctx, |eff| {
                eff.opacity = progress.clamp(0.0, 1.0);
            });
        }
        Transition::Scale { from_scale, .. } => {
            out_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            out_node.update_effects(ctx, |eff| {
                eff.opacity = (1.0 - progress).clamp(0.0, 1.0);
            });
            let current_scale = from_scale + (1.0 - from_scale) * progress;
            current_node.update_effects(ctx, |eff| {
                eff.opacity = progress.clamp(0.0, 1.0);
                eff.scale = current_scale;
            });
        }
        _ => {
            out_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            current_node.update_constraints(ctx, |c| {
                c.positioning = PositionStrategy::Absolute {
                    top: 0.0,
                    left: 0.0,
                    bottom: f32::NAN,
                    right: f32::NAN,
                };
            });
            out_node.update_effects(ctx, |eff| {
                eff.opacity = (1.0 - progress).clamp(0.0, 1.0);
            });
            current_node.update_effects(ctx, |eff| {
                eff.opacity = progress.clamp(0.0, 1.0);
            });
        }
    }
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
        let current_orig_positioning = current_node
            .get_constraints(ctx)
            .map(|c| c.positioning)
            .unwrap_or(PositionStrategy::Inflow);
        container_node.append(ctx, current_node);

        RouterElement {
            container_node,
            active_key: self.key.clone(),
            current_node,
            current_el,
            outgoing: None,
            anim_progress: AnimatedValue::new(1.0),
            anim_start: Instant::now(),
            current_orig_positioning,
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

            // Move current view to outgoing slot
            let old_node = element.current_node;
            let old_el = std::mem::replace(&mut element.current_el, self.view.build(ctx));
            element.outgoing = Some((old_node, old_el));

            // Setup new current view
            let new_node = self.view.get_node(&element.current_el);
            let new_orig_positioning = new_node
                .get_constraints(ctx)
                .map(|c| c.positioning)
                .unwrap_or(PositionStrategy::Inflow);
            element.current_node = new_node;
            element.current_orig_positioning = new_orig_positioning;
            element.active_key = self.key.clone();
            element.container_node.append(ctx, new_node);

            // Configure transition animation
            let duration = self.transition.duration_ms();
            let curve = self.transition.curve();

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
                    &self.transition,
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
                        &self.transition,
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
}
