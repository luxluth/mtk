//! High-performance page and view router with animated transitions.
//!
//! Provides [`router`], which mounts and animates between different views whenever its
//! route key changes.

use std::marker::PhantomData;
use std::time::Instant;

use crate::animation::AnimatedValue;
use crate::style::{PositionStrategy, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::transition::Transition;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A router widget that smoothly animates between pages/views when its route key changes.
pub struct Router<Key, V, Msg> {
    pub(crate) key: Key,
    pub(crate) view: V,
    pub(crate) transition: Transition,
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
pub fn router<Key, V, Msg>(key: Key, view: V) -> Router<Key, V, Msg>
where
    Key: PartialEq + Clone + 'static,
{
    Router {
        key,
        view,
        transition: Transition::fade(),
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
    _marker: PhantomData<State>,
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

        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .apply_to_node(ctx, container_node);

        let current_el = self.view.build(ctx);
        let current_node = self.view.get_node(&current_el);
        container_node.append(ctx, current_node);

        RouterElement {
            container_node,
            active_key: self.key.clone(),
            current_node,
            current_el,
            outgoing: None,
            anim_progress: AnimatedValue::new(1.0),
            anim_start: Instant::now(),
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
            element.current_node = new_node;
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

                // Set initial opacities
                new_node.update_effects(ctx, |eff| {
                    eff.opacity = 0.0;
                });
                old_node.update_effects(ctx, |eff| {
                    eff.opacity = 1.0;
                });

                ctx.request_frame();
            } else {
                if let Some((out_node, _)) = element.outgoing.take() {
                    out_node.remove(ctx);
                    ctx.destroy_node(out_node);
                }
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

                let screen_w = element
                    .container_node
                    .get_computed(ctx)
                    .map(|c| c.w)
                    .unwrap_or(1200.0)
                    .max(600.0);
                let screen_h = element
                    .container_node
                    .get_computed(ctx)
                    .map(|c| c.h)
                    .unwrap_or(800.0)
                    .max(400.0);

                if let Some((out_node, _)) = element.outgoing.as_ref() {
                    match self.transition {
                        Transition::Fade { .. } => {
                            out_node.update_effects(ctx, |eff| {
                                eff.opacity = (1.0 - progress).clamp(0.0, 1.0);
                            });
                            element.current_node.update_effects(ctx, |eff| {
                                eff.opacity = progress.clamp(0.0, 1.0);
                            });
                        }
                        Transition::SlideRight { .. } => {
                            let out_offset = -progress * screen_w;
                            let in_offset = (1.0 - progress) * screen_w;
                            Style::new()
                                .position(PositionStrategy::Absolute {
                                    top: 0.0,
                                    left: out_offset,
                                    bottom: 0.0,
                                    right: 0.0,
                                })
                                .apply_to_node(ctx, *out_node);
                            Style::new()
                                .position(PositionStrategy::Absolute {
                                    top: 0.0,
                                    left: in_offset,
                                    bottom: 0.0,
                                    right: 0.0,
                                })
                                .apply_to_node(ctx, element.current_node);
                            element.current_node.update_effects(ctx, |eff| {
                                eff.opacity = progress.clamp(0.0, 1.0);
                            });
                        }
                        Transition::SlideUp { .. } => {
                            let in_offset = (1.0 - progress) * screen_h;
                            Style::new()
                                .position(PositionStrategy::Absolute {
                                    top: in_offset,
                                    left: 0.0,
                                    bottom: 0.0,
                                    right: 0.0,
                                })
                                .apply_to_node(ctx, element.current_node);
                            out_node.update_effects(ctx, |eff| {
                                eff.opacity = (1.0 - progress).clamp(0.0, 1.0);
                            });
                            element.current_node.update_effects(ctx, |eff| {
                                eff.opacity = progress.clamp(0.0, 1.0);
                            });
                        }
                        _ => {
                            out_node.update_effects(ctx, |eff| {
                                eff.opacity = (1.0 - progress).clamp(0.0, 1.0);
                            });
                            element.current_node.update_effects(ctx, |eff| {
                                eff.opacity = progress.clamp(0.0, 1.0);
                            });
                        }
                    }
                }

                // If transition finished, clean up outgoing view
                if !animating || progress >= 0.999 {
                    if let Some((out_node, _)) = element.outgoing.take() {
                        out_node.remove(ctx);
                        ctx.destroy_node(out_node);
                    }
                    element.current_node.update_effects(ctx, |eff| {
                        eff.opacity = 1.0;
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
