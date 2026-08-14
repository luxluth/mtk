use mtk::{
    AlignItems, JustifyContent, Lens, Size, Style, TextStyle, clr, rgb,
    ui::{
        View, ViewAdaptExt, ViewStyleExt,
        widgets::{column, text, text_area},
    },
    windowing::{Window, WindowAttributes},
};

#[derive(Lens)]
struct AppState {
    pub bio: String,
}

enum AppMsg {
    UpdateBio(String),
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::UpdateBio(bio) => {
            state.bio = bio;
        }
    }
}

fn app(state: &AppState) -> impl View<AppState, Message = AppMsg> + use<> {
    column((
        text("Multi-Line Text Area Demo:").style(Style::new().width(Size::Fit).set_text_style(
            TextStyle {
                font_size: 24.0,
                ..Default::default()
            },
        )),
        text_area()
            .style(
                Style::new()
                    .width(Size::Fixed(450))
                    .height(Size::Fixed(200))
                    .padding(12.0)
                    .border(2.0, rgb!(100, 150, 255))
                    .corner_radius(8.0)
                    .bg_color(rgb!(250, 250, 255))
                    .set_text_style(TextStyle {
                        font_size: 18.0,
                        wrap: true,
                        ..Default::default()
                    }),
            )
            .adapt(AppState::bio, AppMsg::UpdateBio),
        text(format!("Live Output:\n{}", state.bio)).style(
            Style::new()
                .width(Size::Fixed(450))
                .padding(12.0)
                .bg_color(rgb!(240, 240, 240))
                .corner_radius(8.0)
                .set_text_style(TextStyle {
                    font_size: 16.0,
                    wrap: true,
                    ..Default::default()
                }),
        ),
    ))
    .style(
        Style::new()
            .gap(16.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .bg_color(clr!(white)),
    )
}

fn main() {
    let state = AppState {
        bio: "Welcome to MTK TextArea!\nPress Enter for a new line.\nUse Arrow keys Up/Down/Left/Right to navigate.".to_string(),
    };

    let mut window = Window::with(state, update, app);

    window.present_with(
        WindowAttributes::default()
            .with_decorations(true)
            .with_title("MTK TextArea Demo")
            .with_size((800, 600).into()),
    );
}
