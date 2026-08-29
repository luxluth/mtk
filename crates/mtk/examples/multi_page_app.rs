use mtk::animation::Curve;
use mtk::style::{AlignItems, JustifyContent, PositionStrategy, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::transition::Transition;
use mtk::ui::widgets::{button, column, router, row, text};
use mtk::ui::{View, ViewStyleExt};
use mtk::windowing::{Window, WindowAttributes, WindowDimension};
use mtk::{clr, rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    P1,
    P2,
    P3,
    P4,
    P5,
}

impl Page {
    fn number(self) -> usize {
        match self {
            Page::P1 => 1,
            Page::P2 => 2,
            Page::P3 => 3,
            Page::P4 => 4,
            Page::P5 => 5,
        }
    }

    fn from_number(n: usize) -> Self {
        match n {
            1 => Page::P1,
            2 => Page::P2,
            3 => Page::P3,
            4 => Page::P4,
            _ => Page::P5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub current_page: Page,
    pub history: Vec<Page>,
}

#[derive(Clone, Debug)]
pub enum AppMsg {
    GoTo(Page),
    GoBack,
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::GoTo(page) => {
            if state.current_page != page {
                state.history.push(state.current_page);
                state.current_page = page;
            }
        }
        AppMsg::GoBack => {
            if let Some(prev) = state.history.pop() {
                state.current_page = prev;
            }
        }
    }
}

fn render_page(num: usize, state: &AppState) -> impl View<AppState, Message = AppMsg> + use<> {
    let bg_colors = [
        rgb!(238, 242, 255),
        rgb!(254, 242, 242),
        rgb!(240, 253, 244),
        rgb!(254, 249, 195),
        rgb!(245, 243, 255),
    ];
    let accent_colors = [
        rgb!(79, 70, 229),
        rgb!(220, 38, 38),
        rgb!(22, 163, 74),
        rgb!(202, 138, 4),
        rgb!(147, 51, 234),
    ];

    let bg_color = bg_colors[(num - 1) % bg_colors.len()];
    let accent = accent_colors[(num - 1) % accent_colors.len()];

    let mut buttons = Vec::new();
    for i in 1..=5 {
        let is_current = i == num;
        let btn = button(format!("Goto({i})"))
            .on_click(AppMsg::GoTo(Page::from_number(i)))
            .style(if is_current {
                Style::new()
                    .padding_xy(16.0, 10.0)
                    .bg_color(accent)
                    .corner_radius(8.0)
                    .set_text_style(TextStyle {
                        color: clr!(white),
                        font_weight: FontWeight::BOLD,
                        font_size: 14.0,
                        ..Default::default()
                    })
            } else {
                Style::new()
                    .padding_xy(16.0, 10.0)
                    .bg_color(clr!(white))
                    .border(1.0, rgb!(203, 213, 225))
                    .corner_radius(8.0)
                    .set_text_style(TextStyle {
                        color: rgb!(51, 65, 85),
                        font_weight: FontWeight::MEDIUM,
                        font_size: 14.0,
                        ..Default::default()
                    })
            });
        buttons.push(btn);
    }

    if !state.history.is_empty() {
        buttons.push(
            button("Go Back").on_click(AppMsg::GoBack).style(
                Style::new()
                    .padding_xy(16.0, 10.0)
                    .bg_color(rgb!(241, 245, 249))
                    .border(1.0, rgb!(148, 163, 184))
                    .corner_radius(8.0)
                    .set_text_style(TextStyle {
                        color: rgb!(15, 23, 42),
                        font_weight: FontWeight::SEMI_BOLD,
                        font_size: 14.0,
                        ..Default::default()
                    }),
            ),
        );
    }

    column((
        text(format!("Page #{num}")).style(Style::new().set_text_style(TextStyle {
            font_size: 48.0,
            font_weight: FontWeight::BOLD,
            color: accent,
            ..Default::default()
        })),
        row(buttons).style(Style::new().gap(10.0)),
    ))
    .style(
        Style::new()
            .position(PositionStrategy::Absolute {
                top: 0.0,
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
            })
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .bg_color(bg_color)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .gap(24.0),
    )
}

fn app(state: &AppState) -> impl View<AppState, Message = AppMsg> + use<> {
    router(
        state.current_page,
        render_page(state.current_page.number(), state),
    )
    .transition(Transition::Fade {
        duration_ms: 220.0,
        curve: Curve::ease_out(),
    })
}

fn main() {
    let initial_state = AppState {
        current_page: Page::P1,
        history: Vec::new(),
    };

    let mut window = Window::with(initial_state, update, app);

    #[cfg(feature = "debugger")]
    window.enable_terminal_debugger();

    window.present_with(
        WindowAttributes::default()
            .with_decorations(true)
            .with_title("MTK - Multi-Page Transition Demo")
            .with_size(WindowDimension {
                width: 900,
                height: 600,
            }),
    );
}
