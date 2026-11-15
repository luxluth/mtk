use mtk::style::{AlignItems, Edges, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::widgets::{action_buttons, button, column, csd_frame, row, spacer, text};
use mtk::ui::{OsDraggableExt, View, ViewStyleExt};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{Command, clr, rgb, rgba};

#[derive(Clone, Debug)]
pub struct DemoState {
    pub click_count: u32,
    pub status: String,
}

#[derive(Clone, Debug)]
pub enum DemoMsg {
    Increment,
    Reset,
}

fn update(state: &mut DemoState, msg: DemoMsg) -> Command<DemoMsg> {
    match msg {
        DemoMsg::Increment => {
            state.click_count += 1;
            state.status = format!("Clicked {} times", state.click_count);
            Command::none()
        }
        DemoMsg::Reset => {
            state.click_count = 0;
            state.status = "Counter reset".into();
            Command::none()
        }
    }
}

fn app(state: &DemoState) -> impl View<DemoState, Message = DemoMsg> + use<> {
    let text_primary = rgb!(24, 24, 24);
    let text_secondary = rgb!(148, 163, 184);

    // 1. Top custom header with draggable region and ActionButtons placed on the right
    let header = row((
        // Leading logo and title
        row((
            text("+").style(
                Style::new()
                    .padding(Edges::all(0.0).right(8.0))
                    .set_text_style(TextStyle {
                        font_size: 15.0,
                        color: rgb!(99, 102, 241),
                        ..Default::default()
                    }),
            ),
            text("Workstation Audio Hub").style(Style::new().set_text_style(TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::SEMI_BOLD,
                color: text_primary,
                ..Default::default()
            })),
        ))
        .style(Style::new().align_items(AlignItems::Center)),
        // Flexible center spacer (also draggable to move window)
        spacer(),
        action_buttons().dominant_color(clr!(black)),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Fixed(44))
            .padding(Edges::symmetric(14.0, 0.0))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::SpaceBetween)
            .border_bottom(1.0, rgba!(255, 255, 255, 15)),
    )
    .os_draggable(); // Window draggable region with double-click maximize!

    // 2. Application body content
    let body = column((
        text("Modern Composable CSD").style(
            Style::new().set_text_style(TextStyle {
                font_size: 22.0,
                font_weight: FontWeight::BOLD,
                color: text_primary,
                ..Default::default()
            }),
        ),
        text("The entire UI is wrapped in `csd_frame(body)`, allowing custom shadows, borders, and corner radii. The `action_buttons()` component can be composed wherever you like.")
            .style(
                Style::new().width(Size::Fill).set_text_style(TextStyle {
                    font_size: 14.0,
                    color: text_secondary,
                    wrap: true,
                    ..Default::default()
                }),
            ),
        row((
            button("Increment Counter").on_click(DemoMsg::Increment),
            button("Reset").secondary().on_click(DemoMsg::Reset),
        ))
        .style(Style::new().gap(12.0)),
        column((
            text(format!("Clicks: {}", state.click_count)).style(
                Style::new().set_text_style(TextStyle {
                    font_size: 14.0,
                    font_weight: FontWeight::MEDIUM,
                    color: text_primary,
                    ..Default::default()
                }),
            ),
            text(format!("Status: {}", state.status)).style(
                Style::new().set_text_style(TextStyle {
                    font_size: 13.0,
                    color: rgb!(96, 165, 250),
                    ..Default::default()
                }),
            ),
        ))
        .style(
            Style::new()
                .padding(Edges::all(14.0))
                .bg_color(rgba!(255, 255, 255, 10))
                .border(1.0, clr!(0x222222FF))
                .corner_radius(8.0)
                .gap(6.0),
        ),
    ))
    .style(
        Style::new()
            .padding(Edges::all(24.0))
            .gap(16.0)
            .width(Size::Percent(1.0))
            .flex_grow(1.0),
    );

    let app_layout = column((header, body)).style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0)),
    );

    // 3. Wrap everything in a composable CSD Frame styled with shadows, borders, and rounded corners
    csd_frame(app_layout).bg_color(clr!(white))
}

fn main() {
    let initial_state = DemoState {
        click_count: 0,
        status: "Drag header to move window, or double-click header to maximize.".into(),
    };

    let window = Window::with(initial_state, update, app);

    let attrs = WindowAttributes::new()
        .with_title("MTK Composable CSD Demo")
        .with_size((680, 440).into())
        .with_min_size(Some((480, 320).into()))
        .with_csd(true)
        .with_resizable(true);

    window.present_with(attrs);
}
