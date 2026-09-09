#![doc = include_str!("../README.md")]

pub mod animation;
pub mod colors;
pub mod command;
pub mod debugger;
pub mod effects;
pub mod image;
pub mod layer;
pub use mtk_layout as layout;
pub(crate) mod node;
pub mod render;
pub mod style;
pub mod text;
pub mod ui;
pub mod windowing;

use ::winit::keyboard::ModifiersState;
use ::winit::window::Window;
pub use mtk_macro::Lens;

pub use crate::colors::Color;
pub use crate::command::{Command, IntoCommand};
pub use crate::debugger::{LayoutSnapshot, NodeDebugInfo, SourceLocation};
pub use crate::effects::{Border, Effects, Radius};
pub use crate::image::{ImageCache, ImageData, ObjectFit, SvgData, SvgStyle};
pub use crate::layer::*;
pub use crate::layout::{LayoutEngine, NodeId};
pub use crate::node::Node;
pub use crate::render::RenderCommand;
pub use crate::style::*;
pub use crate::text::*;
pub use crate::ui::KineticTracker;
pub use crate::ui::widgets::canvas::{
    CanvasData, CanvasEventDetails, CanvasPainterKind, PaintContext, PixelPainter, WgpuPainter,
};
pub use crate::ui::{
    DragContext, DragPhase, Focusable, FocusableExt, KeyEvent, KeyEventContext, Keyed,
    KeyedViewSequence, keyed, keyed_sequence,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;

/// Represents payload data copied to or retrieved from the system clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardData {
    /// Plain UTF-8 text payload.
    Text(String),
}

/// The central coordinator and state container for the MTK user interface runtime.
///
/// `Context` acts as the primary bridge between high-level Rust [`View`](crate::ui::View) declarations
/// and MTK's underlying pure Rust layout engine. It maintains the live element tree,
/// calculates responsive layouts, handles spatial focus navigation, generates clipped render command streams,
/// and provides persistent access to system capabilities such as the OS clipboard.
///
/// # Architecture & Frame Lifecycle
///
/// The lifetime of a frame inside MTK follows a structured multi-pass execution model managed by `Context`:
///
/// 1. **Tree Construction**: Widgets create and attach layout primitives ([`Node`]) to the context.
/// 2. **Layout Pass (`compute_layout`)**: A multi-pass algorithm calculates intrinsic text sizes,
///    flex dimensions, percentages, and absolute bounds across the tree.
/// 3. **Render List Generation (`build_render_list`)**: The layout tree is flattened into a Z-indexed
///    array of draw commands ([`RenderCommand`]), applying scissor clipping rectangles for scroll views and containers.
/// 4. **Event Dispatch & Picking (`pick`)**: Coordinate hit-testing determines mouse target nodes and routes
///    keyboard/focus events.
///
/// # Examples
///
/// ```rust,ignore
/// use mtk::{Context, Rect};
///
/// let mut ctx = Context::new();
/// let root = ctx.create_node();
/// ctx.root_attach(root);
///
/// // Compute layout for an 800x600 window viewport
/// ctx.compute_layout(800.0, 600.0);
/// ```
///
/// `Context` acts as the primary bridge between high-level Rust [`View`](crate::ui::View) declarations
/// and the pure Rust layout engine, GPU text rasterizers, and event systems.
pub struct Context {
    pub layout: LayoutEngine,
    pub text_sizing_func: Option<
        Box<
            dyn Fn(
                &mut Context,
                Node,
                &str,
                Option<&dyn std::any::Any>,
                f32,
                f32,
            ) -> TextComputedOutput,
        >,
    >,
    pub effects: HashMap<Node, Effects>,
    pub dirty_effects: HashSet<Node>,
    pub scrollbars: HashMap<Node, ScrollbarStyle>,
    pub text_context: SharedTextContext,
    pub focused_node: Option<Node>,
    pub focusable_nodes: Vec<Node>,
    pub window: Option<Arc<dyn Window>>,
    pub modifiers: ModifiersState,
    pub ensure_visible_requests: HashMap<Node, crate::style::Rect>,
    pub clipboard: Arc<Mutex<Option<arboard::Clipboard>>>,
    pub canvases: RefCell<HashMap<Node, CanvasData>>,
    pub images: RefCell<HashMap<Node, (ImageData, ObjectFit)>>,
    pub svgs: RefCell<HashMap<Node, (SvgData, ObjectFit)>>,
    pub dt: f32,
    pub node_sources: HashMap<Node, SourceLocation>,
    pub highlight_node: Option<Node>,
    pub scale_factor: f32,
    pub captured_pointer: Option<PointerCapture>,

    // Core-level Super Layers and User Intermediate Layers
    pub base_layer: InternalLayer,
    pub intermediate_layers: Vec<UserLayer>,
    pub overlay_layer: InternalLayer,
    pub modal_layer: InternalLayer,
    pub active_layer: ActiveLayerId,
}

/// Policy specifying whether the OS cursor should remain free or be locked and hidden during pointer capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorGrabPolicy {
    /// Normal cursor remains visible and moves freely across the screen.
    #[default]
    Normal,
    /// Cursor is hidden and locked to its position for infinite relative delta dragging.
    Locked,
}

/// Active pointer capture metadata tracking which node owns the pointer stream.
#[derive(Clone, Debug)]
pub struct PointerCapture {
    pub node: Node,
    pub policy: CursorGrabPolicy,
    pub initial_pos: (f32, f32),
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Creates a new `Context` initialized with a fresh layout engine, text context,
    /// and event routing tables.
    pub fn new() -> Self {
        Self {
            layout: LayoutEngine::new(),
            text_sizing_func: None,
            effects: HashMap::new(),
            dirty_effects: HashSet::new(),
            scrollbars: HashMap::new(),
            text_context: Arc::new(Mutex::new(TextContext::new())),
            focused_node: None,
            focusable_nodes: Vec::new(),
            window: None,
            modifiers: ModifiersState::default(),
            ensure_visible_requests: HashMap::new(),
            clipboard: Arc::new(Mutex::new(None)),
            canvases: RefCell::new(HashMap::new()),
            images: RefCell::new(HashMap::new()),
            svgs: RefCell::new(HashMap::new()),
            dt: 0.016,
            node_sources: HashMap::new(),
            highlight_node: None,
            scale_factor: 1.0,
            captured_pointer: None,

            base_layer: InternalLayer::new(true),
            intermediate_layers: Vec::new(),
            overlay_layer: InternalLayer::new(false),
            modal_layer: InternalLayer::new(false),
            active_layer: ActiveLayerId::Base,
        }
    }

    /// Captures all pointer events to `node`. Subsequent mouse move and release events
    /// are dispatched to `node` regardless of cursor position.
    pub fn capture_pointer(&mut self, node: Node, policy: CursorGrabPolicy) {
        if policy == CursorGrabPolicy::Locked {
            if let Some(window) = &self.window {
                let _ = window
                    .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                    .or_else(|_| window.set_cursor_grab(winit::window::CursorGrabMode::Confined));
                window.set_cursor_visible(false);
            }
        }
        self.captured_pointer = Some(PointerCapture {
            node,
            policy,
            initial_pos: (0.0, 0.0),
        });
    }

    /// Releases any active pointer capture and restores the normal OS cursor state.
    pub fn release_pointer(&mut self) {
        if let Some(capture) = self.captured_pointer.take() {
            if capture.policy == CursorGrabPolicy::Locked {
                if let Some(window) = &self.window {
                    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
                    window.set_cursor_visible(true);
                }
            }
        }
    }

    /// Returns whether `node` currently holds pointer capture.
    pub fn is_pointer_captured(&self, node: Node) -> bool {
        self.captured_pointer
            .as_ref()
            .is_some_and(|c| c.node == node)
    }

    /// Returns whether any node currently holds pointer capture.
    pub fn has_pointer_capture(&self) -> bool {
        self.captured_pointer.is_some()
    }

    /// Returns the currently captured node, if any.
    pub fn captured_node(&self) -> Option<Node> {
        self.captured_pointer.as_ref().map(|c| c.node)
    }

    /// Returns a reference to the underlying layout engine.
    pub fn layout(&self) -> &LayoutEngine {
        &self.layout
    }

    /// Returns a mutable reference to the underlying layout engine.
    pub fn layout_mut(&mut self) -> &mut LayoutEngine {
        &mut self.layout
    }

    /// Returns a clone of the shared typography context for font measuring and layout caching.
    pub fn text_context(&self) -> SharedTextContext {
        self.text_context.clone()
    }

    /// Returns the currently active layer receiving input events.
    pub fn active_layer(&self) -> ActiveLayerId {
        if self.modal_layer.state.visible {
            ActiveLayerId::Modal
        } else if let Some(last_inter) = self
            .intermediate_layers
            .iter()
            .rfind(|l| l.state.visible && l.blocking)
        {
            ActiveLayerId::Intermediate(last_inter.id)
        } else if self.overlay_layer.state.visible {
            ActiveLayerId::Overlay
        } else {
            ActiveLayerId::Base
        }
    }

    /// Sets the currently active layer.
    pub fn set_active_layer(&mut self, layer: ActiveLayerId) {
        self.active_layer = layer;
    }

    /// Clears any focus belonging to a specific layer and restores focus if available.
    pub fn clear_layer_focus(&mut self, layer: ActiveLayerId) {
        let root = match layer {
            ActiveLayerId::Base => self.base_layer.state.root_node,
            ActiveLayerId::Modal => self.modal_layer.state.root_node,
            ActiveLayerId::Overlay => self.overlay_layer.state.root_node,
            ActiveLayerId::Intermediate(id) => self
                .intermediate_layers
                .iter()
                .find(|l| l.id == id)
                .and_then(|l| l.state.root_node),
        };

        if let (Some(root_node), Some(focused)) = (root, self.focused_node) {
            if focused.is_descendant_of(self, root_node) {
                self.clear_focus();
            }
        }
    }

    /// Returns the elapsed delta time in seconds from the most recent frame tick.
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// Sets the focused node to `node` and requests a window redraw.
    pub fn request_focus(&mut self, node: Node) {
        self.focused_node = Some(node);
        self.request_frame();
    }

    /// Clears the currently focused node and requests a window redraw.
    pub fn clear_focus(&mut self) {
        self.focused_node = None;
        self.request_frame();
    }

    /// Clears the currently focused node, removing any active keyboard focus ring (alias for [`clear_focus`](Self::clear_focus)).
    pub fn blur(&mut self) {
        self.clear_focus();
    }

    /// Checks if a mouse click should blur the currently focused node.
    ///
    /// If a node is currently focused and neither it nor any of its descendants
    /// are present in `hit_nodes`, clears focus and returns `Some(previous_node)`.
    /// If the click occurred inside the focused node or one of its descendants,
    /// focus is preserved and returns `None`.
    pub fn check_click_focus_blur(&mut self, hit_nodes: &[Node]) -> Option<Node> {
        if let Some(focused) = self.focused_node {
            let clicked_same = hit_nodes.iter().any(|n| n.is_descendant_of(self, focused));
            if !clicked_same {
                self.clear_focus();
                return Some(focused);
            }
        }
        None
    }

    /// Requests that a specific rectangular region of `node` be scrolled into view inside parent scroll views.
    pub fn request_ensure_visible(&mut self, node: Node, rect: crate::style::Rect) {
        self.ensure_visible_requests.insert(node, rect);
        self.request_frame();
    }

    /// Registers `node` as focusable for keyboard tab traversal.
    pub fn register_focusable(&mut self, node: Node) {
        if !self.focusable_nodes.contains(&node) {
            self.focusable_nodes.push(node);
        }
    }

    /// Unregisters `node` from keyboard tab traversal. If `node` was currently focused, clears focus.
    pub fn unregister_focusable(&mut self, node: Node) {
        self.focusable_nodes.retain(|n| n != &node);
        if self.focused_node == Some(node) {
            self.clear_focus();
        }
    }

    /// Returns the ordered list of focusable nodes belonging to the currently active layer.
    pub fn active_focusable_nodes(&self) -> Vec<Node> {
        let active = self.active_layer();
        match active {
            ActiveLayerId::Modal => {
                if let Some(modal_root) = self.modal_layer.state.root_node {
                    self.focusable_nodes
                        .iter()
                        .copied()
                        .filter(|n| n.is_descendant_of(self, modal_root))
                        .collect()
                } else {
                    Vec::new()
                }
            }
            ActiveLayerId::Intermediate(id) => {
                if let Some(inter_root) = self
                    .intermediate_layers
                    .iter()
                    .find(|l| l.id == id)
                    .and_then(|l| l.state.root_node)
                {
                    self.focusable_nodes
                        .iter()
                        .copied()
                        .filter(|n| n.is_descendant_of(self, inter_root))
                        .collect()
                } else {
                    Vec::new()
                }
            }
            ActiveLayerId::Overlay => {
                if let Some(overlay_root) = self.overlay_layer.state.root_node {
                    self.focusable_nodes
                        .iter()
                        .copied()
                        .filter(|n| n.is_descendant_of(self, overlay_root))
                        .collect()
                } else {
                    Vec::new()
                }
            }
            ActiveLayerId::Base => {
                let mut layer_roots = Vec::new();
                if let Some(r) = self.modal_layer.state.root_node {
                    layer_roots.push(r);
                }
                for l in &self.intermediate_layers {
                    if let Some(r) = l.state.root_node {
                        layer_roots.push(r);
                    }
                }
                if let Some(r) = self.overlay_layer.state.root_node {
                    layer_roots.push(r);
                }

                self.focusable_nodes
                    .iter()
                    .copied()
                    .filter(|n| !layer_roots.iter().any(|r| n.is_descendant_of(self, *r)))
                    .collect()
            }
        }
    }

    /// Advances keyboard focus to the next registered focusable node in the active layer.
    pub fn focus_next(&mut self) {
        let active_nodes = self.active_focusable_nodes();
        if active_nodes.is_empty() {
            self.clear_focus();
            return;
        }

        if let Some(focused) = self.focused_node {
            if let Some(idx) = active_nodes.iter().position(|n| *n == focused) {
                let next_idx = (idx + 1) % active_nodes.len();
                self.request_focus(active_nodes[next_idx]);
            } else {
                self.request_focus(active_nodes[0]);
            }
        } else {
            self.request_focus(active_nodes[0]);
        }
    }

    /// Reverses keyboard focus to the previous registered focusable node in the active layer.
    pub fn focus_prev(&mut self) {
        let active_nodes = self.active_focusable_nodes();
        if active_nodes.is_empty() {
            self.clear_focus();
            return;
        }

        if let Some(focused) = self.focused_node {
            if let Some(idx) = active_nodes.iter().position(|n| *n == focused) {
                let prev_idx = if idx == 0 {
                    active_nodes.len() - 1
                } else {
                    idx - 1
                };
                self.request_focus(active_nodes[prev_idx]);
            } else {
                self.request_focus(active_nodes[active_nodes.len() - 1]);
            }
        } else {
            self.request_focus(active_nodes[active_nodes.len() - 1]);
        }
    }

    /// Returns the currently focused node, or `None` if no node has focus.
    pub fn focused_node(&self) -> Option<Node> {
        self.focused_node
    }

    /// Returns the current state of active keyboard modifiers (Shift, Ctrl, Alt, Meta).
    pub fn modifiers(&self) -> winit::keyboard::ModifiersState {
        self.modifiers
    }

    /// Copies payload data to the persistent system clipboard.
    ///
    /// Keeps the underlying system clipboard handle alive across application frames to prevent
    /// Linux (X11 / Wayland) clipboard managers from losing selection contents upon drop.
    ///
    /// # Examples
    /// ```rust,ignore
    /// ctx.clipboard_copy(ClipboardData::Text("Hello, World!".to_string()));
    /// ```
    pub fn clipboard_copy(&self, data: ClipboardData) {
        if let Ok(mut guard) = self.clipboard.lock() {
            if guard.is_none() {
                *guard = arboard::Clipboard::new().ok();
            }
            if let Some(cb) = guard.as_mut() {
                match data {
                    ClipboardData::Text(text) => {
                        let _ = cb.set_text(text);
                    }
                }
            } else if let Ok(mut new_cb) = arboard::Clipboard::new() {
                match data {
                    ClipboardData::Text(text) => {
                        let _ = new_cb.set_text(text);
                    }
                }
                *guard = Some(new_cb);
            }
        }
    }

    /// Retrieves payload data from the persistent system clipboard.
    ///
    /// Returns `Some(ClipboardData)` if clipboard content is available, or `None` if
    /// the clipboard is empty or unsupported.
    ///
    /// # Examples
    /// ```rust,ignore
    /// if let Some(ClipboardData::Text(text)) = ctx.clipboard_get() {
    ///     println!("Pasted: {text}");
    /// }
    /// ```
    pub fn clipboard_get(&self) -> Option<ClipboardData> {
        if let Ok(mut guard) = self.clipboard.lock() {
            if guard.is_none() {
                *guard = arboard::Clipboard::new().ok();
            }
            if let Some(cb) = guard.as_mut() {
                if let Ok(text) = cb.get_text() {
                    return Some(ClipboardData::Text(text));
                }
            }
            // If get_text() failed (e.g. stale connection, or guard was None), retry with a fresh Clipboard instance
            if let Ok(mut new_cb) = arboard::Clipboard::new() {
                if let Ok(text) = new_cb.get_text() {
                    let result = Some(ClipboardData::Text(text));
                    *guard = Some(new_cb);
                    return result;
                }
                *guard = Some(new_cb);
            }
        }
        None
    }

    /// Allocates a new layout node in the layout engine. The node is independent until appended to a parent.
    pub fn create_node(&mut self) -> Node {
        Node(self.layout.create_node())
    }

    /// Returns a reference-counted handle to the underlying window, if attached.
    pub fn window(&self) -> Option<Arc<dyn winit::window::Window>> {
        self.window.clone()
    }

    /// Requests a new frame redraw on the associated window.
    pub fn request_frame(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    /// Destroys a node and all of its recursive children from the layout engine and cleans up associated text/effect state.
    pub fn destroy_node(&mut self, node: Node) {
        self.effects.remove(&node);
        self.dirty_effects.remove(&node);
        self.canvases.borrow_mut().remove(&node);
        self.layout.destroy_node(node.0);
    }

    /// Attaches `node` as the root node of the layout tree.
    pub fn root_attach(&mut self, node: Node) {
        self.base_layer.state.root_node = Some(node);
        self.layout.root_attach(node.0);
    }

    /// Detaches the current root node from the layout tree without destroying it.
    pub fn root_drop(&mut self) {
        self.base_layer.state.root_node = None;
        self.layout.root_drop();
    }

    /// Computes the complete bottom-up and top-down layout pass across the node tree given `viewport_width` and `viewport_height`.
    pub fn compute_layout(&mut self, viewport_width: f32, viewport_height: f32) {
        let mut layout = std::mem::take(&mut self.layout);
        let mut sizing_func = self.text_sizing_func.take();
        let text_context = self.text_context.clone();

        layout.compute_layout(
            viewport_width,
            viewport_height,
            |node_id, text, userdata, avail_w, avail_h| {
                if let Some(func) = sizing_func.as_mut() {
                    let out = func(self, Node(node_id), text, userdata, avail_w, avail_h);
                    crate::layout::TextMetrics {
                        width: out.computed_width,
                        height: out.computed_height,
                        baseline_offset: out.baseline_offset,
                    }
                } else {
                    let default_style = TextStyle::default();
                    let (style, spans) = if let Some(info) =
                        userdata.and_then(|u| u.downcast_ref::<crate::TextRenderInfo>())
                    {
                        (&info.style, &info.spans[..])
                    } else if let Some(style) = userdata.and_then(|u| u.downcast_ref::<TextStyle>())
                    {
                        (style, &[][..])
                    } else {
                        (&default_style, &[][..])
                    };
                    let out = crate::text::measure_text(
                        text,
                        style,
                        avail_w,
                        avail_h,
                        &text_context,
                        spans,
                    );
                    crate::layout::TextMetrics {
                        width: out.computed_width,
                        height: out.computed_height,
                        baseline_offset: out.baseline_offset,
                    }
                }
            },
        );

        self.layout = layout;
        self.text_sizing_func = sizing_func;

        self.clamp_scroll_offsets(viewport_width, viewport_height);
    }

    fn clamp_scroll_offsets(&mut self, viewport_width: f32, viewport_height: f32) {
        if self.layout.layout_order.is_empty() {
            return;
        }

        let nodes: Vec<Node> = self.layout.layout_order.iter().map(|&n| Node(n)).collect();

        let mut any_clamped = false;
        for node in nodes {
            if !node.is_valid() {
                continue;
            }

            if let Some(constraints) = node.get_constraints(self) {
                let is_scrollable_y = constraints.overflow == crate::Overflow::Scroll
                    || constraints.overflow == crate::Overflow::Auto;
                let is_scrollable_x = constraints.overflow == crate::Overflow::Scroll
                    || constraints.overflow == crate::Overflow::Auto
                    || constraints.overflow == crate::Overflow::Hidden;

                if is_scrollable_y || is_scrollable_x {
                    if let Some(computed) = node.get_computed(self) {
                        let content_h = node.compute_content_height(self);
                        let max_scroll_y = (content_h - computed.h).max(0.0);

                        let content_w = computed.content_w.max(computed.w);
                        let max_scroll_x = (content_w - computed.w).max(0.0);

                        let mut new_y = constraints.scroll.y;
                        let mut new_x = constraints.scroll.x;

                        if is_scrollable_y && new_y > max_scroll_y {
                            new_y = max_scroll_y;
                        }
                        if is_scrollable_x && new_x > max_scroll_x {
                            new_x = max_scroll_x;
                        }

                        if (new_y - constraints.scroll.y).abs() > 0.001
                            || (new_x - constraints.scroll.x).abs() > 0.001
                        {
                            node.update_constraints(self, |c| {
                                c.scroll.y = new_y;
                                c.scroll.x = new_x;
                            });
                            any_clamped = true;
                        }
                    }
                }
            }
        }

        if any_clamped {
            let mut layout = std::mem::take(&mut self.layout);
            let mut sizing_func = self.text_sizing_func.take();
            let text_context = self.text_context.clone();

            layout.compute_layout(
                viewport_width,
                viewport_height,
                |node_id, text, userdata, avail_w, avail_h| {
                    if let Some(func) = sizing_func.as_mut() {
                        let out = func(self, Node(node_id), text, userdata, avail_w, avail_h);
                        crate::layout::TextMetrics {
                            width: out.computed_width,
                            height: out.computed_height,
                            baseline_offset: out.baseline_offset,
                        }
                    } else {
                        let default_style = TextStyle::default();
                        let (style, spans) = if let Some(info) =
                            userdata.and_then(|u| u.downcast_ref::<crate::TextRenderInfo>())
                        {
                            (&info.style, &info.spans[..])
                        } else if let Some(style) =
                            userdata.and_then(|u| u.downcast_ref::<TextStyle>())
                        {
                            (style, &[][..])
                        } else {
                            (&default_style, &[][..])
                        };
                        let out = crate::text::measure_text(
                            text,
                            style,
                            avail_w,
                            avail_h,
                            &text_context,
                            spans,
                        );
                        crate::layout::TextMetrics {
                            width: out.computed_width,
                            height: out.computed_height,
                            baseline_offset: out.baseline_offset,
                        }
                    }
                },
            );

            self.layout = layout;
            self.text_sizing_func = sizing_func;
        }
    }

    /// Flattens the layout hierarchy into a Z-sorted render command queue clipped to `viewport`.
    pub fn build_render_list(&mut self, viewport: Rect) {
        self.layout.build_render_list(viewport);
    }

    /// Returns an iterator yielding low-level `RenderCommand` items generated by `build_render_list`.
    pub fn render_list(&self) -> impl Iterator<Item = RenderCommand<'_>> {
        self.layout
            .render_list
            .iter()
            .map(|cmd| RenderCommand { cmd })
    }

    /// Sets the custom text sizing trampoline callback invoked by the layout engine during layout computation.
    pub fn set_text_sizing_func<F>(&mut self, func: F)
    where
        F: Fn(&mut Context, Node, &str, Option<&dyn std::any::Any>, f32, f32) -> TextComputedOutput
            + 'static,
    {
        self.text_sizing_func = Some(Box::new(func));
    }

    /// Registers raw font bytes (.ttf or .otf) into the shared text context.
    pub fn register_fonts(&mut self, font_data: Vec<u8>) {
        self.text_context.lock().unwrap().register_fonts(font_data);
    }

    /// Loads and registers a font file from disk into the shared text context.
    pub fn register_font_file(&mut self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        self.text_context.lock().unwrap().register_font_file(path)
    }

    /// Performs a fast $O(1)$ scalar hit test at `(x, y)` and returns a list of hit nodes ordered from top-most child to parent.
    pub fn pick(&mut self, x: f32, y: f32) -> Vec<Node> {
        self.layout.pick(x, y).iter().map(|&n| Node(n)).collect()
    }

    /// Records the Rust source code location for a layout node.
    pub fn set_node_source(&mut self, node: Node, loc: SourceLocation) {
        self.node_sources.insert(node, loc);
    }

    /// Retrieves the source code definition location for a layout node, if recorded.
    pub fn get_node_source(&self, node: Node) -> Option<SourceLocation> {
        self.node_sources.get(&node).copied()
    }

    /// Returns the currently attached root node of the layout tree, if any.
    pub fn root_node(&self) -> Option<Node> {
        self.layout
            .root
            .map(Node)
            .or(self.base_layer.state.root_node)
    }

    /// Counts total active layout nodes in the layout engine.
    pub fn count_nodes(&self) -> usize {
        let mut count = 0;
        if let Some(root) = self.root_node() {
            count = 1 + self.count_children_recursive(root);
        }
        count
    }

    fn count_children_recursive(&self, node: Node) -> usize {
        let children = node.children(self);
        let mut count = children.len();
        for child in children {
            count += self.count_children_recursive(child);
        }
        count
    }

    /// Builds a hierarchical debug snapshot of the entire layout tree for inspector frontends.
    pub fn build_debug_snapshot(
        &self,
        viewport_w: f32,
        viewport_h: f32,
        hovered_node: Option<Node>,
    ) -> LayoutSnapshot {
        let root = self.root_node().and_then(|r| self.build_node_debug_info(r));
        let total_nodes = self.count_nodes();
        LayoutSnapshot {
            root,
            total_nodes,
            viewport_w,
            viewport_h,
            hovered_node: hovered_node.map(|n| n.id()),
        }
    }

    fn build_node_debug_info(&self, node: Node) -> Option<NodeDebugInfo> {
        let computed = node.get_computed(self)?;
        let constraints = node.get_constraints(self);
        let source = self.get_node_source(node);
        let text = node.get_text(self).map(|s| s.to_string());

        let name = if let Some(src) = source {
            src.type_name.to_string()
        } else if text.is_some() {
            "Text".to_string()
        } else {
            format!("Node(#{})", node.id())
        };

        let metrics = if let Some(cons) = constraints {
            crate::debugger::NodeBoxMetrics {
                x: computed.x,
                y: computed.y,
                w: computed.w,
                h: computed.h,
                content_w: computed.content_w,
                content_h: computed.content_h,
                pad_top: cons.padding.top,
                pad_bottom: cons.padding.bottom,
                pad_left: cons.padding.left,
                pad_right: cons.padding.right,
                border_top: cons.border.top,
                border_bottom: cons.border.bottom,
                border_left: cons.border.left,
                border_right: cons.border.right,
                flex_direction: format!("{:?}", cons.flex_direction),
                flex_grow: cons.flex_grow,
                flex_shrink: cons.flex_shrink,
            }
        } else {
            crate::debugger::NodeBoxMetrics {
                x: computed.x,
                y: computed.y,
                w: computed.w,
                h: computed.h,
                content_w: computed.content_w,
                content_h: computed.content_h,
                ..Default::default()
            }
        };

        let children = node
            .children(self)
            .into_iter()
            .filter_map(|c| self.build_node_debug_info(c))
            .collect();

        Some(NodeDebugInfo {
            id: node.id(),
            name,
            source,
            metrics,
            text,
            children,
        })
    }
}

pub mod text_property {
    pub use parley::layout::Alignment;
    pub use parley::style::*;
}

pub mod winit {
    pub use winit::*;
}

pub mod wgpu {
    pub use wgpu::*;
}

pub mod bytemuck {
    pub use bytemuck::*;
}
