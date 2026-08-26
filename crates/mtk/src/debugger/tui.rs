//! Full-featured interactive Ratatui TUI Layout Debugger for MTK.
//!
//! Provides mouse clicking, keyboard navigation, collapsible tree hierarchy outliner,
//! W3C flexbox box model visualization, and source location copying.
//! Fully theme-agnostic (adapts automatically to both Light and Dark terminal palettes).

use std::collections::HashSet;
use std::io::stdout;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::{DebugCommand, DebugEvent, LayoutSnapshot, NodeBoxMetrics, NodeDebugInfo};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout(),
            DisableMouseCapture,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
    }
}

/// Spawns the interactive Ratatui TUI debugger on a background thread.
pub fn spawn_tui_debugger(rx_event: Receiver<DebugEvent>, tx_cmd: Sender<DebugCommand>) {
    thread::Builder::new()
        .name("mtk-tui-debugger".into())
        .spawn(move || {
            if let Err(e) = run_tui_app(rx_event, tx_cmd) {
                eprintln!("[MTK TUI Debugger] Exited with error: {e}");
            }
        })
        .expect("Failed to spawn TUI debugger thread");
}

#[derive(Clone, Debug)]
struct FlatTreeItem {
    id: u64,
    name: String,
    depth: usize,
    has_children: bool,
    is_collapsed: bool,
    text_preview: Option<String>,
    width: f32,
    height: f32,
}

struct TuiAppState {
    snapshot: Option<LayoutSnapshot>,
    selected_node_id: Option<u64>,
    hovered_node_id: Option<u64>,
    collapsed_nodes: HashSet<u64>,
    flat_items: Vec<FlatTreeItem>,
    selected_index: usize,
    scroll_offset: usize,
    filter_query: String,
    is_searching: bool,
    status_message: Option<String>,
    tree_area: Rect,
}

impl TuiAppState {
    fn new() -> Self {
        Self {
            snapshot: None,
            selected_node_id: None,
            hovered_node_id: None,
            collapsed_nodes: HashSet::new(),
            flat_items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            filter_query: String::new(),
            is_searching: false,
            status_message: Some("Ready. Click a node or use ↑/↓ to inspect.".to_string()),
            tree_area: Rect::default(),
        }
    }

    fn update_snapshot(&mut self, snapshot: LayoutSnapshot) {
        if self.selected_node_id.is_none() {
            if let Some(root) = &snapshot.root {
                self.selected_node_id = Some(root.id);
            }
        }
        self.snapshot = Some(snapshot);
        self.rebuild_flat_tree();
    }

    fn rebuild_flat_tree(&mut self) {
        self.flat_items.clear();
        if let Some(snapshot) = &self.snapshot {
            if let Some(root) = &snapshot.root {
                let mut items = Vec::new();
                flatten_node(root, 0, &self.collapsed_nodes, &mut items);

                if !self.filter_query.is_empty() {
                    let query = self.filter_query.to_lowercase();
                    items.retain(|item| {
                        item.name.to_lowercase().contains(&query)
                            || item
                                .text_preview
                                .as_ref()
                                .map(|t| t.to_lowercase().contains(&query))
                                .unwrap_or(false)
                    });
                }

                self.flat_items = items;
            }
        }

        // Adjust selected index
        if let Some(selected_id) = self.selected_node_id {
            if let Some(idx) = self.flat_items.iter().position(|i| i.id == selected_id) {
                self.selected_index = idx;
            }
        }
        if self.selected_index >= self.flat_items.len() && !self.flat_items.is_empty() {
            self.selected_index = self.flat_items.len() - 1;
            self.selected_node_id = Some(self.flat_items[self.selected_index].id);
        }
    }

    fn select_node(&mut self, id: u64, tx_cmd: &Sender<DebugCommand>) {
        self.selected_node_id = Some(id);
        if let Some(idx) = self.flat_items.iter().position(|i| i.id == id) {
            self.selected_index = idx;
        }
        let _ = tx_cmd.send(DebugCommand::HighlightNode(Some(id)));
    }

    fn toggle_collapse(&mut self, id: u64) {
        if self.collapsed_nodes.contains(&id) {
            self.collapsed_nodes.remove(&id);
        } else {
            self.collapsed_nodes.insert(id);
        }
        self.rebuild_flat_tree();
    }

    fn move_selection(&mut self, delta: isize, tx_cmd: &Sender<DebugCommand>) {
        if self.flat_items.is_empty() {
            return;
        }
        let new_idx = (self.selected_index as isize + delta)
            .clamp(0, (self.flat_items.len() - 1) as isize) as usize;
        self.selected_index = new_idx;
        let id = self.flat_items[new_idx].id;
        self.selected_node_id = Some(id);
        let _ = tx_cmd.send(DebugCommand::HighlightNode(Some(id)));
    }

    fn copy_selected_location(&mut self) {
        if self.selected_node_id.is_some() {
            if let Some(node) = self.find_selected_node() {
                if let Some(src) = &node.source {
                    let loc_str = src.link();
                    if let Ok(mut clip) = arboard::Clipboard::new() {
                        let _ = clip.set_text(&loc_str);
                        self.status_message =
                            Some(format!("Copied location to clipboard: {loc_str}"));
                        return;
                    }
                }
            }
        }
        self.status_message = Some("No source location available for selected node.".to_string());
    }

    fn find_selected_node(&self) -> Option<&NodeDebugInfo> {
        let id = self.selected_node_id?;
        let root = self.snapshot.as_ref()?.root.as_ref()?;
        find_node_by_id(root, id)
    }
}

fn flatten_node(
    node: &NodeDebugInfo,
    depth: usize,
    collapsed: &HashSet<u64>,
    out: &mut Vec<FlatTreeItem>,
) {
    let has_children = !node.children.is_empty();
    let is_collapsed = collapsed.contains(&node.id);

    out.push(FlatTreeItem {
        id: node.id,
        name: node.name.clone(),
        depth,
        has_children,
        is_collapsed,
        text_preview: node.text.clone(),
        width: node.metrics.w,
        height: node.metrics.h,
    });

    if !is_collapsed {
        for child in &node.children {
            flatten_node(child, depth + 1, collapsed, out);
        }
    }
}

fn find_node_by_id<'a>(root: &'a NodeDebugInfo, id: u64) -> Option<&'a NodeDebugInfo> {
    if root.id == id {
        return Some(root);
    }
    for child in &root.children {
        if let Some(found) = find_node_by_id(child, id) {
            return Some(found);
        }
    }
    None
}

fn run_tui_app(
    rx_event: Receiver<DebugEvent>,
    tx_cmd: Sender<DebugCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        crossterm::cursor::Hide
    )?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiAppState::new();

    // Initial snapshot request
    let _ = tx_cmd.send(DebugCommand::RequestSnapshot);

    loop {
        // Drain any incoming events from the GUI application
        while let Ok(event) = rx_event.try_recv() {
            match event {
                DebugEvent::LayoutUpdated(snapshot) => {
                    state.update_snapshot(*snapshot);
                }
                DebugEvent::HoveredNode(node_id) => {
                    state.hovered_node_id = node_id;
                }
                DebugEvent::Closed => {
                    return Ok(());
                }
            }
        }

        // Draw TUI Frame
        terminal.draw(|f| {
            render_ui(f, &mut state);
        })?;

        // Handle user input events (crossterm)
        if crossterm::event::poll(Duration::from_millis(16))? {
            match crossterm::event::read()? {
                CEvent::Key(key) => {
                    if state.is_searching {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => {
                                state.is_searching = false;
                            }
                            KeyCode::Backspace => {
                                state.filter_query.pop();
                                state.rebuild_flat_tree();
                            }
                            KeyCode::Char(c) => {
                                state.filter_query.push(c);
                                state.rebuild_flat_tree();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            let _ = tx_cmd.send(DebugCommand::HighlightNode(None));
                            break;
                        }
                        KeyCode::Char('c') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                let _ = tx_cmd.send(DebugCommand::HighlightNode(None));
                                break;
                            } else {
                                state.copy_selected_location();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.move_selection(-1, &tx_cmd);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.move_selection(1, &tx_cmd);
                        }
                        KeyCode::PageUp => {
                            state.move_selection(-10, &tx_cmd);
                        }
                        KeyCode::PageDown => {
                            state.move_selection(10, &tx_cmd);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if let Some(selected_id) = state.selected_node_id {
                                state.toggle_collapse(selected_id);
                            }
                        }
                        KeyCode::Char('/') => {
                            state.is_searching = true;
                            state.filter_query.clear();
                            state.rebuild_flat_tree();
                        }
                        KeyCode::Esc => {
                            if !state.filter_query.is_empty() {
                                state.filter_query.clear();
                                state.rebuild_flat_tree();
                            } else {
                                state.selected_node_id = None;
                                let _ = tx_cmd.send(DebugCommand::HighlightNode(None));
                            }
                        }
                        _ => {}
                    }
                }
                CEvent::Mouse(mouse) => {
                    let tree_area = state.tree_area;
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            if mouse.column >= tree_area.left()
                                && mouse.column < tree_area.right()
                                && mouse.row >= tree_area.top() + 1
                                && mouse.row < tree_area.bottom() - 1
                            {
                                let click_row = (mouse.row - (tree_area.top() + 1)) as usize;
                                let clicked_idx = state.scroll_offset + click_row;
                                if clicked_idx < state.flat_items.len() {
                                    let node_id = state.flat_items[clicked_idx].id;
                                    state.select_node(node_id, &tx_cmd);
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            state.move_selection(-1, &tx_cmd);
                        }
                        MouseEventKind::ScrollDown => {
                            state.move_selection(1, &tx_cmd);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_ui(f: &mut ratatui::Frame, state: &mut TuiAppState) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header Bar
            Constraint::Min(10),   // Main Content Split
            Constraint::Length(3), // Status & Shortcuts Bar
        ])
        .split(size);

    render_header_bar(f, chunks[0], state);
    render_main_split(f, chunks[1], state);
    render_status_bar(f, chunks[2], state);
}

fn render_header_bar(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let (viewport_text, node_count_text) = if let Some(snap) = &state.snapshot {
        (
            format!("{:.0}×{:.0}px", snap.viewport_w, snap.viewport_h),
            format!("{} nodes", snap.total_nodes),
        )
    } else {
        ("Waiting for App...".to_string(), "0 nodes".to_string())
    };

    let title_line = Line::from(vec![
        Span::styled(
            " MTK ",
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "Layout Inspector TUI",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  •  Viewport: "),
        Span::styled(viewport_text, Style::default().fg(Color::Green)),
        Span::raw("  •  Count: "),
        Span::styled(node_count_text, Style::default().fg(Color::Yellow)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(title_line).block(block);
    f.render_widget(paragraph, area);
}

fn render_main_split(f: &mut ratatui::Frame, area: Rect, state: &mut TuiAppState) {
    let splits = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    state.tree_area = splits[0];
    render_outliner_pane(f, splits[0], state);
    render_inspector_pane(f, splits[1], state);
}

fn render_outliner_pane(f: &mut ratatui::Frame, area: Rect, state: &mut TuiAppState) {
    let visible_rows = (area.height.saturating_sub(2)) as usize;

    // Adjust scroll offset to keep selected_index visible
    if state.selected_index < state.scroll_offset {
        state.scroll_offset = state.selected_index;
    } else if state.selected_index >= state.scroll_offset + visible_rows {
        state.scroll_offset = state
            .selected_index
            .saturating_sub(visible_rows.saturating_sub(1));
    }

    let mut lines = Vec::new();

    if state.flat_items.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  Waiting for MTK UI layout tree...",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for (idx, item) in state
            .flat_items
            .iter()
            .enumerate()
            .skip(state.scroll_offset)
            .take(visible_rows)
        {
            let is_selected = idx == state.selected_index;
            let is_hovered = state.hovered_node_id == Some(item.id);

            let fold_icon = if item.has_children {
                if item.is_collapsed { "▶ " } else { "▼ " }
            } else {
                "• "
            };

            let indent = "  ".repeat(item.depth);
            let text_snippet = item
                .text_preview
                .as_ref()
                .map(|t| {
                    if t.len() > 14 {
                        format!(" \"{}...\"", &t[..12])
                    } else {
                        format!(" \"{}\"", t)
                    }
                })
                .unwrap_or_default();

            let size_badge = format!(" [{:.0}×{:.0}]", item.width, item.height);

            let mut spans = vec![
                Span::raw(indent),
                Span::styled(
                    fold_icon,
                    Style::default().fg(if item.has_children {
                        Color::Yellow
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::styled(
                    &item.name,
                    if is_selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    format!(" #{}", item.id),
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            if !text_snippet.is_empty() {
                spans.push(Span::styled(
                    text_snippet,
                    Style::default().fg(Color::Green),
                ));
            }

            spans.push(Span::styled(size_badge, Style::default().fg(Color::Blue)));

            let mut row_style = Style::default();
            if is_selected {
                row_style = Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD);
            } else if is_hovered {
                row_style = Style::default().add_modifier(Modifier::UNDERLINED);
            }

            lines.push(Line::from(spans).style(row_style));
        }
    }

    let title = if state.is_searching || !state.filter_query.is_empty() {
        format!(" Hierarchy Outliner [Filter: {}] ", state.filter_query)
    } else {
        " Hierarchy Outliner (Click / ↑↓) ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_inspector_pane(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let node_opt = state.find_selected_node();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Source Code Location Card
            Constraint::Length(9), // W3C Flexbox Box Model ASCII
            Constraint::Min(6),    // Layout Dimensions Table
        ])
        .split(area);

    render_source_card(f, chunks[0], node_opt);
    render_box_model_card(f, chunks[1], node_opt.map(|n| &n.metrics));
    render_metrics_card(f, chunks[2], node_opt);
}

fn render_source_card(f: &mut ratatui::Frame, area: Rect, node: Option<&NodeDebugInfo>) {
    let lines = if let Some(n) = node {
        if let Some(src) = &n.source {
            vec![
                Line::from(vec![
                    Span::styled("Widget: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        &n.name,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" (Node #{})", n.id),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Location: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        src.link(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                    Span::styled(
                        "  [Press 'c' to copy]",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("Widget: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&n.name, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Location: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Native MTK Primitive", Style::default().fg(Color::DarkGray)),
                ]),
            ]
        }
    } else {
        vec![Line::from(vec![Span::styled(
            "Select a node from the Outliner to inspect source code.",
            Style::default().fg(Color::DarkGray),
        )])]
    };

    let block = Block::default()
        .title(" Source Code Definition ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_box_model_card(f: &mut ratatui::Frame, area: Rect, metrics: Option<&NodeBoxMetrics>) {
    let (iw, ih, pt, pb, pl, pr, bt, bb, bl, br) = if let Some(m) = metrics {
        let pad_l = m.pad_left + m.border_left;
        let pad_t = m.pad_top + m.border_top;
        let pad_r = m.pad_right + m.border_right;
        let pad_b = m.pad_bottom + m.border_bottom;
        let w = (m.w - (pad_l + pad_r)).max(0.0);
        let h = (m.h - (pad_t + pad_b)).max(0.0);
        (
            w,
            h,
            m.pad_top,
            m.pad_bottom,
            m.pad_left,
            m.pad_right,
            m.border_top,
            m.border_bottom,
            m.border_left,
            m.border_right,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    };

    let box_art = vec![
        Line::from(vec![Span::styled(
            "┌─ Padding ──────────────────────────────────────────┐",
            Style::default().fg(Color::Green),
        )]),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("Top: {:.0}px", pt + bt),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                "                                       │",
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(Color::Green)),
            Span::styled(format!("{:.0}", pl + bl), Style::default().fg(Color::Green)),
            Span::styled(
                "  ┌─ Content ────────────────────────┐  ",
                Style::default().fg(Color::Blue),
            ),
            Span::styled(format!("{:.0}", pr + br), Style::default().fg(Color::Green)),
            Span::styled(" │", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("│    │  ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("Size: {:.1} × {:.1} px", iw, ih),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("          │    │", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![Span::styled(
            "│    └──────────────────────────────────┘    │",
            Style::default().fg(Color::Blue),
        )]),
        Line::from(vec![
            Span::styled("│ ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("Bottom: {:.0}px", pb + bb),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                "                                    │",
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![Span::styled(
            "└────────────────────────────────────────────────────┘",
            Style::default().fg(Color::Green),
        )]),
    ];

    let block = Block::default()
        .title(" W3C Flexbox Box Model ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(box_art).block(block);
    f.render_widget(paragraph, area);
}

fn render_metrics_card(f: &mut ratatui::Frame, area: Rect, node: Option<&NodeDebugInfo>) {
    let lines = if let Some(n) = node {
        let m = &n.metrics;
        vec![
            Line::from(vec![
                Span::styled(
                    "Computed Size:         ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:.1} × {:.1} px", m.w, m.h),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Offset (X, Y):         ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("x: {:.1}, y: {:.1}", m.x, m.y), Style::default()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Content Scroll Bounds: ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:.1} × {:.1} px", m.content_w, m.content_h),
                    Style::default(),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Flex Direction:        ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(&m.flex_direction, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Flex Grow / Shrink:    ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{} / {}", m.flex_grow, m.flex_shrink),
                    Style::default(),
                ),
            ]),
        ]
    } else {
        vec![Line::from(vec![Span::styled(
            "No node selected.",
            Style::default().fg(Color::DarkGray),
        )])]
    };

    let block = Block::default()
        .title(" Layout Dimensions & Geometry ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let status_text = state
        .status_message
        .clone()
        .unwrap_or_else(|| "Navigate with ↑/↓ or click any node to inspect.".to_string());

    let line = Line::from(vec![
        Span::styled(
            " Shortcuts: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("[↑/↓] Navigate  ", Style::default()),
        Span::styled("[Click] Inspect  ", Style::default()),
        Span::styled("[Enter] Fold/Expand  ", Style::default()),
        Span::styled("[c] Copy Location  ", Style::default()),
        Span::styled("[/] Search  ", Style::default()),
        Span::styled("[q] Quit", Style::default().fg(Color::Red)),
        Span::raw("  │  "),
        Span::styled(status_text, Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(line).block(block);
    f.render_widget(paragraph, area);
}
