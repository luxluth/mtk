use cosmic_text::Weight;
use mtk::{
    AlignItems, Edges, FlexDirection, JustifyContent, Overflow, Size,
    animation::Curve,
    clr,
    colors::Color,
    effects::{Border, Radius},
    rgb,
    ui::{
        EventKind, View, ViewEventExt,
        style::{AnimationTarget, Style, TextStyle, ViewStyleExt},
        widgets::{column, container, row, text},
    },
    windowing::{Window, WindowAttr},
};
use std::cmp::min;

pub struct CalcState {
    pub display: String,
    pub operand: f64,
    pub operator: Option<char>,
    pub clear_on_next: bool,
    pub pressed_btn: Option<String>,
    theme: Theme,
}

impl CalcState {
    fn input_digit(&mut self, digit: char) {
        if self.clear_on_next {
            self.display = digit.to_string();
            self.clear_on_next = false;
        } else {
            if self.display == "0" && digit != '.' {
                self.display = digit.to_string();
            } else {
                self.display.push(digit);
            }
        }
    }

    fn compute(&mut self) {
        if let Some(op) = self.operator {
            if let Ok(operand2) = self.display.parse::<f64>() {
                let res = match op {
                    '+' => self.operand + operand2,
                    '-' => self.operand - operand2,
                    'x' => self.operand * operand2,
                    '/' => {
                        if operand2 != 0.0 {
                            self.operand / operand2
                        } else {
                            0.0
                        }
                    }
                    _ => operand2,
                };
                self.display = format!("{}", res);
                self.operand = res;
            }
            self.operator = None;
            self.clear_on_next = true;
        }
    }

    fn set_op(&mut self, op: char) {
        if let Ok(val) = self.display.parse::<f64>() {
            self.operand = val;
        }
        self.operator = Some(op);
        self.clear_on_next = true;
    }

    fn clear(&mut self) {
        self.display = "0".to_string();
        self.operand = 0.0;
        self.operator = None;
        self.clear_on_next = true;
    }
}

fn calc_btn<F: Fn(&mut CalcState) + 'static + Clone>(
    is_pressed: bool,
    label: &'static str,
    bg: Color,
    fg: Color,
    border: Color,
    on_click: F,
) -> impl View<CalcState> {
    let on_click_clone = on_click.clone();

    text(label.to_string())
        .style(
            Style::new()
                .update_constraints(|c| {
                    c.padding = Edges::all(20.0);
                    c.border = Edges::all(2.0);
                    c.width = Size::Fill;
                    c.height = Size::Fill;
                    c.justify_content = JustifyContent::Center;
                    c.align_items = AlignItems::Center;
                })
                .set_text_style(TextStyle {
                    font_size: 24.0,
                    alignement: cosmic_text::Align::Center,
                    attrs: cosmic_text::AttrsOwned::new(
                        &cosmic_text::Attrs::new()
                            .color(fg.into())
                            .weight(Weight::BOLD)
                            .family(cosmic_text::Family::Name("IosevkaTerm NF")),
                    ),
                    ..Default::default()
                })
                .bg_color(bg)
                .scale(if is_pressed { 0.95 } else { 1.0 })
                .animate(AnimationTarget::Scale, 100.0, Curve::ease_out())
                .update_effects(|e| {
                    e.border = Border {
                        color: border,
                        radius: Radius::all(12.),
                    };
                })
                .on_hover(move |s| {
                    s.bg_color(rgb!(
                        min(bg.r as u32 + 30, 255) as u8,
                        min(bg.g as u32 + 30, 255) as u8,
                        min(bg.b as u32 + 30, 255) as u8
                    ))
                }),
        )
        .on_event(EventKind::Press, move |s: &mut CalcState| {
            s.pressed_btn = Some(label.to_string());
        })
        .on_event(EventKind::Release, move |s: &mut CalcState| {
            s.pressed_btn = None;
            on_click_clone(s);
        })
}

fn std_btn(is_pressed: bool, label: &'static str, theme: Theme) -> impl View<CalcState> {
    calc_btn(
        is_pressed,
        label,
        theme.std_btn(),
        theme.fg(),
        theme.border(),
        move |s| s.input_digit(label.chars().next().unwrap()),
    )
}

fn op_btn(is_pressed: bool, label: &'static str, op: char, theme: Theme) -> impl View<CalcState> {
    calc_btn(
        is_pressed,
        label,
        theme.op_btn(),
        theme.fg(),
        theme.border(),
        move |s| s.set_op(op),
    )
}

#[allow(unused)]
#[derive(Clone, Copy)]
enum Theme {
    Dark,
    Light,
}

macro_rules! colored {
    ($name:ident, $dark:expr, $light:expr) => {
        fn $name(&self) -> Color {
            match self {
                Theme::Dark => $dark,
                Theme::Light => $light,
            }
        }
    };
}

impl Theme {
    colored!(bg, clr!(0x181818FF), clr!(0xFFFFFFFF));
    colored!(fg, rgb!(255, 255, 255), rgb!(0, 0, 0));
    colored!(border, rgb!(0, 0, 0), rgb!(100, 100, 100));
    colored!(op_btn, rgb!(255, 150, 50), rgb!(255, 150, 50));
    colored!(std_btn, rgb!(40, 40, 40), clr!(0xF5F5F5FF));
    colored!(display, rgb!(30, 30, 30), clr!(0xF5F5F5FF));
}

fn main() {
    env_logger::init();

    let state = CalcState {
        display: "0".to_string(),
        operand: 0.0,
        operator: None,
        clear_on_next: false,
        pressed_btn: None,
        theme: Theme::Light,
    };

    let mut window = Window::with(state, |state: &mut CalcState| {
        column((
            // Display
            container(vec![
                text(state.display.clone()).style(
                    Style::new()
                        .update_constraints(|c| {
                            c.padding = Edges::all(20.0);
                            c.width = Size::Percent(1.0);
                            c.overflow = Overflow::Hidden;
                        })
                        .set_text_style(TextStyle {
                            font_size: 48.0,
                            alignement: cosmic_text::Align::Right,
                            line_height: 48.,
                            attrs: cosmic_text::AttrsOwned::new(
                                &cosmic_text::Attrs::new()
                                    .color(state.theme.fg().into())
                                    .family(cosmic_text::Family::Name("IosevkaTerm NF")),
                            ),
                            ..Default::default()
                        }),
                ),
            ])
            .style(
                Style::new()
                    .update_constraints(|c| {
                        c.width = Size::Percent(1.0);
                        c.height = Size::Fixed(130);
                        c.border = Edges::all(2.0);
                    })
                    .bg_color(state.theme.display())
                    .update_effects(|e| {
                        e.border = Border {
                            color: state.theme.border(),
                            radius: Radius::all(8.),
                        };
                    }),
            ),
            // Row 1
            row((
                calc_btn(
                    state.pressed_btn.as_deref() == Some("C"),
                    "C",
                    rgb!(200, 50, 50),
                    state.theme.fg(),
                    state.theme.border(),
                    |s| s.clear(),
                ),
                calc_btn(
                    state.pressed_btn.as_deref() == Some("+/-"),
                    "+/-",
                    rgb!(80, 80, 80),
                    state.theme.fg(),
                    state.theme.border(),
                    |s| {
                        if let Ok(val) = s.display.parse::<f64>() {
                            s.display = format!("{}", -val);
                        }
                    },
                ),
                calc_btn(
                    state.pressed_btn.as_deref() == Some("%"),
                    "%",
                    rgb!(80, 80, 80),
                    state.theme.fg(),
                    state.theme.border(),
                    |s| {
                        if let Ok(val) = s.display.parse::<f64>() {
                            s.display = format!("{}", val / 100.0);
                        }
                    },
                ),
                op_btn(
                    state.pressed_btn.as_deref() == Some("/"),
                    "/",
                    '/',
                    state.theme,
                ),
            ))
            .style(Style::new().update_constraints(|c| {
                c.width = Size::Percent(1.0);
                c.height = Size::Fill;
                c.gap = 10.0;
                c.flex_direction = FlexDirection::Row;
            })),
            // Row 2
            row((
                std_btn(state.pressed_btn.as_deref() == Some("7"), "7", state.theme),
                std_btn(state.pressed_btn.as_deref() == Some("8"), "8", state.theme),
                std_btn(state.pressed_btn.as_deref() == Some("9"), "9", state.theme),
                op_btn(
                    state.pressed_btn.as_deref() == Some("X"),
                    "X",
                    'x',
                    state.theme,
                ),
            ))
            .style(Style::new().update_constraints(|c| {
                c.width = Size::Percent(1.0);
                c.height = Size::Fill;
                c.gap = 10.0;
                c.flex_direction = FlexDirection::Row;
            })),
            // Row 3
            row((
                std_btn(state.pressed_btn.as_deref() == Some("4"), "4", state.theme),
                std_btn(state.pressed_btn.as_deref() == Some("5"), "5", state.theme),
                std_btn(state.pressed_btn.as_deref() == Some("6"), "6", state.theme),
                op_btn(
                    state.pressed_btn.as_deref() == Some("-"),
                    "-",
                    '-',
                    state.theme,
                ),
            ))
            .style(Style::new().update_constraints(|c| {
                c.width = Size::Percent(1.0);
                c.height = Size::Fill;
                c.gap = 10.0;
                c.flex_direction = FlexDirection::Row;
            })),
            // Row 4
            row((
                std_btn(state.pressed_btn.as_deref() == Some("1"), "1", state.theme),
                std_btn(state.pressed_btn.as_deref() == Some("2"), "2", state.theme),
                std_btn(state.pressed_btn.as_deref() == Some("3"), "3", state.theme),
                op_btn(
                    state.pressed_btn.as_deref() == Some("+"),
                    "+",
                    '+',
                    state.theme,
                ),
            ))
            .style(Style::new().update_constraints(|c| {
                c.width = Size::Percent(1.0);
                c.height = Size::Fill;
                c.gap = 10.0;
                c.flex_direction = FlexDirection::Row;
            })),
            // Row 5
            row((
                std_btn(state.pressed_btn.as_deref() == Some("0"), "0", state.theme),
                std_btn(state.pressed_btn.as_deref() == Some("."), ".", state.theme),
                calc_btn(
                    state.pressed_btn.as_deref() == Some("="),
                    "=",
                    state.theme.op_btn(),
                    state.theme.fg(),
                    state.theme.border(),
                    |s| s.compute(),
                ),
            ))
            .style(Style::new().update_constraints(|c| {
                c.width = Size::Percent(1.0);
                c.height = Size::Fill;
                c.gap = 10.0;
                c.flex_direction = FlexDirection::Row;
            })),
        ))
        .style(
            Style::new()
                .update_constraints(|c| {
                    c.width = Size::Percent(1.0);
                    c.height = Size::Percent(1.0);
                    c.padding = Edges::all(20.0);
                    c.flex_direction = FlexDirection::Column;
                    c.gap = 10.0;
                })
                .bg_color(state.theme.bg()),
        )
    });

    let attr = WindowAttr::default()
        .with_title("MTK Calculator".to_string())
        .with_size((400, 600))
        .with_resizable(false);

    window.present_with(attr);
}
