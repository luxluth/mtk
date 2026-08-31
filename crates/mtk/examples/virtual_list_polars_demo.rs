use mtk::colors::Color;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::adapt;
use mtk::ui::style::ViewStyleExt;
use mtk::ui::widgets::{badge, button, column, input_text, row, text, virtual_list};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{Edges, clr};
use polars::prelude::*;
use std::rc::Rc;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
struct MarketRow {
    id: u32,
    timestamp: String,
    ticker: String,
    price: f64,
    change_pct: f64,
    volume: u32,
    sentiment: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FilterMode {
    All,
    Bullish,
    Gainers,
    HighVolume,
}

struct AppState {
    full_df: DataFrame,
    filtered_rows: Rc<Vec<MarketRow>>,
    search_query: String,
    filter_mode: FilterMode,
    query_time_ms: f64,
    total_count: usize,
}

#[derive(Clone)]
enum AppMsg {
    SetSearch(String),
    SetFilter(FilterMode),
}

fn generate_market_data(count: usize) -> DataFrame {
    let tickers = [
        "NVDA", "AAPL", "MSFT", "AMZN", "GOOGL", "META", "TSLA", "AMD", "BTC", "ETH",
    ];
    let sentiments = ["BULLISH", "BEARISH", "NEUTRAL"];

    let mut ids = Vec::with_capacity(count);
    let mut timestamps = Vec::with_capacity(count);
    let mut ticker_col = Vec::with_capacity(count);
    let mut prices = Vec::with_capacity(count);
    let mut changes = Vec::with_capacity(count);
    let mut volumes = Vec::with_capacity(count);
    let mut sentiment_col = Vec::with_capacity(count);

    let mut rng_state: u64 = 133742069;
    let mut next_rand = || {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((rng_state >> 33) as f64) / 2147483648.0
    };

    for i in 0..count {
        ids.push(i as u32 + 1);
        let sec = (i % 60) as u32;
        let min = ((i / 60) % 60) as u32;
        let hour = ((i / 3600) % 24) as u32;
        timestamps.push(format!(
            "{:02}:{:02}:{:02}.{:03}",
            hour,
            min,
            sec,
            (i % 1000)
        ));

        let t_idx = (next_rand() * tickers.len() as f64) as usize % tickers.len();
        ticker_col.push(tickers[t_idx]);

        let base_price = match tickers[t_idx] {
            "BTC" => 64500.0,
            "ETH" => 3450.0,
            "NVDA" => 125.0,
            "AAPL" => 220.0,
            "MSFT" => 445.0,
            _ => 180.0,
        };
        let price_var = (next_rand() - 0.5) * (base_price * 0.1);
        prices.push((base_price + price_var * 100.0).round() / 100.0);

        let chg = (next_rand() - 0.48) * 12.0;
        changes.push((chg * 100.0).round() / 100.0);

        let vol = ((next_rand() * 50_000.0) + 1_000.0) as u32;
        volumes.push(vol);

        let s_idx = if chg > 1.5 {
            0
        } else if chg < -1.5 {
            1
        } else {
            2
        };
        sentiment_col.push(sentiments[s_idx]);
    }

    df!(
        "id" => ids,
        "timestamp" => timestamps,
        "ticker" => ticker_col,
        "price" => prices,
        "change_pct" => changes,
        "volume" => volumes,
        "sentiment" => sentiment_col,
    )
    .unwrap()
}

fn execute_polars_query(df: &DataFrame, query: &str, mode: FilterMode) -> (Vec<MarketRow>, f64) {
    let t0 = Instant::now();
    let mut lazy = df.clone().lazy();

    let query_clean = query.trim().to_uppercase();
    if !query_clean.is_empty() {
        lazy = lazy.filter(
            col("ticker")
                .eq(lit(query_clean.clone()))
                .or(col("sentiment").eq(lit(query_clean))),
        );
    }

    match mode {
        FilterMode::All => {}
        FilterMode::Bullish => {
            lazy = lazy.filter(col("sentiment").eq(lit("BULLISH")));
        }
        FilterMode::Gainers => {
            lazy = lazy.filter(col("change_pct").gt(lit(0.0)));
        }
        FilterMode::HighVolume => {
            lazy = lazy.filter(col("volume").gt(lit(30000u32)));
        }
    }

    let result_df = lazy.collect().unwrap_or_else(|_| df.clone());
    let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

    let id_ca = result_df.column("id").unwrap().u32().unwrap();
    let ts_ca = result_df.column("timestamp").unwrap().str().unwrap();
    let ticker_ca = result_df.column("ticker").unwrap().str().unwrap();
    let price_ca = result_df.column("price").unwrap().f64().unwrap();
    let chg_ca = result_df.column("change_pct").unwrap().f64().unwrap();
    let vol_ca = result_df.column("volume").unwrap().u32().unwrap();
    let sent_ca = result_df.column("sentiment").unwrap().str().unwrap();

    let row_count = result_df.height();
    let mut rows = Vec::with_capacity(row_count);

    for i in 0..row_count {
        rows.push(MarketRow {
            id: id_ca.get(i).unwrap_or(0),
            timestamp: ts_ca.get(i).unwrap_or("").to_string(),
            ticker: ticker_ca.get(i).unwrap_or("").to_string(),
            price: price_ca.get(i).unwrap_or(0.0),
            change_pct: chg_ca.get(i).unwrap_or(0.0),
            volume: vol_ca.get(i).unwrap_or(0),
            sentiment: sent_ca.get(i).unwrap_or("").to_string(),
        });
    }

    (rows, elapsed)
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::SetSearch(q) => {
            state.search_query = q;
            let (rows, elapsed) =
                execute_polars_query(&state.full_df, &state.search_query, state.filter_mode);
            state.filtered_rows = Rc::new(rows);
            state.query_time_ms = elapsed;
        }
        AppMsg::SetFilter(mode) => {
            state.filter_mode = mode;
            let (rows, elapsed) =
                execute_polars_query(&state.full_df, &state.search_query, state.filter_mode);
            state.filtered_rows = Rc::new(rows);
            state.query_time_ms = elapsed;
        }
    }
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

fn search_query_lens(s: &AppState) -> &String {
    &s.search_query
}

fn filter_chip(
    label: &str,
    mode: FilterMode,
    current: FilterMode,
) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    let is_active = mode == current;
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

    button(label).on_click(AppMsg::SetFilter(mode)).style(
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
    let row_count = 100_000;
    println!("Generating {row_count} financial market records in Polars DataFrame...");
    let df = generate_market_data(row_count);
    let (initial_rows, elapsed) = execute_polars_query(&df, "", FilterMode::All);

    let state = AppState {
        full_df: df,
        filtered_rows: Rc::new(initial_rows),
        search_query: String::new(),
        filter_mode: FilterMode::All,
        query_time_ms: elapsed,
        total_count: row_count,
    };

    let mut window = Window::with(state, update, |state: &AppState| {
        let rows_ref = Rc::clone(&state.filtered_rows);
        let count = rows_ref.len();
        let query_time = state.query_time_ms;
        let total_count = state.total_count;

        // Top Navigation & Stats Bar
        let header_bar = column((
            row((
                row((
                    badge("POLARS + MTK").style(
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
                        "High-Performance Virtualized Table",
                        clr!(0x0F172AFF),
                        18.0,
                        FontWeight::BOLD,
                    ),
                ))
                .style(Style::new().gap(12.0).align_items(AlignItems::Center)),
                row((
                    txt(
                        format!("Query: {query_time:.2} ms"),
                        clr!(0x2563EBFF),
                        12.0,
                        FontWeight::BOLD,
                    ),
                    txt(
                        format!("Showing: {count} / {total_count} rows"),
                        clr!(0x64748BFF),
                        12.0,
                        FontWeight::NORMAL,
                    ),
                ))
                .style(Style::new().gap(16.0).align_items(AlignItems::Center)),
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .justify_content(JustifyContent::SpaceBetween)
                    .align_items(AlignItems::Center),
            ),
            // Filter and Search Toolbar
            row((
                adapt(
                    input_text()
                        .placeholder("Filter by Ticker (e.g. NVDA, BTC) or Sentiment...")
                        .style(
                            Style::new()
                                .width(Size::Fixed(340))
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
                    filter_chip("All Records", FilterMode::All, state.filter_mode),
                    filter_chip("Bullish Only", FilterMode::Bullish, state.filter_mode),
                    filter_chip("Gainers (>0%)", FilterMode::Gainers, state.filter_mode),
                    filter_chip("High Volume", FilterMode::HighVolume, state.filter_mode),
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
            cell_txt("TIMESTAMP", 120, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("TICKER", 100, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("PRICE", 110, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("24H CHANGE", 120, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("VOLUME", 110, clr!(0x475569FF), 12.0, FontWeight::BOLD),
            cell_txt("SENTIMENT", 100, clr!(0x475569FF), 12.0, FontWeight::BOLD),
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

        // Virtualized Table Body
        let vlist = virtual_list((*rows_ref).clone(), 38.0, |idx, item: &MarketRow| {
            let is_even = idx % 2 == 0;
            let row_bg = if is_even {
                clr!(0xFFFFFFFF)
            } else {
                clr!(0xF8FAFCFF)
            };

            let is_positive = item.change_pct >= 0.0;
            let chg_color = if is_positive {
                clr!(0x16A34AFF)
            } else {
                clr!(0xDC2626FF)
            };
            let chg_str = if is_positive {
                format!("+{:.2}%", item.change_pct)
            } else {
                format!("{:.2}%", item.change_pct)
            };

            let sent_bg = match item.sentiment.as_str() {
                "BULLISH" => clr!(0xDCFCE7FF),
                "BEARISH" => clr!(0xFEE2E2FF),
                _ => clr!(0xF1F5F9FF),
            };
            let sent_fg = match item.sentiment.as_str() {
                "BULLISH" => clr!(0x15803DFF),
                "BEARISH" => clr!(0x991B1BFF),
                _ => clr!(0x475569FF),
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
                    120,
                    clr!(0x64748BFF),
                    12.0,
                    FontWeight::NORMAL,
                ),
                cell_txt(
                    item.ticker.clone(),
                    100,
                    clr!(0x0F172AFF),
                    13.0,
                    FontWeight::BOLD,
                ),
                cell_txt(
                    format!("${:.2}", item.price),
                    110,
                    clr!(0x0F172AFF),
                    13.0,
                    FontWeight::NORMAL,
                ),
                cell_txt(chg_str, 120, chg_color, 13.0, FontWeight::BOLD),
                cell_txt(
                    format!("{}", item.volume),
                    110,
                    clr!(0x64748BFF),
                    12.0,
                    FontWeight::NORMAL,
                ),
                badge(item.sentiment.clone()).style(
                    Style::new()
                        .bg_color(sent_bg)
                        .corner_radius(4.0)
                        .set_text_style(
                            TextStyle::new()
                                .color(sent_fg)
                                .font_size(11.0)
                                .font_weight(FontWeight::BOLD),
                        ),
                ),
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .height(Size::Fixed(38))
                    .min_height(38.0)
                    .max_height(38.0)
                    .flex_shrink(0.0)
                    .flex_grow(0.0)
                    .padding_xy(16.0, 0.0)
                    .align_items(AlignItems::Center)
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
            .with_title("MTK + Polars Virtualized Market Streamer (100,000 Rows)")
            .with_size((960, 720).into()),
    );
}
