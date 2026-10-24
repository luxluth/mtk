use mtk::style::{AlignItems, Edges, FlexDirection, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::{
    Either, ViewEventExt, ViewStyleExt, adaptive,
    widgets::{column, container, row, text},
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{clr, rgb};

#[derive(Clone, Debug)]
struct AppState {
    container_width: f32,
    container_height: f32,
}

#[derive(Clone, Debug)]
enum AppMsg {
    Resized(f32, f32),
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::Resized(w, h) => {
            state.container_width = w;
            state.container_height = h;
        }
    }
}

/// Helper function to create a polished light-themed dashboard stat card.
fn metric_card(
    title: &str,
    value: &str,
    change: &str,
    accent_color: mtk::Color,
) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    container((
        text(title).style(Style::new().set_text_style(TextStyle {
            font_size: 13.0,
            color: rgb!(100, 116, 139), // Slate 500
            font_weight: FontWeight::MEDIUM,
            ..Default::default()
        })),
        text(value).style(Style::new().set_text_style(TextStyle {
            font_size: 26.0,
            color: rgb!(15, 23, 42), // Slate 900
            font_weight: FontWeight::BOLD,
            ..Default::default()
        })),
        text(change).style(Style::new().set_text_style(TextStyle {
            font_size: 12.0,
            color: accent_color,
            font_weight: FontWeight::BOLD,
            ..Default::default()
        })),
    ))
    .style(
        Style::new()
            .flex_grow(1.0)
            .width(Size::Fill)
            .padding(Edges::all(20.0))
            .bg_color(clr!(white))
            .border(1.0, rgb!(226, 232, 240)) // Slate 200
            .corner_radius(12.0)
            .flex_direction(FlexDirection::Column)
            .gap(6.0),
    )
}

fn main() {
    let window = Window::with(
        AppState {
            container_width: 0.0,
            container_height: 0.0,
        },
        update,
        |state| {
            let is_wide = state.container_width > 560.0;
            let mode_label = if is_wide {
                "Desktop (3 Columns)"
            } else {
                "Mobile (Stacked)"
            };

            column((
                // Top Light Header Navigation
                container((row((
                    text("Analytics Overview").style(Style::new().set_text_style(TextStyle {
                        font_size: 18.0,
                        color: rgb!(15, 23, 42),
                        font_weight: FontWeight::BOLD,
                        ..Default::default()
                    })),
                    // Layout status badge
                    container((text(format!(
                        "{mode_label} • {:.0}px × {:.0}px",
                        state.container_width, state.container_height
                    )),))
                    .style(
                        Style::new()
                            .padding(Edges::symmetric(12.0, 5.0))
                            .bg_color(rgb!(241, 245, 249)) // Slate 100
                            .border(1.0, rgb!(203, 213, 225)) // Slate 300
                            .corner_radius(20.0)
                            .set_text_style(TextStyle {
                                font_size: 12.0,
                                color: rgb!(30, 41, 59), // Slate 800
                                font_weight: FontWeight::SEMI_BOLD,
                                ..Default::default()
                            }),
                    ),
                ))
                .style(
                    Style::new()
                        .width(Size::Fill)
                        .justify_content(JustifyContent::SpaceBetween)
                        .align_items(AlignItems::Center),
                ),))
                .style(
                    Style::new()
                        .width(Size::Fill)
                        .padding(Edges::symmetric(24.0, 16.0))
                        .bg_color(clr!(white))
                        .border_bottom(1.0, rgb!(226, 232, 240)),
                ),
                // Responsive Area using container query: adapts children based on its resolved bounds
                adaptive(|bounds| {
                    if bounds.w > 560.0 {
                        // Wide viewport: 3 cards side-by-side in a row
                        Either::Left(
                            row((
                                metric_card(
                                    "Total Revenue",
                                    "$48,250",
                                    "+12.4% vs last month",
                                    rgb!(16, 185, 129),
                                ),
                                metric_card(
                                    "Active Users",
                                    "2,420",
                                    "+8.1% this week",
                                    rgb!(37, 99, 235),
                                ),
                                metric_card(
                                    "Conversion Rate",
                                    "3.42%",
                                    "+1.2% over target",
                                    rgb!(124, 58, 237),
                                ),
                            ))
                            .style(Style::new().width(Size::Fill).gap(16.0)),
                        )
                    } else {
                        // Narrow viewport: 3 cards stacked vertically in a column
                        Either::Right(
                            column((
                                metric_card(
                                    "Total Revenue",
                                    "$48,250",
                                    "+12.4% vs last month",
                                    rgb!(16, 185, 129),
                                ),
                                metric_card(
                                    "Active Users",
                                    "2,420",
                                    "+8.1% this week",
                                    rgb!(37, 99, 235),
                                ),
                                metric_card(
                                    "Conversion Rate",
                                    "3.42%",
                                    "+1.2% over target",
                                    rgb!(124, 58, 237),
                                ),
                            ))
                            .style(Style::new().width(Size::Fill).gap(12.0)),
                        )
                    }
                })
                .style(
                    Style::new()
                        .width(Size::Fill)
                        .flex_grow(1.0)
                        .padding(Edges::all(24.0)),
                )
                // Observable layout resize hook
                .on_resize(|_state, bounds| Some(AppMsg::Resized(bounds.w, bounds.h))),
                // Bottom Hint Bar
                container((text(
                    "Resize this window: the cards switch between 3 columns and stacked mobile mode.",
                ),))
                .style(
                    Style::new()
                        .width(Size::Fill)
                        .padding(Edges::symmetric(24.0, 14.0))
                        .bg_color(clr!(white))
                        .border_top(1.0, rgb!(226, 232, 240))
                        .set_text_style(TextStyle {
                            font_size: 13.0,
                            color: rgb!(100, 116, 139),
                            ..Default::default()
                        }),
                ),
            ))
            .style(
                Style::new()
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .bg_color(rgb!(248, 250, 252)), // Slate 50 clean light background
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK Light Themed Adaptive Demo")
        .with_size((720, 450).into());

    window.present_with(attrs);
}
