use mtk::{
    AlignItems, AnimationTarget, FlexDirection, JustifyContent, Overflow, Size, Style, TextStyle,
    animation::Curve,
    clr,
    colors::Color,
    rgb,
    text_property::{self, Alignment, OverflowWrap},
    ui::{
        EventKind, View, ViewEventExt,
        style::ViewStyleExt,
        widgets::{column, container, row, text},
    },
    windowing::{Window, WindowAttributes},
};
use std::cmp::min;

#[derive(Clone)]
pub enum CalcMsg {
    Press(String),
    InputDigit(char),
    SetOp(char),
    Compute,
    Clear,
    ToggleSign,
    Percent,
}

pub fn update(state: &mut CalcState, msg: CalcMsg) {
    // If the message is anything OTHER than a Press, it means a button was released.
    // We can clear the pressed visual state globally here
    if !matches!(msg, CalcMsg::Press(_)) {
        state.pressed_btn = None;
    }

    match msg {
        CalcMsg::Press(label) => state.pressed_btn = Some(label),
        CalcMsg::InputDigit(d) => state.input_digit(d),
        CalcMsg::SetOp(op) => state.set_op(op),
        CalcMsg::Compute => state.compute(),
        CalcMsg::Clear => state.clear(),
        CalcMsg::ToggleSign => {
            if let Ok(val) = state.display.parse::<f64>() {
                state.display = format!("{}", -val);
            }
        }
        CalcMsg::Percent => {
            if let Ok(val) = state.display.parse::<f64>() {
                state.display = format!("{}", val / 100.0);
            }
        }
    }
}

fn calc_btn(
    is_pressed: bool,
    label: &'static str,
    bg: Color,
    fg: Color,
    border: Color,
    action_msg: CalcMsg,
) -> impl View<CalcState, Message = CalcMsg> {
    text(label.to_string())
        .style(
            Style::new()
                .padding(20.0)
                .border(2.0, border)
                .width(Size::Fill)
                .height(Size::Fill)
                .justify_content(JustifyContent::Center)
                .align_items(AlignItems::Center)
                .set_text_style(TextStyle {
                    font_size: 24.0,
                    alignment: Alignment::Center,
                    font_weight: text_property::FontWeight::BOLD,
                    font_family: "IosevkaTerm NF".to_string(),
                    color: fg,
                    ..Default::default()
                })
                .bg_color(bg)
                .scale(if is_pressed { 0.95 } else { 1.0 })
                .animate(AnimationTarget::Scale, 100.0, Curve::ease_out())
                .corner_radius(12.0)
                .on_hover(move |s| {
                    s.bg_color(rgb!(
                        min(bg.r as u32 + 30, 255) as u8,
                        min(bg.g as u32 + 30, 255) as u8,
                        min(bg.b as u32 + 30, 255) as u8
                    ))
                }),
        )
        .on_event(EventKind::Press, move |_state| {
            Some(CalcMsg::Press(label.to_string()))
        })
        .on_event(EventKind::Release, move |_state| Some(action_msg.clone()))
}

fn std_btn(
    is_pressed: bool,
    label: &'static str,
    theme: Theme,
) -> impl View<CalcState, Message = CalcMsg> {
    calc_btn(
        is_pressed,
        label,
        theme.std_btn(),
        theme.fg(),
        theme.border(),
        CalcMsg::InputDigit(label.chars().next().unwrap()),
    )
}

fn op_btn(
    is_pressed: bool,
    label: &'static str,
    op: char,
    theme: Theme,
) -> impl View<CalcState, Message = CalcMsg> {
    calc_btn(
        is_pressed,
        label,
        theme.op_btn(),
        theme.fg(),
        theme.border(),
        CalcMsg::SetOp(op),
    )
}

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
    colored!(op_btn, rgb!(255, 150, 50), clr!(ll_blue));
    colored!(std_btn, rgb!(40, 40, 40), clr!(0xF5F5F5FF));
    colored!(display, rgb!(30, 30, 30), clr!(0xF5F5F5FF));
}

fn app_view(state: &CalcState) -> impl View<CalcState, Message = CalcMsg> + use<> {
    column((
        // Display
        container(vec![
            text(state.display.clone()).style(
                Style::new()
                    .padding(20.0)
                    .width(Size::Percent(1.0))
                    .set_text_style(TextStyle {
                        font_size: 48.0,
                        alignment: Alignment::End,
                        font_family: "IosevkaTerm NF".to_string(),
                        color: state.theme.fg(),
                        wrap: true,
                        overflow_wrap: OverflowWrap::Anywhere,
                        ..Default::default()
                    }),
            ),
        ])
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Fixed(130))
                .border(2.0, state.theme.border())
                .bg_color(state.theme.display())
                .overflow(Overflow::Hidden)
                .corner_radius(8.0),
        ),
        // Row 1
        row((
            calc_btn(
                state.pressed_btn.as_deref() == Some("C"),
                "C",
                rgb!(200, 50, 50),
                state.theme.fg(),
                state.theme.border(),
                CalcMsg::Clear,
            ),
            calc_btn(
                state.pressed_btn.as_deref() == Some("+/-"),
                "+/-",
                rgb!(80, 80, 80),
                state.theme.fg(),
                state.theme.border(),
                CalcMsg::ToggleSign,
            ),
            calc_btn(
                state.pressed_btn.as_deref() == Some("%"),
                "%",
                rgb!(80, 80, 80),
                state.theme.fg(),
                state.theme.border(),
                CalcMsg::Percent,
            ),
            op_btn(
                state.pressed_btn.as_deref() == Some("/"),
                "/",
                '/',
                state.theme,
            ),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Fill)
                .gap(10.0)
                .flex_direction(FlexDirection::Row),
        ),
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
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Fill)
                .gap(10.0)
                .flex_direction(FlexDirection::Row),
        ),
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
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Fill)
                .gap(10.0)
                .flex_direction(FlexDirection::Row),
        ),
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
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Fill)
                .gap(10.0)
                .flex_direction(FlexDirection::Row),
        ),
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
                CalcMsg::Compute,
            ),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Fill)
                .gap(10.0)
                .flex_direction(FlexDirection::Row),
        ),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .padding(20.0)
            .flex_direction(FlexDirection::Column)
            .gap(10.0)
            .bg_color(state.theme.bg()),
    )
}

fn main() {
    env_logger::init();

    let state = CalcState {
        display: "0".to_string(),
        operand: 0.0,
        operator: None,
        clear_on_next: false,
        pressed_btn: None,
        theme: Theme::Dark,
    };

    // Note the new signature: state, update_fn, view_builder(&State)
    let mut window = Window::with(state, update, app_view);

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK Calculator")
            .with_size((400, 600).into())
            .with_min_size(Some((400, 600).into()))
            .with_app_id("dev.luxluth.calculator")
            .with_resizable(true),
    );
}
