use mtk::{
    AlignItems, JustifyContent, Overflow, Size, Style, TextStyle, clr, rgb,
    ui::{
        View, ViewStyleExt,
        widgets::{column, row, text},
    },
    windowing::{Window, WindowAttributes},
};

struct AppState {}

enum AppMsg {}

fn update(_state: &mut AppState, _msg: AppMsg) {}

fn app(_state: &AppState) -> impl View<AppState, Message = AppMsg> + use<> {
    column((
        text("Seamless Scroll Everywhere Demo").style(Style::new().set_text_style(TextStyle {
            font_size: 26.0,
            ..Default::default()
        })),
        text("1. Vertical Intrinsic Scroll:").style(Style::new().set_text_style(TextStyle {
            font_size: 16.0,
            ..Default::default()
        })),
        // 1. Vertical scrolling column
        column((
            text("Box Item 1 - Scroll me!").style(item_style()),
            text("Box Item 2 - Scroll me!").style(item_style()),
            text("Box Item 3 - Scroll me!").style(item_style()),
            text("Box Item 4 - Scroll me!").style(item_style()),
            text("Box Item 5 - Scroll me!").style(item_style()),
            text("Box Item 6 - Scroll me!").style(item_style()),
            text("Box Item 7 - Scroll me!").style(item_style()),
            text("Box Item 8 - Scroll me!").style(item_style()),
        ))
        .style(
            Style::new()
                .width(Size::Fixed(400))
                .height(Size::Fixed(180))
                .gap(10.0)
                .padding(10.0)
                .bg_color(rgb!(245, 245, 250))
                .corner_radius(8.0)
                .overflow(Overflow::Scroll),
        ),
        text("2. Horizontal Intrinsic Scroll:").style(Style::new().set_text_style(TextStyle {
            font_size: 16.0,
            ..Default::default()
        })),
        // 2. Horizontal scrolling row
        row((
            text("Card 1").style(h_item_style()),
            text("Card 2").style(h_item_style()),
            text("Card 3").style(h_item_style()),
            text("Card 4").style(h_item_style()),
            text("Card 5").style(h_item_style()),
            text("Card 6").style(h_item_style()),
            text("Card 7").style(h_item_style()),
            text("Card 8").style(h_item_style()),
        ))
        .style(
            Style::new()
                .width(Size::Fixed(400))
                .height(Size::Fixed(80))
                .gap(10.0)
                .padding(10.0)
                .bg_color(rgb!(250, 245, 240))
                .corner_radius(8.0)
                .overflow(Overflow::Scroll),
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

fn item_style() -> Style {
    Style::new()
        .width(Size::Percent(1.0))
        .height(Size::Fixed(40))
        .padding(8.0)
        .bg_color(rgb!(220, 230, 255))
        .corner_radius(6.0)
        .set_text_style(TextStyle {
            font_size: 16.0,
            ..Default::default()
        })
}

fn h_item_style() -> Style {
    Style::new()
        .width(Size::Fixed(110))
        .height(Size::Percent(1.0))
        .padding(8.0)
        .bg_color(rgb!(255, 230, 210))
        .corner_radius(6.0)
        .set_text_style(TextStyle {
            font_size: 16.0,
            ..Default::default()
        })
}

fn main() {
    let state = AppState {};

    let mut window = Window::with(state, update, app);
    window.present_with(
        WindowAttributes::default()
            .with_decorations(true)
            .with_title("MTK Seamless Scroll Demo")
            .with_size((800, 600).into()),
    );
}
