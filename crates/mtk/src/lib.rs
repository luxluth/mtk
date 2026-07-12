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

pub use mtk_macro::Lens;

use crate::effects::Effects;
pub use crate::node::Node;
use crate::render::RenderCommand;
pub use crate::style::*;
pub use crate::text::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::CString;
use std::sync::Arc;
use std::sync::Mutex;

pub struct Context {
    pub(crate) ctx: *mut sys::muContext,
    pub(crate) texts: HashMap<Node, CString>,
    pub(crate) effects: HashMap<Node, Effects>,
    pub(crate) dirty_effects: HashSet<Node>,
    pub(crate) text_userdatas: HashMap<Node, *mut Box<dyn std::any::Any>>,
    pub(crate) text_context: SharedTextContext,
    pub(crate) focused_node: Option<Node>,
    pub(crate) focusable_nodes: Vec<Node>,
    pub(crate) window: Option<Arc<winit::window::Window>>,
    pub(crate) modifiers: winit::keyboard::ModifiersState,
    pub(crate) ensure_visible_requests: HashMap<Node, crate::style::Rect>,
}

impl Context {
    pub fn new() -> Self {
        let ctx = Box::into_raw(Box::new(unsafe { std::mem::zeroed::<sys::muContext>() }));
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
        }
    }

    pub fn request_focus(&mut self, node: Node) {
        self.focused_node = Some(node);
        self.request_frame();
    }

    pub fn clear_focus(&mut self) {
        self.focused_node = None;
        self.request_frame();
    }

    pub fn request_ensure_visible(&mut self, node: Node, rect: crate::style::Rect) {
        self.ensure_visible_requests.insert(node, rect);
        self.request_frame();
    }

    pub fn register_focusable(&mut self, node: Node) {
        if !self.focusable_nodes.contains(&node) {
            self.focusable_nodes.push(node);
        }
    }

    pub fn unregister_focusable(&mut self, node: Node) {
        self.focusable_nodes.retain(|n| n != &node);
        if self.focused_node == Some(node.clone()) {
            self.clear_focus();
        }
    }

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

    pub fn focused_node(&self) -> Option<Node> {
        self.focused_node
    }

    pub fn modifiers(&self) -> winit::keyboard::ModifiersState {
        self.modifiers
    }

    /// Create a new valid node. It's not inserted in the tree but it exists.
    pub fn create_node(&mut self) -> Node {
        Node(unsafe { sys::muse_node_create(self.ctx) })
    }

    pub fn request_frame(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    /// Destroy a node from the tree removing its children at the same time.
    pub fn destroy_node(&mut self, node: Node) {
        self.texts.remove(&node);
        self.effects.remove(&node);
        self.dirty_effects.remove(&node);

        if let Some(ptr) = self.text_userdatas.remove(&node) {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }

        unsafe {
            sys::muse_node_destroy(self.ctx, node.0);
        }
    }

    /// Set this node as the root of the tree.
    pub fn root_attach(&mut self, node: Node) {
        unsafe {
            sys::muse_root_attach(self.ctx, node.0);
        }
    }

    /// Remove the current root (not cleaned up or destroyed).
    pub fn root_drop(&mut self) {
        unsafe {
            sys::muse_root_drop(self.ctx);
        }
    }

    /// Compute the final layout filling up the context with computed bounds.
    pub fn compute_layout(&mut self, viewport_width: f32, viewport_height: f32) {
        crate::text::CURRENT_CONTEXT.with(|c| c.set(self as *mut Context));
        unsafe {
            sys::muse_compute_layout(self.ctx, viewport_width, viewport_height);
        }
        crate::text::CURRENT_CONTEXT.with(|c| c.set(std::ptr::null_mut()));
    }

    /// Builds a flattened, Z-sorted array of commands to be consumed by the renderer.
    pub fn build_render_list(&mut self, viewport: Rect) {
        unsafe {
            sys::muse_build_render_list(self.ctx, viewport.into());
        }
    }

    /// Iterates over the current render list commands.
    pub fn render_list(&self) -> impl Iterator<Item = RenderCommand<'_>> {
        let list = unsafe { &(*self.ctx).render_list };
        let slice = if list.items.is_null() || list.count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(list.items, list.count) }
        };

        slice.iter().map(|cmd| RenderCommand { cmd })
    }

    /// Sets the text sizing function used during layout computation.
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

    /// Pick nodes under the given coordinates, returning a list of hit nodes.
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
