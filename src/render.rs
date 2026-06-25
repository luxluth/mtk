use crate::Node;
use crate::style::{Computed, Rect};
use crate::sys;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenderCommandKind {
    DrawQuad,
    Text,
}

impl From<sys::muRenderCommandKind> for RenderCommandKind {
    fn from(kind: sys::muRenderCommandKind) -> Self {
        match kind {
            sys::muRenderCommandKind_MU_CMD_DRAWQUAD => RenderCommandKind::DrawQuad,
            sys::muRenderCommandKind_MU_CMD_TEXT => RenderCommandKind::Text,
            _ => RenderCommandKind::DrawQuad,
        }
    }
}

#[derive(Clone)]
pub struct RenderCommand<'a> {
    pub(crate) cmd: &'a sys::muRenderCommand,
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
        self.cmd.kind.into()
    }

    pub fn computed(&self) -> Computed {
        self.cmd.computed.into()
    }

    pub fn clip(&self) -> Rect {
        self.cmd.clip.into()
    }

    pub fn z_index(&self) -> i32 {
        self.cmd.z_index
    }

    pub fn has_clip(&self) -> bool {
        self.cmd.has_clip
    }
}
