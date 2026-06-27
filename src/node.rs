use crate::effects::Effects;
use crate::style::{Computed, Constraints};
use crate::{Context, sys};
use std::hash::Hash;

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
    /// Append a child node to the start of the parent node tree.
    pub fn prepend(&self, ctxt: &mut Context, child: Node) -> bool {
        unsafe { sys::muse_node_prepend(ctxt.ctx, self.0, child.0) }
    }

    /// Detach a node from its parent but don't destroy it,
    /// ideal for moving elements and appending them elsewhere.
    /// If you want to completely remove the node and its subsequent children,
    /// use `Context::destroy_node`.
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

    /// Append a child node to the end of the parent node tree.
    pub fn append(&self, ctxt: &mut Context, child: Node) -> bool {
        if !self.is_valid() || !child.is_valid() {
            return false;
        }
        unsafe { sys::muse_node_append(ctxt.ctx, self.0, child.0) }
    }

    /// Add constraints or overwrite the current existing constraints on a node.
    pub fn set_constraints(&self, ctxt: &mut Context, constraints: Constraints) {
        unsafe {
            sys::muse_constraints_set(ctxt.ctx, self.0, constraints.into());
        }
    }

    /// Get the currently applied constraints on the node.
    pub fn get_constraints(&self, ctxt: &Context) -> Option<Constraints> {
        let ptr = unsafe { sys::muse_constraints_get(ctxt.ctx, self.0) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { *ptr }.into())
        }
    }

    /// Add effects or overwrite the current existing effects on a node.
    pub fn set_effects(&self, ctxt: &mut Context, effects: Effects) {
        ctxt.effects.insert(*self, effects);
        ctxt.dirty_effects.insert(*self);
    }

    /// Fetch, modify, and apply constraints in one go. Useful for making small adjustments.
    pub fn update_constraints<F>(&self, ctxt: &mut Context, update_fn: F)
    where
        F: FnOnce(&mut Constraints),
    {
        let mut constraints = self.get_constraints(ctxt).unwrap_or_default();
        update_fn(&mut constraints);
        self.set_constraints(ctxt, constraints);
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

    /// Builder method to add constraints or overwrite the current existing constraints on a node.
    pub fn with_constraints(self, ctxt: &mut Context, constraints: Constraints) -> Self {
        self.set_constraints(ctxt, constraints);
        self
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
                },
            );
        }
    }

    /// Set text along with arbitrary userdata.
    pub fn set_text_with_userdata<T: 'static>(&self, ctxt: &mut Context, text: &str, userdata: T) {
        let c_string = std::ffi::CString::new(text).unwrap();
        let ptr = c_string.as_ptr();
        ctxt.texts.insert(*self, c_string);

        let any_box: Box<dyn std::any::Any> = Box::new(userdata);
        let userdata_ptr = Box::into_raw(Box::new(any_box));

        if let Some(old_ptr) = ctxt.text_userdatas.insert(*self, userdata_ptr) {
            unsafe {
                let _ = Box::from_raw(old_ptr);
            }
        }

        unsafe {
            sys::muse_text_set(
                ctxt.ctx,
                self.0,
                sys::muText {
                    data: ptr as *mut _,
                    userdata: userdata_ptr as *mut std::ffi::c_void,
                },
            );
        }
    }

    /// Remove the ability of a node to act like a text element.
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
}
