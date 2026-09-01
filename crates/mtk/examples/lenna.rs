use std::path::PathBuf;

use mtk::clr;
use mtk::image::ObjectFit;
use mtk::rgb;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::{Alignment, FontStyle};
use mtk::ui::ViewStyleExt;
use mtk::ui::widgets::{async_image, column, container, text};
use mtk::windowing::{Window, WindowAttributes};

fn main() {
    let image_path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/Lenna_(test_image).png"
    ));

    let mut window = Window::with(
        (),
        |_state, _msg: ()| {},
        move |_state| {
            container((column((
                async_image(image_path.clone())
                    .fit(ObjectFit::Contain)
                    .style(
                        Style::new()
                            .width(Size::Fixed(400))
                            .height(Size::Fixed(400))
                            .corner_radius(12.0)
                            .border(1.0, rgb!(226, 232, 240))
                            .shadow(rgb!(0, 0, 0), 20.0, 0.25),
                    ),
                text("Image of Lena Forsén used in many image processing experiments.").style(
                    Style::new().set_text_style(
                        TextStyle::new()
                            .font_size(14.0)
                            .font_style(FontStyle::Italic)
                            .color(rgb!(100, 116, 139))
                            .alignment(Alignment::Center),
                    ),
                ),
            ))
            .style(Style::new().align_items(AlignItems::Center).gap(16.0)),))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .height(Size::Percent(1.0))
                    .bg_color(clr!(white))
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::Center),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("Lenna Image Example")
        .with_size((640, 640).into());

    window.present_with(attrs);
}
