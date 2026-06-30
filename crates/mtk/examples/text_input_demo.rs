use mtk::{
    AlignItems, JustifyContent, Lens, Size, Style, TextStyle, clr, rgb,
    ui::{
        LensWrap, View, ViewStyleExt,
        widgets::{column, text, text_input},
    },
    windowing::{Window, WindowAttributes},
};

#[derive(Lens)]
struct AppState {
    pub username: String,
}

fn app(state: &mut AppState) -> impl View<AppState> + use<> {
    column((
        text("Enter your username:").style(Style::new().width(Size::Fit).set_text_style(
            TextStyle {
                font_size: 24.0,
                ..Default::default()
            },
        )),
        LensWrap::new(
            text_input().style(
                Style::new()
                    .width(Size::Fixed(300))
                    .height(Size::Fixed(40))
                    .padding(10.0)
                    .border(2.0, rgb!(100, 100, 255))
                    .corner_radius(8.0)
                    .overflow(mtk::Overflow::Hidden),
            ),
            AppState::username,
        ),
        LensWrap::new(
            text_input().style(
                Style::new()
                    .width(Size::Fixed(300))
                    .height(Size::Fixed(40))
                    .padding(10.0)
                    .border(2.0, rgb!(100, 255, 100))
                    .corner_radius(8.0)
                    .overflow(mtk::Overflow::Hidden),
            ),
            AppState::username,
        ),
        text(format!("Hello, {}!", state.username)).style(
            Style::new()
                .width(Size::Fit)
                .padding(20.0)
                .set_text_style(TextStyle {
                    font_size: 24.0,
                    wrap: true,
                    ..Default::default()
                }),
        ),
    ))
    .style(
        Style::new()
            .gap(10.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .width(Size::Percent(1.))
            .height(Size::Percent(1.))
            .bg_color(clr!(white)),
    )
}

fn main() {
    let state = AppState {
        username: "Luthor".to_string(),
    };

    let mut window = Window::with(state, app);

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK Text Input Demo")
            .with_size((800, 600).into()),
    );
}
