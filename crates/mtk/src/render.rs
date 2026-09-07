use crate::Node;
use crate::layout::RenderCommand as LayoutRenderCommand;
use crate::style::{Computed, Rect};

pub use crate::layout::RenderCommandKind;

#[derive(Clone)]
pub struct RenderCommand<'a> {
    pub(crate) cmd: &'a LayoutRenderCommand,
}

impl std::fmt::Debug for RenderCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderCommand")
            .field("kind", &self.kind())
            .field("computed", &self.computed())
            .field("clip", &self.clip())
            .field("z_index", &self.z_index())
            .finish()
    }
}

impl<'a> RenderCommand<'a> {
    pub fn node(&self) -> Node {
        Node(self.cmd.node)
    }

    pub fn kind(&self) -> RenderCommandKind {
        self.cmd.kind
    }

    pub fn computed(&self) -> Computed {
        self.cmd.computed
    }

    pub fn clip(&self) -> Rect {
        self.cmd.clip
    }

    pub fn z_index(&self) -> i32 {
        self.cmd.z_index
    }

    pub fn has_clip(&self) -> bool {
        self.cmd.has_clip
    }
}
