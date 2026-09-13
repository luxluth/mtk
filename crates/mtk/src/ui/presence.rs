//! Conditional presence container with animated enter and exit motions.
//!
//! Provides [`presence`] and [`PresenceViewExt`], which mount and animate any widget
//! into or out of view when a boolean condition changes.

use std::marker::PhantomData;
use std::time::Instant;

use crate::animation::{AnimatedValue, Curve};
use crate::debugger::SourceLocation;
use crate::style::{Overflow, PositionStrategy, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::transition::Motion;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A widget wrapper that animates a child view when it enters or exits the hierarchy.
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
    /// Wraps this view in a [`Presence`] container animated by enter and exit motions.
    fn presence(self, is_visible: bool) -> Presence<Self, ()>;
}

impl<V> PresenceViewExt for V {
    fn presence(self, is_visible: bool) -> Presence<Self, ()> {
        presence(is_visible, self)
    }
}

pub struct PresenceElement<V: View<State>, State> {
    container_node: Node,
    is_visible: bool,
    child: Option<(Node, V::Element)>,
    outgoing: Option<(Node, V::Element)>,
    anim_progress: AnimatedValue<f32>,
    anim_start: Instant,
    is_entering: bool,
    child_orig_positioning: PositionStrategy,
    _marker: PhantomData<State>,
}

fn apply_motion_step(
    ctx: &mut Context,
    motion: &Motion,
    progress: f32,
    node: Node,
    width: f32,
    height: f32,
) {
    let offset_x = motion.resolve_offset_x(progress, width);
    let offset_y = motion.resolve_offset_y(progress, height);
    let opacity = motion.resolve_opacity(progress);
    let scale = motion.resolve_scale(progress);

    let has_offset = offset_x.abs() > 1e-3 || offset_y.abs() > 1e-3;
    if has_offset {
        node.update_constraints(ctx, |c| {
            c.positioning = PositionStrategy::Absolute {
                top: offset_y,
                left: offset_x,
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

impl<V, State, Msg> View<State> for Presence<V, Msg>
where
    V: View<State, Message = Msg>,
{
    type Element = PresenceElement<V, State>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }

        Style::new()
            .width(Size::Fit)
            .height(Size::Fit)
            .overflow(Overflow::Visible)
            .apply_to_node(ctx, container_node);

        let (child, child_orig_positioning) = if self.is_visible {
            let el = self.view.build(ctx);
            let node = self.view.get_node(&el);
            let orig_pos = node
                .get_constraints(ctx)
                .map(|c| c.positioning)
                .unwrap_or(PositionStrategy::Inflow);
            container_node.append(ctx, node);
            (Some((node, el)), orig_pos)
        } else {
            (None, PositionStrategy::Inflow)
        };

        PresenceElement {
            container_node,
            is_visible: self.is_visible,
            child,
            outgoing: None,
            anim_progress: AnimatedValue::new(1.0),
            anim_start: Instant::now(),
            is_entering: false,
            child_orig_positioning,
            _marker: PhantomData,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        match (self.is_visible, element.is_visible) {
            (true, true) => {
                if let Some((_, ref mut el)) = element.child {
                    self.view.rebuild(&prev.view, ctx, el);
                }
            }
            (false, false) => {}
            (true, false) => {
                // Enter transition: clean up any pending outgoing node first
                if let Some((out_node, _)) = element.outgoing.take() {
                    out_node.remove(ctx);
                    ctx.destroy_node(out_node);
                }

                let el = self.view.build(ctx);
                let node = self.view.get_node(&el);
                let orig_pos = node
                    .get_constraints(ctx)
                    .map(|c| c.positioning)
                    .unwrap_or(PositionStrategy::Inflow);
                element.child_orig_positioning = orig_pos;
                element.container_node.append(ctx, node);
                element.child = Some((node, el));
                element.is_visible = true;
                element.is_entering = true;

                if self.duration_ms > 0.0 {
                    element.anim_progress = AnimatedValue::new(0.0);
                    element.anim_start = Instant::now();
                    element
                        .anim_progress
                        .set_target(1.0, 0.0, self.duration_ms as f64, self.curve);

                    let bounds = element.container_node.get_computed(ctx);
                    let (w, h) = bounds.map(|c| (c.w, c.h)).unwrap_or((100.0, 100.0));
                    apply_motion_step(ctx, &self.enter_motion, 0.0, node, w.max(1.0), h.max(1.0));
                    ctx.request_frame();
                } else {
                    node.update_effects(ctx, |eff| {
                        eff.opacity = 1.0;
                        eff.scale = 1.0;
                    });
                }
            }
            (false, true) => {
                // Exit transition: move current child to outgoing
                if let Some((node, el)) = element.child.take() {
                    element.outgoing = Some((node, el));
                    element.is_visible = false;
                    element.is_entering = false;

                    if self.duration_ms > 0.0 {
                        element.anim_progress = AnimatedValue::new(0.0);
                        element.anim_start = Instant::now();
                        element.anim_progress.set_target(
                            1.0,
                            0.0,
                            self.duration_ms as f64,
                            self.curve,
                        );

                        let bounds = element.container_node.get_computed(ctx);
                        let (w, h) = bounds.map(|c| (c.w, c.h)).unwrap_or((100.0, 100.0));
                        apply_motion_step(
                            ctx,
                            &self.exit_motion,
                            0.0,
                            node,
                            w.max(1.0),
                            h.max(1.0),
                        );
                        ctx.request_frame();
                    } else {
                        node.remove(ctx);
                        ctx.destroy_node(node);
                        element.outgoing = None;
                    }
                }
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        if let Some((out_node, _)) = element.outgoing.take() {
            out_node.remove(ctx);
            ctx.destroy_node(out_node);
        }
        if let Some((node, mut el)) = element.child.take() {
            self.view.teardown(ctx, &mut el);
            node.remove(ctx);
            ctx.destroy_node(node);
        }
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
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            if element.anim_progress.is_animating() {
                element.anim_progress.tick(now);
                let progress = element.anim_progress.get();
                let animating = element.anim_progress.is_animating();

                let bounds = element.container_node.get_computed(ctx);
                let (w, h) = bounds.map(|c| (c.w, c.h)).unwrap_or((100.0, 100.0));
                let w = w.max(1.0);
                let h = h.max(1.0);

                if element.is_entering {
                    if let Some((node, _)) = &element.child {
                        apply_motion_step(ctx, &self.enter_motion, progress, *node, w, h);
                        if !animating || progress >= 0.999 {
                            node.update_constraints(ctx, |c| {
                                c.positioning = element.child_orig_positioning;
                            });
                            node.update_effects(ctx, |eff| {
                                eff.opacity = 1.0;
                                eff.scale = 1.0;
                            });
                        } else {
                            ctx.request_frame();
                        }
                    }
                } else if let Some((out_node, _)) = &element.outgoing {
                    apply_motion_step(ctx, &self.exit_motion, progress, *out_node, w, h);
                    if !animating || progress >= 0.999 {
                        if let Some((node, mut el)) = element.outgoing.take() {
                            self.view.teardown(ctx, &mut el);
                            node.remove(ctx);
                            ctx.destroy_node(node);
                        }
                    } else {
                        ctx.request_frame();
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
}

#[cfg(test)]
mod tests {
    use super::*;
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

        View::<()>::rebuild(&p_visible, &p_hidden, &mut ctx, &mut el);
        assert!(el.child.is_some());
        assert!(el.is_entering);

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

        // When switching to hidden, node moves to outgoing and stays alive
        View::<()>::rebuild(&p_hidden, &p_visible, &mut ctx, &mut el);
        assert!(el.child.is_none());
        assert!(el.outgoing.is_some());

        let out_node = el.outgoing.as_ref().unwrap().0;
        let eff = out_node.get_effects(&ctx).unwrap();
        assert_eq!(eff.opacity, 1.0);

        // Advance past duration
        std::thread::sleep(std::time::Duration::from_millis(250));
        View::<()>::handle_event(&p_hidden, &mut el, &(), Event::Tick { dt: 0.25 }, &mut ctx);

        // Deferred unmount now tears down and destroys outgoing node
        assert!(el.outgoing.is_none());
    }

    #[test]
    fn test_presence_extension_trait() {
        let mut ctx = Context::new();
        let view = text::<_, ()>("Hello").presence(true);
        let el = View::<()>::build(&view, &mut ctx);
        assert!(el.child.is_some());
    }
}
