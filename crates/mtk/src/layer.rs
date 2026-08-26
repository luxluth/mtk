use crate::Node;

/// Shared visual and interactive state for a core engine layer.
#[derive(Clone, Debug)]
pub struct LayerState {
    pub root_node: Option<Node>,
    pub visible: bool,
    pub opacity: f32,
    pub transform: [f32; 2],
    pub scale: f32,
    pub focused_node: Option<Node>,
    pub saved_focus: Option<Node>,
}

impl Default for LayerState {
    fn default() -> Self {
        Self {
            root_node: None,
            visible: true,
            opacity: 1.0,
            transform: [0.0, 0.0],
            scale: 1.0,
            focused_node: None,
            saved_focus: None,
        }
    }
}

impl LayerState {
    /// Creates a new hidden layer state.
    pub fn hidden() -> Self {
        Self {
            visible: false,
            opacity: 0.0,
            ..Default::default()
        }
    }
}

/// Framework-managed Super Layer (Base, Overlay, Modal).
#[derive(Clone, Debug, Default)]
pub struct InternalLayer {
    pub state: LayerState,
}

impl InternalLayer {
    pub fn new(visible: bool) -> Self {
        Self {
            state: LayerState {
                visible,
                opacity: if visible { 1.0 } else { 0.0 },
                ..Default::default()
            },
        }
    }
}

/// User-defined Intermediate Layer (e.g. Fullscreen Player, Side Drawer).
#[derive(Clone, Debug)]
pub struct UserLayer {
    pub id: u32,
    pub state: LayerState,
    pub blocking: bool,
}

impl UserLayer {
    pub fn new(id: u32, visible: bool, blocking: bool) -> Self {
        Self {
            id,
            state: LayerState {
                visible,
                opacity: if visible { 1.0 } else { 0.0 },
                ..Default::default()
            },
            blocking,
        }
    }
}

/// Identifies the currently active layer receiving input events and focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActiveLayerId {
    Base,
    Intermediate(u32),
    Overlay,
    Modal,
}
