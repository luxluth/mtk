use crate::Context;
use crate::effects::Effects;
use crate::layout::NodeId;
use crate::style::{Computed, Constraints, ScrollbarStyle};

/// An opaque, generational handle representing a UI layout element.
///
/// `Node` wraps a generational layout `NodeId`. Nodes form the tree hierarchy
/// and carry styling constraints ([`Constraints`](crate::style::Constraints)),
/// computed layout geometry ([`Computed`](crate::style::Computed)), text content, and visual effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Node(pub NodeId);

impl std::ops::Deref for Node {
    type Target = NodeId;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Node {
    /// Returns the unique numeric identifier for this layout node.
    pub fn id(&self) -> u64 {
        self.0.index as u64
    }

    /// Returns the raw underlying layout node identifier.
    pub fn raw(&self) -> NodeId {
        self.0
    }

    /// Constructs a `Node` from a layout node handle.
    pub fn from_raw(raw: NodeId) -> Self {
        Node(raw)
    }

    pub fn get_invalid() -> Node {
        Node(NodeId::INVALID)
    }

    /// Prepend a child node to the start of the parent node tree.
    pub fn prepend(&self, ctxt: &mut Context, child: Node) -> bool {
        ctxt.layout.prepend(self.0, child.0)
    }

    /// Mark this node as dirty, forcing a layout recomputation for it and its ancestors.
    pub fn set_dirty(&self, ctxt: &mut Context) {
        ctxt.layout.set_dirty(self.0);
    }

    /// Remove a child node from its parent.
    ///
    /// If you want to completely remove the node and its subsequent children,
    /// consider calling [Context::destroy_node] after removing it from the parent layout hierarchy.
    pub fn remove(&self, ctxt: &mut Context) -> bool {
        ctxt.layout.remove(self.0)
    }

    /// Put a node after a designated sibling.
    pub fn put_after(&self, ctxt: &mut Context, sibling: Node) -> bool {
        ctxt.layout.put_after(sibling.0, self.0)
    }

    /// Put a node before a designated sibling.
    pub fn put_before(&self, ctxt: &mut Context, sibling: Node) -> bool {
        ctxt.layout.put_before(sibling.0, self.0)
    }

    /// Check if a node is valid.
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Returns the parent of this node in the layout hierarchy, if any.
    pub fn parent(&self, ctxt: &Context) -> Option<Node> {
        ctxt.layout.parent(self.0).map(Node)
    }

    /// Returns the first child of this node in the layout hierarchy, if any.
    pub fn first_child(&self, ctxt: &Context) -> Option<Node> {
        ctxt.layout.first_child(self.0).map(Node)
    }

    /// Returns the last child of this node in the layout hierarchy, if any.
    pub fn last_child(&self, ctxt: &Context) -> Option<Node> {
        ctxt.layout.last_child(self.0).map(Node)
    }

    /// Returns the next sibling of this node in the layout hierarchy, if any.
    pub fn next_sibling(&self, ctxt: &Context) -> Option<Node> {
        ctxt.layout.next_sibling(self.0).map(Node)
    }

    /// Returns the previous sibling of this node in the layout hierarchy, if any.
    pub fn prev_sibling(&self, ctxt: &Context) -> Option<Node> {
        ctxt.layout.prev_sibling(self.0).map(Node)
    }

    /// Returns true if this node is equal to or a descendant of `ancestor`.
    pub fn is_descendant_of(&self, ctxt: &Context, ancestor: Node) -> bool {
        ctxt.layout.is_descendant_of(self.0, ancestor.0)
    }

    /// Append a child node to the end of the parent node tree.
    pub fn append(&self, ctxt: &mut Context, child: Node) -> bool {
        ctxt.layout.append(self.0, child.0)
    }

    /// Set constraints on a node.
    pub fn set_constraints(&self, ctxt: &mut Context, constraints: Constraints) {
        ctxt.layout.set_constraints(self.0, constraints);
    }

    /// Get constraints currently set on a node.
    pub fn get_constraints(&self, ctxt: &Context) -> Option<Constraints> {
        ctxt.layout.get_constraints(self.0).copied()
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
        if let Some(effects) = ctxt.effects.get_mut(self) {
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

    /// Sets the scrollbar style on this node.
    pub fn set_scrollbar_style(&self, ctxt: &mut Context, style: ScrollbarStyle) {
        ctxt.scrollbars.insert(*self, style);
    }

    /// Gets the scrollbar style on this node, if configured.
    pub fn get_scrollbar_style(&self, ctxt: &Context) -> Option<ScrollbarStyle> {
        ctxt.scrollbars.get(self).cloned()
    }

    /// Fetches, modifies, and applies scrollbar style on this node.
    pub fn update_scrollbar_style<F>(&self, ctxt: &mut Context, update_fn: F)
    where
        F: FnOnce(&mut ScrollbarStyle),
    {
        if let Some(sb) = ctxt.scrollbars.get_mut(self) {
            update_fn(sb);
        } else {
            let mut sb = ScrollbarStyle::default();
            update_fn(&mut sb);
            ctxt.scrollbars.insert(*self, sb);
        }
    }

    /// Builder method to set scrollbar style on a node.
    pub fn with_scrollbar_style(self, ctxt: &mut Context, style: ScrollbarStyle) -> Self {
        self.set_scrollbar_style(ctxt, style);
        self
    }

    /// Get the computed bounding box and offset of the node.
    pub fn get_computed(&self, ctxt: &Context) -> Option<Computed> {
        ctxt.layout.get_computed(self.0).copied()
    }

    /// Returns a vector of direct child nodes attached to this parent.
    pub fn children(&self, ctxt: &Context) -> Vec<Node> {
        ctxt.layout.children(self.0).into_iter().map(Node).collect()
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
        let existing_userdata = ctxt
            .layout
            .texts
            .get_mut(self.0)
            .and_then(|t| t.userdata.take());
        ctxt.layout
            .set_text(self.0, text.to_string(), existing_userdata);
    }

    /// Set text along with arbitrary userdata.
    pub fn set_text_with_userdata<T: 'static>(&self, ctxt: &mut Context, text: &str, userdata: T) {
        ctxt.layout
            .set_text(self.0, text.to_string(), Some(Box::new(userdata)));
    }

    /// Remove text from a node.
    pub fn unset_text(&self, ctxt: &mut Context) {
        ctxt.layout.unset_text(self.0);
    }

    /// Get the text associated with this node, if any.
    pub fn get_text<'a>(&self, ctxt: &'a Context) -> Option<&'a str> {
        ctxt.layout.get_text(self.0)
    }

    /// Get the userdata associated with this node, if any.
    pub fn get_text_userdata<'a, T: 'static>(&self, ctxt: &'a Context) -> Option<&'a T> {
        ctxt.layout.get_text_userdata::<T>(self.0)
    }

    /// Get a mutable reference to the userdata associated with this node, if any.
    pub fn get_text_userdata_mut<'a, T: 'static>(
        &self,
        ctxt: &'a mut Context,
    ) -> Option<&'a mut T> {
        ctxt.layout.get_text_userdata_mut::<T>(self.0)
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
