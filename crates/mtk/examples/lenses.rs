use mtk::{
    AlignItems, JustifyContent, Lens, Size, Style, TextStyle, clr, rgb, text_property,
    ui::{
        EventKind, View, ViewEventExt, ViewStyleExt,
        adapter::Adapt,
        widgets::{column, text},
    },
    windowing::{Window, WindowAttributes},
};

#[derive(Clone)]
pub enum CounterMsg {
    Increment,
}

fn counter(count: i32) -> impl View<i32, Message = CounterMsg> {
    text(format!("总数 {}", count))
        .style(
            Style::new()
                .padding(20.0)
                .border(2.0, rgb!(100, 100, 100))
                .corner_radius(8.0)
                .width(Size::Fit)
                .set_text_style(TextStyle {
                    font_size: 48.0,
                    alignment: text_property::Alignment::Center,
                    line_height: 48.,
                    color: clr!(ll_blue),
                    font_family: "IosevkaTerm NF".into(),
                    ..Default::default()
                }),
        )
        .on_event(EventKind::Click, |_| Some(CounterMsg::Increment))
}

#[derive(Clone)]
pub enum ToggleMsg {
    Toggle,
}

// Another generic component for toggling a boolean
fn toggle(is_on: bool) -> impl View<bool, Message = ToggleMsg> {
    text(if is_on {
        "深色模式: ✅".to_string()
    } else {
        "深色模式: ⭕".to_string()
    })
    .style(
        Style::new()
            .padding(20.0)
            .bg_color(if is_on {
                rgb!(50, 50, 50)
            } else {
                rgb!(200, 200, 200)
            })
            .corner_radius(8.0)
            .width(Size::Fit)
            .set_text_style(TextStyle {
                font_size: 24.0,
                alignment: text_property::Alignment::Center,
                color: if is_on {
                    clr!(white).into()
                } else {
                    clr!(black).into()
                },
                font_weight: text_property::FontWeight::BOLD,
                font_family: "IosevkaTerm NF".into(),
                ..Default::default()
            }),
    )
    .on_event(EventKind::Click, |_| Some(ToggleMsg::Toggle))
}

#[derive(Clone)]
enum AppMsg {
    CounterEvent(CounterMsg),
    ToggleEvent(ToggleMsg),
}

#[derive(Lens)]
struct AppState {
    pub click_count: i32,
    pub dark_mode: bool,
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::CounterEvent(CounterMsg::Increment) => {
            state.click_count += 1;
        }
        AppMsg::ToggleEvent(ToggleMsg::Toggle) => {
            state.dark_mode = !state.dark_mode;
        }
    }
}

fn main() {
    env_logger::init();

    let state = AppState {
        click_count: 0,
        dark_mode: false,
    };

    let mut window = Window::with(state, update, |state: &AppState| {
        let bg = if state.dark_mode {
            rgb!(20, 20, 20)
        } else {
            rgb!(240, 240, 240)
        };

        column((
            Adapt::new(
                counter(state.click_count),
                AppState::click_count,
                AppMsg::CounterEvent, // Maps CounterMsg -> AppMsg
            ),
            Adapt::new(
                toggle(state.dark_mode),
                AppState::dark_mode,
                AppMsg::ToggleEvent, // Maps ToggleMsg -> AppMsg
            ),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Percent(1.0))
                .justify_content(JustifyContent::Center)
                .align_items(AlignItems::Center)
                .gap(20.0)
                .bg_color(bg),
        )
    });

    window.present_with(
        WindowAttributes::default()
            .with_title("Lenses Example")
            .with_size((800, 600).into()),
    );
}
