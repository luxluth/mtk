use mtk::animation::{Curve, Spring};
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle, VerticalAlignment};
use mtk::text_property::{Alignment, FontWeight};
use mtk::ui::{
    EventKind, ViewEventExt, ViewStyleExt,
    widgets::{column, row, text},
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{clr, rgb, rgba};

#[derive(Clone)]
struct State {
    vibrancy: f32,
    glass_theme: GlassTheme,
    panel_offset_x: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GlassTheme {
    LightGlass,
    DarkAcrylic,
    VibrantTeal,
}

#[derive(Clone, Debug)]
enum Message {
    SetVibrancy(f32),
    SetTheme(GlassTheme),
    MoveLeft,
    MoveRight,
}

fn button_style(s: Style) -> Style {
    s.padding_xy(12.0, 7.0)
        .corner_radius(8.0)
        .bg_color(rgba!(255, 255, 255, 200))
        .border(1.0, rgba!(203, 213, 225, 200))
        .shadow(rgba!(15, 23, 42, 12), 4.0, 0.1)
        .on_hover(|btn| btn.bg_color(rgba!(255, 255, 255, 255)).scale(1.03))
        .on_active(|btn| btn.scale(0.97).bg_color(rgba!(241, 245, 249, 255)))
        .transition_all(150.0, Curve::spring(Spring::bouncy()))
}

fn active_button_style(s: Style) -> Style {
    s.padding_xy(12.0, 7.0)
        .corner_radius(8.0)
        .bg_color(rgb!(79, 70, 229))
        .border(1.0, rgb!(67, 56, 202))
        .shadow(rgba!(79, 70, 229, 60), 8.0, 0.3)
        .on_hover(|btn| btn.bg_color(rgb!(99, 102, 241)).scale(1.03))
        .on_active(|btn| btn.scale(0.97).bg_color(rgb!(67, 56, 202)))
        .transition_all(150.0, Curve::spring(Spring::bouncy()))
}

fn main() {
    let initial_state = State {
        vibrancy: 0.65,
        glass_theme: GlassTheme::LightGlass,
        panel_offset_x: 0.0,
    };

    let mut window = Window::with(
        initial_state,
        |state, msg: Message| match msg {
            Message::SetVibrancy(v) => state.vibrancy = v.clamp(0.0, 1.0),
            Message::SetTheme(t) => {
                state.glass_theme = t;
                match t {
                    GlassTheme::LightGlass => state.vibrancy = 0.65,
                    GlassTheme::DarkAcrylic => state.vibrancy = 0.75,
                    GlassTheme::VibrantTeal => state.vibrancy = 0.85,
                }
            }
            Message::MoveLeft => state.panel_offset_x = (state.panel_offset_x - 50.0).max(-180.0),
            Message::MoveRight => state.panel_offset_x = (state.panel_offset_x + 50.0).min(180.0),
        },
        |state| {
            // Glass card appearance based on theme
            let (glass_tint, glass_border, body_text_color, title_text_color) =
                match state.glass_theme {
                    GlassTheme::LightGlass => (
                        rgba!(255, 255, 255, 80),
                        rgba!(255, 255, 255, 220),
                        rgb!(30, 41, 59),
                        rgb!(15, 23, 42),
                    ),
                    GlassTheme::DarkAcrylic => (
                        rgba!(15, 23, 42, 160),
                        rgba!(255, 255, 255, 45),
                        rgb!(226, 232, 240),
                        rgb!(255, 255, 255),
                    ),
                    GlassTheme::VibrantTeal => (
                        rgba!(13, 148, 136, 110),
                        rgba!(204, 251, 241, 180),
                        rgb!(240, 253, 250),
                        rgb!(255, 255, 255),
                    ),
                };

            let is_light = state.glass_theme == GlassTheme::LightGlass;
            let is_dark = state.glass_theme == GlassTheme::DarkAcrylic;
            let is_teal = state.glass_theme == GlassTheme::VibrantTeal;

            column((
                // TOP BAR: Title & Summary
                row((
                    column((
                        text("Blur Showcase").style(
                            Style::new().set_text_style(TextStyle {
                                font_size: 22.0,
                                color: rgb!(15, 23, 42),
                                font_weight: FontWeight::BOLD,
                                ..Default::default()
                            }),
                        ),
                        text("High-performance compute texture pyramid downsampling & screen-space vibrancy").style(
                            Style::new().set_text_style(TextStyle {
                                font_size: 13.0,
                                color: rgb!(100, 116, 139),
                                ..Default::default()
                            }),
                        ),
                    )).style(Style::new().gap(2.0)),

                    // Theme selector buttons
                    row((
                        text("Light Glass").style(
                            Style::new()
                                .apply(if is_light { active_button_style } else { button_style })
                                .set_text_style(TextStyle {
                                    font_size: 12.0,
                                    color: if is_light { clr!(white) } else { rgb!(51, 65, 85) },
                                    font_weight: FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }),
                        )
                        .on_event(EventKind::Click, |_| Some(Message::SetTheme(GlassTheme::LightGlass))),
                        text("Dark Acrylic").style(
                            Style::new()
                                .apply(if is_dark { active_button_style } else { button_style })
                                .set_text_style(TextStyle {
                                    font_size: 12.0,
                                    color: if is_dark { clr!(white) } else { rgb!(51, 65, 85) },
                                    font_weight: FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }),
                        )
                        .on_event(EventKind::Click, |_| Some(Message::SetTheme(GlassTheme::DarkAcrylic))),
                        text("Teal Vibrancy").style(
                            Style::new()
                                .apply(if is_teal { active_button_style } else { button_style })
                                .set_text_style(TextStyle {
                                    font_size: 12.0,
                                    color: if is_teal { clr!(white) } else { rgb!(51, 65, 85) },
                                    font_weight: FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }),
                        )
                        .on_event(EventKind::Click, |_| Some(Message::SetTheme(GlassTheme::VibrantTeal))),
                    ))
                    .style(Style::new().gap(8.0)),
                ))
                .style(
                    Style::new()
                        .width(Size::Percent(1.0))
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::SpaceBetween),
                ),

                // ARTWORK LAYER: 3 Vibrant High-Contrast Cards with Text and Badges
                row((
                    // Card 1: Sunset Orange
                    column((
                        text("CARD 01").style(
                            Style::new()
                                .padding_xy(8.0, 3.0)
                                .corner_radius(4.0)
                                .bg_color(rgba!(255, 255, 255, 80))
                                .set_text_style(TextStyle {
                                    font_size: 11.0,
                                    color: clr!(white),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }),
                        ),
                        text("Sunset Ember").style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .set_text_style(TextStyle {
                                    font_size: 20.0,
                                    color: clr!(white),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }),
                        ),
                        text("High-frequency contrast artwork layer designed to evaluate real-time Gaussian bokeh dispersion.").style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .set_text_style(TextStyle {
                                    font_size: 12.5,
                                    color: rgba!(255, 255, 255, 230),
                                    wrap: true,
                                    ..Default::default()
                                }),
                        ),
                    ))
                    .style(
                        Style::new()
                            .width(Size::Fixed(270))
                            .height(Size::Fixed(260))
                            .padding(20.0)
                            .gap(10.0)
                            .corner_radius(20.0)
                            .bg_color(rgb!(249, 115, 22))
                            .shadow(rgba!(249, 115, 22, 140), 32.0, 0.4),
                    ),

                    // Card 2: Indigo Luminescence
                    column((
                        text("CARD 02").style(
                            Style::new()
                                .padding_xy(8.0, 3.0)
                                .corner_radius(4.0)
                                .bg_color(rgba!(255, 255, 255, 80))
                                .set_text_style(TextStyle {
                                    font_size: 11.0,
                                    color: clr!(white),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }),
                        ),
                        text("Deep Indigo").style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .set_text_style(TextStyle {
                                    font_size: 20.0,
                                    color: clr!(white),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }),
                        ),
                        text("Dual Kawase 5-pass compute downsample and tent upsample executing in single frame encoder.").style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .set_text_style(TextStyle {
                                    font_size: 12.5,
                                    color: rgba!(255, 255, 255, 230),
                                    wrap: true,
                                    ..Default::default()
                                }),
                        ),
                    ))
                    .style(
                        Style::new()
                            .width(Size::Fixed(270))
                            .height(Size::Fixed(260))
                            .padding(20.0)
                            .gap(10.0)
                            .corner_radius(20.0)
                            .bg_color(rgb!(99, 102, 241))
                            .shadow(rgba!(99, 102, 241, 150), 36.0, 0.45),
                    ),

                    // Card 3: Emerald Radiance
                    column((
                        text("CARD 03").style(
                            Style::new()
                                .padding_xy(8.0, 3.0)
                                .corner_radius(4.0)
                                .bg_color(rgba!(255, 255, 255, 80))
                                .set_text_style(TextStyle {
                                    font_size: 11.0,
                                    color: clr!(white),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }),
                        ),
                        text("Emerald Jade").style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .set_text_style(TextStyle {
                                    font_size: 20.0,
                                    color: clr!(white),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }),
                        ),
                        text("Texture pyramid storage binding blurs across 1/2, 1/4, 1/8 resolutions smoothly.").style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .set_text_style(TextStyle {
                                    font_size: 12.5,
                                    color: rgba!(255, 255, 255, 230),
                                    wrap: true,
                                    ..Default::default()
                                }),
                        ),
                    ))
                    .style(
                        Style::new()
                            .width(Size::Fixed(270))
                            .height(Size::Fixed(260))
                            .padding(20.0)
                            .gap(10.0)
                            .corner_radius(20.0)
                            .bg_color(rgb!(16, 185, 129))
                            .shadow(rgba!(16, 185, 129, 140), 32.0, 0.4),
                    ),
                ))
                .style(
                    Style::new()
                        .width(Size::Percent(1.0))
                        .gap(20.0)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center),
                ),

                // FLOATING FROSTED GLASS WINDOW (Positioned over the scene with live blur & vibrancy)
                row((
                    column((
                        // Window Top Title Bar
                        row((
                            row((
                                text("⯁").style(
                                    Style::new()
                                        .padding_xy(6.0, 3.0)
                                        .corner_radius(6.0)
                                        .bg_color(if is_dark { rgba!(255, 255, 255, 40) } else { rgba!(79, 70, 229, 30) })
                                        .set_text_style(TextStyle {
                                            font_size: 14.0,
                                            color: if is_dark { rgb!(255, 255, 255) } else { rgb!(79, 70, 229) },
                                            font_weight: FontWeight::BOLD,
                                            vertical_alignment: VerticalAlignment::Center,
                                            ..Default::default()
                                        }),
                                ),
                                text("Frosted Acrylic Glass Inspector").style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 16.0,
                                        color: title_text_color,
                                        font_weight: FontWeight::BOLD,
                                        ..Default::default()
                                    }),
                                ),
                            ))
                            .style(Style::new().gap(8.0).align_items(AlignItems::Center)),

                            // Live status badge
                            text(if state.vibrancy > 0.0 { "Blur Active" } else { "Blur Bypassed" }).style(
                                Style::new()
                                    .padding_xy(10.0, 4.0)
                                    .corner_radius(6.0)
                                    .bg_color(if state.vibrancy > 0.0 { rgba!(16, 185, 129, 40) } else { rgba!(100, 116, 139, 30) })
                                    .border(1.0, if state.vibrancy > 0.0 { rgba!(16, 185, 129, 80) } else { rgba!(100, 116, 139, 60) })
                                    .set_text_style(TextStyle {
                                        font_size: 11.5,
                                        color: if state.vibrancy > 0.0 { rgb!(5, 150, 105) } else { rgb!(100, 116, 139) },
                                        font_weight: FontWeight::SEMI_BOLD,
                                        ..Default::default()
                                    }),
                            ),
                        ))
                        .style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .align_items(AlignItems::Center)
                                .justify_content(JustifyContent::SpaceBetween),
                        ),

                        // Explanation text
                        text(
                            "Move this frosted window left or right to inspect the real-time background blurring \
                             over the colorful cards behind it. Adjust the vibrancy stepper to increase or decrease blur intensity."
                        )
                        .style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .set_text_style(TextStyle {
                                    font_size: 13.0,
                                    color: body_text_color,
                                    wrap: true,
                                    ..Default::default()
                                }),
                        ),

                        // Bottom Control Bar
                        row((
                            // Vibrancy Stepper
                            row((
                                text("Blur Vibrancy:").style(Style::new().set_text_style(TextStyle {
                                    font_size: 13.0,
                                    color: title_text_color,
                                    font_weight: FontWeight::SEMI_BOLD,
                                    vertical_alignment: VerticalAlignment::Center,
                                    ..Default::default()
                                })),
                                text(" - ").style(Style::new().apply(button_style).set_text_style(TextStyle {
                                    font_size: 13.0,
                                    color: rgb!(15, 23, 42),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }))
                                .on_event(EventKind::Click, |s: &State| Some(Message::SetVibrancy(s.vibrancy - 0.15))),
                                text(format!("{:.0}%", state.vibrancy * 100.0)).style(
                                    Style::new()
                                        .width(Size::Fixed(52))
                                        .padding_xy(6.0, 4.0)
                                        .corner_radius(6.0)
                                        .bg_color(if is_dark {
                                            rgba!(255, 255, 255, 30)
                                        } else if is_teal {
                                            rgba!(255, 255, 255, 40)
                                        } else {
                                            rgba!(255, 255, 255, 220)
                                        })
                                        .border(1.0, if is_dark {
                                            rgba!(255, 255, 255, 60)
                                        } else if is_teal {
                                            rgba!(204, 251, 241, 140)
                                        } else {
                                            rgba!(203, 213, 225, 220)
                                        })
                                        .set_text_style(TextStyle {
                                            font_size: 13.0,
                                            color: if is_dark || is_teal {
                                                clr!(white)
                                            } else {
                                                rgb!(15, 23, 42)
                                            },
                                            font_weight: FontWeight::BOLD,
                                            alignment: Alignment::Center,
                                            vertical_alignment: VerticalAlignment::Center,
                                            ..Default::default()
                                        }),
                                ),
                                text(" + ").style(Style::new().apply(button_style).set_text_style(TextStyle {
                                    font_size: 13.0,
                                    color: rgb!(15, 23, 42),
                                    font_weight: FontWeight::BOLD,
                                    ..Default::default()
                                }))
                                .on_event(EventKind::Click, |s: &State| Some(Message::SetVibrancy(s.vibrancy + 0.15))),
                            ))
                            .style(Style::new().gap(6.0).align_items(AlignItems::Center)),

                            // Move Controls
                            row((
                                text("Shift Panel:").style(Style::new().set_text_style(TextStyle {
                                    font_size: 13.0,
                                    color: title_text_color,
                                    font_weight: FontWeight::SEMI_BOLD,
                                    vertical_alignment: VerticalAlignment::Center,
                                    ..Default::default()
                                })),
                                text("◀ Left").style(Style::new().apply(button_style).set_text_style(TextStyle {
                                    font_size: 12.0,
                                    color: rgb!(15, 23, 42),
                                    font_weight: FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }))
                                .on_event(EventKind::Click, |_| Some(Message::MoveLeft)),
                                text("Right ▶").style(Style::new().apply(button_style).set_text_style(TextStyle {
                                    font_size: 12.0,
                                    color: rgb!(15, 23, 42),
                                    font_weight: FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }))
                                .on_event(EventKind::Click, |_| Some(Message::MoveRight)),
                            ))
                            .style(Style::new().gap(6.0).align_items(AlignItems::Center)),
                        ))
                        .style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .align_items(AlignItems::Center)
                                .justify_content(JustifyContent::SpaceBetween),
                        ),
                    ))
                    .style(
                        Style::new()
                            .width(Size::Fixed(720))
                            .padding(22.0)
                            .gap(14.0)
                            .corner_radius(18.0)
                            .bg_color(glass_tint)
                            .border(1.5, glass_border)
                            .shadow(rgba!(15, 23, 42, 40), 28.0, 0.25)
                            .blur(state.vibrancy),
                    ),
                ))
                .style(
                    Style::new()
                        .width(Size::Percent(1.0))
                        .padding_edges(mtk::style::Edges {
                            left: (40.0 + state.panel_offset_x).max(0.0),
                            right: (40.0 - state.panel_offset_x).max(0.0),
                            top: 0.0,
                            bottom: 0.0,
                        })
                        .justify_content(JustifyContent::Center)
                        .transition_all(200.0, Curve::spring(Spring::bouncy())),
                ),
            ))
            .style(
                Style::new()
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .padding(28.0)
                    .gap(20.0)
                    .bg_color(rgb!(241, 245, 249))
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::SpaceBetween),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK Frosted Glass (Dual Kawase Compute Blur)")
        .with_size((900, 720).into());

    window.present_with(attrs);
}
