use mtk::colors::Color;
use mtk::rgb;
use mtk::style::{Size, Style};
use mtk::ui::ViewStyleExt;
use mtk::ui::widgets::{ScrollAxis, ScrollOffset, column, scroll_view, text};
use mtk::windowing::{Window, WindowAttributes};

fn update(_state: &mut (), _msg: ()) {}

fn main() {
    let mut window = Window::with((), update, |()| {
        let mut items = Vec::new();
        let colors = [Color::blue, Color::red, Color::green];

        for i in 1..=10_000 {
            let color = colors[(i % 3) as usize];
            items.push(
                text(format!("Item {}", i)).style(Style::new().padding(20.0).bg_color(color)),
            );
        }

        scroll_view(column(items).style(Style::new().gap(10.0).padding(10.0)))
            .axis(ScrollAxis::Vertical)
            .start_offset_y(ScrollOffset::Percent(1.))
            .style(
                Style::new()
                    .width(Size::Percent(1.))
                    .height(Size::Percent(1.))
                    .bg_color(rgb!(50, 50, 50)),
            )
    });

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK Scroll Demo")
            .with_resizable(false)
            .with_size((800, 600).into()),
    );
}
