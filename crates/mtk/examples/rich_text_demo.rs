use mtk::clr;
use mtk::rgb;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text::TextSpan;
use mtk::text_property::{FontStyle, FontWeight};
use mtk::ui::ViewStyleExt;
use mtk::ui::widgets::{SpanGeometry, column, container, rich_text, row, text};
use mtk::windowing::{Window, WindowAttributes};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    Keyword(&'static str),
    FunctionName,
    Parameter(&'static str),
    TypeName,
    Variable(&'static str),
    Number,
}

#[derive(Default)]
struct DemoState {
    hovered_token: Option<(TokenKind, SpanGeometry)>,
    status_msg: Option<String>,
}

#[derive(Clone, Debug)]
enum DemoMsg {
    TokenHovered(TokenKind, bool, SpanGeometry),
    TokenClicked(TokenKind, SpanGeometry),
}

fn main() {
    let code = "\
fn calculate_area(radius: f64) -> f64 {
    let pi = 3.14159;
    pi * radius * radius
}";

    // Token byte spans
    let spans = vec![
        TextSpan::new(0..2)
            .color(rgb!(86, 156, 214))
            .bold()
            .id(TokenKind::Keyword("fn: declares a function")),
        TextSpan::new(3..17)
            .color(rgb!(220, 220, 170))
            .id(TokenKind::FunctionName),
        TextSpan::new(18..24)
            .color(rgb!(156, 220, 254))
            .id(TokenKind::Parameter("radius: input argument")),
        TextSpan::new(26..29)
            .color(rgb!(78, 201, 176))
            .id(TokenKind::TypeName),
        TextSpan::new(34..37)
            .color(rgb!(78, 201, 176))
            .id(TokenKind::TypeName),
        TextSpan::new(44..47)
            .color(rgb!(86, 156, 214))
            .bold()
            .id(TokenKind::Keyword("let: binds a local variable")),
        TextSpan::new(48..50)
            .color(rgb!(156, 220, 254))
            .id(TokenKind::Variable("pi: local constant")),
        TextSpan::new(53..60)
            .color(rgb!(181, 206, 168))
            .id(TokenKind::Number),
        TextSpan::new(66..68)
            .color(rgb!(156, 220, 254))
            .id(TokenKind::Variable("pi: local constant")),
        TextSpan::new(71..77)
            .color(rgb!(156, 220, 254))
            .id(TokenKind::Parameter("radius: input argument")),
        TextSpan::new(80..86)
            .color(rgb!(156, 220, 254))
            .id(TokenKind::Parameter("radius: input argument")),
    ];

    let mut window = Window::with(
        DemoState::default(),
        |state, msg: DemoMsg| match msg {
            DemoMsg::TokenHovered(token, hovered, geom) => {
                if hovered {
                    state.hovered_token = Some((token, geom));
                } else if state.hovered_token.as_ref().map(|(t, _)| t) == Some(&token) {
                    state.hovered_token = None;
                }
            }
            DemoMsg::TokenClicked(token, geom) => {
                state.status_msg = Some(format!(
                    "Clicked: {token:?} at screen [x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}]",
                    geom.x, geom.y, geom.w, geom.h
                ));
            }
        },
        move |state: &DemoState| {
            let tooltip_text = match &state.hovered_token {
                Some((TokenKind::Keyword(doc), geom)) => format!(
                    "Keyword: {doc}\nRect: [x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}]  •  local: [x: {:.1}, y: {:.1}]",
                    geom.x, geom.y, geom.w, geom.h, geom.local_rect.x, geom.local_rect.y
                ),
                Some((TokenKind::FunctionName, geom)) => format!(
                    "Function: calculate_area(radius: f64) -> f64\nRect: [x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}]  •  local: [x: {:.1}, y: {:.1}]",
                    geom.x, geom.y, geom.w, geom.h, geom.local_rect.x, geom.local_rect.y
                ),
                Some((TokenKind::Parameter(doc), geom)) => format!(
                    "Parameter: {doc}\nRect: [x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}]  •  local: [x: {:.1}, y: {:.1}]",
                    geom.x, geom.y, geom.w, geom.h, geom.local_rect.x, geom.local_rect.y
                ),
                Some((TokenKind::TypeName, geom)) => format!(
                    "Type: 64-bit IEEE 754 floating-point (f64)\nRect: [x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}]  •  local: [x: {:.1}, y: {:.1}]",
                    geom.x, geom.y, geom.w, geom.h, geom.local_rect.x, geom.local_rect.y
                ),
                Some((TokenKind::Variable(doc), geom)) => format!(
                    "Variable: {doc}\nRect: [x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}]  •  local: [x: {:.1}, y: {:.1}]",
                    geom.x, geom.y, geom.w, geom.h, geom.local_rect.x, geom.local_rect.y
                ),
                Some((TokenKind::Number, geom)) => format!(
                    "Literal: f64 float literal\nRect: [x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}]  •  local: [x: {:.1}, y: {:.1}]",
                    geom.x, geom.y, geom.w, geom.h, geom.local_rect.x, geom.local_rect.y
                ),
                None => state
                    .status_msg
                    .clone()
                    .unwrap_or_else(|| "Hover over code tokens to inspect type info\nGeometric Rect data, or click them.".into()),
            };

            container((column((
                // Header
                text("Rich Text and Code Syntax Highlighting Demo").style(
                    Style::new().set_text_style(
                        TextStyle::new()
                            .font_size(18.0)
                            .font_weight(FontWeight::BOLD)
                            .color(rgb!(15, 23, 42)),
                    ),
                ),
                // Code Editor Window Frame
                column((
                    // Window titlebar
                    row((text("main.rs").style(Style::new().set_text_style(
                        TextStyle::new().font_size(12.0).color(rgb!(148, 163, 184)),
                    )),))
                    .style(
                        Style::new()
                            .width(Size::Percent(1.0))
                            .bg_color(rgb!(30, 41, 59))
                            .padding(10.0)
                            .corner_radius_top(8.0)
                            .border_bottom(1.0, rgb!(51, 65, 85)),
                    ),
                    // Code content
                    rich_text(code)
                        .spans(spans.clone())
                        .text_style(
                            TextStyle::new()
                                .font_family("Maple Mono NF, monospace")
                                .font_size(15.0)
                                .color(rgb!(212, 212, 212)),
                        )
                        .style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .padding(16.0)
                                .corner_radius_bottom(8.0),
                        )
                        .on_span_hover(|token, hovered, geom| {
                            Some(DemoMsg::TokenHovered(token, hovered, geom))
                        })
                        .on_span_click(|token, geom| Some(DemoMsg::TokenClicked(token, geom))),
                ))
                .style(
                    Style::new()
                        .width(Size::Fixed(500))
                        .bg_color(rgb!(15, 23, 42))
                        .corner_radius(8.0)
                        .border(1.0, rgb!(51, 65, 85))
                        .shadow(rgb!(0, 0, 0), 16.0, 0.2)
                        .align_items(AlignItems::Stretch),
                ),
                // Hover Inspector / LSP popover info
                container((text(tooltip_text).style(
                    Style::new().set_text_style(
                        TextStyle::new()
                            .font_size(13.0)
                            .font_style(FontStyle::Italic)
                            .wrap(true)
                            .color(if state.hovered_token.is_some() {
                                rgb!(30, 64, 175)
                            } else {
                                rgb!(100, 116, 139)
                            }),
                    ),
                ),))
                .style(
                    Style::new()
                        .width(Size::Fixed(500))
                        .bg_color(if state.hovered_token.is_some() {
                            rgb!(239, 246, 255)
                        } else {
                            rgb!(241, 245, 249)
                        })
                        .padding(12.0)
                        .corner_radius(6.0)
                        .border(1.0, rgb!(226, 232, 240)),
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
        .with_title("MTK Rich Text Code Editor Demo")
        .with_size((640, 520).into());

    window.present_with(attrs);
}
