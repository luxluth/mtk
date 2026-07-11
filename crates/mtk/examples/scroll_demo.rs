use mtk::clr;
use mtk::colors::Color;
use mtk::style::{Size, Style};
use mtk::ui::ViewStyleExt;
use mtk::ui::memoize::memoize;
use mtk::ui::widgets::{ScrollAxis, ScrollOffset, column, scroll_view, text};
use mtk::windowing::{Window, WindowAttributes};
use std::rc::Rc;

struct AppState {
    rows: Vec<Rc<RowData>>,
}

#[derive(Clone, PartialEq, Eq)]
struct RowData {
    label: String,
    color: Color,
}

fn update(_state: &mut AppState, _msg: ()) {}

fn main() {
    let mut initial_rows = Vec::new();
    let colors = [Color::blue, Color::red, Color::green, Color::ll_blue];

    for i in 1..=100 {
        let color = colors[(i % colors.len()) as usize];

        initial_rows.push(Rc::new(RowData {
            color,
            label: format!("row({i}) {color:?}"),
        }));
    }

    let state = AppState { rows: initial_rows };

    let mut window = Window::with(state, update, |state: &AppState| {
        let mut items = Vec::new();

        for row_rc in &state.rows {
            items.push(memoize(Rc::clone(row_rc), |data| {
                text(data.label.clone()).style(Style::new().padding(20.0).bg_color(data.color))
            }));
        }

        scroll_view(column(items).style(Style::new().gap(10.0).padding(10.0)))
            .axis(ScrollAxis::Vertical)
            .start_offset_y(ScrollOffset::Percent(1.))
            // .no_scrollbar()
            .style(
                Style::new()
                    .width(Size::Percent(1.))
                    .height(Size::Percent(1.))
                    .bg_color(clr!(white)),
            )
    });

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK Scroll Demo")
            .with_resizable(false)
            .with_size((800, 600).into()),
    );
}
