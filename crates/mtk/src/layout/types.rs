use super::sparse_set::NodeId;
use crate::style::{Computed, Rect};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Hierarchy {
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline_offset: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CachedTextMeasurement {
    pub avail_w: f32,
    pub avail_h: f32,
    pub metrics: TextMetrics,
}

#[derive(Default)]
pub struct TextNode {
    pub content: String,
    pub userdata: Option<Box<dyn std::any::Any>>,
    pub cache: Option<CachedTextMeasurement>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderCommandKind {
    DrawQuad,
    Text,
    ScrollbarV,
    ScrollbarH,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderCommand {
    pub node: NodeId,
    pub kind: RenderCommandKind,
    pub computed: Computed,
    pub clip: Rect,
    pub z_index: i32,
    pub has_clip: bool,
}
