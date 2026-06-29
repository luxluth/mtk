use mtk::{
    AlignItems, JustifyContent, Lens, Size, clr, rgb, text_property,
    ui::{
        EventKind, LensWrap, Style, View, ViewEventExt, ViewStyleExt,
        style::TextStyle,
        widgets::{column, text},
    },
    windowing::{Window, WindowAttributes},
};

#[derive(Lens)]
struct AppState {
    pub click_count: i32,
    pub dark_mode: bool,
}

fn counter(count: i32) -> impl View<i32> {
    text(format!("总数 {}", count))
        .style(
            Style::new()
                .padding(20.0)
                .border(2.0, rgb!(100, 100, 100))
                .corner_radius(8.0)
                .width(Size::Fit)
                .set_text_style(TextStyle {
                    font_size: 48.0,
                    alignement: text_property::Align::Center,
                    line_height: 48.,
                    attrs: text_property::AttrsOwned::new(
                        &text_property::Attrs::new()
                            .color(clr!(ll_blue).into())
                            .family(text_property::Family::Name("IosevkaTerm NF")),
                    ),
                    ..Default::default()
                }),
        )
        .on_event(EventKind::Click, |s: &mut i32| *s += 1)
}

// Another generic component for toggling a boolean
fn toggle(is_on: bool) -> impl View<bool> {
    text(if is_on {
        "深色模式: ON".to_string()
    } else {
        "深色模式: OFF".to_string()
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
                alignement: text_property::Align::Center,
                attrs: text_property::AttrsOwned::new(
                    &&text_property::Attrs::new()
                        .color(if is_on {
                            clr!(white).into()
                        } else {
                            clr!(black).into()
                        })
                        .weight(text_property::Weight::BOLD)
                        .family(text_property::Family::Name("IosevkaTerm NF")),
                ),
                ..Default::default()
            }),
    )
    .on_event(EventKind::Click, |s: &mut bool| *s = !*s)
}

fn main() {
    env_logger::init();

    let state = AppState {
        click_count: 0,
        dark_mode: false,
    };

    let mut window = Window::with(state, |state: &mut AppState| {
        let bg = if state.dark_mode {
            rgb!(20, 20, 20)
        } else {
            rgb!(240, 240, 240)
        };

        column((
            LensWrap::new(counter(state.click_count), AppState::click_count),
            LensWrap::new(toggle(state.dark_mode), AppState::dark_mode),
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
            .with_title("Lenses Example".to_string())
            .with_size((800, 600).into()),
    );
}
