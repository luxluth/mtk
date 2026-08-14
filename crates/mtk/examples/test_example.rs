use mtk::clr;
use mtk::style::{Size, Style};
use mtk::ui::ViewStyleExt;
use mtk::ui::widgets::text;
use mtk::windowing::{Window, WindowAttributes};

fn main() {
    let mut window = Window::with(
        (),
        |_state, _msg: ()| {},
        |_state| {
            text("Hello, MTK!").style(
                Style::new()
                    .bg_color(clr!(white))
                    .padding(10.)
                    .width(Size::Percent(1.))
                    .height(Size::Percent(1.)),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("Hello MTK")
        .with_size((400, 200).into());

    window.present_with(attrs);
}
