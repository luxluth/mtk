use std::marker::PhantomData;
use std::time::Instant;
use winit::keyboard::{Key, NamedKey};

use crate::animation::AnimatedValue;
use crate::colors::Color;
use crate::layer::ActiveLayerId;
use crate::layer::UserLayer;
use crate::style::{AlignItems, JustifyContent, PositionStrategy, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::transition::Transition;
use crate::ui::{Event, View};
use crate::{Context, Node, rgba};

/// A high-level visual layer surface stacked above a base view with animated transitions,
/// optional backdrop scrims, focus isolation, and automatic dismiss events.
pub struct Layer<BaseV, LayerV, Msg, F = fn() -> Msg> {
    pub(crate) base_view: BaseV,
    pub(crate) layer_view: LayerV,
    pub(crate) is_active: bool,
    pub(crate) layer_id: ActiveLayerId,
    pub(crate) transition: Transition,
    pub(crate) dim_background: bool,
    pub(crate) scrim_color: Color,
    pub(crate) is_modal: bool,
    pub(crate) close_on_escape: bool,
    pub(crate) close_on_backdrop: bool,
    pub(crate) on_dismiss: Option<F>,
    pub(crate) _marker: PhantomData<Msg>,
}

/// Creates a new `Layer` stacking an overlay view over a base view.
pub fn layer<BaseV, LayerV, Msg>(
    base_view: BaseV,
    is_active: bool,
    layer_view: LayerV,
) -> Layer<BaseV, LayerV, Msg, fn() -> Msg> {
    Layer {
        base_view,
        layer_view,
        is_active,
        layer_id: ActiveLayerId::Base,
        transition: Transition::fade(),
        dim_background: false,
        scrim_color: rgba!(0, 0, 0, 130),
        is_modal: false,
        close_on_escape: true,
        close_on_backdrop: true,
        on_dismiss: None,
        _marker: PhantomData,
    }
}

impl<BaseV, LayerV, Msg, F> Layer<BaseV, LayerV, Msg, F> {
    /// Sets the active layer identifier for this layer.
    pub fn layer_id(mut self, id: ActiveLayerId) -> Self {
        self.layer_id = id;
        self
    }

    /// Sets the transition animation physics for this layer.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = transition;
        self
    }

    /// Sets whether a darkened scrim is rendered behind this layer.
    pub fn dim_background(mut self, dim: bool) -> Self {
        self.dim_background = dim;
        self
    }

    /// Sets custom backdrop scrim color.
    pub fn scrim_color(mut self, color: Color) -> Self {
        self.scrim_color = color;
        self
    }

    /// Configures modal behavior (focus trapping and event blocking on lower layers).
    pub fn set_modal(mut self, is_modal: bool) -> Self {
        self.is_modal = is_modal;
        self
    }

    /// Configures whether pressing Escape dismisses this layer.
    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    /// Configures whether clicking the backdrop dims dismisses this layer.
    pub fn close_on_backdrop(mut self, close: bool) -> Self {
        self.close_on_backdrop = close;
        self
    }

    /// Sets the callback invoked when the layer is dismissed via Escape or backdrop click.
    pub fn on_dismiss<NewF: Fn() -> Msg>(
        self,
        on_dismiss: NewF,
    ) -> Layer<BaseV, LayerV, Msg, NewF> {
        Layer {
            base_view: self.base_view,
            layer_view: self.layer_view,
            is_active: self.is_active,
            layer_id: self.layer_id,
            transition: self.transition,
            dim_background: self.dim_background,
            scrim_color: self.scrim_color,
            is_modal: self.is_modal,
            close_on_escape: self.close_on_escape,
            close_on_backdrop: self.close_on_backdrop,
            on_dismiss: Some(on_dismiss),
            _marker: PhantomData,
        }
    }

    /// Sets the callback invoked when the layer is closed (alias for `on_dismiss`).
    pub fn on_close<NewF: Fn() -> Msg>(self, on_close: NewF) -> Layer<BaseV, LayerV, Msg, NewF> {
        self.on_dismiss(on_close)
    }
}

pub struct LayerElement<BaseEl, LayerEl> {
    pub(crate) root_node: Node,
    pub(crate) base_el: BaseEl,
    pub(crate) overlay_node: Node,
    pub(crate) scrim_node: Node,
    pub(crate) content_wrapper_node: Node,
    pub(crate) layer_el: LayerEl,
    pub(crate) is_mounted: bool,
    pub(crate) anim_progress: AnimatedValue<f32>,
    pub(crate) anim_start: Instant,
    pub(crate) saved_focus: Option<Node>,
}

impl<BaseV, LayerV, State, Msg, F> View<State> for Layer<BaseV, LayerV, Msg, F>
where
    BaseV: View<State, Message = Msg>,
    LayerV: View<State, Message = Msg>,
    F: Fn() -> Msg,
{
    type Element = LayerElement<BaseV::Element, LayerV::Element>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let root_node = ctx.create_node();
        let overlay_node = ctx.create_node();
        let scrim_node = ctx.create_node();
        let content_wrapper_node = ctx.create_node();

        // 1. Build Base View
        let base_el = self.base_view.build(ctx);
        let base_node = self.base_view.get_node(&base_el);

        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .apply_to_node(ctx, root_node);

        root_node.append(ctx, base_node);
        ctx.base_layer.state.root_node = Some(base_node);

        // 2. Setup Top-level Overlay Container (Full Screen Overlay Plane)
        Style::new()
            .position(PositionStrategy::Absolute {
                top: 0.0,
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
            })
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .z_index(9000)
            .apply_to_node(ctx, overlay_node);

        // 3. Setup Backdrop Scrim
        let initial_progress = if self.is_active { 1.0 } else { 0.0 };
        let anim_progress = AnimatedValue::new(initial_progress);
        let anim_start = Instant::now();

        let initial_alpha = if self.dim_background && self.is_active {
            self.scrim_color.a
        } else {
            0
        };

        Style::new()
            .position(PositionStrategy::Absolute {
                top: 0.0,
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
            })
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .bg_color(Color::new(
                self.scrim_color.r,
                self.scrim_color.g,
                self.scrim_color.b,
                initial_alpha,
            ))
            .z_index(9001)
            .apply_to_node(ctx, scrim_node);

        overlay_node.append(ctx, scrim_node);

        // 4. Setup Content Wrapper
        let mut wrapper_style = Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .z_index(9002);

        if self.is_modal {
            wrapper_style = wrapper_style
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center);
        }

        wrapper_style.apply_to_node(ctx, content_wrapper_node);

        let layer_el = self.layer_view.build(ctx);
        let layer_node = self.layer_view.get_node(&layer_el);
        content_wrapper_node.append(ctx, layer_node);
        overlay_node.append(ctx, content_wrapper_node);

        let mut is_mounted = false;
        let mut saved_focus = None;

        if self.is_active {
            root_node.append(ctx, overlay_node);
            is_mounted = true;
            saved_focus = ctx.focused_node();
        }

        // Register in Core Context layer stack
        match self.layer_id {
            ActiveLayerId::Modal => {
                ctx.modal_layer.state.root_node = Some(layer_node);
                ctx.modal_layer.state.visible = self.is_active;
            }
            ActiveLayerId::Intermediate(id) => {
                ctx.intermediate_layers.retain(|l| l.id != id);
                let mut user_layer = UserLayer::new(id, self.is_active, self.is_modal);
                user_layer.state.root_node = Some(layer_node);
                ctx.intermediate_layers.push(user_layer);
            }
            ActiveLayerId::Overlay => {
                ctx.overlay_layer.state.root_node = Some(layer_node);
                ctx.overlay_layer.state.visible = self.is_active;
            }
            ActiveLayerId::Base => {}
        }
        ctx.active_layer = ctx.active_layer();

        LayerElement {
            root_node,
            base_el,
            overlay_node,
            scrim_node,
            content_wrapper_node,
            layer_el,
            is_mounted,
            anim_progress,
            anim_start,
            saved_focus,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        self.base_view
            .rebuild(&prev.base_view, ctx, &mut element.base_el);
        self.layer_view
            .rebuild(&prev.layer_view, ctx, &mut element.layer_el);

        // Sync visibility to Core Context layer stack
        let layer_node = self.layer_view.get_node(&element.layer_el);
        match self.layer_id {
            ActiveLayerId::Modal => {
                ctx.modal_layer.state.visible = self.is_active;
                ctx.modal_layer.state.root_node = Some(layer_node);
            }
            ActiveLayerId::Intermediate(id) => {
                if let Some(l) = ctx.intermediate_layers.iter_mut().find(|l| l.id == id) {
                    l.state.visible = self.is_active;
                    l.blocking = self.is_modal;
                    l.state.root_node = Some(layer_node);
                }
            }
            ActiveLayerId::Overlay => {
                ctx.overlay_layer.state.visible = self.is_active;
                ctx.overlay_layer.state.root_node = Some(layer_node);
            }
            ActiveLayerId::Base => {}
        }
        ctx.active_layer = ctx.active_layer();

        if self.is_active != prev.is_active {
            let target = if self.is_active { 1.0f32 } else { 0.0f32 };
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            let duration = self.transition.duration_ms();
            let curve = self.transition.curve();

            if self.is_active && !element.is_mounted {
                element.root_node.append(ctx, element.overlay_node);
                element.is_mounted = true;
                element.saved_focus = ctx.focused_node();
                if self.is_modal {
                    ctx.clear_focus();
                }
            } else if !self.is_active {
                // Instantly clear focus if focused node was inside this layer
                if let Some(focused) = ctx.focused_node() {
                    if focused.is_descendant_of(ctx, element.overlay_node) {
                        if let Some(saved) = element.saved_focus {
                            ctx.request_focus(saved);
                        } else {
                            ctx.clear_focus();
                        }
                    }
                }
            }

            element
                .anim_progress
                .set_target(target, now, duration as f64, curve);
            ctx.request_frame();
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.clear_layer_focus(self.layer_id);
        self.layer_view.teardown(ctx, &mut element.layer_el);
        self.base_view.teardown(ctx, &mut element.base_el);
        element.overlay_node.remove(ctx);
        element.root_node.remove(ctx);
        ctx.destroy_node(element.scrim_node);
        ctx.destroy_node(element.content_wrapper_node);
        ctx.destroy_node(element.overlay_node);
        ctx.destroy_node(element.root_node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.root_node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        // Animation tick handling for smooth layer transitions
        if let Event::Tick { .. } = event {
            let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
            element.anim_progress.tick(now);
            let animating = element.anim_progress.is_animating();
            let progress = element.anim_progress.get();

            if element.is_mounted {
                // Update Scrim alpha
                if self.dim_background {
                    let target_a = self.scrim_color.a as f32;
                    let current_a = (target_a * progress.clamp(0.0, 1.0)) as u8;
                    element.scrim_node.update_effects(ctx, |eff| {
                        eff.background_color = Color::new(
                            self.scrim_color.r,
                            self.scrim_color.g,
                            self.scrim_color.b,
                            current_a,
                        );
                    });
                }

                // Apply Layer Physics Transforms
                let screen_h = element
                    .root_node
                    .get_computed(ctx)
                    .map(|c| c.h)
                    .unwrap_or(1200.0)
                    .max(800.0);
                let screen_w = element
                    .root_node
                    .get_computed(ctx)
                    .map(|c| c.w)
                    .unwrap_or(1600.0)
                    .max(1000.0);

                match self.transition {
                    Transition::SlideUp { .. } => {
                        let offset_y = (1.0 - progress) * screen_h;
                        let mut style = Style::new()
                            .position(PositionStrategy::Absolute {
                                top: offset_y,
                                left: 0.0,
                                bottom: 0.0,
                                right: 0.0,
                            })
                            .width(Size::Percent(1.0))
                            .height(Size::Percent(1.0))
                            .z_index(9002);
                        if self.is_modal {
                            style = style
                                .align_items(AlignItems::Center)
                                .justify_content(JustifyContent::Center);
                        }
                        style.apply_to_node(ctx, element.content_wrapper_node);
                        element.content_wrapper_node.update_effects(ctx, |eff| {
                            eff.opacity = progress.clamp(0.0, 1.0);
                        });
                    }
                    Transition::SlideDown { .. } => {
                        let offset_y = -(1.0 - progress) * screen_h;
                        let mut style = Style::new()
                            .position(PositionStrategy::Absolute {
                                top: offset_y,
                                left: 0.0,
                                bottom: 0.0,
                                right: 0.0,
                            })
                            .width(Size::Percent(1.0))
                            .height(Size::Percent(1.0))
                            .z_index(9002);
                        if self.is_modal {
                            style = style
                                .align_items(AlignItems::Center)
                                .justify_content(JustifyContent::Center);
                        }
                        style.apply_to_node(ctx, element.content_wrapper_node);
                        element.content_wrapper_node.update_effects(ctx, |eff| {
                            eff.opacity = progress.clamp(0.0, 1.0);
                        });
                    }
                    Transition::SlideRight { .. } => {
                        let offset_x = (1.0 - progress) * screen_w;
                        let mut style = Style::new()
                            .position(PositionStrategy::Absolute {
                                top: 0.0,
                                left: offset_x,
                                bottom: 0.0,
                                right: 0.0,
                            })
                            .width(Size::Percent(1.0))
                            .height(Size::Percent(1.0))
                            .z_index(9002);
                        if self.is_modal {
                            style = style
                                .align_items(AlignItems::Center)
                                .justify_content(JustifyContent::Center);
                        }
                        style.apply_to_node(ctx, element.content_wrapper_node);
                        element.content_wrapper_node.update_effects(ctx, |eff| {
                            eff.opacity = progress.clamp(0.0, 1.0);
                        });
                    }
                    Transition::SlideLeft { .. } => {
                        let offset_x = -(1.0 - progress) * screen_w;
                        let mut style = Style::new()
                            .position(PositionStrategy::Absolute {
                                top: 0.0,
                                left: offset_x,
                                bottom: 0.0,
                                right: 0.0,
                            })
                            .width(Size::Percent(1.0))
                            .height(Size::Percent(1.0))
                            .z_index(9002);
                        if self.is_modal {
                            style = style
                                .align_items(AlignItems::Center)
                                .justify_content(JustifyContent::Center);
                        }
                        style.apply_to_node(ctx, element.content_wrapper_node);
                        element.content_wrapper_node.update_effects(ctx, |eff| {
                            eff.opacity = progress.clamp(0.0, 1.0);
                        });
                    }
                    Transition::Fade { .. } => {
                        element.content_wrapper_node.update_effects(ctx, |eff| {
                            eff.opacity = progress.clamp(0.0, 1.0);
                        });
                    }
                    Transition::Scale { from_scale, .. } => {
                        let current_scale = from_scale + (1.0 - from_scale) * progress;
                        element.content_wrapper_node.update_effects(ctx, |eff| {
                            eff.opacity = progress.clamp(0.0, 1.0);
                            eff.scale = current_scale;
                        });
                    }
                    Transition::None => {}
                }

                // If finished dismissing (progress reached 0.0), unmount overlay node and restore focus
                if !self.is_active && progress <= 0.001 && element.is_mounted {
                    if let Some(focused) = ctx.focused_node() {
                        if focused.is_descendant_of(ctx, element.overlay_node) {
                            if let Some(saved) = element.saved_focus {
                                ctx.request_focus(saved);
                            } else {
                                ctx.clear_focus();
                            }
                        }
                    }

                    element.overlay_node.remove(ctx);
                    element.is_mounted = false;
                }

                if animating {
                    ctx.request_frame();
                }
            }
        }

        // Deterministic Single-Target Event Routing:
        // Only dispatch input events to this layer if it is the active layer!
        let is_active_layer = ctx.active_layer() == self.layer_id;

        if self.is_active && element.is_mounted && is_active_layer {
            // Check for Escape key to dismiss
            if self.close_on_escape {
                if let Event::KeyboardInput { ref event, .. } = event {
                    if event.state.is_pressed() && event.logical_key == Key::Named(NamedKey::Escape)
                    {
                        if let Some(ref dismiss_fn) = self.on_dismiss {
                            if let Some(focused) = ctx.focused_node() {
                                if focused.is_descendant_of(ctx, element.overlay_node) {
                                    if let Some(saved) = element.saved_focus {
                                        ctx.request_focus(saved);
                                    } else {
                                        ctx.clear_focus();
                                    }
                                }
                            }
                            return (EventResult::Handled, Some(dismiss_fn()));
                        }
                    }
                }
            }

            // Route directly to the active layer view
            let (layer_res, layer_msg) =
                self.layer_view
                    .handle_event(&mut element.layer_el, state, event.clone(), ctx);
            if layer_res == EventResult::Handled || layer_msg.is_some() {
                return (layer_res, layer_msg);
            }

            // Check for backdrop click
            if self.close_on_backdrop {
                if let Event::MouseInput {
                    pressed: true,
                    ref hit_nodes,
                    ..
                } = event
                {
                    let layer_node = self.layer_view.get_node(&element.layer_el);
                    let hit_layer = hit_nodes.contains(&layer_node);
                    let hit_scrim = hit_nodes.contains(&element.scrim_node)
                        || hit_nodes.contains(&element.overlay_node)
                        || hit_nodes.contains(&element.content_wrapper_node);

                    if hit_scrim && !hit_layer {
                        if let Some(ref dismiss_fn) = self.on_dismiss {
                            if let Some(focused) = ctx.focused_node() {
                                if focused.is_descendant_of(ctx, element.overlay_node) {
                                    if let Some(saved) = element.saved_focus {
                                        ctx.request_focus(saved);
                                    } else {
                                        ctx.clear_focus();
                                    }
                                }
                            }
                            return (EventResult::Handled, Some(dismiss_fn()));
                        }
                    }
                }
            }

            // If modal/blocking, prevent events from falling through to base view
            if self.is_modal {
                match event {
                    Event::MouseInput { .. } | Event::CursorMoved { .. } => {
                        return (EventResult::Handled, None);
                    }
                    _ => {}
                }
            }
        }

        // Forward event down to base view if not consumed by an active modal
        self.base_view
            .handle_event(&mut element.base_el, state, event, ctx)
    }
}

/// Extension trait on all [`View`] types to easily attach layers, sheets, and modal surfaces.
pub trait ViewLayerExt: Sized {
    /// Stacks a custom intermediate layer surface over this view with customizable transition physics.
    fn intermediate<LayerV, Msg>(
        self,
        id: u32,
        is_active: bool,
        layer_view: LayerV,
        transition: Transition,
    ) -> Layer<Self, LayerV, Msg, fn() -> Msg> {
        layer(self, is_active, layer_view)
            .layer_id(ActiveLayerId::Intermediate(id))
            .transition(transition)
            .dim_background(true)
            .set_modal(true)
    }

    /// Stacks a slide-up bottom sheet over this view with background dimming.
    fn sheet<LayerV, Msg>(
        self,
        is_active: bool,
        sheet_view: LayerV,
    ) -> Layer<Self, LayerV, Msg, fn() -> Msg> {
        layer(self, is_active, sheet_view)
            .layer_id(ActiveLayerId::Intermediate(1))
            .transition(Transition::slide_up())
            .dim_background(true)
            .set_modal(true)
    }

    /// Stacks a modal dialog over this view with scale/fade animation and dimmed scrim.
    fn modal<LayerV, Msg>(
        self,
        is_active: bool,
        dialog_view: LayerV,
    ) -> Layer<Self, LayerV, Msg, fn() -> Msg> {
        layer(self, is_active, dialog_view)
            .layer_id(ActiveLayerId::Modal)
            .transition(Transition::scale())
            .dim_background(true)
            .set_modal(true)
    }
}

impl<V> ViewLayerExt for V {}
