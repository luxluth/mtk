use crate::effects::Effects;
use crate::style::{Computed, Constraints};
use crate::{Context, sys};
use std::hash::Hash;

/// An opaque, generational handle representing a UI layout element.
///
/// `Node` wraps a C-level layout node (`sys::muNode`). Nodes form the tree hierarchy
/// and carry styling constraints ([`Constraints`](crate::style::Constraints)),
/// computed layout geometry ([`Computed`](crate::style::Computed)), text content, and visual effects.
#[derive(Clone, Copy, Debug)]
pub struct Node(pub(crate) sys::muNode);

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sys::muse_muid_eq(self.0, other.0) }
    }
}

impl Eq for Node {}

impl std::ops::Deref for Node {
    type Target = sys::muNode;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.numeral.hash(state);
        self.generation.hash(state);
    }
}

impl Node {
    /// Returns the unique numeric identifier for this layout node.
    pub fn id(&self) -> u64 {
        self.0.numeral as u64
    }

    pub fn get_invalid() -> Node {
        Node(unsafe { crate::sys::muse_muid_invalid() })
    }

    /// Prepend a child node to the start of the parent node tree.
    pub fn prepend(&self, ctxt: &mut Context, child: Node) -> bool {
        unsafe { sys::muse_node_prepend(ctxt.ctx, self.0, child.0) }
    }

    /// Mark this node as dirty, forcing a layout recomputation for it and its ancestors.
    pub fn set_dirty(&self, ctxt: &mut Context) {
        unsafe { sys::muse_node_set_dirty(ctxt.ctx, self.0) }
    }

    /// Remove a child node from its parent.
    ///
    /// If you want to completely remove the node and its subsequent children,
    /// consider calling [Context::destroy_node] after removing it from the parent layout hierarchy.
    pub fn remove(&self, ctxt: &mut Context) -> bool {
        unsafe { sys::muse_node_remove(ctxt.ctx, self.0) }
    }

    /// Put a node after a designated sibling.
    pub fn put_after(&self, ctxt: &mut Context, sibling: Node) -> bool {
        unsafe { sys::muse_node_put_after(ctxt.ctx, sibling.0, self.0) }
    }

    /// Put a node before a designated sibling.
    pub fn put_before(&self, ctxt: &mut Context, sibling: Node) -> bool {
        unsafe { sys::muse_node_put_before(ctxt.ctx, sibling.0, self.0) }
    }

    /// Check if a node is valid.
    pub fn is_valid(&self) -> bool {
        unsafe { sys::muse_muid_is_valid(self.0) }
    }

    /// Returns the parent of this node in the layout hierarchy, if any.
    pub fn parent(&self, ctxt: &Context) -> Option<Node> {
        let p = unsafe { sys::muse_node_parent(ctxt.ctx, self.0) };
        if unsafe { sys::muse_muid_is_valid(p) } {
            Some(Node(p))
        } else {
            None
        }
    }

    /// Returns true if this node is equal to or a descendant of `ancestor`.
    pub fn is_descendant_of(&self, ctxt: &Context, ancestor: Node) -> bool {
        if *self == ancestor {
            return true;
        }
        let mut curr = *self;
        while let Some(p) = curr.parent(ctxt) {
            if p == ancestor {
                return true;
            }
            curr = p;
        }
        false
    }

    /// Append a child node to the end of the parent node tree.
    pub fn append(&self, ctxt: &mut Context, child: Node) -> bool {
        if !self.is_valid() || !child.is_valid() {
            return false;
        }
        unsafe { sys::muse_node_append(ctxt.ctx, self.0, child.0) }
    }

    /// Set constraints on a node.
    pub fn set_constraints(&self, ctxt: &mut Context, constraints: Constraints) {
        unsafe {
            sys::muse_constraints_set(ctxt.ctx, self.0, constraints.into());
        }
    }

    /// Get constraints currently set on a node.
    pub fn get_constraints(&self, ctxt: &Context) -> Option<Constraints> {
        let cons = unsafe { sys::muse_constraints_get(ctxt.ctx, self.0) };
        if cons.is_null() {
            None
        } else {
            Some(unsafe { *cons }.into())
        }
    }

    /// Fetch, modify, and apply constraints in one go. Useful for making small adjustments.
    pub fn update_constraints<F>(&self, ctxt: &mut Context, update_fn: F)
    where
        F: FnOnce(&mut Constraints),
    {
        let existing = self.get_constraints(ctxt);
        let old_constraints = existing.unwrap_or_default();
        let mut new_constraints = old_constraints.clone();

        update_fn(&mut new_constraints);
        if existing.is_none() || old_constraints != new_constraints {
            self.set_constraints(ctxt, new_constraints);
        }
    }

    /// Builder method to add constraints or overwrite the current existing constraints on a node.
    pub fn with_constraints(self, ctxt: &mut Context, constraints: Constraints) -> Self {
        self.set_constraints(ctxt, constraints);
        self
    }

    /// Set effects on a node.
    pub fn set_effects(&self, ctxt: &mut Context, effects: Effects) {
        ctxt.effects.insert(*self, effects);
        ctxt.dirty_effects.insert(*self);
    }

    /// Get effects on a node.
    pub fn get_effects(&self, ctxt: &Context) -> Option<Effects> {
        ctxt.effects.get(self).cloned()
    }

    /// Fetch, modify, and apply effects in one go.
    pub fn update_effects<F>(&self, ctxt: &mut Context, update_fn: F)
    where
        F: FnOnce(&mut Effects),
    {
        if let Some(effects) = ctxt.effects.get_mut(&self) {
            update_fn(effects);
            ctxt.dirty_effects.insert(*self);
        } else {
            let mut effects = Effects::default();
            update_fn(&mut effects);
            ctxt.effects.insert(*self, effects);
            ctxt.dirty_effects.insert(*self);
        }
    }

    /// Builder method to add effects or overwrite the current existing effects on a node.
    pub fn with_effects(self, ctxt: &mut Context, effects: Effects) -> Self {
        self.set_effects(ctxt, effects);
        self
    }

    /// Get the computed bounding box and offset of the node.
    pub fn get_computed(&self, ctxt: &Context) -> Option<Computed> {
        let comp = unsafe { sys::muse_computed_get(ctxt.ctx, self.0) };
        if comp.is_null() {
            None
        } else {
            Some(unsafe { *comp }.into())
        }
    }

    /// Returns a vector of direct child nodes attached to this parent.
    pub fn children(&self, ctxt: &Context) -> Vec<Node> {
        let mut list = Vec::new();
        let mut curr = unsafe { sys::muse_first_child_get(ctxt.ctx, self.0) };
        let null_val = sys::MUSE_SPARSE_NULL as usize;
        while curr.numeral != null_val && curr.generation != null_val {
            list.push(Node(curr));
            curr = unsafe { sys::muse_next_sibling_get(ctxt.ctx, curr) };
        }
        list
    }

    /// Computes the total content height of this node.
    pub fn compute_content_height(&self, ctxt: &Context) -> f32 {
        let computed = match self.get_computed(ctxt) {
            Some(c) => c,
            None => return 0.0,
        };
        computed.content_h.max(computed.h)
    }

    /// Transform a node into a text element, making it partake in text sizing.
    pub fn set_text(&self, ctxt: &mut Context, text: &str) {
        let c_string = std::ffi::CString::new(text).unwrap();
        let ptr = c_string.as_ptr();
        ctxt.texts.insert(*self, c_string);

        // Preserve existing userdata if any
        let existing_userdata = ctxt
            .text_userdatas
            .get(self)
            .copied()
            .unwrap_or(std::ptr::null_mut());

        unsafe {
            sys::muse_text_set(
                ctxt.ctx,
                self.0,
                sys::muText {
                    data: ptr as *mut _,
                    userdata: existing_userdata as *mut std::ffi::c_void,
                    cached_avail_w: -1.0,
                    cached_avail_h: -1.0,
                    cached_output: sys::muTextComputedOutput {
                        computed_width: 0.0,
                        computed_height: 0.0,
                        baseline_offset: 0.0,
                    },
                    is_cached: false,
                },
            );
        }
    }

    /// Set text along with arbitrary userdata.
    pub fn set_text_with_userdata<T: 'static>(&self, ctxt: &mut Context, text: &str, userdata: T) {
        let c_string = std::ffi::CString::new(text).unwrap();
        let ptr = c_string.as_ptr();
        ctxt.texts.insert(*self, c_string);

        let boxed: Box<Box<dyn std::any::Any>> = Box::new(Box::new(userdata));
        let raw_ptr = Box::into_raw(boxed);
        ctxt.text_userdatas.insert(*self, raw_ptr);

        unsafe {
            sys::muse_text_set(
                ctxt.ctx,
                self.0,
                sys::muText {
                    data: ptr as *mut _,
                    userdata: raw_ptr as *mut std::ffi::c_void,
                    cached_avail_w: -1.0,
                    cached_avail_h: -1.0,
                    cached_output: sys::muTextComputedOutput {
                        computed_width: 0.0,
                        computed_height: 0.0,
                        baseline_offset: 0.0,
                    },
                    is_cached: false,
                },
            );
        }
    }

    /// Remove text from a node.
    pub fn unset_text(&self, ctxt: &mut Context) {
        ctxt.texts.remove(self);
        if let Some(ptr) = ctxt.text_userdatas.remove(self) {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
        unsafe {
            sys::muse_text_unset(ctxt.ctx, self.0);
        }
    }

    /// Get the text associated with this node, if any.
    pub fn get_text<'a>(&self, ctxt: &'a Context) -> Option<&'a str> {
        ctxt.texts.get(self).and_then(|c_str| c_str.to_str().ok())
    }

    /// Get the userdata associated with this node, if any.
    pub fn get_text_userdata<'a, T: 'static>(&self, ctxt: &'a Context) -> Option<&'a T> {
        ctxt.text_userdatas.get(self).and_then(|ptr| {
            let b = unsafe { &**ptr };
            b.downcast_ref::<T>()
        })
    }

    /// Get a mutable reference to the userdata associated with this node, if any.
    pub fn get_text_userdata_mut<'a, T: 'static>(
        &self,
        ctxt: &'a mut Context,
    ) -> Option<&'a mut T> {
        ctxt.text_userdatas.get_mut(self).and_then(|ptr| {
            let b = unsafe { &mut **ptr };
            b.downcast_mut::<T>()
        })
    }

    /// Returns the bounding rectangles (in local coordinates `[x, y, w, h]`) of a byte range in the node's text.
    pub fn get_text_range_geometry(
        &self,
        ctx: &Context,
        range: std::ops::Range<usize>,
    ) -> Vec<[f32; 4]> {
        let Some(text) = self.get_text(ctx) else {
            return Vec::new();
        };

        let default_style = crate::TextStyle::default();
        let (style, spans) =
            if let Some(info) = self.get_text_userdata::<crate::TextRenderInfo>(ctx) {
                (&info.style, &info.spans[..])
            } else if let Some(style) = self.get_text_userdata::<crate::TextStyle>(ctx) {
                (style, &[][..])
            } else {
                (&default_style, &[][..])
            };

        let computed = self.get_computed(ctx).unwrap_or_default();
        let constraints = self.get_constraints(ctx).unwrap_or_default();
        let inner_w = (computed.w - constraints.padding.left - constraints.padding.right).max(0.0);
        let inner_h = (computed.h - constraints.padding.top - constraints.padding.bottom).max(0.0);

        crate::text::get_range_geometry(
            text,
            style,
            inner_w,
            inner_h,
            range,
            &ctx.text_context,
            spans,
        )
    }
}
