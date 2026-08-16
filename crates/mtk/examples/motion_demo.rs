use mtk::animation::{Curve, Keyframes, Repeat, Spring};
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle, VerticalAlignment};
use mtk::ui::{
    EventKind, ViewEventExt, ViewStyleExt,
    widgets::{column, row, text},
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{clr, rgb, rgba};

#[derive(Clone)]
struct State {
    click_count: u32,
    is_active: bool,
}

#[derive(Clone, Debug)]
enum Message {
    Increment,
    ToggleActive,
}

fn glassmorphic(s: Style) -> Style {
    s.bg_color(rgba!(255, 255, 255, 230))
        .border(1.5, rgba!(218, 226, 238, 240))
        .corner_radius(16.0)
        .padding(20.0)
        .shadow(rgba!(100, 116, 139, 50), 16.0, 0.25)
}

fn card_panel(s: Style) -> Style {
    s.bg_color(rgba!(255, 255, 255, 245))
        .border(1.5, rgb!(226, 232, 240))
        .corner_radius(14.0)
        .padding(20.0)
        .shadow(rgba!(148, 163, 184, 40), 14.0, 0.2)
}

fn bouncy_btn(s: Style) -> Style {
    s.corner_radius(10.0)
        .padding_xy(18.0, 10.0)
        .bg_color(rgb!(79, 70, 229))
        .on_hover(|btn| {
            btn.bg_color(rgb!(99, 102, 241))
                .scale(1.06)
                .shadow(rgba!(99, 102, 241, 100), 10.0, 0.4)
        })
        .on_active(|btn| btn.scale(0.94).bg_color(rgb!(67, 56, 202)))
        .transition_all(200.0, Curve::spring(Spring::bouncy()))
}

fn main() {
    let initial_state = State {
        click_count: 0,
        is_active: false,
    };

    // Continuous pulsing beacon keyframe timeline
    let pulse_beacon = Keyframes::new()
        .keyframe(0.0, Style::new().scale(1.0).opacity(0.))
        .keyframe(0.5, Style::new().scale(1.3).opacity(1.))
        .keyframe(1.0, Style::new().scale(1.0).opacity(0.))
        .duration_ms(1400.0)
        .repeat(Repeat::PingPong)
        .curve(Curve::ease_in_out());

    let mut window = Window::with(
        initial_state,
        |state, msg: Message| match msg {
            Message::Increment => state.click_count += 1,
            Message::ToggleActive => state.is_active = !state.is_active,
        },
        move |state| {
            let is_active = state.is_active;
            let count_text = format!("Clicks: {}", state.click_count);

            column((
                // Header section
                row((
                    // Pulsing live status beacon
                    text("●")
                        .style(Style::new().padding(4.0).set_text_style(TextStyle {
                            font_size: 18.0,
                            color: rgb!(16, 185, 129),
                            ..Default::default()
                        }))
                        .animate_keyframes(pulse_beacon.clone()),
                    text("MTK Motion & Composable Styling Engine").style(
                        Style::new().padding(4.0).set_text_style(TextStyle {
                            font_size: 22.0,
                            color: rgb!(15, 23, 42),
                            ..Default::default()
                        }),
                    ),
                ))
                .style(
                    Style::new()
                        .gap(10.0)
                        .align_items(AlignItems::Center)
                        .padding_xy(0.0, 8.0),
                ),
                // Card 1: Composable Mixins & Hover Transitions
                column((
                    text("1. Reusable Mixin + Spring Transitions").style(
                        Style::new().padding(2.0).set_text_style(TextStyle {
                            font_size: 16.0,
                            color: rgb!(30, 41, 59),
                            ..Default::default()
                        }),
                    ),
                    text("Hover and press the button to observe momentum spring scaling.").style(
                        Style::new().padding(2.0).set_text_style(TextStyle {
                            font_size: 13.0,
                            color: rgb!(100, 116, 139),
                            ..Default::default()
                        }),
                    ),
                    row((
                        text("⚡ Bouncy Spring Button")
                            .style(Style::new().apply(bouncy_btn).set_text_style(TextStyle {
                                font_size: 14.0,
                                color: clr!(white),
                                vertical_alignment: VerticalAlignment::Center,
                                ..Default::default()
                            }))
                            .on_event(EventKind::Click, |_| Some(Message::Increment)),
                        text(count_text).style(
                            Style::new()
                                .padding_xy(12.0, 6.0)
                                .corner_radius(8.0)
                                .bg_color(rgb!(241, 245, 249))
                                .border(1.0, rgb!(226, 232, 240))
                                .set_text_style(TextStyle {
                                    font_size: 14.0,
                                    color: rgb!(51, 65, 85),
                                    vertical_alignment: VerticalAlignment::Center,
                                    ..Default::default()
                                }),
                        ),
                    ))
                    .style(Style::new().gap(12.0).align_items(AlignItems::Center)),
                ))
                .style(
                    Style::new()
                        .apply(glassmorphic)
                        .gap(10.0)
                        .width(Size::Fixed(640))
                        .on_hover(|c| {
                            c.border(1.5, rgb!(99, 102, 241)).shadow(
                                rgba!(99, 102, 241, 70),
                                18.0,
                                0.3,
                            )
                        })
                        .transition_all(250.0, Curve::ease_out()),
                ),
                // Card 2: Conditional Styling (.when) & Dynamic Toggle
                column((
                    text("2. Conditional Styling (.when) & Dynamic State").style(
                        Style::new().padding(2.0).set_text_style(TextStyle {
                            font_size: 16.0,
                            color: rgb!(30, 41, 59),
                            ..Default::default()
                        }),
                    ),
                    text("Click to toggle dynamic conditional overrides:").style(
                        Style::new().padding(2.0).set_text_style(TextStyle {
                            font_size: 13.0,
                            color: rgb!(100, 116, 139),
                            ..Default::default()
                        }),
                    ),
                    text(if is_active {
                        "Status: ACTIVE [ONLINE]"
                    } else {
                        "Status: IDLE [STANDBY]"
                    })
                    .style(
                        Style::new()
                            .padding_xy(20.0, 10.0)
                            .corner_radius(8.0)
                            .bg_color(rgb!(241, 245, 249))
                            .border(1.0, rgb!(203, 213, 225))
                            .when(is_active, |s| {
                                s.bg_color(rgb!(16, 185, 129))
                                    .border(1.0, rgb!(16, 185, 129))
                                    .shadow(rgba!(16, 185, 129, 90), 12.0, 0.4)
                            })
                            .on_hover(|s| s.scale(1.05))
                            .on_active(|s| s.scale(0.95))
                            .set_text_style(TextStyle {
                                font_size: 14.0,
                                color: if is_active {
                                    clr!(white)
                                } else {
                                    rgb!(71, 85, 105)
                                },
                                vertical_alignment: VerticalAlignment::Center,
                                ..Default::default()
                            })
                            .transition_all(200.0, Curve::spring(Spring::bouncy())),
                    )
                    .on_event(EventKind::Click, |_| Some(Message::ToggleActive)),
                ))
                .style(
                    Style::new()
                        .apply(card_panel)
                        .gap(10.0)
                        .width(Size::Fixed(640))
                        .when(is_active, |s| {
                            s.border(1.5, rgb!(16, 185, 129)).shadow(
                                rgba!(16, 185, 129, 60),
                                16.0,
                                0.25,
                            )
                        })
                        .transition_all(300.0, Curve::ease_out()),
                ),
            ))
            .style(
                Style::new()
                    .gap(18.0)
                    .padding(32.0)
                    .bg_color(rgb!(243, 246, 250))
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::Center),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK Motion & Styling Engine Showcase")
        .with_size((720, 560).into());

    window.present_with(attrs);
}
