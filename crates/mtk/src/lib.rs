#![doc = include_str!("../../../README.md")]

pub mod animation;
pub mod colors;
pub mod debugger;
pub mod effects;
pub mod image;
pub mod layer;
pub(crate) mod node;
pub mod render;
pub mod style;
pub(crate) mod sys;
pub mod text;
pub mod ui;
pub mod windowing;

use ::winit::keyboard::ModifiersState;
use ::winit::window::Window;
pub use mtk_macro::Lens;

pub use crate::colors::Color;
use crate::debugger::{LayoutSnapshot, NodeDebugInfo, SourceLocation};
use crate::effects::Effects;
pub use crate::image::{ImageData, ObjectFit, SvgData};
pub use crate::layer::*;
pub use crate::node::Node;
pub use crate::render::RenderCommand;
pub use crate::style::*;
pub use crate::text::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::CString;
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
/// and MTK's underlying C layout engine (`sys::muContext`). It maintains the live element tree,
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
/// and the low-level C layout engine ([`muse.h`](crate::sys)), GPU text rasterizers, and event systems.
pub struct Context {
    pub(crate) ctx: *mut sys::muContext,
    pub(crate) texts: HashMap<Node, CString>,
    pub(crate) effects: HashMap<Node, Effects>,
    pub(crate) dirty_effects: HashSet<Node>,
    pub(crate) text_userdatas: HashMap<Node, *mut Box<dyn std::any::Any>>,
    pub(crate) text_context: SharedTextContext,
    pub(crate) focused_node: Option<Node>,
    pub(crate) focusable_nodes: Vec<Node>,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) modifiers: ModifiersState,
    pub(crate) ensure_visible_requests: HashMap<Node, crate::style::Rect>,
    pub(crate) clipboard: Arc<Mutex<Option<arboard::Clipboard>>>,
    pub(crate) canvases: RefCell<HashMap<Node, crate::ui::widgets::CanvasData>>,
    pub(crate) images: RefCell<HashMap<Node, (ImageData, ObjectFit)>>,
    pub(crate) svgs: RefCell<HashMap<Node, (SvgData, ObjectFit)>>,
    pub(crate) dt: f32,
    pub(crate) node_sources: HashMap<Node, SourceLocation>,
    pub(crate) highlight_node: Option<Node>,

    // Core-level Super Layers and User Intermediate Layers
    pub base_layer: InternalLayer,
    pub intermediate_layers: Vec<UserLayer>,
    pub overlay_layer: InternalLayer,
    pub modal_layer: InternalLayer,
    pub active_layer: ActiveLayerId,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Creates a new `Context` initialized with a zeroed C layout context, text context,
    /// and persistent system clipboard handle.
    pub fn new() -> Self {
        let ctx = Box::into_raw(Box::new(unsafe { std::mem::zeroed::<sys::muContext>() }));
        let clipboard = Arc::new(Mutex::new(arboard::Clipboard::new().ok()));
        Self {
            ctx,
            texts: HashMap::new(),
            effects: HashMap::new(),
            dirty_effects: HashSet::new(),
            text_userdatas: HashMap::new(),
            text_context: Arc::new(Mutex::new(TextContext::new())),
            focused_node: None,
            focusable_nodes: Vec::new(),
            window: None,
            modifiers: winit::keyboard::ModifiersState::default(),
            ensure_visible_requests: HashMap::new(),
            clipboard,
            canvases: RefCell::new(HashMap::new()),
            images: RefCell::new(HashMap::new()),
            svgs: RefCell::new(HashMap::new()),
            dt: 0.016,
            node_sources: HashMap::new(),
            highlight_node: None,

            base_layer: InternalLayer::new(true),
            intermediate_layers: Vec::new(),
            overlay_layer: InternalLayer::new(false),
            modal_layer: InternalLayer::new(false),
            active_layer: ActiveLayerId::Base,
        }
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
        if let Ok(mut guard) = self.clipboard.lock()
            && let Some(cb) = guard.as_mut()
        {
            match data {
                ClipboardData::Text(text) => {
                    let _ = cb.set_text(text);
                }
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
        if let Ok(mut guard) = self.clipboard.lock()
            && let Some(cb) = guard.as_mut()
            && let Ok(text) = cb.get_text()
        {
            return Some(ClipboardData::Text(text));
        }
        None
    }

    /// Allocates a new layout node in the C engine. The node is independent until appended to a parent.
    pub fn create_node(&mut self) -> Node {
        let node = Node(unsafe { sys::muse_node_create(self.ctx) });
        unsafe {
            let default_cons: sys::muConstraints = Constraints::default().into();
            sys::muse_constraints_set(self.ctx, node.0, default_cons);
        }
        node
    }

    /// Requests a new frame redraw on the associated window.
    pub fn request_frame(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    /// Destroys a node and all of its recursive children from the C engine and cleans up associated text/effect state.
    pub fn destroy_node(&mut self, node: Node) {
        self.texts.remove(&node);
        self.effects.remove(&node);
        self.dirty_effects.remove(&node);
        self.canvases.borrow_mut().remove(&node);

        if let Some(ptr) = self.text_userdatas.remove(&node) {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        unsafe {
            sys::muse_node_destroy(self.ctx, node.0);
        }
    }

    /// Attaches `node` as the root node of the layout tree.
    pub fn root_attach(&mut self, node: Node) {
        self.base_layer.state.root_node = Some(node);
        unsafe {
            sys::muse_root_attach(self.ctx, node.0);
        }
    }

    /// Detaches the current root node from the layout tree without destroying it.
    pub fn root_drop(&mut self) {
        self.base_layer.state.root_node = None;
        unsafe {
            sys::muse_root_drop(self.ctx);
        }
    }

    /// Computes the complete bottom-up and top-down layout pass across the node tree given `viewport_width` and `viewport_height`.
    pub fn compute_layout(&mut self, viewport_width: f32, viewport_height: f32) {
        crate::text::CURRENT_CONTEXT.with(|c| c.set(self as *mut Context));
        unsafe {
            sys::muse_compute_layout(self.ctx, viewport_width, viewport_height);
        }
        crate::text::CURRENT_CONTEXT.with(|c| c.set(std::ptr::null_mut()));
    }

    /// Flattens the layout hierarchy into a Z-sorted render command queue clipped to `viewport`.
    pub fn build_render_list(&mut self, viewport: Rect) {
        unsafe {
            sys::muse_build_render_list(self.ctx, viewport.into());
        }
    }

    /// Returns an iterator yielding low-level `RenderCommand` items generated by `build_render_list`.
    pub fn render_list(&self) -> impl Iterator<Item = RenderCommand<'_>> {
        let list = unsafe { &(*self.ctx).render_list };
        let slice = if list.items.is_null() || list.count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(list.items, list.count) }
        };

        slice.iter().map(|cmd| RenderCommand { cmd })
    }

    /// Sets the custom text sizing trampoline callback invoked by the C engine during layout computation.
    pub fn set_text_sizing_func<F>(&mut self, func: F)
    where
        F: Fn(&mut Context, Node, &str, Option<&dyn std::any::Any>, f32, f32) -> TextComputedOutput
            + 'static,
    {
        crate::text::SIZING_FUNCS.with(|funcs| {
            funcs.borrow_mut().insert(self.ctx as usize, Box::new(func));
        });
        unsafe {
            (*self.ctx).text_sizing_func = Some(crate::text::text_sizing_trampoline);
        }
    }

    /// Performs a fast $O(1)$ scalar hit test at `(x, y)` and returns a list of hit nodes ordered from top-most child to parent.
    pub fn pick(&mut self, x: f32, y: f32) -> Vec<Node> {
        let list = unsafe { sys::muse_node_pick(self.ctx, x, y) };
        if list.items.is_null() || list.count == 0 {
            return Vec::new();
        }

        let slice = unsafe { std::slice::from_raw_parts(list.items, list.count) };
        slice.iter().map(|sys_node| Node(*sys_node)).collect()
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
        let r = unsafe { (*self.ctx).root };
        if unsafe { sys::muse_muid_is_valid(r) } {
            Some(Node(r))
        } else {
            self.base_layer.state.root_node
        }
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

impl Drop for Context {
    fn drop(&mut self) {
        crate::text::SIZING_FUNCS.with(|funcs| {
            funcs.borrow_mut().remove(&(self.ctx as usize));
        });

        for (_, ptr) in self.text_userdatas.drain() {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        unsafe {
            sys::muse_context_free(self.ctx);
            let _ = Box::from_raw(self.ctx);
        }
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
