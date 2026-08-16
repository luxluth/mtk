#![doc = include_str!("../../../README.md")]

pub mod animation;
pub mod colors;
pub mod effects;
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
use crate::effects::Effects;
pub use crate::node::Node;
use crate::render::RenderCommand;
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
///
/// // Generate clipping render commands
/// ctx.build_render_list(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 });
/// for cmd in ctx.render_list() {
///     // Process render commands for WGPU / Canvas
/// }
/// ```
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
    pub(crate) dt: f32,
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
            dt: 0.016,
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
        if self.focused_node == Some(node.clone()) {
            self.clear_focus();
        }
    }

    /// Advances keyboard focus to the next registered focusable node in order.
    pub fn focus_next(&mut self) {
        if self.focusable_nodes.is_empty() {
            return;
        }
        if let Some(focused) = &self.focused_node {
            if let Some(idx) = self.focusable_nodes.iter().position(|n| n == focused) {
                let next_idx = (idx + 1) % self.focusable_nodes.len();
                self.focused_node = Some(self.focusable_nodes[next_idx].clone());
            } else {
                self.focused_node = Some(self.focusable_nodes[0].clone());
            }
        } else {
            self.focused_node = Some(self.focusable_nodes[0].clone());
        }
    }

    /// Reverses keyboard focus to the previous registered focusable node in order.
    pub fn focus_prev(&mut self) {
        if self.focusable_nodes.is_empty() {
            return;
        }
        if let Some(focused) = &self.focused_node {
            if let Some(idx) = self.focusable_nodes.iter().position(|n| n == focused) {
                let prev_idx = if idx == 0 {
                    self.focusable_nodes.len() - 1
                } else {
                    idx - 1
                };
                self.focused_node = Some(self.focusable_nodes[prev_idx].clone());
            } else {
                self.focused_node =
                    Some(self.focusable_nodes[self.focusable_nodes.len() - 1].clone());
            }
        } else {
            self.focused_node = Some(self.focusable_nodes[self.focusable_nodes.len() - 1].clone());
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
            if let Some(cb) = guard.as_mut() {
                match data {
                    ClipboardData::Text(text) => {
                        let _ = cb.set_text(text);
                    }
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
        if let Ok(mut guard) = self.clipboard.lock() {
            if let Some(cb) = guard.as_mut() {
                if let Ok(text) = cb.get_text() {
                    return Some(ClipboardData::Text(text));
                }
            }
        }
        None
    }

    /// Allocates a new layout node in the C engine. The node is independent until appended to a parent.
    pub fn create_node(&mut self) -> Node {
        Node(unsafe { sys::muse_node_create(self.ctx) })
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
        unsafe {
            sys::muse_root_attach(self.ctx, node.0);
        }
    }

    /// Detaches the current root node from the layout tree without destroying it.
    pub fn root_drop(&mut self) {
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
            if !list.items.is_null() {
                unsafe extern "C" {
                    fn free(ptr: *mut std::ffi::c_void);
                }
                unsafe {
                    free(list.items as *mut std::ffi::c_void);
                }
            }
            return Vec::new();
        }

        let slice = unsafe { std::slice::from_raw_parts(list.items, list.count) };
        let nodes = slice.iter().map(|sys_node| Node(*sys_node)).collect();

        unsafe extern "C" {
            fn free(ptr: *mut std::ffi::c_void);
        }
        unsafe {
            free(list.items as *mut std::ffi::c_void);
        }

        nodes
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
