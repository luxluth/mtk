//! Client-Side Decorations (CSD), composable window frames, and action button widgets.
//!
//! Provides [`csd_frame`], [`CsdFrame`], [`CsdFrameExt`], [`action_buttons`], [`ActionButtons`],
//! and [`ActionButton`].
//!
//! Designed for modern, borderless workstation applications where developers want complete
//! composability over window borders, shadows, and title/control placement.

use std::marker::PhantomData;

use crate::animation::Curve;
use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::effects::{BoxShadow, Radius};
use crate::image::{ObjectFit, SvgData};
use crate::style::{AlignItems, Edges, FlexDirection, JustifyContent, Overflow, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::style::{StyledView, StyledViewState, ViewStyleExt};
use crate::ui::widgets::{Container, Svg, SvgElement, container, svg};
use crate::ui::{Event, View};
use crate::{Context, CursorIcon, Node, ResizeDirection, TiledEdges, clr, rgb, rgba};

/// Raw SVG markup for the window close icon.
pub const CLOSE_ICON_SVG: &str = include_str!("internal_icons/closeIcon.svg");
/// Raw SVG markup for the window minimize / hide icon.
pub const HIDE_ICON_SVG: &str = include_str!("internal_icons/hideIcon.svg");
/// Raw SVG markup for the window maximize / upsize icon.
pub const UPSIZE_ICON_SVG: &str = include_str!("internal_icons/upsizeIcon.svg");
/// Raw SVG markup for the window restore / downsize icon.
pub const DOWNSIZE_ICON_SVG: &str = include_str!("internal_icons/downsizeIcon.svg");

/// An action button that can be displayed in an [`ActionButtons`] group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionButton {
    /// Minimize / hide button.
    Hide,
    /// Maximize / restore button (toggles between upsize and downsize).
    Expand,
    /// Close button.
    Close,
}

type ButtonView<Msg> = StyledView<Container<(StyledView<Svg<Msg>>,)>>;
type ButtonElement = ((Node, ((SvgElement, StyledViewState),)), StyledViewState);

/// A sleek, composable window action buttons widget matching modern desktop aesthetics.
///
/// Consists of up to three circular frosted-glass buttons ([`ActionButton::Hide`],
/// [`ActionButton::Expand`], and [`ActionButton::Close`]).
///
/// Each button leverages [`StyledView`] for hardware-accelerated animated transitions on hover and active click.
pub struct ActionButtons<Msg> {
    pub(crate) buttons: Vec<ActionButton>,
    pub(crate) circle_size: f32,
    pub(crate) icon_size: f32,
    pub(crate) stroke_width: f32,
    pub(crate) gap: f32,
    pub(crate) colors: Dominants,
    pub(crate) on_minimize: Option<Msg>,
    pub(crate) on_maximize: Option<Msg>,
    pub(crate) on_close: Option<Msg>,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

impl<Msg> ActionButtons<Msg> {
    pub fn new(color: Color) -> Self {
        ActionButtons {
            buttons: vec![
                ActionButton::Hide,
                ActionButton::Expand,
                ActionButton::Close,
            ],
            circle_size: 20.0,
            icon_size: 10.0,
            stroke_width: 1.0,
            gap: 6.0,
            colors: Dominants::new(color),
            on_minimize: None,
            on_maximize: None,
            on_close: None,
            source_loc: Some(SourceLocation::here("ActionButtons")),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Dominants {
    bg_color: Color,
    border_color: Color,
    icon_color: Color,
    hover_bg: Color,
}

impl Dominants {
    fn new(color: Color) -> Self {
        let is_dark = color.is_dark();
        Self {
            bg_color: color.darker(0.2).with_alpha(if is_dark { 40 } else { 10 }),
            border_color: color.with_alpha(if is_dark { 10 } else { 5 }),
            icon_color: color,
            hover_bg: if is_dark {
                color.darker(0.2)
            } else {
                color.lighter(0.2)
            }
            .with_alpha(if is_dark { 100 } else { 20 }),
        }
    }
}

/// Creates a new `ActionButtons` component with default frosted glass styling (20px circles, 6px gap).
///
/// Displays `[ActionButton::Hide, ActionButton::Expand, ActionButton::Close]` by default.
#[track_caller]
pub fn action_buttons<Msg>() -> ActionButtons<Msg> {
    ActionButtons::new(clr!(white))
}

/// Alias for [`action_buttons`].
#[inline]
pub fn window_controls<Msg>() -> ActionButtons<Msg> {
    action_buttons()
}

/// Backward compatibility alias.
pub type WindowControls<Msg> = ActionButtons<Msg>;

impl<Msg: Clone> ActionButtons<Msg> {
    /// Sets the list of buttons to display and their order (maximum of 3 buttons).
    ///
    /// # Examples
    /// ```rust,ignore
    /// // Only show Close and Minimize:
    /// action_buttons().buttons(&[ActionButton::Hide, ActionButton::Close])
    /// ```
    pub fn buttons(mut self, buttons: &[ActionButton]) -> Self {
        self.buttons = buttons.iter().copied().take(3).collect();
        self
    }

    /// Sets the diameter of each circular button (defaults to 20px).
    pub fn circle_size(mut self, size: f32) -> Self {
        self.circle_size = size;
        self
    }

    /// Sets the size of the inner SVG icons (defaults to 10px).
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }

    /// Sets the SVG line stroke width (defaults to 1.0).
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the horizontal spacing between circular buttons (defaults to 6px).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Sets the dominant color of the icons. Every other part of the colors will
    /// be derived from it
    pub fn dominant_color(mut self, color: Color) -> Self {
        self.colors = Dominants::new(color);
        self
    }

    /// Sets a custom message to emit when the close button is clicked.
    pub fn on_close(mut self, msg: Msg) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// Sets a custom message to emit when the minimize button is clicked.
    pub fn on_minimize(mut self, msg: Msg) -> Self {
        self.on_minimize = Some(msg);
        self
    }

    /// Sets a custom message to emit when the maximize / restore button is clicked.
    pub fn on_maximize(mut self, msg: Msg) -> Self {
        self.on_maximize = Some(msg);
        self
    }

    fn create_button_view(&self, kind: ActionButton, is_maximized: bool) -> ButtonView<Msg> {
        let svg_markup = match kind {
            ActionButton::Hide => HIDE_ICON_SVG,
            ActionButton::Expand => {
                if is_maximized {
                    DOWNSIZE_ICON_SVG
                } else {
                    UPSIZE_ICON_SVG
                }
            }
            ActionButton::Close => CLOSE_ICON_SVG,
        };

        let svg_data = SvgData::from_str(svg_markup).expect("Valid internal SVG");

        let icon_widget = svg::<Msg>(svg_data)
            .color(self.colors.icon_color)
            .fill(self.colors.icon_color)
            .stroke(self.colors.icon_color)
            .stroke_width(self.stroke_width)
            .fit(ObjectFit::Contain)
            .style(
                Style::new()
                    .width(Size::Fixed(self.icon_size as u32))
                    .height(Size::Fixed(self.icon_size as u32)),
            );

        let radius = self.circle_size * 0.5;

        let btn_style = Style::new()
            .width(Size::Fixed(self.circle_size as u32))
            .height(Size::Fixed(self.circle_size as u32))
            .corner_radius(radius)
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center)
            .bg_color(self.colors.bg_color)
            .border(1.0, self.colors.border_color)
            .on_hover(|s| s.bg_color(self.colors.hover_bg))
            .on_active(|s| s.scale(0.92))
            .transition_all(200.0, Curve::ease_out());

        container((icon_widget,)).style(btn_style)
    }
}

/// Represents an instantiated button slot backed by [`StyledView`].
pub(crate) struct ButtonSlot<Msg> {
    pub(crate) kind: ActionButton,
    pub(crate) view: ButtonView<Msg>,
    pub(crate) element: ButtonElement,
}

/// Persistent DOM element state for [`ActionButtons`].
pub struct ActionButtonsElement<Msg> {
    container_node: Node,
    pub(crate) slots: Vec<ButtonSlot<Msg>>,
    is_maximized: bool,
}

/// Backward compatibility alias.
pub type WindowControlsElement<Msg> = ActionButtonsElement<Msg>;

impl<State, Msg: Clone + 'static> View<State> for ActionButtons<Msg> {
    type Element = ActionButtonsElement<Msg>;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let container_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(container_node, loc);
        }

        Style::new()
            .flex_direction(FlexDirection::Row)
            .align_items(AlignItems::Center)
            .gap(self.gap)
            .height(Size::Fixed(self.circle_size as u32))
            .apply_to_node(ctx, container_node);

        let is_maximized = ctx.is_window_maximized();
        let mut slots = Vec::with_capacity(self.buttons.len());

        for &kind in &self.buttons {
            let view = self.create_button_view(kind, is_maximized);
            let element = View::<State>::build(&view, ctx);
            let btn_node = View::<State>::get_node(&view, &element);
            container_node.append(ctx, btn_node);

            slots.push(ButtonSlot {
                kind,
                view,
                element,
            });
        }

        ActionButtonsElement {
            container_node,
            slots,
            is_maximized,
        }
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let current_max = ctx.is_window_maximized();

        if current_max != element.is_maximized {
            element.is_maximized = current_max;
            for slot in &mut element.slots {
                if slot.kind == ActionButton::Expand {
                    let new_view = self.create_button_view(slot.kind, current_max);
                    View::<State>::rebuild(&new_view, &slot.view, ctx, &mut slot.element);
                    slot.view = new_view;
                }
            }
        } else {
            for slot in &mut element.slots {
                let new_view = self.create_button_view(slot.kind, current_max);
                View::<State>::rebuild(&new_view, &slot.view, ctx, &mut slot.element);
                slot.view = new_view;
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        for slot in &mut element.slots {
            View::<State>::teardown(&slot.view, ctx, &mut slot.element);
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
        for slot in &mut element.slots {
            let slot_node = View::<State>::get_node(&slot.view, &slot.element);
            let was_active = slot.element.1.is_active;

            let (res, msg) = View::<State>::handle_event(
                &slot.view,
                &mut slot.element,
                state,
                event.clone(),
                ctx,
            );
            if msg.is_some() {
                return (res, msg);
            }

            if let Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed,
                hit_nodes,
                ..
            } = &event
            {
                if *pressed {
                    if hit_nodes.contains(&slot_node) {
                        return (EventResult::Handled, None);
                    }
                } else if was_active && hit_nodes.contains(&slot_node) {
                    match slot.kind {
                        ActionButton::Close => {
                            if let Some(m) = self.on_close.clone() {
                                return (EventResult::Handled, Some(m));
                            } else {
                                ctx.request_window_close();
                                return (EventResult::Handled, None);
                            }
                        }
                        ActionButton::Expand => {
                            if let Some(m) = self.on_maximize.clone() {
                                return (EventResult::Handled, Some(m));
                            } else {
                                ctx.toggle_maximize_window();
                                let current_max = ctx.is_window_maximized();
                                element.is_maximized = current_max;
                                for s in &mut element.slots {
                                    if s.kind == ActionButton::Expand {
                                        let new_view = self.create_button_view(s.kind, current_max);
                                        View::<State>::rebuild(
                                            &new_view,
                                            &s.view,
                                            ctx,
                                            &mut s.element,
                                        );
                                        s.view = new_view;
                                    }
                                }
                                ctx.request_frame();
                                return (EventResult::Handled, None);
                            }
                        }
                        ActionButton::Hide => {
                            if let Some(m) = self.on_minimize.clone() {
                                return (EventResult::Handled, Some(m));
                            } else {
                                ctx.minimize_window();
                                return (EventResult::Handled, None);
                            }
                        }
                    }
                }
            }

            if res == EventResult::Handled {
                return (res, None);
            }
        }

        (EventResult::Ignored, None)
    }
}

/// A composable outer window frame widget that decorates an application view with
/// custom borders, rounded corners, box shadows, and 8-direction interactive resize borders.
///
/// Automatically reserves transparent outer buffer margin when floating so analytical drop shadows
/// are never clipped by the OS surface, and automatically collapses margin and corner radii to `0`
/// when the window is maximized.
pub struct CsdFrame<V> {
    pub(crate) inner: V,
    pub(crate) shadow_margin: f32,
    pub(crate) resize_border_width: f32,
    pub(crate) resizable: bool,
    pub(crate) custom_style: Option<Style>,
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Wraps `inner` in a fully stylable CSD window frame with non-clipped drop shadows and resize borders.
///
/// # Examples
/// ```rust,ignore
/// use mtk::ui::widgets::csd_frame;
/// use mtk::{BoxShadow, Style, rgb, rgba};
///
/// csd_frame(app_body)
///     .corner_radius(12.0)
///     .border(1.0, rgba!(255, 255, 255, 20))
///     .shadow(BoxShadow::drop(rgba!(0, 0, 0, 160)).offset(0.0, 8.0).blur(24.0))
/// ```
#[track_caller]
pub fn csd_frame<V>(inner: V) -> CsdFrame<V> {
    CsdFrame {
        inner,
        shadow_margin: 0.0,
        resize_border_width: 8.0,
        resizable: true,
        custom_style: None,
        source_loc: Some(SourceLocation::here("CsdFrame")),
    }
}

impl<V> CsdFrame<V> {
    /// Sets the transparent margin reserved for outer drop shadows when floating (defaults to 0px for clean zero-gap edge snapping).
    pub fn shadow_margin(mut self, margin: f32) -> Self {
        self.shadow_margin = margin;
        self
    }

    /// Sets the width of the interactive resize border strip (defaults to 8px).
    pub fn resize_border_width(mut self, width: f32) -> Self {
        self.resize_border_width = width;
        self
    }

    /// Enables or disables 8-direction interactive window edge resizing (defaults to `true`).
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Applies custom [`Style`] overrides to the outer window frame.
    pub fn style(mut self, style: Style) -> Self {
        self.custom_style = Some(style);
        self
    }

    /// Sets the corner radius of the window frame when floating (defaults to 12px).
    pub fn corner_radius(mut self, radius: f32) -> Self {
        let mut st = self.custom_style.unwrap_or_default();
        st = st.corner_radius(radius);
        self.custom_style = Some(st);
        self
    }

    /// Sets the outer border width and color of the window frame.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        let mut st = self.custom_style.unwrap_or_default();
        st = st.border(width, color);
        self.custom_style = Some(st);
        self
    }

    /// Adds an analytical drop shadow to the window frame.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        let mut st = self.custom_style.unwrap_or_default();
        st = st.box_shadow(shadow);
        self.custom_style = Some(st);
        self
    }

    /// Sets the background color of the window frame.
    pub fn bg_color(mut self, color: Color) -> Self {
        let mut st = self.custom_style.unwrap_or_default();
        st = st.bg_color(color);
        self.custom_style = Some(st);
        self
    }

    /// Sets internal padding for the window frame.
    pub fn padding(mut self, padding: Edges) -> Self {
        let mut st = self.custom_style.unwrap_or_default();
        st = st.padding(padding);
        self.custom_style = Some(st);
        self
    }

    fn resolve_frame_style(&self, is_max_or_fullscreen: bool, tiled: TiledEdges) -> Style {
        let base_radius = 12.0;
        let base_border_width = 1.0;
        let border_color = rgba!(0, 0, 0, 80);

        let (radius, border_edges, shadow) = if is_max_or_fullscreen || tiled.is_all() {
            (Radius::all(0.0), Edges::all(0.0), BoxShadow::default())
        } else if tiled.is_any() {
            let r = Radius {
                tl: if tiled.top || tiled.left {
                    0.0
                } else {
                    base_radius
                },
                tr: if tiled.top || tiled.right {
                    0.0
                } else {
                    base_radius
                },
                bl: if tiled.bottom || tiled.left {
                    0.0
                } else {
                    base_radius
                },
                br: if tiled.bottom || tiled.right {
                    0.0
                } else {
                    base_radius
                },
            };
            let b = Edges {
                top: if tiled.top { 0.0 } else { base_border_width },
                right: if tiled.right { 0.0 } else { base_border_width },
                bottom: if tiled.bottom { 0.0 } else { base_border_width },
                left: if tiled.left { 0.0 } else { base_border_width },
            };
            (r, b, BoxShadow::default())
        } else {
            (
                Radius::all(base_radius),
                Edges::all(base_border_width),
                BoxShadow::inset(rgba!(255, 255, 255, 18))
                    .offset(0.0, 1.0)
                    .blur(0.0),
            )
        };

        let base_style = Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .corner_radius_precise(radius)
            .border_edges(border_edges, border_color)
            .bg_color(rgb!(15, 23, 42))
            .box_shadow(shadow)
            .overflow(Overflow::Hidden);

        let mut st = if let Some(ref custom) = self.custom_style {
            base_style.merge(custom.clone())
        } else {
            base_style
        };

        if is_max_or_fullscreen || tiled.is_all() {
            st = st.corner_radius(0.0);
            st = st.border(0.0, Color::transparent);
            st.base_effects.box_shadow = BoxShadow::default();
            st.base_effects.additional_shadows.clear();
        } else if tiled.is_any() {
            st = st.corner_radius_precise(radius);
            st = st.border_edges(border_edges, border_color);
            st.base_effects.box_shadow = BoxShadow::default();
            st.base_effects.additional_shadows.clear();
        }

        st
    }
}

/// Extension trait providing `.csd_frame()` on any view.
pub trait CsdFrameExt: Sized {
    /// Wraps this view in a [`CsdFrame`].
    fn csd_frame(self) -> CsdFrame<Self> {
        csd_frame(self)
    }
}

impl<V> CsdFrameExt for V {}

/// Persistent DOM element state for [`CsdFrame`].
pub struct CsdFrameElement<VEl> {
    root_node: Node,
    frame_node: Node,
    inner_element: VEl,
    is_max_or_fullscreen: bool,
    tiled_edges: TiledEdges,
    hovered_resize: Option<ResizeDirection>,
}

impl<State, V: View<State>> View<State> for CsdFrame<V> {
    type Element = CsdFrameElement<V::Element>;
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let root_node = ctx.create_node();
        let frame_node = ctx.create_node();

        if let Some(loc) = self.source_loc {
            ctx.set_node_source(frame_node, loc);
        }

        let inner_element = self.inner.build(ctx);
        let inner_node = self.inner.get_node(&inner_element);

        let is_max_or_fullscreen = ctx.is_window_maximized() || ctx.is_window_fullscreen();
        let tiled_edges = ctx.window_tiled_edges();
        let is_tiled_or_max = is_max_or_fullscreen || tiled_edges.is_any();
        let root_padding = if is_tiled_or_max || self.shadow_margin <= 0.0 {
            Edges::all(0.0)
        } else {
            Edges::all(self.shadow_margin)
        };

        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .padding(root_padding)
            .apply_to_node(ctx, root_node);

        let active_style = self.resolve_frame_style(is_max_or_fullscreen, tiled_edges);
        active_style.apply_to_node(ctx, frame_node);

        frame_node.append(ctx, inner_node);
        root_node.append(ctx, frame_node);

        CsdFrameElement {
            root_node,
            frame_node,
            inner_element,
            is_max_or_fullscreen,
            tiled_edges,
            hovered_resize: None,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let is_max_or_fullscreen = ctx.is_window_maximized() || ctx.is_window_fullscreen();
        let current_tiled_edges = ctx.window_tiled_edges();
        let current_tiled_or_max = is_max_or_fullscreen || current_tiled_edges.is_any();

        if is_max_or_fullscreen != element.is_max_or_fullscreen
            || current_tiled_edges != element.tiled_edges
            || self.shadow_margin != prev.shadow_margin
            || self.custom_style != prev.custom_style
        {
            element.is_max_or_fullscreen = is_max_or_fullscreen;
            element.tiled_edges = current_tiled_edges;

            let root_padding = if current_tiled_or_max || self.shadow_margin <= 0.0 {
                Edges::all(0.0)
            } else {
                Edges::all(self.shadow_margin)
            };

            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Percent(1.0))
                .padding(root_padding)
                .apply_to_node(ctx, element.root_node);

            let active_style = self.resolve_frame_style(is_max_or_fullscreen, current_tiled_edges);
            active_style.apply_to_node(ctx, element.frame_node);
        }

        self.inner
            .rebuild(&prev.inner, ctx, &mut element.inner_element);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.inner_element);
        element.root_node.remove(ctx);
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
        let is_max_or_fullscreen = ctx.is_window_maximized() || ctx.is_window_fullscreen();
        if !is_max_or_fullscreen && !element.tiled_edges.is_all() && self.resizable {
            match event {
                Event::CursorMoved { x, y, .. } => {
                    let resize_dir = self.detect_resize_direction(
                        ctx,
                        element.frame_node,
                        element.tiled_edges,
                        x,
                        y,
                    );

                    if element.hovered_resize != resize_dir {
                        element.hovered_resize = resize_dir;
                        if let Some(dir) = resize_dir {
                            ctx.set_cursor_icon(dir.into());
                        } else {
                            ctx.set_cursor_icon(CursorIcon::Default);
                        }
                    }

                    if resize_dir.is_some() {
                        return (EventResult::Handled, None);
                    }
                }
                Event::MouseInput {
                    button: winit::event::MouseButton::Left,
                    pressed: true,
                    ..
                } => {
                    if let Some(dir) = element.hovered_resize {
                        let _ = ctx.drag_resize_window(dir);
                        return (EventResult::Handled, None);
                    }
                }
                _ => {}
            }
        }

        self.inner
            .handle_event(&mut element.inner_element, state, event, ctx)
    }
}

impl<V> CsdFrame<V> {
    fn detect_resize_direction(
        &self,
        ctx: &mut Context,
        frame_node: Node,
        tiled: TiledEdges,
        x: f32,
        y: f32,
    ) -> Option<ResizeDirection> {
        let computed = frame_node.get_computed(ctx)?;
        let fx = computed.x;
        let fy = computed.y;
        let fw = computed.w;
        let fh = computed.h;

        if fw <= 0.0 || fh <= 0.0 {
            return None;
        }

        let bw = self.resize_border_width;

        let in_outer = x >= fx - bw && x <= fx + fw + bw && y >= fy - bw && y <= fy + fh + bw;
        if !in_outer {
            return None;
        }

        let near_left = !tiled.left && x <= fx + bw;
        let near_right = !tiled.right && x >= fx + fw - bw;
        let near_top = !tiled.top && y <= fy + bw;
        let near_bottom = !tiled.bottom && y >= fy + fh - bw;

        match (near_top, near_bottom, near_left, near_right) {
            (true, _, true, _) => Some(ResizeDirection::NorthWest),
            (true, _, _, true) => Some(ResizeDirection::NorthEast),
            (_, true, true, _) => Some(ResizeDirection::SouthWest),
            (_, true, _, true) => Some(ResizeDirection::SouthEast),
            (true, _, _, _) => Some(ResizeDirection::North),
            (_, true, _, _) => Some(ResizeDirection::South),
            (_, _, true, _) => Some(ResizeDirection::West),
            (_, _, _, true) => Some(ResizeDirection::East),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::text;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestMsg {
        Close,
        Minimize,
        Maximize,
    }

    #[test]
    fn test_action_buttons_custom_list_and_click() {
        let mut ctx = Context::new();
        let buttons = action_buttons::<TestMsg>()
            .buttons(&[
                ActionButton::Close,
                ActionButton::Hide,
                ActionButton::Expand,
            ])
            .dominant_color(rgb!(200, 200, 200))
            .stroke_width(2.0)
            .on_close(TestMsg::Close)
            .on_minimize(TestMsg::Minimize)
            .on_maximize(TestMsg::Maximize);

        let mut element = View::<()>::build(&buttons, &mut ctx);
        let node = View::<()>::get_node(&buttons, &element);
        assert!(ctx.layout.is_valid(node.0));
        assert_eq!(element.slots.len(), 3);

        let close_node = View::<()>::get_node(&element.slots[0].view, &element.slots[0].element);

        // Hover over close button -> StyledView sets hover state
        let (hover_res, _) = View::<()>::handle_event(
            &buttons,
            &mut element,
            &(),
            Event::CursorMoved {
                x: 0.0,
                y: 0.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: vec![close_node],
            },
            &mut ctx,
        );
        assert_eq!(hover_res, EventResult::Ignored);
        assert!(element.slots[0].element.1.is_hovered);

        // Press down on close button
        let (down_res, down_msg) = View::<()>::handle_event(
            &buttons,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 0.0,
                y: 0.0,
                hit_nodes: vec![close_node],
            },
            &mut ctx,
        );
        assert_eq!(down_res, EventResult::Handled);
        assert_eq!(down_msg, None);
        assert!(element.slots[0].element.1.is_active);

        // Release close button -> emits TestMsg::Close
        let (up_res, up_msg) = View::<()>::handle_event(
            &buttons,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                x: 0.0,
                y: 0.0,
                hit_nodes: vec![close_node],
            },
            &mut ctx,
        );
        assert_eq!(up_res, EventResult::Handled);
        assert_eq!(up_msg, Some(TestMsg::Close));

        View::<()>::teardown(&buttons, &mut ctx, &mut element);
    }

    #[test]
    fn test_action_buttons_default_blend_in() {
        let mut ctx = Context::new();
        let buttons = action_buttons::<()>();
        let mut element = View::<()>::build(&buttons, &mut ctx);
        assert_eq!(element.slots.len(), 3);
        let close_node = View::<()>::get_node(&element.slots[2].view, &element.slots[2].element);

        // Press down on close button
        let _ = View::<()>::handle_event(
            &buttons,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: true,
                x: 0.0,
                y: 0.0,
                hit_nodes: vec![close_node],
            },
            &mut ctx,
        );

        // Release close button without custom message handler -> calls ctx.request_window_close()
        let (up_res, up_msg) = View::<()>::handle_event(
            &buttons,
            &mut element,
            &(),
            Event::MouseInput {
                button: winit::event::MouseButton::Left,
                pressed: false,
                x: 0.0,
                y: 0.0,
                hit_nodes: vec![close_node],
            },
            &mut ctx,
        );

        assert_eq!(up_res, EventResult::Handled);
        assert_eq!(up_msg, None);
        assert!(ctx.close_requested);

        View::<()>::teardown(&buttons, &mut ctx, &mut element);
    }

    #[test]
    fn test_csd_frame_build_and_style() {
        let mut ctx = Context::new();
        let frame = csd_frame(text::<&str, ()>("Hello Window"))
            .shadow_margin(14.0)
            .corner_radius(10.0)
            .border(1.0, rgb!(51, 65, 85))
            .bg_color(rgb!(15, 23, 42));

        let mut element = View::<()>::build(&frame, &mut ctx);
        let node = View::<()>::get_node(&frame, &element);
        assert!(ctx.layout.is_valid(node.0));
        assert!(!element.is_max_or_fullscreen);
        assert!(!element.tiled_edges.is_any());
        View::<()>::teardown(&frame, &mut ctx, &mut element);
    }

    #[test]
    fn test_csd_frame_tiled_and_maximized_rebuild() {
        let mut ctx = Context::new();
        let frame = csd_frame(text::<&str, ()>("Hello Window")).shadow_margin(14.0);

        let mut element = View::<()>::build(&frame, &mut ctx);
        assert!(!element.is_max_or_fullscreen);
        assert!(!element.tiled_edges.is_any());

        // Simulate snapping to left edge (half-screen tiled)
        ctx.set_window_tiled_edges_override(Some(TiledEdges {
            top: false,
            right: false,
            bottom: false,
            left: true,
        }));
        View::<()>::rebuild(&frame, &frame, &mut ctx, &mut element);
        assert!(element.tiled_edges.left);
        assert!(!element.tiled_edges.right);

        // Simulate restoring to floating
        ctx.set_window_tiled_override(Some(false));
        View::<()>::rebuild(&frame, &frame, &mut ctx, &mut element);
        assert!(!element.tiled_edges.is_any());

        View::<()>::teardown(&frame, &mut ctx, &mut element);
    }
}
