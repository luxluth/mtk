use crate::sparse_set::NodeId;
use crate::types::{Rect, RenderCommand};

const MAX_DEPTH: usize = 6;
const MAX_ITEMS_PER_NODE: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadItem {
    pub bounds: Rect,
    pub node: NodeId,
    pub render_order: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadNode {
    pub bounds: Rect,
    pub first_child: u32,
    pub item_start: u32,
    pub item_count: u32,
}

#[derive(Default, Debug)]
pub struct Quadtree {
    nodes: Vec<QuadNode>,
    items: Vec<QuadItem>,
    scratch_items: Vec<QuadItem>,
    query_scratch: Vec<QuadItem>,
}

#[inline]
fn classify(b: &Rect, mid_x: f32, mid_y: f32) -> usize {
    if b.x + b.w < mid_x && b.y + b.h < mid_y {
        1 // NW
    } else if b.x > mid_x && b.y + b.h < mid_y {
        2 // NE
    } else if b.x + b.w < mid_x && b.y > mid_y {
        3 // SW
    } else if b.x > mid_x && b.y > mid_y {
        4 // SE
    } else {
        0 // Cross / touches boundary
    }
}

impl Quadtree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.items.clear();
        self.scratch_items.clear();
        self.query_scratch.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn build(&mut self, viewport: Rect, render_list: &[RenderCommand]) {
        self.clear();

        if render_list.is_empty() {
            return;
        }

        let mut min_x = viewport.x;
        let mut min_y = viewport.y;
        let mut max_x = viewport.x + viewport.w;
        let mut max_y = viewport.y + viewport.h;

        for (order, cmd) in render_list.iter().enumerate() {
            let (bx, by, bw, bh) = if cmd.has_clip {
                let x1 = cmd.computed.x.max(cmd.clip.x);
                let y1 = cmd.computed.y.max(cmd.clip.y);
                let x2 = (cmd.computed.x + cmd.computed.w).min(cmd.clip.x + cmd.clip.w);
                let y2 = (cmd.computed.y + cmd.computed.h).min(cmd.clip.y + cmd.clip.h);
                if x2 < x1 || y2 < y1 {
                    continue;
                }
                (x1, y1, x2 - x1, y2 - y1)
            } else {
                if cmd.computed.w < 0.0 || cmd.computed.h < 0.0 {
                    continue;
                }
                (
                    cmd.computed.x,
                    cmd.computed.y,
                    cmd.computed.w,
                    cmd.computed.h,
                )
            };

            min_x = min_x.min(bx);
            min_y = min_y.min(by);
            max_x = max_x.max(bx + bw);
            max_y = max_y.max(by + bh);

            self.items.push(QuadItem {
                bounds: Rect {
                    x: bx,
                    y: by,
                    w: bw,
                    h: bh,
                },
                node: cmd.node,
                render_order: order as u32,
            });
        }

        if self.items.is_empty() {
            return;
        }

        let root_bounds = Rect {
            x: min_x,
            y: min_y,
            w: (max_x - min_x).max(1.0),
            h: (max_y - min_y).max(1.0),
        };

        self.scratch_items.resize(
            self.items.len(),
            QuadItem {
                bounds: Rect::default(),
                node: NodeId::INVALID,
                render_order: 0,
            },
        );

        self.nodes.push(QuadNode {
            bounds: root_bounds,
            first_child: u32::MAX,
            item_start: 0,
            item_count: 0,
        });

        let total_items = self.items.len();
        self.subdivide(0, 0, total_items, 0);
    }

    fn subdivide(&mut self, node_idx: usize, start: usize, len: usize, depth: usize) {
        let node_bounds = self.nodes[node_idx].bounds;
        let can_subdivide = len > MAX_ITEMS_PER_NODE
            && depth < MAX_DEPTH
            && node_bounds.w > 1.0
            && node_bounds.h > 1.0;

        if !can_subdivide {
            self.nodes[node_idx].item_start = start as u32;
            self.nodes[node_idx].item_count = len as u32;
            self.nodes[node_idx].first_child = u32::MAX;
            return;
        }

        let mid_x = node_bounds.x + node_bounds.w * 0.5;
        let mid_y = node_bounds.y + node_bounds.h * 0.5;

        let mut counts = [0usize; 5];
        for i in 0..len {
            let cat = classify(&self.items[start + i].bounds, mid_x, mid_y);
            counts[cat] += 1;
        }

        // If all items cross or boundary-touch the midpoint, keep as leaf
        if counts[0] == len {
            self.nodes[node_idx].item_start = start as u32;
            self.nodes[node_idx].item_count = len as u32;
            self.nodes[node_idx].first_child = u32::MAX;
            return;
        }

        let mut offsets = [0usize; 5];
        offsets[0] = start;
        offsets[1] = offsets[0] + counts[0];
        offsets[2] = offsets[1] + counts[1];
        offsets[3] = offsets[2] + counts[2];
        offsets[4] = offsets[3] + counts[3];

        let base_offsets = offsets;

        for i in 0..len {
            let item = self.items[start + i];
            let cat = classify(&item.bounds, mid_x, mid_y);
            self.scratch_items[offsets[cat]] = item;
            offsets[cat] += 1;
        }

        self.items[start..start + len].copy_from_slice(&self.scratch_items[start..start + len]);

        let first_child = self.nodes.len() as u32;
        self.nodes[node_idx].item_start = start as u32;
        self.nodes[node_idx].item_count = counts[0] as u32;
        self.nodes[node_idx].first_child = first_child;

        let half_w = node_bounds.w * 0.5;
        let half_h = node_bounds.h * 0.5;

        // NW (first_child + 0)
        self.nodes.push(QuadNode {
            bounds: Rect {
                x: node_bounds.x,
                y: node_bounds.y,
                w: half_w,
                h: half_h,
            },
            first_child: u32::MAX,
            item_start: base_offsets[1] as u32,
            item_count: counts[1] as u32,
        });

        // NE (first_child + 1)
        self.nodes.push(QuadNode {
            bounds: Rect {
                x: mid_x,
                y: node_bounds.y,
                w: half_w,
                h: half_h,
            },
            first_child: u32::MAX,
            item_start: base_offsets[2] as u32,
            item_count: counts[2] as u32,
        });

        // SW (first_child + 2)
        self.nodes.push(QuadNode {
            bounds: Rect {
                x: node_bounds.x,
                y: mid_y,
                w: half_w,
                h: half_h,
            },
            first_child: u32::MAX,
            item_start: base_offsets[3] as u32,
            item_count: counts[3] as u32,
        });

        // SE (first_child + 3)
        self.nodes.push(QuadNode {
            bounds: Rect {
                x: mid_x,
                y: mid_y,
                w: half_w,
                h: half_h,
            },
            first_child: u32::MAX,
            item_start: base_offsets[4] as u32,
            item_count: counts[4] as u32,
        });

        if counts[1] > 0 {
            self.subdivide(
                first_child as usize + 0,
                base_offsets[1],
                counts[1],
                depth + 1,
            );
        }
        if counts[2] > 0 {
            self.subdivide(
                first_child as usize + 1,
                base_offsets[2],
                counts[2],
                depth + 1,
            );
        }
        if counts[3] > 0 {
            self.subdivide(
                first_child as usize + 2,
                base_offsets[3],
                counts[3],
                depth + 1,
            );
        }
        if counts[4] > 0 {
            self.subdivide(
                first_child as usize + 3,
                base_offsets[4],
                counts[4],
                depth + 1,
            );
        }
    }

    pub fn query_point(&mut self, x: f32, y: f32, out: &mut Vec<NodeId>) {
        out.clear();
        self.query_scratch.clear();

        if self.nodes.is_empty() {
            return;
        }

        if !self.nodes[0].bounds.contains(x, y) {
            return;
        }

        let mut curr_idx = 0;

        loop {
            let node = self.nodes[curr_idx];

            let start = node.item_start as usize;
            let count = node.item_count as usize;
            for i in 0..count {
                let item = self.items[start + i];
                if item.bounds.contains(x, y) {
                    self.query_scratch.push(item);
                }
            }

            if node.first_child == u32::MAX {
                break;
            }

            let mid_x = node.bounds.x + node.bounds.w * 0.5;
            let mid_y = node.bounds.y + node.bounds.h * 0.5;

            let in_left = x < mid_x;
            let in_right = x > mid_x;
            let in_top = y < mid_y;
            let in_bottom = y > mid_y;

            let child_offset = if in_left && in_top {
                0 // NW
            } else if in_right && in_top {
                1 // NE
            } else if in_left && in_bottom {
                2 // SW
            } else if in_right && in_bottom {
                3 // SE
            } else {
                // Point is on boundary line; child items were strictly filtered
                break;
            };

            curr_idx = (node.first_child + child_offset) as usize;
        }

        if self.query_scratch.is_empty() {
            return;
        }

        // Descending order by render_order (topmost first)
        self.query_scratch
            .sort_unstable_by_key(|b| std::cmp::Reverse(b.render_order));

        let mut last_node = None;
        for item in &self.query_scratch {
            if Some(item.node) == last_node {
                continue;
            }
            last_node = Some(item.node);
            out.push(item.node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Computed, RenderCommandKind};

    fn make_cmd(
        node: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        clip: Option<Rect>,
        z_index: i32,
    ) -> RenderCommand {
        let (has_clip, clip_rect) = match clip {
            Some(c) => (true, c),
            None => (false, Rect::default()),
        };
        RenderCommand {
            node: NodeId::new(node, 0),
            kind: RenderCommandKind::DrawQuad,
            computed: Computed {
                x,
                y,
                w,
                h,
                ..Default::default()
            },
            clip: clip_rect,
            z_index,
            has_clip,
        }
    }

    fn linear_pick(render_list: &[RenderCommand], x: f32, y: f32) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut last_checked = None;

        for cmd in render_list.iter().rev() {
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
            out.push(node);
        }

        out
    }

    #[test]
    fn test_empty_quadtree() {
        let mut qt = Quadtree::new();
        qt.build(Rect::new(0.0, 0.0, 800.0, 600.0), &[]);
        assert!(qt.is_empty());

        let mut out = Vec::new();
        qt.query_point(100.0, 100.0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_single_item() {
        let mut qt = Quadtree::new();
        let cmds = vec![make_cmd(1, 50.0, 50.0, 100.0, 100.0, None, 0)];
        qt.build(Rect::new(0.0, 0.0, 800.0, 600.0), &cmds);

        let mut out = Vec::new();
        qt.query_point(100.0, 100.0, &mut out);
        assert_eq!(out, vec![NodeId::new(1, 0)]);

        qt.query_point(10.0, 10.0, &mut out);
        assert!(out.is_empty());

        // Test boundary (inclusive)
        qt.query_point(50.0, 50.0, &mut out);
        assert_eq!(out, vec![NodeId::new(1, 0)]);

        qt.query_point(150.0, 150.0, &mut out);
        assert_eq!(out, vec![NodeId::new(1, 0)]);

        qt.query_point(150.1, 150.0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_z_order_descending() {
        let mut qt = Quadtree::new();
        // Background node 1, child button node 2 on top
        let cmds = vec![
            make_cmd(1, 0.0, 0.0, 200.0, 200.0, None, 0),
            make_cmd(2, 50.0, 50.0, 50.0, 50.0, None, 1),
        ];
        qt.build(Rect::new(0.0, 0.0, 800.0, 600.0), &cmds);

        let mut out = Vec::new();
        qt.query_point(60.0, 60.0, &mut out);
        // Node 2 is on top of Node 1
        assert_eq!(out, vec![NodeId::new(2, 0), NodeId::new(1, 0)]);

        // Point outside node 2 but inside node 1
        qt.query_point(10.0, 10.0, &mut out);
        assert_eq!(out, vec![NodeId::new(1, 0)]);
    }

    #[test]
    fn test_clipping() {
        let mut qt = Quadtree::new();
        let clip = Some(Rect::new(20.0, 20.0, 40.0, 40.0));
        let cmds = vec![make_cmd(1, 0.0, 0.0, 100.0, 100.0, clip, 0)];
        qt.build(Rect::new(0.0, 0.0, 800.0, 600.0), &cmds);

        let mut out = Vec::new();
        // (30, 30) is inside both computed (0..100) and clip (20..60)
        qt.query_point(30.0, 30.0, &mut out);
        assert_eq!(out, vec![NodeId::new(1, 0)]);

        // (10, 10) is inside computed but outside clip
        qt.query_point(10.0, 10.0, &mut out);
        assert!(out.is_empty());

        // (70, 70) is inside computed but outside clip
        qt.query_point(70.0, 70.0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let mut qt = Quadtree::new();
        // Two consecutive render commands for node 1 (e.g. DrawQuad and Text)
        let cmds = vec![
            make_cmd(1, 0.0, 0.0, 100.0, 100.0, None, 0),
            make_cmd(1, 10.0, 10.0, 50.0, 20.0, None, 0),
        ];
        qt.build(Rect::new(0.0, 0.0, 800.0, 600.0), &cmds);

        let mut out = Vec::new();
        qt.query_point(20.0, 20.0, &mut out);
        assert_eq!(out, vec![NodeId::new(1, 0)]);
    }

    #[test]
    fn test_deep_subdivision_and_equivalence() {
        let mut cmds = Vec::new();
        let mut id = 1;

        // Generate 60 elements across a 800x600 grid to force tree subdivision (MAX_ITEMS_PER_NODE = 16)
        for row in 0..10 {
            for col in 0..6 {
                let x = col as f32 * 120.0 + 10.0;
                let y = row as f32 * 55.0 + 5.0;
                let w = 80.0;
                let h = 40.0;
                let clip = if id % 3 == 0 {
                    Some(Rect::new(x + 5.0, y + 5.0, w - 10.0, h - 10.0))
                } else {
                    None
                };
                cmds.push(make_cmd(id, x, y, w, h, clip, 0));
                id += 1;
            }
        }

        // Also add a few large background elements that cross the center
        cmds.push(make_cmd(id, 0.0, 0.0, 800.0, 600.0, None, -1));
        id += 1;
        cmds.push(make_cmd(id, 350.0, 250.0, 100.0, 100.0, None, 10));

        let mut qt = Quadtree::new();
        qt.build(Rect::new(0.0, 0.0, 800.0, 600.0), &cmds);

        assert!(qt.node_count() > 1, "Quadtree should have subdivided");

        // Verify equivalence between quadtree and linear scan across 100 probe points
        let mut qt_out = Vec::new();
        for px in (0..=800).step_by(80) {
            for py in (0..=600).step_by(60) {
                let x = px as f32 + 0.5;
                let y = py as f32 + 0.5;

                let expected = linear_pick(&cmds, x, y);
                qt.query_point(x, y, &mut qt_out);

                assert_eq!(
                    qt_out, expected,
                    "Mismatch at point ({}, {}): quadtree {:?} vs linear {:?}",
                    x, y, qt_out, expected
                );
            }
        }
    }
}
