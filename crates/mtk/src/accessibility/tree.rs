use crate::{Context, Node};
use accesskit::{
    Node as AkNode, NodeId as AkNodeId, Rect as AkRect, Role, TreeId, TreeInfo, TreeUpdate,
};

/// Converts an MTK layout `Node` into an `accesskit::NodeId` by packing generation and index.
#[inline(always)]
pub fn to_accesskit_id(node: Node) -> AkNodeId {
    let val = ((node.0.generation as u64) << 32) | (node.0.index as u64);
    AkNodeId(val)
}

/// Unpacks an `accesskit::NodeId` back into an MTK layout `Node`.
#[inline(always)]
pub fn from_accesskit_id(id: AkNodeId) -> Node {
    let val = id.0;
    Node(crate::layout::NodeId {
        index: val as u32,
        generation: (val >> 32) as u32,
    })
}

/// Builds an `accesskit::TreeUpdate` from the current MTK `Context`.
pub fn build_tree_update(ctx: &mut Context, is_full: bool) -> TreeUpdate {
    let root = match ctx.root_node() {
        Some(r) => r,
        None => {
            return TreeUpdate {
                nodes: Vec::new(),
                tree: None,
                tree_id: TreeId::ROOT,
                focus: AkNodeId(0),
            };
        }
    };

    let root_id = to_accesskit_id(root);
    let focus = ctx.focused_node.map(to_accesskit_id).unwrap_or(root_id);

    let tree = if is_full {
        let mut t = TreeInfo::new(root_id);
        t.toolkit_name = Some("MTK".to_string());
        Some(t)
    } else {
        None
    };

    let mut nodes = Vec::new();
    let mut stack = vec![root];

    while let Some(curr) = stack.pop() {
        let curr_id = to_accesskit_id(curr);
        let children_list = curr.children(ctx);
        let child_ids: Vec<AkNodeId> = children_list.iter().copied().map(to_accesskit_id).collect();

        let mut ak_node = if let Some(info) = ctx.accessibility_nodes.get(&curr) {
            let mut n = AkNode::new(info.role);
            if let Some(label) = &info.label {
                n.set_label(label.as_str());
            }
            if let Some(val) = &info.value {
                n.set_value(val.as_str());
            }
            if let Some(desc) = &info.description {
                n.set_description(desc.as_str());
            }
            if info.disabled {
                n.set_disabled();
            }
            if info.hidden {
                n.set_hidden();
            }
            if let Some(toggled) = info.toggled {
                n.set_toggled(if toggled {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                });
            }
            if let Some(num) = info.numeric_value {
                n.set_numeric_value(num);
            }
            if let Some(min) = info.min_numeric_value {
                n.set_min_numeric_value(min);
            }
            if let Some(max) = info.max_numeric_value {
                n.set_max_numeric_value(max);
            }
            if let Some(step) = info.step {
                n.set_numeric_value_step(step);
            }
            for &action in &info.actions {
                n.add_action(action);
            }
            n
        } else if let Some(text) = curr.get_text(ctx) {
            let mut n = AkNode::new(Role::Label);
            n.set_label(text);
            n
        } else if curr == root {
            AkNode::new(Role::Window)
        } else {
            AkNode::new(Role::GenericContainer)
        };

        if let Some(computed) = curr.get_computed(ctx) {
            ak_node.set_bounds(AkRect {
                x0: computed.x as f64,
                y0: computed.y as f64,
                x1: (computed.x + computed.w) as f64,
                y1: (computed.y + computed.h) as f64,
            });
        }

        ak_node.set_children(child_ids);
        nodes.push((curr_id, ak_node));

        for child in children_list.into_iter().rev() {
            stack.push(child);
        }
    }

    ctx.dirty_accessibility.clear();

    TreeUpdate {
        nodes,
        tree,
        tree_id: TreeId::ROOT,
        focus,
    }
}
