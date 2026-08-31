use mtk::colors::Color;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::adapt;
use mtk::ui::style::ViewStyleExt;
use mtk::ui::widgets::{badge, button, column, input_text, row, text, virtual_list};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{Edges, clr};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
struct LogRecord {
    id: usize,
    timestamp: String,
    service: &'static str,
    level: &'static str,
    message: String,
    latency_ms: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FilterLevel {
    All,
    Errors,
    Warnings,
    Info,
}

struct AppState {
    all_logs: Vec<LogRecord>,
    filtered_logs: Rc<Vec<LogRecord>>,
    search_query: String,
    filter_level: FilterLevel,
}

#[derive(Clone)]
enum AppMsg {
    SetSearch(String),
    SetFilter(FilterLevel),
}

fn generate_logs(count: usize) -> Vec<LogRecord> {
    let services = [
        "auth-service",
        "payment-gw",
        "indexer-v2",
        "db-cluster",
        "ingress-router",
        "cache-redis",
    ];
    let levels = ["INFO", "WARN", "ERROR", "DEBUG"];
    let messages = [
        "Connection pool established with read replica",
        "TLS handshake succeeded for upstream peer",
        "Cache miss fallback to primary key-value store",
        "High latency threshold exceeded on endpoint /v1/checkout",
        "Token verification failed: expired signature payload",
        "Database query timeout after 3000ms on index scan",
        "Background garbage collection completed successfully",
        "Rate limit bucket saturated for client IP 192.168.1.42",
    ];

    let mut logs = Vec::with_capacity(count);
    for i in 0..count {
        let sec = (i % 60) as u32;
        let min = ((i / 60) % 60) as u32;
        let hour = ((i / 3600) % 24) as u32;
        let ms = (i * 37) % 1000;

        let svc = services[i % services.len()];
        let level = match (i * 7) % 10 {
            0..=5 => levels[0], // INFO (60%)
            6..=7 => levels[1], // WARN (20%)
            8 => levels[2],     // ERROR (10%)
            _ => levels[3],     // DEBUG (10%)
        };
        let msg = messages[(i * 3 + (i % 5)) % messages.len()];
        let latency = ((i * 13) % 240 + 5) as u32;

        logs.push(LogRecord {
            id: i + 1,
            timestamp: format!("{:02}:{:02}:{:02}.{:03}", hour, min, sec, ms),
            service: svc,
            level,
            message: format!("{msg} [req_id: #{:06x}]", i * 1337),
            latency_ms: latency,
        });
    }
    logs
}

fn apply_filter(logs: &[LogRecord], query: &str, level: FilterLevel) -> Vec<LogRecord> {
    let q = query.trim().to_lowercase();
    logs.iter()
        .filter(|log| {
            let matches_level = match level {
                FilterLevel::All => true,
                FilterLevel::Errors => log.level == "ERROR",
                FilterLevel::Warnings => log.level == "WARN",
                FilterLevel::Info => log.level == "INFO",
            };
            if !matches_level {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            log.service.to_lowercase().contains(&q)
                || log.message.to_lowercase().contains(&q)
                || log.level.to_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::SetSearch(q) => {
            state.search_query = q;
            let filtered = apply_filter(&state.all_logs, &state.search_query, state.filter_level);
            state.filtered_logs = Rc::new(filtered);
        }
        AppMsg::SetFilter(level) => {
            state.filter_level = level;
            let filtered = apply_filter(&state.all_logs, &state.search_query, state.filter_level);
            state.filtered_logs = Rc::new(filtered);
        }
    }
}

fn search_query_lens(s: &AppState) -> &String {
    &s.search_query
}

fn cell_txt(
    label: impl Into<String>,
    width: u32,
    color: Color,
    size: f32,
    weight: FontWeight,
) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    text(label.into()).style(
        Style::new().width(Size::Fixed(width)).set_text_style(
            TextStyle::new()
                .color(color)
                .font_size(size)
                .font_weight(weight),
        ),
    )
}

fn txt(
    label: impl Into<String>,
    color: Color,
    size: f32,
    weight: FontWeight,
) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    text(label.into()).style(
        Style::new().set_text_style(
            TextStyle::new()
                .color(color)
                .font_size(size)
                .font_weight(weight),
        ),
    )
}

fn filter_chip(
    label: &str,
    level: FilterLevel,
    current: FilterLevel,
) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    let is_active = level == current;
    let bg = if is_active {
        clr!(0x2563EBFF)
    } else {
        clr!(0xF1F5F9FF)
    };
    let text_clr = if is_active {
        clr!(0xFFFFFFFF)
    } else {
        clr!(0x475569FF)
    };

    button(label).on_click(AppMsg::SetFilter(level)).style(
        Style::new()
            .padding_xy(12.0, 6.0)
            .corner_radius(6.0)
            .bg_color(bg)
            .border(
                1.0,
                if is_active {
                    clr!(0x1D4ED8FF)
                } else {
                    clr!(0xCBD5E1FF)
                },
            )
            .set_text_style(
                TextStyle::new()
                    .color(text_clr)
                    .font_size(12.0)
                    .font_weight(FontWeight::NORMAL),
            ),
    )
}

fn main() {
    let total_count = 1_000_000;
    println!("Generating {total_count} log entries in memory...");
    let all_logs = generate_logs(total_count);
    let initial_filtered = all_logs.clone();

    let state = AppState {
        all_logs,
        filtered_logs: Rc::new(initial_filtered),
        search_query: String::new(),
        filter_level: FilterLevel::All,
    };

    let mut window = Window::with(state, update, |state: &AppState| {
        let logs_ref = Rc::clone(&state.filtered_logs);
        let count = logs_ref.len();
        let total = state.all_logs.len();

        // Top Navigation & Toolbar
        let header_bar = column((
            row((
                row((
                    badge("MTK VIRTUAL LIST").style(
                        Style::new()
                            .bg_color(clr!(0x2563EBFF))
                            .corner_radius(4.0)
                            .padding_edges(Edges::lr(4.))
                            .set_text_style(
                                TextStyle::new()
                                    .color(clr!(0xFFFFFFFF))
                                    .font_weight(FontWeight::BOLD),
                            ),
                    ),
                    txt(
                        "High-Performance System Log Streamer",
                        clr!(0x0F172AFF),
                        18.0,
                        FontWeight::BOLD,
                    ),
                ))
                .style(Style::new().gap(12.0).align_items(AlignItems::Center)),
                row((txt(
                    format!("Showing: {count} / {total} entries (1,000,000 rows)"),
                    clr!(0x2563EBFF),
                    12.0,
                    FontWeight::BOLD,
                ),))
                .style(Style::new().align_items(AlignItems::Center)),
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .justify_content(JustifyContent::SpaceBetween)
                    .align_items(AlignItems::Center),
            ),
            // Search and filter chips
            row((
                adapt(
                    input_text()
                        .placeholder("Filter by Service, Level, or Message content...")
                        .style(
                            Style::new()
                                .width(Size::Fixed(360))
                                .height(Size::Fixed(32))
                                .padding_xy(10.0, 0.0)
                                .bg_color(clr!(0xFFFFFFFF))
                                .corner_radius(6.0)
                                .border(1.0, clr!(0xCBD5E1FF))
                                .set_text_style(
                                    TextStyle::new().color(clr!(0x0F172AFF)).font_size(13.0),
                                ),
                        ),
                    search_query_lens,
                    AppMsg::SetSearch,
                ),
                row((
                    filter_chip("All (1,000,000)", FilterLevel::All, state.filter_level),
                    filter_chip("Errors Only", FilterLevel::Errors, state.filter_level),
                    filter_chip("Warnings", FilterLevel::Warnings, state.filter_level),
                    filter_chip("Info", FilterLevel::Info, state.filter_level),
                ))
                .style(Style::new().gap(8.0)),
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .gap(16.0)
                    .align_items(AlignItems::Center),
            ),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .padding(16.0)
                .gap(12.0)
                .bg_color(clr!(0xF8FAFCFF))
                .border(1.0, clr!(0xE2E8F0FF)),
        );

        // Table Header
        let table_header = row((
            cell_txt("ID", 70, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("TIME", 110, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("LEVEL", 80, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("SERVICE", 130, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("LATENCY", 80, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("MESSAGE", 450, clr!(0x475569FF), 12.0, FontWeight::BOLD),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Fixed(36))
                .padding_xy(16.0, 0.0)
                .align_items(AlignItems::Center)
                .bg_color(clr!(0xF1F5F9FF))
                .border(1.0, clr!(0xE2E8F0FF)),
        );

        // Virtualized List
        let vlist = virtual_list((*logs_ref).clone(), 36.0, |idx, item: &LogRecord| {
            let is_even = idx % 2 == 0;
            let row_bg = if is_even {
                clr!(0xFFFFFFFF)
            } else {
                clr!(0xF8FAFCFF)
            };

            let (level_bg, level_fg) = match item.level {
                "ERROR" => (clr!(0xFEE2E2FF), clr!(0x991B1BFF)),
                "WARN" => (clr!(0xFEF3C7FF), clr!(0x92400EFF)),
                "INFO" => (clr!(0xDBEAFEFF), clr!(0x1E40AFFF)),
                _ => (clr!(0xF1F5F9FF), clr!(0x475569FF)),
            };

            let latency_color = if item.latency_ms > 150 {
                clr!(0xDC2626FF)
            } else if item.latency_ms > 80 {
                clr!(0xD97706FF)
            } else {
                clr!(0x16A34AFF)
            };

            row((
                cell_txt(
                    format!("#{}", item.id),
                    70,
                    clr!(0x64748BFF),
                    12.0,
                    FontWeight::NORMAL,
                ),
                cell_txt(
                    item.timestamp.clone(),
                    110,
                    clr!(0x64748BFF),
                    12.0,
                    FontWeight::NORMAL,
                ),
                badge(item.level).style(
                    Style::new()
                        .width(Size::Fixed(64))
                        .bg_color(level_bg)
                        .corner_radius(4.0)
                        .set_text_style(
                            TextStyle::new()
                                .color(level_fg)
                                .font_size(11.0)
                                .font_weight(FontWeight::BOLD),
                        ),
                ),
                cell_txt(item.service, 130, clr!(0x0F172AFF), 12.0, FontWeight::BOLD),
                cell_txt(
                    format!("{}ms", item.latency_ms),
                    80,
                    latency_color,
                    12.0,
                    FontWeight::BOLD,
                ),
                cell_txt(
                    item.message.clone(),
                    450,
                    clr!(0x334155FF),
                    12.0,
                    FontWeight::NORMAL,
                ),
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .height(Size::Fixed(36))
                    .min_height(36.0)
                    .max_height(36.0)
                    .flex_shrink(0.0)
                    .flex_grow(0.0)
                    .padding_xy(16.0, 0.0)
                    .align_items(AlignItems::Center)
                    .gap(10.0)
                    .bg_color(row_bg)
                    .on_hover(|s| s.bg_color(clr!(0xEFF6FFFF))),
            )
        })
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .flex_grow(1.0)
                .flex_shrink(1.0)
                .min_height(0.0)
                .bg_color(clr!(0xFFFFFFFF)),
        );

        column((header_bar, table_header, vlist)).style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Percent(1.0))
                .bg_color(clr!(0xFFFFFFFF)),
        )
    });

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK Virtualized 1,000,000 Log Streamer Demo")
            .with_size((960, 720).into()),
    );
}
