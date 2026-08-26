//! MTK Interactive Layout Debugger & Source Code Inspector.
//!
//! Provides bidirectional event streaming, layout tree introspection,
//! W3C Flexbox box model analysis, and source-code location tracking.

use std::sync::mpsc::{Receiver, Sender, channel};

#[cfg(feature = "debugger")]
pub mod tui;

/// Source code definition location for a UI element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
    pub type_name: &'static str,
}

impl SourceLocation {
    /// Captures the caller's source code location.
    #[track_caller]
    pub fn here(type_name: &'static str) -> Self {
        let loc = std::panic::Location::caller();
        Self {
            file: loc.file(),
            line: loc.line(),
            column: loc.column(),
            type_name,
        }
    }

    /// Formatted human-readable string representation.
    pub fn display(&self) -> String {
        format!(
            "{}:{}:{} ({})",
            self.file, self.line, self.column, self.type_name
        )
    }

    /// Formatted path for editor navigation (e.g. `src/main.rs:42:10`).
    pub fn link(&self) -> String {
        format!("{}:{}:{}", self.file, self.line, self.column)
    }

    /// Generates an ANSI OSC 8 clickable terminal hyperlink if supported by the terminal.
    pub fn osc8_link(&self, label: &str) -> String {
        format!(
            "\x1b]8;;file://{}\x1b\\{}\x1b]8;;\x1b\\",
            self.link(),
            label
        )
    }
}

/// Geometric and flexbox metrics for a layout node.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NodeBoxMetrics {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub content_w: f32,
    pub content_h: f32,
    pub pad_top: f32,
    pub pad_bottom: f32,
    pub pad_left: f32,
    pub pad_right: f32,
    pub border_top: f32,
    pub border_bottom: f32,
    pub border_left: f32,
    pub border_right: f32,
    pub flex_direction: String,
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

/// Snapshot of a single node in the MTK layout tree.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeDebugInfo {
    pub id: u64,
    pub name: String,
    pub source: Option<SourceLocation>,
    pub metrics: NodeBoxMetrics,
    pub text: Option<String>,
    pub children: Vec<NodeDebugInfo>,
}

/// Complete hierarchical snapshot of the UI layout tree.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSnapshot {
    pub root: Option<NodeDebugInfo>,
    pub total_nodes: usize,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub hovered_node: Option<u64>,
}

/// Events emitted from the MTK GUI Window to the Debugger.
#[derive(Clone, Debug)]
pub enum DebugEvent {
    LayoutUpdated(Box<LayoutSnapshot>),
    HoveredNode(Option<u64>),
    Closed,
}

/// Commands sent from the Debugger to the MTK GUI Window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugCommand {
    HighlightNode(Option<u64>),
    RequestSnapshot,
}

/// Trait implemented by layout debugger frontends.
pub trait LayoutDebugger: Send + 'static {
    fn on_attach(&mut self, tx_cmd: Sender<DebugCommand>);
    fn on_event(&mut self, event: DebugEvent);
}

/// Spawns the interactive Ratatui Terminal UI Layout Debugger in a background thread.
pub struct TerminalDebugger;

impl TerminalDebugger {
    pub fn spawn() -> (Sender<DebugEvent>, Receiver<DebugCommand>) {
        let (tx_event, rx_event) = channel();
        let (tx_cmd, rx_cmd) = channel();

        #[cfg(feature = "debugger")]
        {
            tui::spawn_tui_debugger(rx_event, tx_cmd);
        }

        #[cfg(not(feature = "debugger"))]
        {
            let _ = rx_event;
            let _ = tx_cmd;
            eprintln!(
                "[MTK] Note: enable feature `debugger` in Cargo.toml for interactive TUI outliner."
            );
        }

        (tx_event, rx_cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::{column, text};
    use crate::ui::{View, ViewStyleExt};
    use crate::{Context, Size, Style};

    #[test]
    fn test_source_location_tracking() {
        let loc = SourceLocation::here("TestWidget");
        assert_eq!(loc.type_name, "TestWidget");
        assert!(loc.file.ends_with("mod.rs") || loc.file.ends_with("debugger.rs"));
        assert!(loc.line > 0);
        assert_eq!(
            loc.display(),
            format!("{}:{}:{} (TestWidget)", loc.file, loc.line, loc.column)
        );
    }

    #[test]
    fn test_layout_snapshot_with_sources() {
        let mut ctx = Context::new();

        let v = column((
            text::<_, ()>("First"),
            text::<_, ()>("Second").style(Style::new().width(Size::Fixed(100))),
        ));

        let el = View::<()>::build(&v, &mut ctx);
        let root = View::<()>::get_node(&v, &el);
        ctx.root_attach(root);
        ctx.compute_layout(800.0, 600.0);

        let snapshot = ctx.build_debug_snapshot(800.0, 600.0, None);
        assert_eq!(snapshot.total_nodes, 3);
        assert!(snapshot.root.is_some());

        let root_info = snapshot.root.unwrap();
        assert_eq!(root_info.name, "Column");
        assert_eq!(root_info.children.len(), 2);
        assert_eq!(root_info.children[0].name, "Text");
        assert_eq!(root_info.children[0].text.as_deref(), Some("First"));
        assert_eq!(root_info.children[1].name, "Text");
        assert_eq!(root_info.children[1].text.as_deref(), Some("Second"));
    }

    #[test]
    fn test_debugger_mpsc_communication() {
        let (tx, rx) = channel();

        struct MockDebugger {
            events: Vec<DebugEvent>,
        }

        impl LayoutDebugger for MockDebugger {
            fn on_attach(&mut self, _tx_cmd: Sender<DebugCommand>) {}
            fn on_event(&mut self, event: DebugEvent) {
                self.events.push(event);
            }
        }

        let mut debugger = MockDebugger { events: Vec::new() };
        let (tx_cmd, _rx_cmd) = channel();
        debugger.on_attach(tx_cmd);

        let snapshot = Box::new(LayoutSnapshot {
            root: None,
            total_nodes: 0,
            viewport_w: 800.0,
            viewport_h: 600.0,
            hovered_node: Some(42),
        });

        tx.send(DebugEvent::LayoutUpdated(snapshot)).unwrap();
        tx.send(DebugEvent::HoveredNode(Some(42))).unwrap();

        while let Ok(ev) = rx.try_recv() {
            debugger.on_event(ev);
        }

        assert_eq!(debugger.events.len(), 2);
        assert!(matches!(debugger.events[0], DebugEvent::LayoutUpdated(_)));
        assert!(matches!(
            debugger.events[1],
            DebugEvent::HoveredNode(Some(42))
        ));
    }
}
