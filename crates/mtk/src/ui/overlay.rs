//! Anchored flyouts and floating overlays that escape ancestor clipping containers.
//!
//! Provides [`overlay`], [`Overlay`], and [`ViewOverlayExt`] for popovers, dropdowns,
//! and context menus that position themselves relative to an anchor or cursor coordinates
//! with collision detection (flipping/clamping) and presence enter/exit motions.

use std::marker::PhantomData;
use std::time::Instant;

use crate::animation::{AnimatedValue, Curve};
use crate::debugger::SourceLocation;
use crate::style::{Overflow, PositionStrategy, Rect, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::transition::Motion;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// Target anchor alignment for floating overlay placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OverlayPlacement {
    /// Placed below the anchor, aligned to its leading/left edge.
    #[default]
    BottomStart,
    /// Placed below the anchor, centered horizontally.
    Bottom,
    /// Placed below the anchor, aligned to its trailing/right edge.
    BottomEnd,
    /// Placed above the anchor, aligned to its leading/left edge.
    TopStart,
    /// Placed above the anchor, centered horizontally.
    Top,
    /// Placed above the anchor, aligned to its trailing/right edge.
    TopEnd,
    /// Placed to the left of the anchor, aligned to its top edge.
    LeftStart,
    /// Placed to the left of the anchor, centered vertically.
    Left,
    /// Placed to the left of the anchor, aligned to its bottom edge.
    LeftEnd,
    /// Placed to the right of the anchor, aligned to its top edge.
    RightStart,
    /// Placed to the right of the anchor, centered vertically.
    Right,
    /// Placed to the right of the anchor, aligned to its bottom edge.
    RightEnd,
}

/// Calculates relative `(left, top)` coordinates for a floating overlay against its anchor
/// or cursor reference point, handling viewport collision flipping and edge clamping.
pub fn compute_overlay_position(
    placement: OverlayPlacement,
    offset: f32,
    anchor_bounds: Rect,
    anchor_point: Option<(f32, f32)>,
    flyout_w: f32,
    flyout_h: f32,
    viewport_w: f32,
    viewport_h: f32,
    auto_flip: bool,
    auto_clamp: bool,
    margin: f32,
) -> (f32, f32, OverlayPlacement) {
    let (ref_x, ref_y, ref_w, ref_h) = if let Some((px, py)) = anchor_point {
        (px, py, 0.0, 0.0)
    } else {
        (
            anchor_bounds.x,
            anchor_bounds.y,
            anchor_bounds.w,
            anchor_bounds.h,
        )
    };

    let mut resolved = placement;

    let compute_coords = |p: OverlayPlacement| -> (f32, f32) {
        match p {
            OverlayPlacement::BottomStart => (ref_x, ref_y + ref_h + offset),
            OverlayPlacement::Bottom => (ref_x + (ref_w - flyout_w) * 0.5, ref_y + ref_h + offset),
            OverlayPlacement::BottomEnd => (ref_x + ref_w - flyout_w, ref_y + ref_h + offset),
            OverlayPlacement::TopStart => (ref_x, ref_y - flyout_h - offset),
            OverlayPlacement::Top => (ref_x + (ref_w - flyout_w) * 0.5, ref_y - flyout_h - offset),
            OverlayPlacement::TopEnd => (ref_x + ref_w - flyout_w, ref_y - flyout_h - offset),
            OverlayPlacement::LeftStart => (ref_x - flyout_w - offset, ref_y),
            OverlayPlacement::Left => (ref_x - flyout_w - offset, ref_y + (ref_h - flyout_h) * 0.5),
            OverlayPlacement::LeftEnd => (ref_x - flyout_w - offset, ref_y + ref_h - flyout_h),
            OverlayPlacement::RightStart => (ref_x + ref_w + offset, ref_y),
            OverlayPlacement::Right => (ref_x + ref_w + offset, ref_y + (ref_h - flyout_h) * 0.5),
            OverlayPlacement::RightEnd => (ref_x + ref_w + offset, ref_y + ref_h - flyout_h),
        }
    };

    let (mut gx, mut gy) = compute_coords(resolved);

    if auto_flip {
        match resolved {
            OverlayPlacement::BottomStart
            | OverlayPlacement::Bottom
            | OverlayPlacement::BottomEnd => {
                let bottom_overflow = gy + flyout_h > viewport_h - margin;
                let top_space = ref_y - margin;
                let bottom_space = viewport_h - margin - (ref_y + ref_h);
                if bottom_overflow && top_space > bottom_space {
                    resolved = match resolved {
                        OverlayPlacement::BottomStart => OverlayPlacement::TopStart,
                        OverlayPlacement::Bottom => OverlayPlacement::Top,
                        OverlayPlacement::BottomEnd => OverlayPlacement::TopEnd,
                        _ => resolved,
                    };
                    let (_, new_y) = compute_coords(resolved);
                    gy = new_y;
                }
            }
            OverlayPlacement::TopStart | OverlayPlacement::Top | OverlayPlacement::TopEnd => {
                let top_overflow = gy < margin;
                let top_space = ref_y - margin;
                let bottom_space = viewport_h - margin - (ref_y + ref_h);
                if top_overflow && bottom_space > top_space {
                    resolved = match resolved {
                        OverlayPlacement::TopStart => OverlayPlacement::BottomStart,
                        OverlayPlacement::Top => OverlayPlacement::Bottom,
                        OverlayPlacement::TopEnd => OverlayPlacement::BottomEnd,
                        _ => resolved,
                    };
                    let (_, new_y) = compute_coords(resolved);
                    gy = new_y;
                }
            }
            OverlayPlacement::RightStart | OverlayPlacement::Right | OverlayPlacement::RightEnd => {
                let right_overflow = gx + flyout_w > viewport_w - margin;
                let left_space = ref_x - margin;
                let right_space = viewport_w - margin - (ref_x + ref_w);
                if right_overflow && left_space > right_space {
                    resolved = match resolved {
                        OverlayPlacement::RightStart => OverlayPlacement::LeftStart,
                        OverlayPlacement::Right => OverlayPlacement::Left,
                        OverlayPlacement::RightEnd => OverlayPlacement::LeftEnd,
                        _ => resolved,
                    };
                    let (new_x, _) = compute_coords(resolved);
                    gx = new_x;
                }
            }
            OverlayPlacement::LeftStart | OverlayPlacement::Left | OverlayPlacement::LeftEnd => {
                let left_overflow = gx < margin;
                let left_space = ref_x - margin;
                let right_space = viewport_w - margin - (ref_x + ref_w);
                if left_overflow && right_space > left_space {
                    resolved = match resolved {
                        OverlayPlacement::LeftStart => OverlayPlacement::RightStart,
                        OverlayPlacement::Left => OverlayPlacement::Right,
                        OverlayPlacement::LeftEnd => OverlayPlacement::RightEnd,
                        _ => resolved,
                    };
                    let (new_x, _) = compute_coords(resolved);
                    gx = new_x;
                }
            }
        }
    }

    if auto_clamp {
        let min_x = margin;
        let max_x = (viewport_w - margin - flyout_w).max(min_x);
        gx = gx.clamp(min_x, max_x);

        let min_y = margin;
        let max_y = (viewport_h - margin - flyout_h).max(min_y);
        gy = gy.clamp(min_y, max_y);
    }

    let rel_x = gx - anchor_bounds.x;
    let rel_y = gy - anchor_bounds.y;

    (rel_x, rel_y, resolved)
}

/// A floating overlay view anchored to a base component or cursor position, escaping ancestor clipping.
pub struct Overlay<AnchorV, FlyoutV, Msg, F = fn() -> Msg> {
    pub(crate) anchor_view: AnchorV,
    pub(crate) flyout_view: FlyoutV,
    pub(crate) is_open: bool,
    pub(crate) placement: OverlayPlacement,
    pub(crate) offset: f32,
    pub(crate) anchor_point: Option<(f32, f32)>,
    pub(crate) auto_flip: bool,
    pub(crate) auto_clamp: bool,
    pub(crate) viewport_margin: f32,
    pub(crate) close_on_escape: bool,
    pub(crate) close_on_click_outside: bool,
    pub(crate) on_dismiss: Option<F>,
    pub(crate) enter_motion: Motion,
    pub(crate) exit_motion: Motion,
    pub(crate) duration_ms: f32,
    pub(crate) curve: Curve,
    pub(crate) source_loc: Option<SourceLocation>,
    pub(crate) _marker: PhantomData<Msg>,
}

/// Creates a new floating [`Overlay`] view wrapping `anchor_view` and anchoring `flyout_view`.
#[track_caller]
pub fn overlay<AnchorV, FlyoutV, Msg>(
    anchor_view: AnchorV,
    flyout_view: FlyoutV,
) -> Overlay<AnchorV, FlyoutV, Msg, fn() -> Msg> {
    Overlay {
        anchor_view,
        flyout_view,
        is_open: false,
        placement: OverlayPlacement::BottomStart,
        offset: 4.0,
        anchor_point: None,
        auto_flip: true,
        auto_clamp: true,
        viewport_margin: 8.0,
        close_on_escape: true,
        close_on_click_outside: true,
        on_dismiss: None,
        enter_motion: Motion::fade_in(),
        exit_motion: Motion::fade_out(),
        duration_ms: 150.0,
        curve: Curve::ease_out(),
        source_loc: Some(SourceLocation::here("Overlay")),
        _marker: PhantomData,
    }
}

impl<AnchorV, FlyoutV, Msg, F> Overlay<AnchorV, FlyoutV, Msg, F> {
    /// Sets whether the floating flyout is currently open.
    pub fn is_open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Sets the preferred placement orientation relative to the anchor.
    pub fn placement(mut self, placement: OverlayPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets pixel offset distance between anchor and flyout.
    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    /// Explicitly anchors the overlay to a specific window point `(x, y)` instead of the widget bounding box.
    pub fn anchor_point(mut self, point: Option<(f32, f32)>) -> Self {
        self.anchor_point = point;
        self
    }

    /// Sets whether collision detection flips the placement when near window edges.
    pub fn auto_flip(mut self, auto_flip: bool) -> Self {
        self.auto_flip = auto_flip;
        self
    }

    /// Sets whether the overlay is clamped to prevent overflowing the viewport boundaries.
    pub fn auto_clamp(mut self, auto_clamp: bool) -> Self {
        self.auto_clamp = auto_clamp;
        self
    }

    /// Sets the minimum distance in logical pixels maintained between the flyout and the viewport edges.
    pub fn viewport_margin(mut self, margin: f32) -> Self {
        self.viewport_margin = margin;
        self
    }

    /// Configures whether pressing Escape automatically triggers dismissal.
    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    /// Configures whether clicking outside both the anchor and flyout triggers dismissal.
    pub fn close_on_click_outside(mut self, close: bool) -> Self {
        self.close_on_click_outside = close;
        self
    }

    /// Sets the callback invoked when the overlay is dismissed via Escape or outside click.
    pub fn on_dismiss<NewF: Fn() -> Msg>(
        self,
        on_dismiss: NewF,
    ) -> Overlay<AnchorV, FlyoutV, Msg, NewF> {
        Overlay {
            anchor_view: self.anchor_view,
            flyout_view: self.flyout_view,
            is_open: self.is_open,
            placement: self.placement,
            offset: self.offset,
            anchor_point: self.anchor_point,
            auto_flip: self.auto_flip,
            auto_clamp: self.auto_clamp,
            viewport_margin: self.viewport_margin,
            close_on_escape: self.close_on_escape,
            close_on_click_outside: self.close_on_click_outside,
            on_dismiss: Some(on_dismiss),
            enter_motion: self.enter_motion,
            exit_motion: self.exit_motion,
            duration_ms: self.duration_ms,
            curve: self.curve,
            source_loc: self.source_loc,
            _marker: PhantomData,
        }
    }

    /// Sets the enter animation motion.
    pub fn enter(mut self, motion: Motion) -> Self {
        self.enter_motion = motion;
        self
    }

    /// Sets the exit animation motion.
    pub fn exit(mut self, motion: Motion) -> Self {
        self.exit_motion = motion;
        self
    }

    /// Sets transition duration in milliseconds.
    pub fn duration_ms(mut self, duration_ms: f32) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Sets easing curve for transition animations.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }
}

/// Persistent state element for an [`Overlay`].
pub struct OverlayElement<AnchorEl, FlyoutEl> {
    container_node: Node,
    anchor_element: AnchorEl,
    anchor_node: Node,
    flyout_element: Option<(Node, FlyoutEl)>,
    outgoing_flyout: Option<(Node, FlyoutEl)>,
    is_open: bool,
    anim_progress: AnimatedValue<f32>,
    anim_start: Instant,
    is_entering: bool,
    base_pos: (f32, f32),
    resolved_placement: OverlayPlacement,
}

fn apply_overlay_motion_step(
    ctx: &mut Context,
    motion: &Motion,
    progress: f32,
    node: Node,
    base_top: f32,
    base_left: f32,
    width: f32,
    height: f32,
) {
    let offset_x = motion.resolve_offset_x(progress, width);
    let offset_y = motion.resolve_offset_y(progress, height);
    let opacity = motion.resolve_opacity(progress);
    let scale = motion.resolve_scale(progress);

    node.update_constraints(ctx, |c| {
        c.positioning = PositionStrategy::Absolute {
            top: base_top + offset_y,
            left: base_left + offset_x,
            bottom: f32::NAN,
            right: f32::NAN,
        };
        c.unclipped = true;
        c.z_index = 9000;
    });

    node.update_effects(ctx, |eff| {
        eff.opacity = opacity.clamp(0.0, 1.0);
        eff.scale = scale;
    });
}

fn update_overlay_geometry<AnchorEl, FlyoutEl, AnchorV, FlyoutV, Msg, F>(
    overlay: &Overlay<AnchorV, FlyoutV, Msg, F>,
    ctx: &mut Context,
    element: &mut OverlayElement<AnchorEl, FlyoutEl>,
) {
    let target = element
        .flyout_element
        .as_ref()
        .map(|(n, _)| *n)
        .or_else(|| element.outgoing_flyout.as_ref().map(|(n, _)| *n));

    if let Some(node) = target {
        let anchor_bounds = element
            .container_node
            .get_computed(ctx)
            .map(|c| Rect::new(c.x, c.y, c.w, c.h))
            .unwrap_or(Rect::new(0.0, 0.0, 100.0, 30.0));

        let flyout_bounds = node
            .get_computed(ctx)
            .map(|c| Rect::new(c.x, c.y, c.w, c.h))
            .unwrap_or(Rect::new(0.0, 0.0, 150.0, 100.0));

        let (vw, vh) = ctx.viewport_size();

        let (rel_x, rel_y, placement) = compute_overlay_position(
            overlay.placement,
            overlay.offset,
            anchor_bounds,
            overlay.anchor_point,
            flyout_bounds.w.max(1.0),
            flyout_bounds.h.max(1.0),
            vw,
            vh,
            overlay.auto_flip,
            overlay.auto_clamp,
            overlay.viewport_margin,
        );

        element.base_pos = (rel_x, rel_y);
        element.resolved_placement = placement;

        let progress = element.anim_progress.get();
        let motion = if element.is_entering {
            &overlay.enter_motion
        } else {
            &overlay.exit_motion
        };

        apply_overlay_motion_step(
            ctx,
            motion,
            progress,
            node,
            rel_y,
            rel_x,
            flyout_bounds.w.max(1.0),
            flyout_bounds.h.max(1.0),
        );
    }
}

impl<State, AnchorV, FlyoutV, Msg, F> View<State> for Overlay<AnchorV, FlyoutV, Msg, F>
where
    AnchorV: View<State, Message = Msg>,
    FlyoutV: View<State, Message = Msg>,
    F: Fn() -> Msg + 'static,
    Msg: 'static,
{
    type Element = OverlayElement<AnchorV::Element, FlyoutV::Element>;
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

        let anchor_element = self.anchor_view.build(ctx);
        let anchor_node = self.anchor_view.get_node(&anchor_element);
        container_node.append(ctx, anchor_node);

        let mut element = OverlayElement {
            container_node,
            anchor_element,
            anchor_node,
            flyout_element: None,
            outgoing_flyout: None,
            is_open: false,
            anim_progress: AnimatedValue::new(1.0),
            anim_start: Instant::now(),
            is_entering: false,
            base_pos: (0.0, 0.0),
            resolved_placement: self.placement,
        };

        if self.is_open {
            let fl_el = self.flyout_view.build(ctx);
            let fl_node = self.flyout_view.get_node(&fl_el);

            fl_node.update_constraints(ctx, |c| {
                c.unclipped = true;
                c.z_index = 9000;
            });

            container_node.append(ctx, fl_node);
            element.flyout_element = Some((fl_node, fl_el));
            element.is_open = true;
            element.is_entering = true;
            ctx.show_overlay(fl_node);

            update_overlay_geometry(self, ctx, &mut element);
        }

        element
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.anchor_view
            .rebuild(&prev.anchor_view, ctx, &mut element.anchor_element);

        match (self.is_open, element.is_open) {
            (true, true) => {
                if let Some((fl_node, ref mut fl_el)) = element.flyout_element {
                    self.flyout_view.rebuild(&prev.flyout_view, ctx, fl_el);
                    fl_node.update_constraints(ctx, |c| {
                        c.unclipped = true;
                        c.z_index = 9000;
                    });
                }
                update_overlay_geometry(self, ctx, element);
            }
            (false, false) => {}
            (true, false) => {
                if let Some((out_node, mut out_el)) = element.outgoing_flyout.take() {
                    self.flyout_view.teardown(ctx, &mut out_el);
                    out_node.remove(ctx);
                    ctx.destroy_node(out_node);
                }

                let fl_el = self.flyout_view.build(ctx);
                let fl_node = self.flyout_view.get_node(&fl_el);

                fl_node.update_constraints(ctx, |c| {
                    c.unclipped = true;
                    c.z_index = 9000;
                });

                element.container_node.append(ctx, fl_node);
                element.flyout_element = Some((fl_node, fl_el));
                element.is_open = true;
                element.is_entering = true;
                ctx.show_overlay(fl_node);

                if self.duration_ms > 0.0 {
                    element.anim_progress = AnimatedValue::new(0.0);
                    element.anim_start = Instant::now();
                    element
                        .anim_progress
                        .set_target(1.0, 0.0, self.duration_ms as f64, self.curve);
                    update_overlay_geometry(self, ctx, element);
                    ctx.request_frame();
                } else {
                    element.anim_progress = AnimatedValue::new(1.0);
                    update_overlay_geometry(self, ctx, element);
                }
            }
            (false, true) => {
                ctx.hide_overlay();
                if let Some((fl_node, fl_el)) = element.flyout_element.take() {
                    element.is_open = false;
                    element.is_entering = false;

                    if self.duration_ms > 0.0 {
                        element.outgoing_flyout = Some((fl_node, fl_el));
                        element.anim_progress = AnimatedValue::new(0.0);
                        element.anim_start = Instant::now();
                        element.anim_progress.set_target(
                            1.0,
                            0.0,
                            self.duration_ms as f64,
                            self.curve,
                        );
                        update_overlay_geometry(self, ctx, element);
                        ctx.request_frame();
                    } else {
                        let mut fl_el = fl_el;
                        self.flyout_view.teardown(ctx, &mut fl_el);
                        fl_node.remove(ctx);
                        ctx.destroy_node(fl_node);
                    }
                }
            }
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
        if let Some(sibling) = next_sibling {
            element.container_node.put_before(ctx, sibling);
        } else {
            parent.append(ctx, element.container_node);
        }
        self.rebuild(prev, ctx, element);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.hide_overlay();
        if let Some((out_node, mut out_el)) = element.outgoing_flyout.take() {
            self.flyout_view.teardown(ctx, &mut out_el);
            out_node.remove(ctx);
            ctx.destroy_node(out_node);
        }
        if let Some((fl_node, mut fl_el)) = element.flyout_element.take() {
            self.flyout_view.teardown(ctx, &mut fl_el);
            fl_node.remove(ctx);
            ctx.destroy_node(fl_node);
        }
        self.anchor_view.teardown(ctx, &mut element.anchor_element);
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

                update_overlay_geometry(self, ctx, element);

                if !animating || progress >= 0.999 {
                    if !element.is_entering {
                        if let Some((node, mut el)) = element.outgoing_flyout.take() {
                            self.flyout_view.teardown(ctx, &mut el);
                            node.remove(ctx);
                            ctx.destroy_node(node);
                        }
                    }
                } else {
                    ctx.request_frame();
                }
            }
        }

        if self.close_on_escape && element.is_open {
            if let Event::KeyboardInput {
                event: key_event, ..
            } = &event
            {
                if key_event.state.is_pressed() {
                    let is_escape = matches!(
                        key_event.logical_key.as_ref(),
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
                    );
                    if is_escape {
                        if let Some(ref on_dismiss) = self.on_dismiss {
                            return (EventResult::Handled, Some(on_dismiss()));
                        }
                    }
                }
            }
        }

        if self.close_on_click_outside && element.is_open {
            if let Event::MouseInput {
                pressed: true,
                hit_nodes,
                ..
            } = &event
            {
                let inside_anchor = hit_nodes.contains(&element.container_node)
                    || hit_nodes.contains(&element.anchor_node)
                    || hit_nodes
                        .iter()
                        .any(|n| n.is_descendant_of(ctx, element.anchor_node));

                let inside_flyout = if let Some((node, _)) = element.flyout_element {
                    hit_nodes.contains(&node)
                        || hit_nodes.iter().any(|n| n.is_descendant_of(ctx, node))
                } else {
                    false
                };

                if !inside_anchor && !inside_flyout {
                    if let Some(ref on_dismiss) = self.on_dismiss {
                        return (EventResult::Handled, Some(on_dismiss()));
                    }
                }
            }
        }

        let (flyout_res, flyout_msg) = if let Some((_, ref mut fl_el)) = element.flyout_element {
            self.flyout_view
                .handle_event(fl_el, state, event.clone(), ctx)
        } else {
            (EventResult::Ignored, None)
        };

        if flyout_msg.is_some() {
            return (flyout_res, flyout_msg);
        }

        let (anchor_res, anchor_msg) =
            self.anchor_view
                .handle_event(&mut element.anchor_element, state, event, ctx);

        (flyout_res.or(anchor_res), anchor_msg)
    }
}

/// Extension trait enabling fluid `.overlay(...)` and `.context_menu(...)` on any [`View`].
pub trait ViewOverlayExt: Sized {
    /// Attaches an anchored floating overlay to this view.
    fn overlay<FlyoutV, Msg>(self, flyout: FlyoutV) -> Overlay<Self, FlyoutV, Msg, fn() -> Msg>;

    /// Attaches a context menu flyout that opens at cursor coordinates on right-click.
    fn context_menu<MenuV, Msg>(
        self,
        is_open: bool,
        cursor_pos: Option<(f32, f32)>,
        menu: MenuV,
    ) -> Overlay<Self, MenuV, Msg, fn() -> Msg>;
}

impl<V> ViewOverlayExt for V {
    fn overlay<FlyoutV, Msg>(self, flyout: FlyoutV) -> Overlay<Self, FlyoutV, Msg, fn() -> Msg> {
        overlay(self, flyout)
    }

    fn context_menu<MenuV, Msg>(
        self,
        is_open: bool,
        cursor_pos: Option<(f32, f32)>,
        menu: MenuV,
    ) -> Overlay<Self, MenuV, Msg, fn() -> Msg> {
        overlay(self, menu)
            .is_open(is_open)
            .anchor_point(cursor_pos)
            .placement(OverlayPlacement::BottomStart)
            .offset(2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ViewStyleExt;
    use crate::ui::widgets::text;

    #[test]
    fn test_overlay_bottom_placement_coordinates() {
        let anchor_bounds = Rect::new(100.0, 100.0, 80.0, 30.0);
        let (rel_x, rel_y, placement) = compute_overlay_position(
            OverlayPlacement::BottomStart,
            4.0,
            anchor_bounds,
            None,
            120.0,
            60.0,
            800.0,
            600.0,
            true,
            true,
            8.0,
        );

        assert_eq!(placement, OverlayPlacement::BottomStart);
        assert_eq!(rel_x, 0.0);
        assert_eq!(rel_y, 34.0); // 30.0 (anchor_h) + 4.0 (offset)
    }

    #[test]
    fn test_overlay_collision_flip_bottom_to_top() {
        // Anchor near bottom edge of 800x600 window: y=560, h=30 -> bottom is 590
        let anchor_bounds = Rect::new(100.0, 560.0, 80.0, 30.0);
        let (rel_x, rel_y, placement) = compute_overlay_position(
            OverlayPlacement::BottomStart,
            4.0,
            anchor_bounds,
            None,
            120.0,
            60.0,
            800.0,
            600.0,
            true,
            true,
            8.0,
        );

        // Should flip to TopStart because 560 + 30 + 4 + 60 = 654 > 592
        assert_eq!(placement, OverlayPlacement::TopStart);
        assert_eq!(rel_x, 0.0);
        assert_eq!(rel_y, -64.0); // -60.0 (flyout_h) - 4.0 (offset)
    }

    #[test]
    fn test_overlay_collision_flip_right_to_left() {
        // Anchor near right edge: x=750, w=40 in 800x600
        let anchor_bounds = Rect::new(750.0, 200.0, 40.0, 30.0);
        let (_rel_x, _rel_y, placement) = compute_overlay_position(
            OverlayPlacement::RightStart,
            4.0,
            anchor_bounds,
            None,
            100.0,
            50.0,
            800.0,
            600.0,
            true,
            true,
            8.0,
        );

        assert_eq!(placement, OverlayPlacement::LeftStart);
    }

    #[test]
    fn test_overlay_collision_clamp_viewport_bounds() {
        // Anchor right at window edge (x=780, w=20)
        let anchor_bounds = Rect::new(780.0, 100.0, 20.0, 30.0);
        let (rel_x, _rel_y, _) = compute_overlay_position(
            OverlayPlacement::BottomStart,
            4.0,
            anchor_bounds,
            None,
            100.0,
            50.0,
            800.0,
            600.0,
            false, // no flip, just clamp
            true,
            8.0,
        );

        // Max global x is 800 - 8 - 100 = 692. rel_x = 692 - 780 = -88.0
        assert_eq!(rel_x, -88.0);
    }

    #[test]
    fn test_overlay_cursor_anchored_positioning() {
        let anchor_bounds = Rect::new(100.0, 100.0, 200.0, 200.0);
        let cursor_point = Some((250.0, 180.0));
        let (rel_x, rel_y, _) = compute_overlay_position(
            OverlayPlacement::BottomStart,
            2.0,
            anchor_bounds,
            cursor_point,
            80.0,
            40.0,
            800.0,
            600.0,
            true,
            true,
            8.0,
        );

        // rel_x = 250 - 100 = 150.0
        // rel_y = 180 + 0 + 2 - 100 = 82.0
        assert_eq!(rel_x, 150.0);
        assert_eq!(rel_y, 82.0);
    }

    #[test]
    fn test_overlay_outside_click_and_escape_dismissal() {
        let mut ctx = Context::new();
        let target = text::<_, &'static str>("Anchor")
            .overlay(text::<_, &'static str>("Menu"))
            .is_open(true)
            .on_dismiss(|| "Dismissed");

        let mut element = View::<()>::build(&target, &mut ctx);

        // Click outside both anchor and flyout (hit_nodes is empty or some other node)
        let other_node = ctx.create_node();
        let (click_res, click_msg) = View::<()>::handle_event(
            &target,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                hit_nodes: vec![other_node],
                x: 0.0,
                y: 0.0,
            },
            &mut ctx,
        );
        assert_eq!(click_res, EventResult::Handled);
        assert_eq!(click_msg, Some("Dismissed"));

        // Press Escape key
        let esc_event = crate::ui::KeyEvent::new(
            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            winit::event::ElementState::Pressed,
        );
        let (esc_res, esc_msg) = View::<()>::handle_event(
            &target,
            &mut element,
            &(),
            Event::KeyboardInput {
                event: esc_event,
                is_synthetic: false,
            },
            &mut ctx,
        );
        assert_eq!(esc_res, EventResult::Handled);
        assert_eq!(esc_msg, Some("Dismissed"));
    }

    #[test]
    fn test_overlay_presence_motions() {
        let mut ctx = Context::new();
        let target_closed = text::<_, ()>("Anchor")
            .overlay(text::<_, ()>("Flyout"))
            .is_open(false)
            .enter(Motion::fade_in())
            .exit(Motion::fade_out())
            .duration_ms(100.0);

        let mut element = View::<()>::build(&target_closed, &mut ctx);
        assert!(!element.is_open);
        assert!(element.flyout_element.is_none());
        assert!(!ctx.is_overlay_open());

        // Transition: closed -> open
        let target_open = text::<_, ()>("Anchor")
            .overlay(text::<_, ()>("Flyout"))
            .is_open(true)
            .enter(Motion::fade_in())
            .exit(Motion::fade_out())
            .duration_ms(100.0);

        View::<()>::rebuild(&target_open, &target_closed, &mut ctx, &mut element);
        assert!(element.is_open);
        assert!(element.flyout_element.is_some());
        assert!(ctx.is_overlay_open());

        // Verify unclipped constraint and z_index on flyout
        let (fl_node, _) = element.flyout_element.unwrap();
        let fl_cons = fl_node.get_constraints(&ctx).unwrap();
        assert!(fl_cons.unclipped);
        assert_eq!(fl_cons.z_index, 9000);

        // Transition: open -> closed
        View::<()>::rebuild(&target_closed, &target_open, &mut ctx, &mut element);
        assert!(!element.is_open);
        assert!(element.outgoing_flyout.is_some());
        assert!(!ctx.is_overlay_open());
    }

    #[test]
    fn test_flyout_hover_preserves_unclipped_and_positioning() {
        let mut ctx = Context::new();
        let menu_item = text::<_, ()>("Menu Item").style(
            Style::new()
                .padding(8.0)
                .on_hover(|s| s.bg_color(crate::Color::white)),
        );
        let flyout_menu = crate::ui::widgets::column((menu_item,))
            .style(Style::new().width(Size::Fixed(180)).padding(6.0));
        let overlay_view = text::<_, ()>("Anchor")
            .overlay(flyout_menu)
            .is_open(true)
            .duration_ms(0.0);

        let mut element = View::<()>::build(&overlay_view, &mut ctx);
        let fl_node = element.flyout_element.as_ref().unwrap().0;

        let fl_cons = fl_node.get_constraints(&ctx).unwrap();
        assert!(fl_cons.unclipped);
        assert_eq!(fl_cons.z_index, 9000);
        assert!(matches!(
            fl_cons.positioning,
            PositionStrategy::Absolute { .. }
        ));

        // Send hover event hitting the flyout node
        let _ = View::<()>::handle_event(
            &overlay_view,
            &mut element,
            &(),
            Event::CursorMoved {
                x: 10.0,
                y: 10.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: vec![fl_node],
            },
            &mut ctx,
        );

        // Verify that after hover event, flyout node STILL has unclipped, z_index: 9000, and Absolute positioning!
        let fl_cons_after = fl_node.get_constraints(&ctx).unwrap();
        assert!(
            fl_cons_after.unclipped,
            "fl_cons.unclipped must remain true after hover"
        );
        assert_eq!(
            fl_cons_after.z_index, 9000,
            "fl_cons.z_index must remain 9000 after hover"
        );
        assert!(
            matches!(fl_cons_after.positioning, PositionStrategy::Absolute { .. }),
            "fl_cons.positioning must remain Absolute after hover"
        );
    }
}
