pub mod info;
pub mod tree;
pub mod view_ext;

#[cfg(feature = "accessibility")]
pub mod adapter;

#[cfg(test)]
mod tests;

pub use info::AccessibleInfo;
pub use tree::{build_tree_update, from_accesskit_id, to_accesskit_id};
pub use view_ext::{AccessibleView, AccessibleViewExt};

pub use accesskit::{
    Action, ActionData, ActionRequest, Node as AkNode, NodeId as AkNodeId, Role, Toggled, TreeId,
    TreeInfo, TreeUpdate,
};
