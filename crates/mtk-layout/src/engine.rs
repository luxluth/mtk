use super::sparse_set::{NodeId, SparseSet};
use super::types::{
    AlignItems, AlignSelf, CachedTextMeasurement, Computed, Constraints, FlexDirection, FlexWrap,
    Hierarchy, JustifyContent, Overflow, PositionStrategy, Rect, RenderCommand, RenderCommandKind,
    Size, TextMetrics, TextNode,
};

#[derive(Clone, Copy, Debug, Default)]
struct FlexItemScratch {
    node: NodeId,
    basis: f32,
    min_size: f32,
    max_size: f32,
    flex_grow: f32,
    flex_shrink: f32,
    target_size: f32,
    frozen: bool,
}

pub struct LayoutEngine {
    pub hierarchies: SparseSet<Hierarchy>,
    pub constraints: SparseSet<Constraints>,
    pub computed: SparseSet<Computed>,
    pub dirties: SparseSet<()>,
    pub texts: SparseSet<TextNode>,

    pub available_ids: Vec<NodeId>,
    pub next_numeral: u32,
    pub root: Option<NodeId>,

    pub layout_order: Vec<NodeId>,
    pub layout_order_dirty: bool,

    pub render_list: Vec<RenderCommand>,
    pub render_list_dirty: bool,

    pub pick_list: Vec<NodeId>,
    pub scratch_children: Vec<NodeId>,
    scratch_flex_items: Vec<FlexItemScratch>,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            hierarchies: SparseSet::new(),
            constraints: SparseSet::new(),
            computed: SparseSet::new(),
            dirties: SparseSet::new(),
            texts: SparseSet::new(),

            available_ids: Vec::new(),
            next_numeral: 0,
            root: None,

            layout_order: Vec::new(),
            layout_order_dirty: true,

            render_list: Vec::new(),
            render_list_dirty: true,

            pick_list: Vec::new(),
            scratch_children: Vec::new(),
            scratch_flex_items: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn is_valid(&self, node: NodeId) -> bool {
        node.is_valid() && self.hierarchies.has(node)
    }

    pub fn create_node(&mut self) -> NodeId {
        let node = if let Some(mut recycled) = self.available_ids.pop() {
            recycled.generation = recycled.generation.wrapping_add(1);
            recycled
        } else {
            let id = NodeId {
                index: self.next_numeral,
                generation: 0,
            };
            self.next_numeral += 1;
            id
        };

        self.hierarchies.insert(node, Hierarchy::default());
        self.constraints.insert(node, Constraints::default());
        self.computed.insert(node, Computed::default());
        self.set_dirty(node);

        node
    }

    pub fn destroy_node(&mut self, node: NodeId) {
        if !self.is_valid(node) {
            return;
        }

        self.remove(node);

        // Recursively destroy children
        if let Some(hrc) = self.hierarchies.get(node).copied() {
            let mut curr = hrc.first_child;
            while let Some(child) = curr {
                let next = self.hierarchies.get(child).and_then(|h| h.next_sibling);
                self.destroy_node(child);
                curr = next;
            }
        }

        self.hierarchies.remove(node);
        self.constraints.remove(node);
        self.computed.remove(node);
        self.dirties.remove(node);
        self.texts.remove(node);

        self.available_ids.push(node);
        self.layout_order_dirty = true;
        self.render_list_dirty = true;
    }

    pub fn root_attach(&mut self, node: NodeId) {
        self.root = Some(node);
        self.layout_order_dirty = true;
        self.set_dirty(node);
    }

    pub fn root_drop(&mut self) {
        self.root = None;
        self.layout_order_dirty = true;
        self.render_list_dirty = true;
    }

    #[inline(always)]
    pub fn set_dirty(&mut self, node: NodeId) {
        if node.is_valid() {
            self.dirties.insert(node, ());
            self.render_list_dirty = true;
        }
    }

    #[inline(always)]
    pub fn is_dirty(&self, node: NodeId) -> bool {
        self.dirties.has(node)
    }

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.hierarchies.get(node).and_then(|h| h.parent)
    }

    pub fn first_child(&self, node: NodeId) -> Option<NodeId> {
        self.hierarchies.get(node).and_then(|h| h.first_child)
    }

    #[inline(always)]
    pub fn last_child(&self, node: NodeId) -> Option<NodeId> {
        self.hierarchies.get(node).and_then(|h| h.last_child)
    }

    pub fn next_sibling(&self, node: NodeId) -> Option<NodeId> {
        self.hierarchies.get(node).and_then(|h| h.next_sibling)
    }

    #[inline(always)]
    pub fn prev_sibling(&self, node: NodeId) -> Option<NodeId> {
        self.hierarchies.get(node).and_then(|h| h.prev_sibling)
    }

    pub fn children(&self, parent: NodeId) -> Vec<NodeId> {
        let mut list = Vec::new();
        let mut curr = self.first_child(parent);
        while let Some(child) = curr {
            list.push(child);
            curr = self.next_sibling(child);
        }
        list
    }

    pub fn is_descendant_of(&self, node: NodeId, ancestor: NodeId) -> bool {
        if node == ancestor {
            return true;
        }
        let mut curr = node;
        while let Some(p) = self.parent(curr) {
            if p == ancestor {
                return true;
            }
            curr = p;
        }
        false
    }

    pub fn remove(&mut self, node: NodeId) -> bool {
        if !self.is_valid(node) {
            return false;
        }

        let (parent, prev, next) = {
            let Some(hrc) = self.hierarchies.get(node) else {
                return false;
            };
            (hrc.parent, hrc.prev_sibling, hrc.next_sibling)
        };

        let Some(parent_node) = parent else {
            return true;
        };

        if let Some(prev_node) = prev {
            if let Some(prev_hrc) = self.hierarchies.get_mut(prev_node) {
                prev_hrc.next_sibling = next;
            }
        } else if let Some(parent_hrc) = self.hierarchies.get_mut(parent_node) {
            parent_hrc.first_child = next;
        }

        if let Some(next_node) = next {
            if let Some(next_hrc) = self.hierarchies.get_mut(next_node) {
                next_hrc.prev_sibling = prev;
            }
        } else if let Some(parent_hrc) = self.hierarchies.get_mut(parent_node) {
            parent_hrc.last_child = prev;
        }

        if let Some(hrc) = self.hierarchies.get_mut(node) {
            hrc.parent = None;
            hrc.prev_sibling = None;
            hrc.next_sibling = None;
        }

        self.layout_order_dirty = true;
        self.set_dirty(parent_node);
        true
    }

    pub fn append(&mut self, parent: NodeId, child: NodeId) -> bool {
        if !self.is_valid(parent) || !self.is_valid(child) || parent == child {
            return false;
        }

        self.remove(child);

        let last_child = self.hierarchies.get(parent).and_then(|h| h.last_child);

        if let Some(last) = last_child {
            if let Some(last_hrc) = self.hierarchies.get_mut(last) {
                last_hrc.next_sibling = Some(child);
            }
            if let Some(child_hrc) = self.hierarchies.get_mut(child) {
                child_hrc.parent = Some(parent);
                child_hrc.prev_sibling = Some(last);
                child_hrc.next_sibling = None;
            }
            if let Some(parent_hrc) = self.hierarchies.get_mut(parent) {
                parent_hrc.last_child = Some(child);
            }
        } else {
            if let Some(child_hrc) = self.hierarchies.get_mut(child) {
                child_hrc.parent = Some(parent);
                child_hrc.prev_sibling = None;
                child_hrc.next_sibling = None;
            }
            if let Some(parent_hrc) = self.hierarchies.get_mut(parent) {
                parent_hrc.first_child = Some(child);
                parent_hrc.last_child = Some(child);
            }
        }

        self.layout_order_dirty = true;
        self.set_dirty(parent);
        true
    }

    pub fn prepend(&mut self, parent: NodeId, child: NodeId) -> bool {
        if !self.is_valid(parent) || !self.is_valid(child) || parent == child {
            return false;
        }

        self.remove(child);

        let first_child = self.hierarchies.get(parent).and_then(|h| h.first_child);

        if let Some(first) = first_child {
            if let Some(first_hrc) = self.hierarchies.get_mut(first) {
                first_hrc.prev_sibling = Some(child);
            }
            if let Some(child_hrc) = self.hierarchies.get_mut(child) {
                child_hrc.parent = Some(parent);
                child_hrc.prev_sibling = None;
                child_hrc.next_sibling = Some(first);
            }
            if let Some(parent_hrc) = self.hierarchies.get_mut(parent) {
                parent_hrc.first_child = Some(child);
            }
        } else {
            if let Some(child_hrc) = self.hierarchies.get_mut(child) {
                child_hrc.parent = Some(parent);
                child_hrc.prev_sibling = None;
                child_hrc.next_sibling = None;
            }
            if let Some(parent_hrc) = self.hierarchies.get_mut(parent) {
                parent_hrc.first_child = Some(child);
                parent_hrc.last_child = Some(child);
            }
        }

        self.layout_order_dirty = true;
        self.set_dirty(parent);
        true
    }

    pub fn put_before(&mut self, sibling: NodeId, node: NodeId) -> bool {
        if !self.is_valid(sibling) || !self.is_valid(node) || sibling == node {
            return false;
        }

        let Some(parent) = self.parent(sibling) else {
            return false;
        };

        self.remove(node);

        let sibling_prev = self.hierarchies.get(sibling).and_then(|h| h.prev_sibling);

        if let Some(prev) = sibling_prev {
            if let Some(prev_hrc) = self.hierarchies.get_mut(prev) {
                prev_hrc.next_sibling = Some(node);
            }
        } else if let Some(parent_hrc) = self.hierarchies.get_mut(parent) {
            parent_hrc.first_child = Some(node);
        }

        if let Some(node_hrc) = self.hierarchies.get_mut(node) {
            node_hrc.parent = Some(parent);
            node_hrc.prev_sibling = sibling_prev;
            node_hrc.next_sibling = Some(sibling);
        }

        if let Some(sibling_hrc) = self.hierarchies.get_mut(sibling) {
            sibling_hrc.prev_sibling = Some(node);
        }

        self.layout_order_dirty = true;
        self.set_dirty(parent);
        true
    }

    pub fn put_after(&mut self, sibling: NodeId, node: NodeId) -> bool {
        if !self.is_valid(sibling) || !self.is_valid(node) || sibling == node {
            return false;
        }

        let Some(parent) = self.parent(sibling) else {
            return false;
        };

        self.remove(node);

        let sibling_next = self.hierarchies.get(sibling).and_then(|h| h.next_sibling);

        if let Some(next) = sibling_next {
            if let Some(next_hrc) = self.hierarchies.get_mut(next) {
                next_hrc.prev_sibling = Some(node);
            }
        } else if let Some(parent_hrc) = self.hierarchies.get_mut(parent) {
            parent_hrc.last_child = Some(node);
        }

        if let Some(node_hrc) = self.hierarchies.get_mut(node) {
            node_hrc.parent = Some(parent);
            node_hrc.prev_sibling = Some(sibling);
            node_hrc.next_sibling = sibling_next;
        }

        if let Some(sibling_hrc) = self.hierarchies.get_mut(sibling) {
            sibling_hrc.next_sibling = Some(node);
        }

        self.layout_order_dirty = true;
        self.set_dirty(parent);
        true
    }

    pub fn set_constraints(&mut self, node: NodeId, constraints: Constraints) {
        if self.is_valid(node) {
            self.constraints.insert(node, constraints);
            self.set_dirty(node);
        }
    }

    pub fn get_constraints(&self, node: NodeId) -> Option<&Constraints> {
        self.constraints.get(node)
    }

    pub fn get_constraints_mut(&mut self, node: NodeId) -> Option<&mut Constraints> {
        self.constraints.get_mut(node)
    }

    pub fn get_computed(&self, node: NodeId) -> Option<&Computed> {
        self.computed.get(node)
    }

    pub fn get_computed_mut(&mut self, node: NodeId) -> Option<&mut Computed> {
        self.computed.get_mut(node)
    }

    pub fn set_text(
        &mut self,
        node: NodeId,
        text: String,
        userdata: Option<Box<dyn std::any::Any>>,
    ) {
        if self.is_valid(node) {
            self.texts.insert(
                node,
                TextNode {
                    content: text,
                    userdata,
                    cache: None,
                },
            );
            self.set_dirty(node);
        }
    }

    pub fn unset_text(&mut self, node: NodeId) {
        if self.is_valid(node) {
            self.texts.remove(node);
            self.set_dirty(node);
        }
    }

    pub fn get_text(&self, node: NodeId) -> Option<&str> {
        self.texts.get(node).map(|t| t.content.as_str())
    }

    pub fn get_text_userdata<T: 'static>(&self, node: NodeId) -> Option<&T> {
        self.texts
            .get(node)
            .and_then(|t| t.userdata.as_ref())
            .and_then(|b| b.downcast_ref::<T>())
    }

    pub fn get_text_userdata_mut<T: 'static>(&mut self, node: NodeId) -> Option<&mut T> {
        self.texts
            .get_mut(node)
            .and_then(|t| t.userdata.as_mut())
            .and_then(|b| b.downcast_mut::<T>())
    }

    fn collect_layout_order_recursive(&mut self, node: NodeId) {
        self.layout_order.push(node);
        let mut curr = self.first_child(node);
        while let Some(child) = curr {
            self.collect_layout_order_recursive(child);
            curr = self.next_sibling(child);
        }
    }

    pub fn ensure_layout_order(&mut self) {
        if !self.layout_order_dirty && !self.layout_order.is_empty() {
            return;
        }
        self.layout_order.clear();
        if let Some(root) = self.root {
            self.collect_layout_order_recursive(root);
        }
        self.layout_order_dirty = false;
    }

    fn clamp_min_max(comp: &mut Computed, cons: &Constraints) {
        if cons.min_width > 0.0 && comp.w < cons.min_width {
            comp.w = cons.min_width;
        }
        if cons.max_width.is_finite() && comp.w > cons.max_width {
            comp.w = cons.max_width;
        }
        if cons.min_height > 0.0 && comp.h < cons.min_height {
            comp.h = cons.min_height;
        }
        if cons.max_height.is_finite() && comp.h > cons.max_height {
            comp.h = cons.max_height;
        }
    }

    fn apply_aspect_ratio(comp: &mut Computed, cons: &Constraints) {
        if cons.aspect_ratio <= 0.0 {
            return;
        }

        let ar = cons.aspect_ratio;
        let is_w_fixed = matches!(cons.width, Size::Fixed(_) | Size::Percent(_));
        let is_h_fixed = matches!(cons.height, Size::Fixed(_) | Size::Percent(_));

        if is_w_fixed && is_h_fixed {
            return;
        }

        if is_w_fixed && comp.w > 0.0 {
            comp.h = comp.w / ar;
            Self::clamp_min_max(comp, cons);
            return;
        }

        if is_h_fixed && comp.h > 0.0 {
            comp.w = comp.h * ar;
            Self::clamp_min_max(comp, cons);
            return;
        }

        if comp.w > 0.0 && (comp.h == 0.0 || cons.height == Size::Fit) {
            comp.h = comp.w / ar;
            Self::clamp_min_max(comp, cons);
        } else if comp.h > 0.0 && (comp.w == 0.0 || cons.width == Size::Fit) {
            comp.w = comp.h * ar;
            Self::clamp_min_max(comp, cons);
        }
    }

    fn get_flex_basis(
        cons: &Constraints,
        comp: &Computed,
        is_row_dir: bool,
        inner_main: f32,
    ) -> f32 {
        if let Size::Percent(p) = cons.flex_basis {
            inner_main * p
        } else if let Size::Fixed(px) = cons.flex_basis {
            px as f32
        } else if is_row_dir && matches!(cons.width, Size::Percent(_)) {
            if let Size::Percent(p) = cons.width {
                inner_main * p
            } else {
                0.0
            }
        } else if !is_row_dir && matches!(cons.height, Size::Percent(_)) {
            if let Size::Percent(p) = cons.height {
                inner_main * p
            } else {
                0.0
            }
        } else if is_row_dir && matches!(cons.width, Size::Fixed(_)) {
            if let Size::Fixed(px) = cons.width {
                px as f32
            } else {
                0.0
            }
        } else if !is_row_dir && matches!(cons.height, Size::Fixed(_)) {
            if let Size::Fixed(px) = cons.height {
                px as f32
            } else {
                0.0
            }
        } else {
            let is_main_fill = if is_row_dir {
                cons.width == Size::Fill
            } else {
                cons.height == Size::Fill
            };
            if cons.flex_grow > 0.0 || is_main_fill {
                return 0.0;
            }
            if cons.aspect_ratio > 0.0 {
                if is_row_dir && comp.w == 0.0 && comp.h > 0.0 {
                    return comp.h * cons.aspect_ratio;
                } else if !is_row_dir && comp.h == 0.0 && comp.w > 0.0 {
                    return comp.w / cons.aspect_ratio;
                }
            }
            if is_row_dir { comp.w } else { comp.h }
        }
    }

    fn is_row(dir: FlexDirection) -> bool {
        matches!(dir, FlexDirection::Row | FlexDirection::RowReverse)
    }

    fn is_reverse(dir: FlexDirection) -> bool {
        matches!(
            dir,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        )
    }

    fn shift_subtree(&mut self, node: NodeId, dx: f32, dy: f32) {
        let mut curr = self.first_child(node);
        while let Some(child) = curr {
            if let Some(comp) = self.computed.get_mut(child) {
                comp.x += dx;
                comp.y += dy;
            }
            self.shift_subtree(child, dx, dy);
            curr = self.next_sibling(child);
        }
    }

    fn bubble_up_fit_size(&mut self, node: NodeId) {
        let mut curr = self.parent(node);
        while let Some(parent) = curr {
            let Some(p_cons) = self.constraints.get(parent).cloned() else {
                break;
            };
            if p_cons.width != Size::Fit && p_cons.height != Size::Fit {
                break;
            }

            let p_off_w = p_cons.padding.left
                + p_cons.border.left
                + p_cons.padding.right
                + p_cons.border.right;
            let p_off_h = p_cons.padding.top
                + p_cons.border.top
                + p_cons.padding.bottom
                + p_cons.border.bottom;

            let is_p_row = Self::is_row(p_cons.flex_direction);
            let is_p_wrap = matches!(p_cons.flex_wrap, FlexWrap::Wrap | FlexWrap::WrapReverse);

            let mut changed = false;

            if !is_p_wrap {
                let mut sum_main = 0.0;
                let mut max_cross = 0.0f32;
                let mut child_count = 0;

                let mut ch = self.first_child(parent);
                while let Some(c) = ch {
                    let c_cons = self.constraints.get(c);
                    let is_abs = c_cons.is_some_and(|c| {
                        matches!(c.positioning, PositionStrategy::Absolute { .. })
                    });
                    if !is_abs && let Some(c_comp) = self.computed.get(c) {
                        child_count += 1;
                        if is_p_row {
                            sum_main += c_comp.w;
                            if c_comp.h > max_cross {
                                max_cross = c_comp.h;
                            }
                        } else {
                            sum_main += c_comp.h;
                            if c_comp.w > max_cross {
                                max_cross = c_comp.w;
                            }
                        }
                    }
                    ch = self.next_sibling(c);
                }

                if child_count > 1 {
                    sum_main += p_cons.gap * (child_count - 1) as f32;
                }

                let needed_w = if is_p_row { sum_main } else { max_cross } + p_off_w;
                let needed_h = if !is_p_row { sum_main } else { max_cross } + p_off_h;

                if let Some(p_comp) = self.computed.get_mut(parent) {
                    if p_cons.width == Size::Fit && (needed_w - p_comp.w).abs() > 1e-3 {
                        p_comp.w = needed_w;
                        Self::clamp_min_max(p_comp, &p_cons);
                        changed = true;
                    }
                    if p_cons.height == Size::Fit && (needed_h - p_comp.h).abs() > 1e-3 {
                        p_comp.h = needed_h;
                        Self::clamp_min_max(p_comp, &p_cons);
                        changed = true;
                    }
                }
            } else {
                let p_comp_w = self.computed.get(parent).map_or(0.0, |c| c.w);
                let p_inner_w = (p_comp_w - p_off_w).max(0.0);
                if is_p_row && p_cons.height == Size::Fit && p_inner_w > 0.0 {
                    let mut line_w = 0.0;
                    let mut line_max_h = 0.0f32;
                    let mut total_wrap_h = 0.0;
                    let mut line_items = 0;
                    let mut lines = 0;

                    let mut ch = self.first_child(parent);
                    while let Some(c) = ch {
                        let is_abs = self.constraints.get(c).is_some_and(|c| {
                            matches!(c.positioning, PositionStrategy::Absolute { .. })
                        });
                        if !is_abs && let Some(c_comp) = self.computed.get(c) {
                            let needed = c_comp.w + if line_items > 0 { p_cons.gap } else { 0.0 };
                            if line_items > 0 && line_w + needed > p_inner_w {
                                total_wrap_h += line_max_h;
                                lines += 1;
                                line_w = c_comp.w;
                                line_max_h = c_comp.h;
                                line_items = 1;
                            } else {
                                line_w += needed;
                                if c_comp.h > line_max_h {
                                    line_max_h = c_comp.h;
                                }
                                line_items += 1;
                            }
                        }
                        ch = self.next_sibling(c);
                    }

                    if line_items > 0 {
                        total_wrap_h += line_max_h;
                        lines += 1;
                    }
                    if lines > 1 {
                        total_wrap_h += p_cons.gap * (lines - 1) as f32;
                    }

                    let needed_h = total_wrap_h + p_off_h;
                    if let Some(p_comp) = self.computed.get_mut(parent)
                        && (needed_h - p_comp.h).abs() > 1e-3
                    {
                        p_comp.h = needed_h;
                        Self::clamp_min_max(p_comp, &p_cons);
                        changed = true;
                    }
                }
            }

            if !changed {
                break;
            }
            curr = self.parent(parent);
        }
    }

    pub fn compute_layout<F>(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
        mut measure_text: F,
    ) where
        F: FnMut(NodeId, &str, Option<&dyn std::any::Any>, f32, f32) -> TextMetrics,
    {
        let Some(root) = self.root else {
            return;
        };

        if self.dirties.is_empty() {
            return;
        }

        self.ensure_layout_order();
        if self.layout_order.is_empty() {
            return;
        }

        // PASS 1: Dirty Propagation
        let mut dirty_idx = 0;
        while dirty_idx < self.dirties.len() {
            let dirty_node = self.dirties.dense()[dirty_idx];
            dirty_idx += 1;

            if !self.constraints.has(dirty_node) {
                continue;
            }
            let Some(hrc) = self.hierarchies.get(dirty_node).copied() else {
                continue;
            };

            // A) Pull up
            let mut curr_parent = hrc.parent;
            while let Some(parent) = curr_parent {
                if self.dirties.has(parent) {
                    break;
                }
                let is_fit = self
                    .constraints
                    .get(parent)
                    .map(|c| c.width == Size::Fit || c.height == Size::Fit);
                if let Some(fit) = is_fit {
                    self.set_dirty(parent);
                    if fit {
                        curr_parent = self.parent(parent);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // B) Push down
            let mut curr_child = hrc.first_child;
            while let Some(child) = curr_child {
                if !self.dirties.has(child) {
                    self.set_dirty(child);
                }
                curr_child = self.next_sibling(child);
            }
        }

        let viewport_bounds = Computed {
            x: 0.0,
            y: 0.0,
            w: viewport_width,
            h: viewport_height,
            content_w: viewport_width,
            content_h: viewport_height,
        };

        // PASS 2: Available Space (Top-Down)
        let order_len = self.layout_order.len();
        for i in 0..order_len {
            let node = self.layout_order[i];
            if !self.dirties.has(node) {
                continue;
            }

            let parent_node = self.parent(node);
            let mut parent_bounds = viewport_bounds;

            if let Some(parent) = parent_node
                && let Some(p_comp) = self.computed.get(parent)
            {
                let p_cons = self.constraints.get(parent);
                let off_l = p_cons.map_or(0.0, |c| c.padding.left + c.border.left);
                let off_t = p_cons.map_or(0.0, |c| c.padding.top + c.border.top);
                let off_r = p_cons.map_or(0.0, |c| c.padding.right + c.border.right);
                let off_b = p_cons.map_or(0.0, |c| c.padding.bottom + c.border.bottom);

                parent_bounds.x = p_comp.x + off_l;
                parent_bounds.y = p_comp.y + off_t;
                parent_bounds.w = (p_comp.w - (off_l + off_r)).max(0.0);
                parent_bounds.h = (p_comp.h - (off_t + off_b)).max(0.0);
            }

            let is_root = node == root;
            if let Some(cons) = self.constraints.get(node).cloned()
                && let Some(comp) = self.computed.get_mut(node)
            {
                let parent_cons = parent_node.and_then(|p| self.constraints.get(p));
                let parent_is_column = parent_cons.is_none_or(|c| !Self::is_row(c.flex_direction));
                let parent_is_row = parent_cons.is_some_and(|c| Self::is_row(c.flex_direction));
                let is_abs = matches!(cons.positioning, PositionStrategy::Absolute { .. });

                // WIDTH
                comp.w = match cons.width {
                    Size::Fixed(px) => px as f32,
                    Size::Percent(p) => parent_bounds.w * p,
                    Size::Fill if is_root => parent_bounds.w,
                    Size::Fill if (parent_is_column || is_abs) && parent_bounds.w > 0.0 => {
                        parent_bounds.w
                    }
                    _ => 0.0,
                };

                // HEIGHT
                comp.h = match cons.height {
                    Size::Fixed(px) => px as f32,
                    Size::Percent(p) => parent_bounds.h * p,
                    Size::Fill if is_root => parent_bounds.h,
                    Size::Fill if (parent_is_row || is_abs) && parent_bounds.h > 0.0 => {
                        parent_bounds.h
                    }
                    _ => 0.0,
                };

                Self::clamp_min_max(comp, &cons);
                Self::apply_aspect_ratio(comp, &cons);

                // ABSOLUTE POSITIONING
                if let PositionStrategy::Absolute {
                    top,
                    left,
                    bottom,
                    right,
                } = cons.positioning
                {
                    let has_left = left.is_finite();
                    let has_right = right.is_finite();
                    let has_top = top.is_finite();
                    let has_bottom = bottom.is_finite();

                    if has_left && has_right {
                        if matches!(cons.width, Size::Fit | Size::Fill) {
                            comp.w = parent_bounds.w - left - right;
                            comp.x = parent_bounds.x + left;
                        } else {
                            comp.x = parent_bounds.x + left;
                        }
                    } else if has_left {
                        comp.x = parent_bounds.x + left;
                    } else if has_right {
                        comp.x = parent_bounds.x + parent_bounds.w - right - comp.w;
                    }

                    if has_top && has_bottom {
                        if matches!(cons.height, Size::Fit | Size::Fill) {
                            comp.h = parent_bounds.h - top - bottom;
                            comp.y = parent_bounds.y + top;
                        } else {
                            comp.y = parent_bounds.y + top;
                        }
                    } else if has_top {
                        comp.y = parent_bounds.y + top;
                    } else if has_bottom {
                        comp.y = parent_bounds.y + parent_bounds.h - bottom - comp.h;
                    }
                }
            }
        }

        // PASS 3: Intrinsic Sizing (Bottom-Up)
        for i in (0..order_len).rev() {
            let node = self.layout_order[i];
            if !self.dirties.has(node) {
                continue;
            }

            let Some(cons) = self.constraints.get(node).cloned() else {
                continue;
            };

            let fit_w = cons.width == Size::Fit;
            let fit_h = cons.height == Size::Fit;

            if fit_w || fit_h {
                let comp_w = self.computed.get(node).map_or(0.0, |c| c.w);
                let comp_h = self.computed.get(node).map_or(0.0, |c| c.h);

                let intrinsic_w;
                let intrinsic_h;

                if self.texts.has(node) {
                    let off_w = cons.padding.left
                        + cons.border.left
                        + cons.padding.right
                        + cons.border.right;
                    let off_h = cons.padding.top
                        + cons.border.top
                        + cons.padding.bottom
                        + cons.border.bottom;
                    let avail_w = if fit_w {
                        f32::INFINITY
                    } else if comp_w > off_w {
                        comp_w - off_w
                    } else {
                        comp_w
                    };
                    let avail_h = if fit_h {
                        f32::INFINITY
                    } else if comp_h > off_h {
                        comp_h - off_h
                    } else {
                        comp_h
                    };

                    let text_metrics = {
                        let text_node = self.texts.get_mut(node).unwrap();
                        if let Some(ref cache) = text_node.cache {
                            if cache.avail_w == avail_w && cache.avail_h == avail_h {
                                cache.metrics
                            } else {
                                let m = measure_text(
                                    node,
                                    &text_node.content,
                                    text_node.userdata.as_deref(),
                                    avail_w,
                                    avail_h,
                                );
                                text_node.cache = Some(CachedTextMeasurement {
                                    avail_w,
                                    avail_h,
                                    metrics: m,
                                });
                                m
                            }
                        } else {
                            let m = measure_text(
                                node,
                                &text_node.content,
                                text_node.userdata.as_deref(),
                                avail_w,
                                avail_h,
                            );
                            text_node.cache = Some(CachedTextMeasurement {
                                avail_w,
                                avail_h,
                                metrics: m,
                            });
                            m
                        }
                    };

                    intrinsic_w = text_metrics.width;
                    intrinsic_h = text_metrics.height;
                } else {
                    let is_row_dir = Self::is_row(cons.flex_direction);
                    let is_wrapping =
                        matches!(cons.flex_wrap, FlexWrap::Wrap | FlexWrap::WrapReverse);

                    if !is_wrapping {
                        let mut sum_main = 0.0;
                        let mut max_cross = 0.0f32;
                        let mut child_count = 0;

                        let mut curr = self.first_child(node);
                        while let Some(child) = curr {
                            let c_cons = self.constraints.get(child);
                            let is_abs = c_cons.is_some_and(|c| {
                                matches!(c.positioning, PositionStrategy::Absolute { .. })
                            });
                            if !is_abs && let Some(c_comp) = self.computed.get(child) {
                                child_count += 1;
                                if is_row_dir {
                                    sum_main += c_comp.w;
                                    if c_comp.h > max_cross {
                                        max_cross = c_comp.h;
                                    }
                                } else {
                                    sum_main += c_comp.h;
                                    if c_comp.w > max_cross {
                                        max_cross = c_comp.w;
                                    }
                                }
                            }
                            curr = self.next_sibling(child);
                        }

                        if child_count > 1 {
                            sum_main += cons.gap * (child_count - 1) as f32;
                        }

                        intrinsic_w = if is_row_dir { sum_main } else { max_cross };
                        intrinsic_h = if !is_row_dir { sum_main } else { max_cross };
                    } else {
                        let off_w = cons.padding.left
                            + cons.border.left
                            + cons.padding.right
                            + cons.border.right;
                        let off_h = cons.padding.top
                            + cons.border.top
                            + cons.padding.bottom
                            + cons.border.bottom;
                        let mut max_line_main = if is_row_dir {
                            comp_w - off_w
                        } else {
                            comp_h - off_h
                        };
                        if max_line_main <= 0.0 {
                            max_line_main = f32::INFINITY;
                        }

                        let mut cur_line_main = 0.0;
                        let mut cur_line_cross = 0.0f32;
                        let mut cur_line_count = 0;
                        let mut total_cross = 0.0;
                        let mut max_main_used = 0.0f32;
                        let mut line_count = 0;

                        let mut curr = self.first_child(node);
                        while let Some(child) = curr {
                            let c_cons = self.constraints.get(child);
                            let is_abs = c_cons.is_some_and(|c| {
                                matches!(c.positioning, PositionStrategy::Absolute { .. })
                            });
                            if !is_abs && let Some(c_comp) = self.computed.get(child) {
                                let child_main = if is_row_dir { c_comp.w } else { c_comp.h };
                                let child_cross = if is_row_dir { c_comp.h } else { c_comp.w };
                                let needed =
                                    child_main + if cur_line_count > 0 { cons.gap } else { 0.0 };

                                if cur_line_count > 0 && cur_line_main + needed > max_line_main {
                                    total_cross += cur_line_cross;
                                    line_count += 1;
                                    if cur_line_main > max_main_used {
                                        max_main_used = cur_line_main;
                                    }
                                    cur_line_main = child_main;
                                    cur_line_cross = child_cross;
                                    cur_line_count = 1;
                                } else {
                                    cur_line_main += needed;
                                    if child_cross > cur_line_cross {
                                        cur_line_cross = child_cross;
                                    }
                                    cur_line_count += 1;
                                }
                            }
                            curr = self.next_sibling(child);
                        }

                        if cur_line_count > 0 {
                            total_cross += cur_line_cross;
                            line_count += 1;
                            if cur_line_main > max_main_used {
                                max_main_used = cur_line_main;
                            }
                        }

                        if line_count > 1 {
                            total_cross += cons.gap * (line_count - 1) as f32;
                        }

                        intrinsic_w = if is_row_dir {
                            max_main_used
                        } else {
                            total_cross
                        };
                        intrinsic_h = if !is_row_dir {
                            max_main_used
                        } else {
                            total_cross
                        };
                    }
                }

                let off_w =
                    cons.padding.left + cons.border.left + cons.padding.right + cons.border.right;
                let off_h =
                    cons.padding.top + cons.border.top + cons.padding.bottom + cons.border.bottom;

                if let Some(comp) = self.computed.get_mut(node) {
                    if fit_w {
                        comp.w = intrinsic_w + off_w;
                    }
                    if fit_h {
                        comp.h = intrinsic_h + off_h;
                    }
                    Self::clamp_min_max(comp, &cons);
                    Self::apply_aspect_ratio(comp, &cons);
                }
            }
        }

        // PASS 4: Flex Distribution (Top-Down)
        for i in 0..order_len {
            let node = self.layout_order[i];
            if !self.dirties.has(node) {
                continue;
            }

            let Some(cons) = self.constraints.get(node).cloned() else {
                continue;
            };
            let Some(comp) = self.computed.get(node).copied() else {
                continue;
            };

            let off_w =
                cons.padding.left + cons.border.left + cons.padding.right + cons.border.right;
            let off_h =
                cons.padding.top + cons.border.top + cons.padding.bottom + cons.border.bottom;
            let mut inner_w = (comp.w - off_w).max(0.0);
            let mut inner_h = (comp.h - off_h).max(0.0);

            let is_row_dir = Self::is_row(cons.flex_direction);
            let mut available_main = if is_row_dir { inner_w } else { inner_h };

            // A) Pre-resolve cross-axis dimensions & main percentages
            let mut curr = self.first_child(node);
            while let Some(child) = curr {
                let c_cons_opt = self.constraints.get(child).cloned();
                if let Some(c_cons) = c_cons_opt {
                    let is_abs = matches!(c_cons.positioning, PositionStrategy::Absolute { .. });
                    if !is_abs {
                        let effective_align = match c_cons.align_self {
                            AlignSelf::Start => AlignItems::Start,
                            AlignSelf::Center => AlignItems::Center,
                            AlignSelf::End => AlignItems::End,
                            AlignSelf::Stretch => AlignItems::Stretch,
                            AlignSelf::Auto => cons.align_items,
                        };

                        let mut modified = false;
                        if let Some(c_comp) = self.computed.get_mut(child) {
                            if !is_row_dir {
                                if c_cons.width == Size::Fill
                                    || effective_align == AlignItems::Stretch
                                {
                                    if !matches!(c_cons.width, Size::Fixed(_)) {
                                        c_comp.w = inner_w;
                                        modified = true;
                                    }
                                } else if let Size::Percent(p) = c_cons.width {
                                    c_comp.w = inner_w * p;
                                    modified = true;
                                }

                                if let Size::Percent(p) = c_cons.height {
                                    c_comp.h = inner_h * p;
                                    modified = true;
                                }
                            } else {
                                if c_cons.height == Size::Fill
                                    || effective_align == AlignItems::Stretch
                                {
                                    if !matches!(c_cons.height, Size::Fixed(_)) {
                                        c_comp.h = inner_h;
                                        modified = true;
                                    }
                                } else if let Size::Percent(p) = c_cons.height {
                                    c_comp.h = inner_h * p;
                                    modified = true;
                                }

                                if let Size::Percent(p) = c_cons.width {
                                    c_comp.w = inner_w * p;
                                    modified = true;
                                }
                            }

                            if modified || c_cons.aspect_ratio > 0.0 {
                                Self::clamp_min_max(c_comp, &c_cons);
                                Self::apply_aspect_ratio(c_comp, &c_cons);
                            }
                        }

                        // Re-measure wrapped text with resolved available width
                        if self.texts.has(child) {
                            let c_comp_copied =
                                self.computed.get(child).copied().unwrap_or_default();
                            let c_off_w = c_cons.padding.left
                                + c_cons.border.left
                                + c_cons.padding.right
                                + c_cons.border.right;
                            let c_off_h = c_cons.padding.top
                                + c_cons.border.top
                                + c_cons.padding.bottom
                                + c_cons.border.bottom;
                            let c_avail_w = if c_cons.width == Size::Fit {
                                f32::INFINITY
                            } else if c_comp_copied.w > c_off_w {
                                c_comp_copied.w - c_off_w
                            } else {
                                f32::INFINITY
                            };
                            let c_avail_h = if c_comp_copied.h > c_off_h {
                                c_comp_copied.h - c_off_h
                            } else {
                                f32::INFINITY
                            };

                            let text_metrics = {
                                let text_node = self.texts.get_mut(child).unwrap();
                                let m = measure_text(
                                    child,
                                    &text_node.content,
                                    text_node.userdata.as_deref(),
                                    c_avail_w,
                                    c_avail_h,
                                );
                                text_node.cache = Some(CachedTextMeasurement {
                                    avail_w: c_avail_w,
                                    avail_h: c_avail_h,
                                    metrics: m,
                                });
                                m
                            };

                            let needed_w = text_metrics.width + c_off_w;
                            let needed_h = text_metrics.height + c_off_h;

                            if let Some(c_comp) = self.computed.get_mut(child) {
                                if c_cons.width == Size::Fit {
                                    c_comp.w = needed_w;
                                }
                                if c_cons.height == Size::Fit || needed_h > c_comp.h {
                                    c_comp.h = needed_h;
                                }
                                Self::clamp_min_max(c_comp, &c_cons);
                            }
                        }
                    }
                }
                curr = self.next_sibling(child);
            }

            // Update Size::Fit dimensions if any child expanded during remeasurement
            if cons.height == Size::Fit
                && !matches!(cons.overflow, Overflow::Scroll | Overflow::Hidden)
            {
                let mut fit_h = 0.0;
                let mut in_flow_items = 0;
                let mut max_cross_h = 0.0f32;

                let mut curr_ch = self.first_child(node);
                while let Some(ch) = curr_ch {
                    let is_ch_abs = self.constraints.get(ch).is_some_and(|c| {
                        matches!(c.positioning, PositionStrategy::Absolute { .. })
                    });
                    if !is_ch_abs && let Some(ch_comp) = self.computed.get(ch) {
                        in_flow_items += 1;
                        if is_row_dir {
                            if ch_comp.h > max_cross_h {
                                max_cross_h = ch_comp.h;
                            }
                        } else {
                            fit_h += ch_comp.h;
                        }
                    }
                    curr_ch = self.next_sibling(ch);
                }

                if !is_row_dir && in_flow_items > 1 {
                    fit_h += cons.gap * (in_flow_items - 1) as f32;
                }
                let needed_parent_h = if is_row_dir { max_cross_h } else { fit_h } + off_h;

                if needed_parent_h > comp.h {
                    if let Some(comp_mut) = self.computed.get_mut(node) {
                        comp_mut.h = needed_parent_h;
                        Self::clamp_min_max(comp_mut, &cons);
                        inner_h = (comp_mut.h - off_h).max(0.0);
                        if !is_row_dir {
                            available_main = inner_h;
                        }
                    }
                    self.bubble_up_fit_size(node);
                }
            }

            if cons.width == Size::Fit
                && !matches!(cons.overflow, Overflow::Scroll | Overflow::Hidden)
            {
                let mut fit_w = 0.0;
                let mut in_flow_items = 0;
                let mut max_cross_w = 0.0f32;

                let mut curr_ch = self.first_child(node);
                while let Some(ch) = curr_ch {
                    let is_ch_abs = self.constraints.get(ch).is_some_and(|c| {
                        matches!(c.positioning, PositionStrategy::Absolute { .. })
                    });
                    if !is_ch_abs && let Some(ch_comp) = self.computed.get(ch) {
                        in_flow_items += 1;
                        if is_row_dir {
                            fit_w += ch_comp.w;
                        } else {
                            if ch_comp.w > max_cross_w {
                                max_cross_w = ch_comp.w;
                            }
                        }
                    }
                    curr_ch = self.next_sibling(ch);
                }

                if is_row_dir && in_flow_items > 1 {
                    fit_w += cons.gap * (in_flow_items - 1) as f32;
                }
                let needed_parent_w = if is_row_dir { fit_w } else { max_cross_w } + off_w;

                if needed_parent_w > comp.w {
                    if let Some(comp_mut) = self.computed.get_mut(node) {
                        comp_mut.w = needed_parent_w;
                        Self::clamp_min_max(comp_mut, &cons);
                        inner_w = (comp_mut.w - off_w).max(0.0);
                        if is_row_dir {
                            available_main = inner_w;
                        }
                    }
                    self.bubble_up_fit_size(node);
                }
            }

            // B) Collect in-flow flex items into scratch_flex_items
            self.scratch_flex_items.clear();
            let mut total_basis = 0.0;
            let mut in_flow_count = 0;

            let mut curr = self.first_child(node);
            while let Some(child) = curr {
                if let (Some(c_cons), Some(c_comp)) =
                    (self.constraints.get(child), self.computed.get(child))
                    && !matches!(c_cons.positioning, PositionStrategy::Absolute { .. })
                {
                    in_flow_count += 1;
                    let basis = Self::get_flex_basis(c_cons, c_comp, is_row_dir, available_main);
                    total_basis += basis;

                    let is_main_fill = if is_row_dir {
                        c_cons.width == Size::Fill
                    } else {
                        c_cons.height == Size::Fill
                    };
                    let grow = if c_cons.flex_grow > 0.0 {
                        c_cons.flex_grow
                    } else if is_main_fill {
                        1.0
                    } else {
                        0.0
                    };
                    let shrink = c_cons.flex_shrink;
                    let (min_size, max_size) = if is_row_dir {
                        (c_cons.min_width, c_cons.max_width)
                    } else {
                        (c_cons.min_height, c_cons.max_height)
                    };

                    self.scratch_flex_items.push(FlexItemScratch {
                        node: child,
                        basis,
                        min_size,
                        max_size,
                        flex_grow: grow,
                        flex_shrink: shrink,
                        target_size: basis,
                        frozen: false,
                    });
                }
                curr = self.next_sibling(child);
            }

            if in_flow_count > 1 {
                total_basis += cons.gap * (in_flow_count - 1) as f32;
            }

            let initial_free_space = available_main - total_basis;

            if initial_free_space > 0.0 {
                // C1) W3C Multi-Iteration Flex Grow (Freeze-and-Loop)
                for item in &mut self.scratch_flex_items {
                    if item.flex_grow <= 0.0 {
                        item.frozen = true;
                    }
                }

                loop {
                    let mut used_space = if in_flow_count > 1 {
                        cons.gap * (in_flow_count - 1) as f32
                    } else {
                        0.0
                    };
                    let mut unfrozen_grow_sum = 0.0;
                    let mut has_unfrozen = false;

                    for item in &self.scratch_flex_items {
                        if item.frozen {
                            used_space += item.target_size;
                        } else {
                            used_space += item.basis;
                            unfrozen_grow_sum += item.flex_grow;
                            has_unfrozen = true;
                        }
                    }

                    let remaining_free_space = available_main - used_space;

                    if !has_unfrozen || unfrozen_grow_sum <= 0.0 || remaining_free_space <= 0.0 {
                        break;
                    }

                    let mut violation_occurred = false;

                    for i in 0..self.scratch_flex_items.len() {
                        let item = &mut self.scratch_flex_items[i];
                        if item.frozen {
                            continue;
                        }

                        let share = (item.flex_grow / unfrozen_grow_sum) * remaining_free_space;
                        let tentative_size = item.basis + share;

                        if tentative_size > item.max_size {
                            item.target_size = item.max_size;
                            item.frozen = true;
                            violation_occurred = true;
                        } else if tentative_size < item.min_size {
                            item.target_size = item.min_size;
                            item.frozen = true;
                            violation_occurred = true;
                        } else {
                            item.target_size = tentative_size;
                        }
                    }

                    if !violation_occurred {
                        for item in &mut self.scratch_flex_items {
                            item.frozen = true;
                        }
                        break;
                    }
                }

                for item in &self.scratch_flex_items {
                    if item.flex_grow > 0.0
                        && let Some(c_comp) = self.computed.get_mut(item.node)
                    {
                        if is_row_dir {
                            c_comp.w = item.target_size;
                        } else {
                            c_comp.h = item.target_size;
                        }
                        if let Some(c_cons) = self.constraints.get(item.node) {
                            Self::clamp_min_max(c_comp, c_cons);
                            Self::apply_aspect_ratio(c_comp, c_cons);
                        }
                    }
                }
            } else if initial_free_space < 0.0
                && available_main > 0.0
                && !matches!(cons.overflow, Overflow::Scroll | Overflow::Auto)
            {
                // C2) W3C Multi-Iteration Flex Shrink (Freeze-and-Loop)
                for item in &mut self.scratch_flex_items {
                    if item.flex_shrink <= 0.0 || item.basis <= 0.0 {
                        item.frozen = true;
                    }
                }

                loop {
                    let mut used_space = if in_flow_count > 1 {
                        cons.gap * (in_flow_count - 1) as f32
                    } else {
                        0.0
                    };
                    let mut unfrozen_scaled_shrink_sum = 0.0;
                    let mut has_unfrozen = false;

                    for item in &self.scratch_flex_items {
                        if item.frozen {
                            used_space += item.target_size;
                        } else {
                            used_space += item.basis;
                            unfrozen_scaled_shrink_sum += item.flex_shrink * item.basis;
                            has_unfrozen = true;
                        }
                    }

                    let remaining_overflow = used_space - available_main;

                    if !has_unfrozen
                        || unfrozen_scaled_shrink_sum <= 0.0
                        || remaining_overflow <= 0.0
                    {
                        break;
                    }

                    let mut violation_occurred = false;

                    for i in 0..self.scratch_flex_items.len() {
                        let item = &mut self.scratch_flex_items[i];
                        if item.frozen {
                            continue;
                        }

                        let shrink_ratio =
                            (item.flex_shrink * item.basis) / unfrozen_scaled_shrink_sum;
                        let shrink_amount = shrink_ratio * remaining_overflow;
                        let tentative_size = (item.basis - shrink_amount).max(0.0);

                        if tentative_size < item.min_size {
                            item.target_size = item.min_size;
                            item.frozen = true;
                            violation_occurred = true;
                        } else if tentative_size > item.max_size {
                            item.target_size = item.max_size;
                            item.frozen = true;
                            violation_occurred = true;
                        } else {
                            item.target_size = tentative_size;
                        }
                    }

                    if !violation_occurred {
                        for item in &mut self.scratch_flex_items {
                            item.frozen = true;
                        }
                        break;
                    }
                }

                for item in &self.scratch_flex_items {
                    if item.flex_shrink > 0.0
                        && item.basis > 0.0
                        && let Some(c_comp) = self.computed.get_mut(item.node)
                    {
                        if is_row_dir {
                            c_comp.w = item.target_size;
                        } else {
                            c_comp.h = item.target_size;
                        }
                        if let Some(c_cons) = self.constraints.get(item.node) {
                            Self::clamp_min_max(c_comp, c_cons);
                            Self::apply_aspect_ratio(c_comp, c_cons);
                        }
                    }
                }
            }

            // C3) Post-Flex Cross-Axis Re-measurement (W3C Flexbox Step 9.4)
            let mut any_cross_changed = false;
            if is_row_dir {
                for item in &self.scratch_flex_items {
                    let Some(c_cons) = self.constraints.get(item.node).cloned() else {
                        continue;
                    };
                    if self.texts.has(item.node) {
                        let c_comp_copied =
                            self.computed.get(item.node).copied().unwrap_or_default();
                        let c_off_w = c_cons.padding.left
                            + c_cons.border.left
                            + c_cons.padding.right
                            + c_cons.border.right;
                        let c_off_h = c_cons.padding.top
                            + c_cons.border.top
                            + c_cons.padding.bottom
                            + c_cons.border.bottom;
                        let c_avail_w = if c_comp_copied.w > c_off_w {
                            c_comp_copied.w - c_off_w
                        } else {
                            f32::INFINITY
                        };
                        let c_avail_h = if c_comp_copied.h > c_off_h {
                            c_comp_copied.h - c_off_h
                        } else {
                            f32::INFINITY
                        };

                        let text_metrics = {
                            let text_node = self.texts.get_mut(item.node).unwrap();
                            let m = measure_text(
                                item.node,
                                &text_node.content,
                                text_node.userdata.as_deref(),
                                c_avail_w,
                                c_avail_h,
                            );
                            text_node.cache = Some(CachedTextMeasurement {
                                avail_w: c_avail_w,
                                avail_h: c_avail_h,
                                metrics: m,
                            });
                            m
                        };

                        let needed_h = text_metrics.height + c_off_h;
                        if let Some(c_comp) = self.computed.get_mut(item.node)
                            && (c_cons.height == Size::Fit || needed_h > c_comp.h)
                            && (needed_h - c_comp.h).abs() > 1e-3
                        {
                            c_comp.h = needed_h;
                            Self::clamp_min_max(c_comp, &c_cons);
                            any_cross_changed = true;
                        }
                    } else if c_cons.aspect_ratio > 0.0
                        && !matches!(c_cons.height, Size::Fixed(_) | Size::Percent(_))
                        && let Some(c_comp) = self.computed.get_mut(item.node)
                    {
                        let needed_h = c_comp.w / c_cons.aspect_ratio;
                        if (needed_h - c_comp.h).abs() > 1e-3 {
                            c_comp.h = needed_h;
                            Self::clamp_min_max(c_comp, &c_cons);
                            any_cross_changed = true;
                        }
                    }
                }

                if any_cross_changed
                    && cons.height == Size::Fit
                    && !matches!(cons.overflow, Overflow::Scroll | Overflow::Hidden)
                {
                    let mut max_cross_h = 0.0f32;
                    let mut curr_ch = self.first_child(node);
                    while let Some(ch) = curr_ch {
                        let is_ch_abs = self.constraints.get(ch).is_some_and(|c| {
                            matches!(c.positioning, PositionStrategy::Absolute { .. })
                        });
                        if !is_ch_abs
                            && let Some(ch_comp) = self.computed.get(ch)
                            && ch_comp.h > max_cross_h
                        {
                            max_cross_h = ch_comp.h;
                        }
                        curr_ch = self.next_sibling(ch);
                    }
                    let needed_parent_h = max_cross_h + off_h;
                    if (needed_parent_h - comp.h).abs() > 1e-3 {
                        if let Some(comp_mut) = self.computed.get_mut(node) {
                            comp_mut.h = needed_parent_h;
                            Self::clamp_min_max(comp_mut, &cons);
                        }
                        self.bubble_up_fit_size(node);
                    }
                }
            } else {
                for item in &self.scratch_flex_items {
                    let Some(c_cons) = self.constraints.get(item.node).cloned() else {
                        continue;
                    };
                    if c_cons.aspect_ratio > 0.0
                        && !matches!(c_cons.width, Size::Fixed(_) | Size::Percent(_))
                        && let Some(c_comp) = self.computed.get_mut(item.node)
                    {
                        let needed_w = c_comp.h * c_cons.aspect_ratio;
                        if (needed_w - c_comp.w).abs() > 1e-3 {
                            c_comp.w = needed_w;
                            Self::clamp_min_max(c_comp, &c_cons);
                            any_cross_changed = true;
                        }
                    }
                }

                if any_cross_changed
                    && cons.width == Size::Fit
                    && !matches!(cons.overflow, Overflow::Scroll | Overflow::Hidden)
                {
                    let mut max_cross_w = 0.0f32;
                    let mut curr_ch = self.first_child(node);
                    while let Some(ch) = curr_ch {
                        let is_ch_abs = self.constraints.get(ch).is_some_and(|c| {
                            matches!(c.positioning, PositionStrategy::Absolute { .. })
                        });
                        if !is_ch_abs
                            && let Some(ch_comp) = self.computed.get(ch)
                            && ch_comp.w > max_cross_w
                        {
                            max_cross_w = ch_comp.w;
                        }
                        curr_ch = self.next_sibling(ch);
                    }
                    let needed_parent_w = max_cross_w + off_w;
                    if (needed_parent_w - comp.w).abs() > 1e-3 {
                        if let Some(comp_mut) = self.computed.get_mut(node) {
                            comp_mut.w = needed_parent_w;
                            Self::clamp_min_max(comp_mut, &cons);
                        }
                        self.bubble_up_fit_size(node);
                    }
                }
            }

            // D) Absolute children percent and fill
            let mut curr = self.first_child(node);
            while let Some(child) = curr {
                let c_cons_opt = self.constraints.get(child).cloned();
                if let Some(c_cons) = c_cons_opt
                    && matches!(c_cons.positioning, PositionStrategy::Absolute { .. })
                    && let Some(c_comp) = self.computed.get_mut(child)
                {
                    match c_cons.width {
                        Size::Percent(p) => c_comp.w = inner_w * p,
                        Size::Fill => c_comp.w = inner_w,
                        _ => {}
                    }
                    match c_cons.height {
                        Size::Percent(p) => c_comp.h = inner_h * p,
                        Size::Fill => c_comp.h = inner_h,
                        _ => {}
                    }
                    Self::clamp_min_max(c_comp, &c_cons);
                    Self::apply_aspect_ratio(c_comp, &c_cons);
                }
                curr = self.next_sibling(child);
            }

            // E) Recompute wrapping container cross dimension
            if matches!(cons.flex_wrap, FlexWrap::Wrap | FlexWrap::WrapReverse)
                && is_row_dir
                && cons.height == Size::Fit
                && inner_w > 0.0
            {
                let mut line_w = 0.0;
                let mut line_max_h = 0.0f32;
                let mut total_wrap_h = 0.0;
                let mut line_items = 0;
                let mut lines = 0;

                let mut curr = self.first_child(node);
                while let Some(c) = curr {
                    let is_abs = self.constraints.get(c).is_some_and(|c| {
                        matches!(c.positioning, PositionStrategy::Absolute { .. })
                    });
                    if !is_abs && let Some(c_comp) = self.computed.get(c) {
                        let needed = c_comp.w + if line_items > 0 { cons.gap } else { 0.0 };
                        if line_items > 0 && line_w + needed > inner_w {
                            total_wrap_h += line_max_h;
                            lines += 1;
                            line_w = c_comp.w;
                            line_max_h = c_comp.h;
                            line_items = 1;
                        } else {
                            line_w += needed;
                            if c_comp.h > line_max_h {
                                line_max_h = c_comp.h;
                            }
                            line_items += 1;
                        }
                    }
                    curr = self.next_sibling(c);
                }

                if line_items > 0 {
                    total_wrap_h += line_max_h;
                    lines += 1;
                }
                if lines > 1 {
                    total_wrap_h += cons.gap * (lines - 1) as f32;
                }

                if let Some(comp_mut) = self.computed.get_mut(node) {
                    let new_h = total_wrap_h + off_h;
                    if (new_h - comp_mut.h).abs() > 1e-3 {
                        comp_mut.h = new_h;
                        Self::clamp_min_max(comp_mut, &cons);
                        self.bubble_up_fit_size(node);
                    }
                }
            }
        }

        // PASS 5: Positional Alignment (Top-Down)
        if let Some(r_comp) = self.computed.get_mut(root) {
            r_comp.x = 0.0;
            r_comp.y = 0.0;
        }

        for i in 0..order_len {
            let node = self.layout_order[i];
            if !self.dirties.has(node) {
                continue;
            }

            let Some(cons) = self.constraints.get(node).cloned() else {
                continue;
            };
            let Some(comp) = self.computed.get(node).copied() else {
                continue;
            };

            let off_l = cons.padding.left + cons.border.left;
            let off_t = cons.padding.top + cons.border.top;
            let off_r = cons.padding.right + cons.border.right;
            let off_b = cons.padding.bottom + cons.border.bottom;

            let inner_w = (comp.w - (off_l + off_r)).max(0.0);
            let inner_h = (comp.h - (off_t + off_b)).max(0.0);

            let is_row_dir = Self::is_row(cons.flex_direction);
            let is_rev = Self::is_reverse(cons.flex_direction);

            let inner_main = if is_row_dir { inner_w } else { inner_h };
            let inner_cross = if is_row_dir { inner_h } else { inner_w };

            let base_x = comp.x
                - if cons.scroll.x < 0.0 {
                    0.0
                } else {
                    cons.scroll.x
                };
            let base_y = comp.y
                - if cons.scroll.y < 0.0 {
                    0.0
                } else {
                    cons.scroll.y
                };
            let cross_start = if is_row_dir {
                base_y + off_t
            } else {
                base_x + off_l
            };

            // Flex Wrap Layout
            if matches!(cons.flex_wrap, FlexWrap::Wrap | FlexWrap::WrapReverse) {
                self.scratch_children.clear();
                let mut curr = self.first_child(node);
                while let Some(c) = curr {
                    let c_cons_opt = self.constraints.get(c).cloned();
                    if let Some(c_cons) = c_cons_opt
                        && let PositionStrategy::Absolute {
                            top,
                            left,
                            bottom,
                            right,
                        } = c_cons.positioning
                    {
                        if let Some(c_comp) = self.computed.get_mut(c) {
                            let mut abs_x = base_x;
                            let mut abs_y = base_y;
                            if left.is_finite() {
                                abs_x = base_x + left;
                            } else if right.is_finite() {
                                abs_x = base_x + comp.w - c_comp.w - right;
                            }
                            if top.is_finite() {
                                abs_y = base_y + top;
                            } else if bottom.is_finite() {
                                abs_y = base_y + comp.h - c_comp.h - bottom;
                            }
                            c_comp.x = abs_x;
                            c_comp.y = abs_y;
                        }
                        curr = self.next_sibling(c);
                        continue;
                    }
                    self.scratch_children.push(c);
                    curr = self.next_sibling(c);
                }

                let child_cnt = self.scratch_children.len();
                let mut line_start = 0;
                let mut cur_cross_start = cross_start;

                while line_start < child_cnt {
                    let mut line_main_used = 0.0;
                    let mut line_cross_max = 0.0f32;
                    let mut line_child_count = 0;
                    let mut line_end = line_start;

                    while line_end < child_cnt {
                        let c = self.scratch_children[line_end];
                        if let Some(c_comp) = self.computed.get(c) {
                            let child_main = if is_row_dir { c_comp.w } else { c_comp.h };
                            let child_cross = if is_row_dir { c_comp.h } else { c_comp.w };
                            let needed =
                                child_main + if line_child_count > 0 { cons.gap } else { 0.0 };

                            if line_child_count > 0 && line_main_used + needed > inner_main {
                                break;
                            }

                            line_main_used += needed;
                            if child_cross > line_cross_max {
                                line_cross_max = child_cross;
                            }
                            line_child_count += 1;
                        }
                        line_end += 1;
                    }

                    let remaining_main = (inner_main - line_main_used).max(0.0);
                    let mut start_main_offset = 0.0;
                    let mut space_between = cons.gap;

                    match cons.justify_content {
                        JustifyContent::Center => start_main_offset = remaining_main / 2.0,
                        JustifyContent::End => start_main_offset = remaining_main,
                        JustifyContent::SpaceBetween => {
                            if line_child_count > 1 {
                                space_between =
                                    remaining_main / (line_child_count - 1) as f32 + cons.gap;
                            }
                        }
                        JustifyContent::SpaceAround => {
                            if line_child_count > 0 {
                                space_between = remaining_main / line_child_count as f32 + cons.gap;
                                start_main_offset = (space_between - cons.gap) / 2.0;
                            }
                        }
                        JustifyContent::SpaceEvenly => {
                            if line_child_count > 0 {
                                space_between =
                                    remaining_main / (line_child_count + 1) as f32 + cons.gap;
                                start_main_offset = space_between - cons.gap;
                            }
                        }
                        JustifyContent::Start => {}
                    }

                    let mut cursor_main = if is_row_dir {
                        if is_rev {
                            base_x + comp.w - off_r - start_main_offset
                        } else {
                            base_x + off_l + start_main_offset
                        }
                    } else {
                        if is_rev {
                            base_y + comp.h - off_b - start_main_offset
                        } else {
                            base_y + off_t + start_main_offset
                        }
                    };

                    for idx in line_start..line_end {
                        let c = self.scratch_children[idx];
                        let c_cons = self.constraints.get(c).cloned().unwrap();
                        if let Some(c_comp) = self.computed.get_mut(c) {
                            let child_cross = if is_row_dir { c_comp.h } else { c_comp.w };
                            let mut cross_offset = 0.0;
                            let effective_align = match c_cons.align_self {
                                AlignSelf::Start => AlignItems::Start,
                                AlignSelf::Center => AlignItems::Center,
                                AlignSelf::End => AlignItems::End,
                                AlignSelf::Stretch => AlignItems::Stretch,
                                AlignSelf::Auto => cons.align_items,
                            };

                            match effective_align {
                                AlignItems::Center => {
                                    cross_offset = (line_cross_max - child_cross) / 2.0
                                }
                                AlignItems::End => cross_offset = line_cross_max - child_cross,
                                AlignItems::Stretch => {
                                    if is_row_dir && !matches!(c_cons.height, Size::Fixed(_)) {
                                        c_comp.h = line_cross_max;
                                    } else if !is_row_dir && !matches!(c_cons.width, Size::Fixed(_))
                                    {
                                        c_comp.w = line_cross_max;
                                    }
                                }
                                AlignItems::Start => {}
                            }

                            if is_row_dir {
                                let child_x = if is_rev {
                                    cursor_main - c_comp.w
                                } else {
                                    cursor_main
                                };
                                c_comp.x = child_x;
                                c_comp.y = cur_cross_start + cross_offset;
                                if is_rev {
                                    cursor_main -= c_comp.w + space_between;
                                } else {
                                    cursor_main += c_comp.w + space_between;
                                }
                            } else {
                                let child_y = if is_rev {
                                    cursor_main - c_comp.h
                                } else {
                                    cursor_main
                                };
                                c_comp.x = cur_cross_start + cross_offset;
                                c_comp.y = child_y;
                                if is_rev {
                                    cursor_main -= c_comp.h + space_between;
                                } else {
                                    cursor_main += c_comp.h + space_between;
                                }
                            }
                        }
                    }

                    cur_cross_start += line_cross_max + cons.gap;
                    line_start = line_end;
                }
                continue;
            }

            // Standard Flex Layout (No-Wrap)
            let mut total_main = 0.0;
            let mut child_count = 0;

            let mut curr = self.first_child(node);
            while let Some(child) = curr {
                let is_abs = self
                    .constraints
                    .get(child)
                    .is_some_and(|c| matches!(c.positioning, PositionStrategy::Absolute { .. }));
                if !is_abs && let Some(c_comp) = self.computed.get(child) {
                    total_main += if is_row_dir { c_comp.w } else { c_comp.h };
                    child_count += 1;
                }
                curr = self.next_sibling(child);
            }

            if child_count > 1 {
                total_main += cons.gap * (child_count - 1) as f32;
            }

            let remaining_main = (inner_main - total_main).max(0.0);
            let mut start_main_offset = 0.0;
            let mut space_between = cons.gap;

            match cons.justify_content {
                JustifyContent::Center => start_main_offset = remaining_main / 2.0,
                JustifyContent::End => start_main_offset = remaining_main,
                JustifyContent::SpaceBetween => {
                    if child_count > 1 {
                        space_between = remaining_main / (child_count - 1) as f32 + cons.gap;
                    }
                }
                JustifyContent::SpaceAround => {
                    if child_count > 0 {
                        space_between = remaining_main / child_count as f32 + cons.gap;
                        start_main_offset = (space_between - cons.gap) / 2.0;
                    }
                }
                JustifyContent::SpaceEvenly => {
                    if child_count > 0 {
                        space_between = remaining_main / (child_count + 1) as f32 + cons.gap;
                        start_main_offset = space_between - cons.gap;
                    }
                }
                JustifyContent::Start => {}
            }

            let mut cursor_main = if is_row_dir {
                if is_rev {
                    base_x + comp.w - off_r - start_main_offset
                } else {
                    base_x + off_l + start_main_offset
                }
            } else {
                if is_rev {
                    base_y + comp.h - off_b - start_main_offset
                } else {
                    base_y + off_t + start_main_offset
                }
            };

            // Layout children forward or reverse
            let mut children_order = self.children(node);
            if is_rev {
                children_order.reverse();
            }

            for child in children_order {
                let c_cons = self.constraints.get(child).cloned().unwrap();
                if let Some(c_comp) = self.computed.get_mut(child) {
                    if let PositionStrategy::Absolute {
                        top,
                        left,
                        bottom,
                        right,
                    } = c_cons.positioning
                    {
                        let mut abs_x = base_x;
                        let mut abs_y = base_y;
                        if left.is_finite() {
                            abs_x = base_x + left;
                        } else if right.is_finite() {
                            abs_x = base_x + comp.w - c_comp.w - right;
                        }
                        if top.is_finite() {
                            abs_y = base_y + top;
                        } else if bottom.is_finite() {
                            abs_y = base_y + comp.h - c_comp.h - bottom;
                        }
                        c_comp.x = abs_x;
                        c_comp.y = abs_y;
                    } else {
                        let child_cross = if is_row_dir { c_comp.h } else { c_comp.w };
                        let mut cross_offset = 0.0;
                        let effective_align = match c_cons.align_self {
                            AlignSelf::Start => AlignItems::Start,
                            AlignSelf::Center => AlignItems::Center,
                            AlignSelf::End => AlignItems::End,
                            AlignSelf::Stretch => AlignItems::Stretch,
                            AlignSelf::Auto => cons.align_items,
                        };

                        match effective_align {
                            AlignItems::Center => cross_offset = (inner_cross - child_cross) / 2.0,
                            AlignItems::End => cross_offset = inner_cross - child_cross,
                            AlignItems::Stretch => {
                                if is_row_dir && !matches!(c_cons.height, Size::Fixed(_)) {
                                    c_comp.h = inner_cross;
                                } else if !is_row_dir && !matches!(c_cons.width, Size::Fixed(_)) {
                                    c_comp.w = inner_cross;
                                }
                            }
                            AlignItems::Start => {}
                        }

                        if is_row_dir {
                            let child_x = if is_rev {
                                cursor_main - c_comp.w
                            } else {
                                cursor_main
                            };
                            c_comp.x = child_x;
                            c_comp.y = cross_start + cross_offset;
                            if is_rev {
                                cursor_main -= c_comp.w + space_between;
                            } else {
                                cursor_main += c_comp.w + space_between;
                            }
                        } else {
                            let child_y = if is_rev {
                                cursor_main - c_comp.h
                            } else {
                                cursor_main
                            };
                            c_comp.x = cross_start + cross_offset;
                            c_comp.y = child_y;
                            if is_rev {
                                cursor_main -= c_comp.h + space_between;
                            } else {
                                cursor_main += c_comp.h + space_between;
                            }
                        }
                    }
                }
            }
        }

        // PASS 5.5: Content Bounds Calculation (Bottom-Up)
        for i in (0..order_len).rev() {
            let node = self.layout_order[i];
            if !self.dirties.has(node) {
                continue;
            }

            let Some(comp) = self.computed.get(node).copied() else {
                continue;
            };
            let cons_opt = self.constraints.get(node).cloned();

            let mut max_w = comp.w;
            let mut max_h = comp.h;

            let scroll_x = cons_opt
                .as_ref()
                .map_or(0.0, |c| if c.scroll.x < 0.0 { 0.0 } else { c.scroll.x });
            let scroll_y = cons_opt
                .as_ref()
                .map_or(0.0, |c| if c.scroll.y < 0.0 { 0.0 } else { c.scroll.y });

            if self.texts.has(node) {
                let off_w = cons_opt.as_ref().map_or(0.0, |c| {
                    c.padding.left + c.border.left + c.padding.right + c.border.right
                });
                let off_h = cons_opt.as_ref().map_or(0.0, |c| {
                    c.padding.top + c.border.top + c.padding.bottom + c.border.bottom
                });
                let avail_w = if comp.w > off_w {
                    comp.w - off_w
                } else {
                    f32::INFINITY
                };
                let avail_h = if comp.h > off_h {
                    comp.h - off_h
                } else {
                    f32::INFINITY
                };

                let text_metrics = {
                    let text_node = self.texts.get_mut(node).unwrap();
                    let m = measure_text(
                        node,
                        &text_node.content,
                        text_node.userdata.as_deref(),
                        avail_w,
                        avail_h,
                    );
                    text_node.cache = Some(CachedTextMeasurement {
                        avail_w,
                        avail_h,
                        metrics: m,
                    });
                    m
                };
                max_w = text_metrics.width + off_w;
                max_h = text_metrics.height + off_h;
            } else {
                let mut curr = self.first_child(node);
                while let Some(child) = curr {
                    if let Some(c_comp) = self.computed.get(child) {
                        let child_bottom = (c_comp.y - comp.y) + c_comp.h + scroll_y;
                        let child_right = (c_comp.x - comp.x) + c_comp.w + scroll_x;
                        if child_bottom > max_h {
                            max_h = child_bottom;
                        }
                        if child_right > max_w {
                            max_w = child_right;
                        }
                    }
                    curr = self.next_sibling(child);
                }
            }

            if let Some(ref cons) = cons_opt {
                max_h += cons.padding.bottom;
                max_w += cons.padding.right;
            }

            let content_w = max_w.max(comp.w);
            let content_h = max_h.max(comp.h);

            if let Some(comp_mut) = self.computed.get_mut(node) {
                comp_mut.content_w = content_w;
                comp_mut.content_h = content_h;
            }

            let mut dx = 0.0;
            let mut dy = 0.0;

            if let Some(cons_mut) = self.constraints.get_mut(node) {
                if cons_mut.scroll.y < 0.0 {
                    let inner_h =
                        (comp.h - cons_mut.padding.top - cons_mut.padding.bottom).max(0.0);
                    let max_y = (content_h - inner_h).max(0.0);
                    let pct = (-cons_mut.scroll.y - 0.0001).clamp(0.0, 1.0);
                    let new_scroll_y = pct * max_y;
                    cons_mut.scroll.y = new_scroll_y;
                    dy = -new_scroll_y;
                }
                if cons_mut.scroll.x < 0.0 {
                    let inner_w =
                        (comp.w - cons_mut.padding.left - cons_mut.padding.right).max(0.0);
                    let max_x = (content_w - inner_w).max(0.0);
                    let pct = (-cons_mut.scroll.x - 0.0001).clamp(0.0, 1.0);
                    let new_scroll_x = pct * max_x;
                    cons_mut.scroll.x = new_scroll_x;
                    dx = -new_scroll_x;
                }
            }

            if dx != 0.0 || dy != 0.0 {
                self.shift_subtree(node, dx, dy);
            }
        }

        // PASS 6: Clear dirties
        self.dirties.clear();
        self.render_list_dirty = true;
    }

    fn flatten_recursive(
        &self,
        node: NodeId,
        list: &mut Vec<(RenderCommand, usize)>,
        seq: &mut usize,
        current_clip: Rect,
        has_clip: bool,
        inherited_z: i32,
    ) {
        if !self.is_valid(node) {
            return;
        }

        let cons = self.constraints.get(node);
        let z = cons
            .and_then(|c| {
                if c.z_index != 0 {
                    Some(c.z_index)
                } else {
                    None
                }
            })
            .unwrap_or(inherited_z);
        let Some(comp) = self.computed.get(node).copied() else {
            return;
        };

        let mut new_clip = current_clip;
        let mut new_has_clip = has_clip;

        if let Some(c) = cons
            && matches!(c.overflow, Overflow::Hidden | Overflow::Scroll)
        {
            let cx = comp.x + c.border.left;
            let cy = comp.y + c.border.top;
            let cw = comp.w - c.border.left - c.border.right;
            let ch = comp.h - c.border.top - c.border.bottom;

            if has_clip {
                let x1 = current_clip.x.max(cx);
                let y1 = current_clip.y.max(cy);
                let x2 = (current_clip.x + current_clip.w).min(cx + cw);
                let y2 = (current_clip.y + current_clip.h).min(cy + ch);

                new_clip.x = x1;
                new_clip.y = y1;
                new_clip.w = (x2 - x1).max(0.0);
                new_clip.h = (y2 - y1).max(0.0);
            } else {
                new_clip = Rect {
                    x: cx,
                    y: cy,
                    w: cw,
                    h: ch,
                };
                new_has_clip = true;
            }
        }

        let mut visible = true;
        if has_clip
            && (comp.x >= current_clip.x + current_clip.w
                || comp.x + comp.w <= current_clip.x
                || comp.y >= current_clip.y + current_clip.h
                || comp.y + comp.h <= current_clip.y)
        {
            visible = false;
        }

        if visible {
            let item_seq = *seq;
            *seq += 1;
            list.push((
                RenderCommand {
                    node,
                    kind: RenderCommandKind::DrawQuad,
                    computed: comp,
                    clip: current_clip,
                    z_index: z,
                    has_clip,
                },
                item_seq,
            ));

            if self.texts.has(node) {
                let text_clip = if cons
                    .is_some_and(|c| matches!(c.overflow, Overflow::Hidden | Overflow::Scroll))
                {
                    new_clip
                } else {
                    current_clip
                };
                let text_has_clip = if cons
                    .is_some_and(|c| matches!(c.overflow, Overflow::Hidden | Overflow::Scroll))
                {
                    new_has_clip
                } else {
                    has_clip
                };
                let text_seq = *seq;
                *seq += 1;
                list.push((
                    RenderCommand {
                        node,
                        kind: RenderCommandKind::Text,
                        computed: comp,
                        clip: text_clip,
                        z_index: z,
                        has_clip: text_has_clip,
                    },
                    text_seq,
                ));
            }
        }

        if new_has_clip && (new_clip.w <= 0.0 || new_clip.h <= 0.0) {
            return;
        }

        let mut curr = self.first_child(node);
        while let Some(child) = curr {
            self.flatten_recursive(child, list, seq, new_clip, new_has_clip, z);
            curr = self.next_sibling(child);
        }

        // Scrollbars
        if let Some(c) = cons
            && matches!(c.overflow, Overflow::Scroll | Overflow::Auto)
            && c.scrollbar_visible
        {
            let inner_h = comp.h - c.padding.top - c.padding.bottom;
            let inner_w = comp.w - c.padding.left - c.padding.right;

            if comp.content_h > comp.h + 0.5 && inner_h > 0.0 {
                let sb_seq = *seq;
                *seq += 1;
                list.push((
                    RenderCommand {
                        node,
                        kind: RenderCommandKind::ScrollbarV,
                        computed: comp,
                        clip: new_clip,
                        z_index: z,
                        has_clip: new_has_clip,
                    },
                    sb_seq,
                ));
            }

            if comp.content_w > comp.w + 0.5 && inner_w > 0.0 {
                let sb_seq = *seq;
                *seq += 1;
                list.push((
                    RenderCommand {
                        node,
                        kind: RenderCommandKind::ScrollbarH,
                        computed: comp,
                        clip: new_clip,
                        z_index: z,
                        has_clip: new_has_clip,
                    },
                    sb_seq,
                ));
            }
        }
    }

    pub fn build_render_list(&mut self, viewport: Rect) {
        let Some(root) = self.root else {
            self.render_list.clear();
            return;
        };

        if !self.render_list_dirty {
            return;
        }

        self.render_list.clear();
        let mut temp_list = Vec::new();
        let mut seq = 0;

        self.flatten_recursive(root, &mut temp_list, &mut seq, viewport, true, 0);

        // Stable sort: primary z_index, secondary sequence
        temp_list.sort_by(|a, b| {
            if a.0.z_index != b.0.z_index {
                a.0.z_index.cmp(&b.0.z_index)
            } else {
                a.1.cmp(&b.1)
            }
        });

        self.render_list = temp_list.into_iter().map(|(cmd, _)| cmd).collect();
        self.render_list_dirty = false;
    }

    pub fn pick(&mut self, x: f32, y: f32) -> &[NodeId] {
        self.pick_list.clear();

        if self.root.is_none() || self.render_list.is_empty() {
            return &self.pick_list;
        }

        let mut last_checked = None;

        for cmd in self.render_list.iter().rev() {
            let node = cmd.node;
            if Some(node) == last_checked {
                continue;
            }

            let in_bounds = x >= cmd.computed.x
                && x <= cmd.computed.x + cmd.computed.w
                && y >= cmd.computed.y
                && y <= cmd.computed.y + cmd.computed.h;

            if !in_bounds {
                continue;
            }

            if cmd.has_clip
                && (x < cmd.clip.x
                    || x > cmd.clip.x + cmd.clip.w
                    || y < cmd.clip.y
                    || y > cmd.clip.y + cmd.clip.h)
            {
                continue;
            }

            last_checked = Some(node);
            self.pick_list.push(node);
        }

        &self.pick_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_measure(
        _node: NodeId,
        _text: &str,
        _userdata: Option<&dyn std::any::Any>,
        _avail_w: f32,
        _avail_h: f32,
    ) -> TextMetrics {
        TextMetrics::default()
    }

    #[test]
    fn test_w3c_flex_grow_freeze_and_loop_max_violation() {
        let mut engine = LayoutEngine::new();

        let root = engine.create_node();
        let mut root_cons = Constraints::default();
        root_cons.width = Size::Fixed(500);
        root_cons.height = Size::Fixed(100);
        root_cons.flex_direction = FlexDirection::Row;
        engine.set_constraints(root, root_cons);

        let child_a = engine.create_node();
        let mut cons_a = Constraints::default();
        cons_a.width = Size::Fixed(100);
        cons_a.height = Size::Fixed(100);
        cons_a.flex_grow = 1.0;
        cons_a.max_width = 150.0;
        engine.set_constraints(child_a, cons_a);

        let child_b = engine.create_node();
        let mut cons_b = Constraints::default();
        cons_b.width = Size::Fixed(100);
        cons_b.height = Size::Fixed(100);
        cons_b.flex_grow = 1.0;
        engine.set_constraints(child_b, cons_b);

        engine.append(root, child_a);
        engine.append(root, child_b);
        engine.root_attach(root);

        engine.compute_layout(500.0, 100.0, dummy_measure);

        let comp_a = engine.get_computed(child_a).unwrap();
        let comp_b = engine.get_computed(child_b).unwrap();

        // Total width: 500. Basis A=100, B=100. Free space=300.
        // Equal split would give +150 to both (A=250, B=250).
        // But A has max_width 150, so A freezes at 150 (takes 50).
        // Remaining free space 250 goes entirely to B (100 + 250 = 350).
        assert!((comp_a.w - 150.0).abs() < 1e-3, "comp_a.w was {}", comp_a.w);
        assert!((comp_b.w - 350.0).abs() < 1e-3, "comp_b.w was {}", comp_b.w);
    }

    #[test]
    fn test_w3c_flex_shrink_freeze_and_loop_min_violation() {
        let mut engine = LayoutEngine::new();

        let root = engine.create_node();
        let mut root_cons = Constraints::default();
        root_cons.width = Size::Fixed(300);
        root_cons.height = Size::Fixed(100);
        root_cons.flex_direction = FlexDirection::Row;
        engine.set_constraints(root, root_cons);

        let child_a = engine.create_node();
        let mut cons_a = Constraints::default();
        cons_a.width = Size::Fixed(200);
        cons_a.height = Size::Fixed(100);
        cons_a.flex_shrink = 1.0;
        cons_a.min_width = 180.0;
        engine.set_constraints(child_a, cons_a);

        let child_b = engine.create_node();
        let mut cons_b = Constraints::default();
        cons_b.width = Size::Fixed(200);
        cons_b.height = Size::Fixed(100);
        cons_b.flex_shrink = 1.0;
        engine.set_constraints(child_b, cons_b);

        engine.append(root, child_a);
        engine.append(root, child_b);
        engine.root_attach(root);

        engine.compute_layout(300.0, 100.0, dummy_measure);

        let comp_a = engine.get_computed(child_a).unwrap();
        let comp_b = engine.get_computed(child_b).unwrap();

        // Total basis: 400. Available: 300. Overflow to shrink: 100.
        // Equal shrink would shrink each by 50 (A=150, B=150).
        // But A has min_width 180, so A freezes at 180 (only shrinks by 20).
        // B absorbs remaining 80 overflow (200 - 80 = 120).
        // Total = 180 + 120 = 300.
        assert!((comp_a.w - 180.0).abs() < 1e-3, "comp_a.w was {}", comp_a.w);
        assert!((comp_b.w - 120.0).abs() < 1e-3, "comp_b.w was {}", comp_b.w);
    }

    #[test]
    fn test_wrapped_text_expands_fit_parent_container() {
        let mut engine = LayoutEngine::new();

        let root = engine.create_node();
        let mut root_cons = Constraints::default();
        root_cons.width = Size::Fixed(400);
        root_cons.height = Size::Fixed(600);
        root_cons.flex_direction = FlexDirection::Column;
        engine.set_constraints(root, root_cons);

        let card = engine.create_node();
        let mut card_cons = Constraints::default();
        card_cons.width = Size::Fixed(200);
        card_cons.height = Size::Fit;
        card_cons.padding = crate::Edges {
            top: 10.0,
            bottom: 10.0,
            left: 10.0,
            right: 10.0,
        };
        engine.set_constraints(card, card_cons);

        let text_child = engine.create_node();
        let mut text_cons = Constraints::default();
        text_cons.width = Size::Fill;
        text_cons.height = Size::Fit;
        engine.set_constraints(text_child, text_cons);
        engine.set_text(
            text_child,
            "Some long wrapping description text".to_string(),
            None,
        );

        let other_child = engine.create_node();
        let mut other_cons = Constraints::default();
        other_cons.width = Size::Fill;
        other_cons.height = Size::Fixed(50);
        engine.set_constraints(other_child, other_cons);

        engine.append(card, text_child);
        engine.append(card, other_child);
        engine.append(root, card);
        engine.root_attach(root);

        let wrapping_measure = |_node: NodeId,
                                _text: &str,
                                _userdata: Option<&dyn std::any::Any>,
                                avail_w: f32,
                                _avail_h: f32| {
            if avail_w < 190.0 && avail_w > 0.0 {
                TextMetrics {
                    width: 180.0,
                    height: 60.0,
                    baseline_offset: 15.0,
                }
            } else {
                TextMetrics {
                    width: 250.0,
                    height: 20.0,
                    baseline_offset: 15.0,
                }
            }
        };

        engine.compute_layout(400.0, 600.0, wrapping_measure);

        let card_comp = engine.get_computed(card).unwrap();
        let text_comp = engine.get_computed(text_child).unwrap();
        let other_comp = engine.get_computed(other_child).unwrap();

        assert_eq!(text_comp.h, 60.0);
        assert_eq!(card_comp.h, 130.0);
        assert_eq!(other_comp.y, 70.0);
        assert!(other_comp.y + other_comp.h <= card_comp.h);
    }

    #[test]
    fn test_scrollbar_visible_flag_controls_scrollbar_generation() {
        let mut engine = LayoutEngine::new();
        let root = engine.create_node();
        let mut root_cons = Constraints::default();
        root_cons.width = Size::Fixed(100);
        root_cons.height = Size::Fixed(100);
        root_cons.overflow = Overflow::Scroll;
        root_cons.scrollbar_visible = true;
        engine.set_constraints(root, root_cons);

        let child = engine.create_node();
        let mut child_cons = Constraints::default();
        child_cons.width = Size::Fixed(200);
        child_cons.height = Size::Fixed(200);
        engine.set_constraints(child, child_cons);

        engine.append(root, child);
        engine.root_attach(root);

        let no_measure = |_node: NodeId,
                          _text: &str,
                          _userdata: Option<&dyn std::any::Any>,
                          _w: f32,
                          _h: f32| TextMetrics {
            width: 0.0,
            height: 0.0,
            baseline_offset: 0.0,
        };

        engine.compute_layout(100.0, 100.0, no_measure);
        engine.build_render_list(Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        });

        let has_v = engine
            .render_list
            .iter()
            .any(|cmd| matches!(cmd.kind, RenderCommandKind::ScrollbarV));
        let has_h = engine
            .render_list
            .iter()
            .any(|cmd| matches!(cmd.kind, RenderCommandKind::ScrollbarH));
        assert!(
            has_v,
            "Vertical scrollbar should be present when scrollbar_visible is true"
        );
        assert!(
            has_h,
            "Horizontal scrollbar should be present when scrollbar_visible is true"
        );

        // Now disable scrollbar_visible and rebuild render list
        root_cons.scrollbar_visible = false;
        engine.set_constraints(root, root_cons);
        engine.compute_layout(100.0, 100.0, no_measure);
        engine.build_render_list(Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        });

        let has_v_disabled = engine
            .render_list
            .iter()
            .any(|cmd| matches!(cmd.kind, RenderCommandKind::ScrollbarV));
        let has_h_disabled = engine
            .render_list
            .iter()
            .any(|cmd| matches!(cmd.kind, RenderCommandKind::ScrollbarH));
        assert!(
            !has_v_disabled,
            "Vertical scrollbar should NOT be present when scrollbar_visible is false"
        );
        assert!(
            !has_h_disabled,
            "Horizontal scrollbar should NOT be present when scrollbar_visible is false"
        );
    }

    #[test]
    fn test_flex_row_wrapped_text_expands_fit_row_and_parent_column() {
        let mut engine = LayoutEngine::new();

        // Root container: 400 x 600 fixed
        let root = engine.create_node();
        let mut root_cons = Constraints::default();
        root_cons.width = Size::Fixed(400);
        root_cons.height = Size::Fixed(600);
        root_cons.flex_direction = FlexDirection::Column;
        engine.set_constraints(root, root_cons);

        // Outer column: width Fill (400), height Fit, padding 10 on all sides
        let column = engine.create_node();
        let mut col_cons = Constraints::default();
        col_cons.width = Size::Fill;
        col_cons.height = Size::Fit;
        col_cons.flex_direction = FlexDirection::Column;
        col_cons.padding = crate::Edges {
            top: 10.0,
            bottom: 10.0,
            left: 10.0,
            right: 10.0,
        };
        engine.set_constraints(column, col_cons);

        // Inner row: width Fill (380), height Fit, gap 4, align_items Center
        let row = engine.create_node();
        let mut row_cons = Constraints::default();
        row_cons.width = Size::Fill;
        row_cons.height = Size::Fit;
        row_cons.flex_direction = FlexDirection::Row;
        row_cons.gap = 4.0;
        row_cons.align_items = AlignItems::Center;
        engine.set_constraints(row, row_cons);

        // Svg icon: fixed 18 x 18
        let svg = engine.create_node();
        let mut svg_cons = Constraints::default();
        svg_cons.width = Size::Fixed(18);
        svg_cons.height = Size::Fixed(18);
        engine.set_constraints(svg, svg_cons);

        // Text child: flex_grow 1.0, width Fill, height Fit
        let text_child = engine.create_node();
        let mut text_cons = Constraints::default();
        text_cons.flex_grow = 1.0;
        text_cons.width = Size::Fill;
        text_cons.height = Size::Fit;
        engine.set_constraints(text_child, text_cons);
        engine.set_text(text_child, "Album name that wraps".to_string(), None);

        engine.append(row, svg);
        engine.append(row, text_child);
        engine.append(column, row);
        engine.append(root, column);
        engine.root_attach(root);

        // Available width inside row is 380 - 18 (svg) - 4 (gap) = 358.
        // When avail_w <= 360, text wraps into 3 lines (height 54.0).
        // Otherwise on single line, height is 18.0.
        let wrapping_measure = |_node: NodeId,
                                _text: &str,
                                _userdata: Option<&dyn std::any::Any>,
                                avail_w: f32,
                                _avail_h: f32| {
            if avail_w <= 360.0 && avail_w > 0.0 {
                TextMetrics {
                    width: 358.0,
                    height: 54.0,
                    baseline_offset: 14.0,
                }
            } else {
                TextMetrics {
                    width: 500.0,
                    height: 18.0,
                    baseline_offset: 14.0,
                }
            }
        };

        engine.compute_layout(400.0, 600.0, wrapping_measure);

        let text_comp = engine.get_computed(text_child).unwrap();
        let row_comp = engine.get_computed(row).unwrap();
        let col_comp = engine.get_computed(column).unwrap();
        let svg_comp = engine.get_computed(svg).unwrap();

        // text width is 380 - 18 - 4 = 358
        assert_eq!(text_comp.w, 358.0);
        // text height expanded to 54.0 because it wrapped
        assert_eq!(text_comp.h, 54.0);
        // row height expanded to fit the 54.0 text
        assert_eq!(row_comp.h, 54.0);
        // column height expanded to row height (54.0) + padding top (10.0) + padding bottom (10.0) = 74.0
        assert_eq!(col_comp.h, 74.0);
        // svg icon should be vertically centered: (54.0 - 18.0) / 2 = 18.0 offset within row
        assert_eq!(svg_comp.y - row_comp.y, 18.0);
    }

    #[test]
    fn test_flex_row_with_nested_column_wrapped_text() {
        let mut engine = LayoutEngine::new();

        let root = engine.create_node();
        let mut root_cons = Constraints::default();
        root_cons.width = Size::Fixed(300);
        root_cons.height = Size::Fixed(400);
        root_cons.flex_direction = FlexDirection::Column;
        engine.set_constraints(root, root_cons);

        let row = engine.create_node();
        let mut row_cons = Constraints::default();
        row_cons.width = Size::Fill;
        row_cons.height = Size::Fit;
        row_cons.flex_direction = FlexDirection::Row;
        engine.set_constraints(row, row_cons);

        let inner_col = engine.create_node();
        let mut inner_col_cons = Constraints::default();
        inner_col_cons.flex_grow = 1.0;
        inner_col_cons.width = Size::Fill;
        inner_col_cons.height = Size::Fit;
        inner_col_cons.flex_direction = FlexDirection::Column;
        engine.set_constraints(inner_col, inner_col_cons);

        let text_child = engine.create_node();
        let mut text_cons = Constraints::default();
        text_cons.width = Size::Fill;
        text_cons.height = Size::Fit;
        engine.set_constraints(text_child, text_cons);
        engine.set_text(text_child, "Nested text".to_string(), None);

        engine.append(inner_col, text_child);
        engine.append(row, inner_col);
        engine.append(root, row);
        engine.root_attach(root);

        let wrapping_measure = |_node: NodeId,
                                _text: &str,
                                _userdata: Option<&dyn std::any::Any>,
                                avail_w: f32,
                                _avail_h: f32| {
            if avail_w <= 300.0 && avail_w > 0.0 {
                TextMetrics {
                    width: 300.0,
                    height: 70.0,
                    baseline_offset: 14.0,
                }
            } else {
                TextMetrics {
                    width: 500.0,
                    height: 20.0,
                    baseline_offset: 14.0,
                }
            }
        };

        engine.compute_layout(300.0, 400.0, wrapping_measure);

        let text_comp = engine.get_computed(text_child).unwrap();
        let inner_col_comp = engine.get_computed(inner_col).unwrap();
        let row_comp = engine.get_computed(row).unwrap();

        assert_eq!(text_comp.h, 70.0);
        assert_eq!(inner_col_comp.h, 70.0);
        assert_eq!(row_comp.h, 70.0);
    }
}
