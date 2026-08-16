use mtk::animation::{Curve, Spring};
use mtk::style::{
    AlignItems, JustifyContent, LineHeight, Size, Style, TextStyle, VerticalAlignment,
};
use mtk::text_property::{Alignment, FontStyle, FontWeight};
use mtk::ui::widgets::{ScrollAxis, scroll_view};
use mtk::ui::{
    EventKind, ViewEventExt, ViewStyleExt,
    widgets::{column, row, text},
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{clr, rgb, rgba};

#[derive(Clone)]
struct State {
    font_size: f32,
    weight: FontWeight,
    is_italic: bool,
    is_strikethrough: bool,
    is_underline: bool,
    alignment: Alignment,
}

#[derive(Clone, Debug)]
enum Message {
    IncreaseSize,
    DecreaseSize,
    SetWeight(FontWeight),
    ToggleItalic,
    ToggleStrikethrough,
    ToggleUnderline,
    SetAlignment(Alignment),
}

fn button_style(s: Style) -> Style {
    s.padding_xy(14.0, 8.0)
        .corner_radius(8.0)
        .bg_color(rgb!(241, 245, 249))
        .border(1.0, rgb!(203, 213, 225))
        .on_hover(|btn| btn.bg_color(rgb!(226, 232, 240)).scale(1.04))
        .on_active(|btn| btn.scale(0.96).bg_color(rgb!(203, 213, 225)))
        .transition_all(150.0, Curve::spring(Spring::bouncy()))
}

fn active_button_style(s: Style) -> Style {
    s.padding_xy(14.0, 8.0)
        .corner_radius(8.0)
        .bg_color(rgb!(79, 70, 229))
        .border(1.0, rgb!(67, 56, 202))
        .shadow(rgba!(79, 70, 229, 60), 8.0, 0.3)
        .on_hover(|btn| btn.bg_color(rgb!(99, 102, 241)).scale(1.04))
        .on_active(|btn| btn.scale(0.96).bg_color(rgb!(67, 56, 202)))
        .transition_all(150.0, Curve::spring(Spring::bouncy()))
}

fn card_panel(s: Style) -> Style {
    s.bg_color(rgba!(255, 255, 255, 245))
        .border(1.5, rgb!(226, 232, 240))
        .corner_radius(12.0)
        .padding(18.0)
        .shadow(rgba!(148, 163, 184, 30), 12.0, 0.2)
}

fn scroll_card_panel(s: Style) -> Style {
    s.bg_color(rgba!(255, 255, 255, 245))
        .border(1.5, rgb!(226, 232, 240))
        .corner_radius(12.0)
        .shadow(rgba!(148, 163, 184, 30), 12.0, 0.2)
}

fn main() {
    let initial_state = State {
        font_size: 24.0,
        weight: FontWeight::NORMAL,
        is_italic: false,
        is_strikethrough: false,
        is_underline: false,
        alignment: Alignment::Start,
    };

    let mut window = Window::with(
        initial_state,
        |state, msg: Message| match msg {
            Message::IncreaseSize => {
                if state.font_size < 48.0 {
                    state.font_size += 2.0;
                }
            }
            Message::DecreaseSize => {
                if state.font_size > 12.0 {
                    state.font_size -= 2.0;
                }
            }
            Message::SetWeight(w) => state.weight = w,
            Message::ToggleItalic => state.is_italic = !state.is_italic,
            Message::ToggleStrikethrough => state.is_strikethrough = !state.is_strikethrough,
            Message::ToggleUnderline => state.is_underline = !state.is_underline,
            Message::SetAlignment(a) => state.alignment = a,
        },
        |state| {
            let current_size_label = format!("{:.0} px", state.font_size);
            let is_normal = state.weight == FontWeight::NORMAL;
            let is_semi = state.weight == FontWeight::SEMI_BOLD;
            let is_bold = state.weight == FontWeight::BOLD;

            column((
                // Header
                row((
                    text("Aa").style(
                        Style::new()
                            .padding_xy(10.0, 4.0)
                            .corner_radius(8.0)
                            .bg_color(rgb!(79, 70, 229))
                            .set_text_style(TextStyle {
                                font_size: 20.0,
                                color: clr!(white),
                                font_weight: FontWeight::BOLD,
                                ..Default::default()
                            }),
                    ),
                    text("MTK Typography").style(
                        Style::new().padding(4.0).set_text_style(TextStyle {
                            font_size: 22.0,
                            color: rgb!(15, 23, 42),
                            font_weight: FontWeight::BOLD,
                            ..Default::default()
                        }),
                    ),
                ))
                .style(
                    Style::new()
                        .gap(12.0)
                        .align_items(AlignItems::Center)
                        .padding_xy(0.0, 4.0),
                ),

                // Controls Toolbar (Scrollable)
                scroll_view(
                    row((
                        // Font Size Controls
                        row((
                            text("Size:").style(Style::new().padding(4.0).set_text_style(TextStyle {
                                font_size: 14.0,
                                color: rgb!(71, 85, 105),
                                font_weight: FontWeight::SEMI_BOLD,
                                vertical_alignment: VerticalAlignment::Center,
                                ..Default::default()
                            })),
                            text(" - ").style(Style::new().apply(button_style).set_text_style(TextStyle {
                                font_size: 14.0,
                                color: rgb!(30, 41, 59),
                                font_weight: FontWeight::BOLD,
                                ..Default::default()
                            }))
                            .on_event(EventKind::Click, |_| Some(Message::DecreaseSize)),
                            text(current_size_label).style(
                                Style::new()
                                    .width(Size::Fixed(64))
                                    .padding_xy(8.0, 6.0)
                                    .corner_radius(6.0)
                                    .bg_color(rgb!(241, 245, 249))
                                    .border(1.0, rgb!(226, 232, 240))
                                    .set_text_style(TextStyle {
                                        font_size: 14.0,
                                        color: rgb!(15, 23, 42),
                                        font_weight: FontWeight::SEMI_BOLD,
                                        vertical_alignment: VerticalAlignment::Center,
                                        alignment: Alignment::Center,
                                        ..Default::default()
                                    }),
                            ),
                            text(" + ").style(Style::new().apply(button_style).set_text_style(TextStyle {
                                font_size: 14.0,
                                color: rgb!(30, 41, 59),
                                font_weight: FontWeight::BOLD,
                                ..Default::default()
                            }))
                            .on_event(EventKind::Click, |_| Some(Message::IncreaseSize)),
                        ))
                        .style(Style::new().gap(8.0).align_items(AlignItems::Center)),

                        // Weight Controls
                        row((
                            text("Regular").style(
                                Style::new()
                                    .apply(if is_normal { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if is_normal { clr!(white) } else { rgb!(51, 65, 85) },
                                        font_weight: FontWeight::NORMAL,
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::SetWeight(FontWeight::NORMAL))),
                            text("SemiBold").style(
                                Style::new()
                                    .apply(if is_semi { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if is_semi { clr!(white) } else { rgb!(51, 65, 85) },
                                        font_weight: FontWeight::SEMI_BOLD,
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::SetWeight(FontWeight::SEMI_BOLD))),
                            text("Bold").style(
                                Style::new()
                                    .apply(if is_bold { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if is_bold { clr!(white) } else { rgb!(51, 65, 85) },
                                        font_weight: FontWeight::BOLD,
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::SetWeight(FontWeight::BOLD))),
                        ))
                        .style(Style::new().gap(6.0).align_items(AlignItems::Center)),

                        // Style Toggles
                        row((
                            text("Italic").style(
                                Style::new()
                                    .apply(if state.is_italic { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if state.is_italic { clr!(white) } else { rgb!(51, 65, 85) },
                                        font_style: FontStyle::Italic,
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::ToggleItalic)),
                            text("Strike").style(
                                Style::new()
                                    .apply(if state.is_strikethrough { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if state.is_strikethrough { clr!(white) } else { rgb!(51, 65, 85) },
                                        strikethrough: true,
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::ToggleStrikethrough)),
                            text("Underline").style(
                                Style::new()
                                    .apply(if state.is_underline { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if state.is_underline { clr!(white) } else { rgb!(51, 65, 85) },
                                        underline: true,
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::ToggleUnderline)),
                        ))
                        .style(Style::new().gap(6.0).align_items(AlignItems::Center)),

                        // Alignment Toggles
                        row((
                            text("Left").style(
                                Style::new()
                                    .apply(if state.alignment == Alignment::Start { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if state.alignment == Alignment::Start { clr!(white) } else { rgb!(51, 65, 85) },
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::SetAlignment(Alignment::Start))),
                            text("Center").style(
                                Style::new()
                                    .apply(if state.alignment == Alignment::Center { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if state.alignment == Alignment::Center { clr!(white) } else { rgb!(51, 65, 85) },
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::SetAlignment(Alignment::Center))),
                            text("Right").style(
                                Style::new()
                                    .apply(if state.alignment == Alignment::End { active_button_style } else { button_style })
                                    .set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: if state.alignment == Alignment::End { clr!(white) } else { rgb!(51, 65, 85) },
                                        ..Default::default()
                                    }),
                            )
                            .on_event(EventKind::Click, |_| Some(Message::SetAlignment(Alignment::End))),
                        ))
                        .style(Style::new().gap(6.0).align_items(AlignItems::Center)),
                    ))
                    .style(
                        Style::new()
                            .width(Size::Fit)
                            .padding_xy(16.0, 14.0)
                            .gap(18.0)
                            .align_items(AlignItems::Center),
                    ),
                )
                .axis(ScrollAxis::Horizontal)
                .style(
                    Style::new()
                        .apply(scroll_card_panel)
                        .width(Size::Fixed(720))
                ),

                // Interactive Preview Sandbox
                column((
                    text("Interactive Typography Preview").style(
                        Style::new().padding(2.0).set_text_style(TextStyle {
                            font_size: 13.0,
                            color: rgb!(100, 116, 139),
                            font_weight: FontWeight::SEMI_BOLD,
                            ..Default::default()
                        }),
                    ),
                    text("The quick brown fox jumps over the lazy dog.").style(
                        Style::new()
                            .width(Size::Percent(1.0))
                            .padding(4.0)
                            .set_text_style(TextStyle {
                                font_size: state.font_size,
                                color: rgb!(15, 23, 42),
                                font_weight: state.weight,
                                font_style: if state.is_italic { FontStyle::Italic } else { FontStyle::Normal },
                                strikethrough: state.is_strikethrough,
                                underline: state.is_underline,
                                alignment: state.alignment,
                                wrap: true,
                                ..Default::default()
                            }),
                    ),
                    text("0123456789 — Special Characters: !@#$%^&*()_+{}[]:;\"'<>?,./~`").style(
                        Style::new()
                            .width(Size::Percent(1.0))
                            .padding(2.0)
                            .set_text_style(TextStyle {
                                font_size: (state.font_size * 0.75).max(12.0),
                                color: rgb!(79, 70, 229),
                                font_weight: state.weight,
                                font_style: if state.is_italic { FontStyle::Italic } else { FontStyle::Normal },
                                alignment: state.alignment,
                                wrap: true,
                                ..Default::default()
                            }),
                    ),
                    text("千里之行，始于足下 — 敏捷的棕色狐狸跃过懒狗。").style(
                        Style::new()
                            .width(Size::Percent(1.0))
                            .padding(2.0)
                            .set_text_style(TextStyle {
                                font_size: (state.font_size * 0.85).max(13.0),
                                color: rgb!(13, 148, 136),
                                font_weight: state.weight,
                                font_style: if state.is_italic { FontStyle::Italic } else { FontStyle::Normal },
                                strikethrough: state.is_strikethrough,
                                underline: state.is_underline,
                                alignment: state.alignment,
                                wrap: true,
                                ..Default::default()
                            }),
                    ),
                ))
                .style(
                    Style::new()
                        .apply(card_panel)
                        .width(Size::Fixed(720))
                        .gap(8.0),
                ),

                // Multi-line Paragraph & Line Height Card
                column((
                    text("Paragraph Flow and Multilingual CJK Support").style(
                        Style::new().padding(2.0).set_text_style(TextStyle {
                            font_size: 14.0,
                            color: rgb!(30, 41, 59),
                            font_weight: FontWeight::BOLD,
                            ..Default::default()
                        }),
                    ),
                    text(
                        "MTK utilizes Parley for text shaping and layout with 4-phase subpixel glyph \
                         positioning and TrueType hinting. Characters are rasterized into a GPU texture \
                         atlas with 2-pixel bleed isolation, ensuring crisp kerning, razor-sharp stems, and flawless \
                         typographic hierarchy across all display resolutions."
                    )
                    .style(
                        Style::new()
                            .width(Size::Fixed(680))
                            .set_text_style(TextStyle {
                                font_size: 14.0,
                                line_height: LineHeight::Relative(1.5),
                                color: rgb!(51, 65, 85),
                                wrap: true,
                                ..Default::default()
                            }),
                    ),
                    text(
                        "汉字排版展示：落霞与孤鹜齐飞，秋水共长天一色。MTK 具备完整的 CJK 多语言字符塑形与字体回退机制，\
                         在中英文混排时保持优雅的字基线与统一的行高间距。"
                    )
                    .style(
                        Style::new()
                            .width(Size::Fixed(680))
                            .set_text_style(TextStyle {
                                font_size: 14.0,
                                line_height: LineHeight::Relative(1.6),
                                color: rgb!(71, 85, 105),
                                wrap: true,
                                ..Default::default()
                            }),
                    ),
                ))
                .style(
                    Style::new()
                        .apply(card_panel)
                        .width(Size::Fixed(720))
                        .gap(8.0),
                ),
            ))
            .style(
                Style::new()
                    .gap(14.0)
                    .padding(28.0)
                    .bg_color(rgb!(248, 250, 252))
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::Center),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK Typography Showcase")
        .with_size((820, 760).into());

    window.present_with(attrs);
}
