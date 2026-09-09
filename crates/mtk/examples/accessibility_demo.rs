use mtk::accessibility::AccessibleViewExt;
use mtk::style::{AlignItems, FlexDirection, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::adapter::adapt;
use mtk::ui::widgets::{
    button, checkbox, column, divider, input_text, radio_group, row, slider, switch, text,
};
use mtk::ui::{View, ViewStyleExt};
use mtk::windowing::{Window, WindowAttributes, WindowDimension};
use mtk::{Lens, clr, rgb};

#[derive(Clone, Debug, Lens)]
pub struct A11yDemoState {
    pub counter: i32,
    pub high_contrast: bool,
    pub screen_reader_hints: bool,
    pub volume: f32,
    pub speech_rate_index: usize,
    pub user_name: String,
    pub status: String,
}

#[derive(Clone, Debug)]
pub enum A11yDemoMsg {
    Increment,
    Decrement,
    Reset,
    ToggleHighContrast(bool),
    ToggleHints(bool),
    SetVolume(f32),
    SelectSpeechRate(usize),
    UpdateUserName(String),
}

fn update(state: &mut A11yDemoState, msg: A11yDemoMsg) {
    match msg {
        A11yDemoMsg::Increment => {
            state.counter += 1;
            state.status = format!("Counter incremented to {}", state.counter);
        }
        A11yDemoMsg::Decrement => {
            state.counter -= 1;
            state.status = format!("Counter decremented to {}", state.counter);
        }
        A11yDemoMsg::Reset => {
            state.counter = 0;
            state.status = "Counter reset to 0".to_string();
        }
        A11yDemoMsg::ToggleHighContrast(on) => {
            state.high_contrast = on;
            state.status = format!("High Contrast {}", if on { "enabled" } else { "disabled" });
        }
        A11yDemoMsg::ToggleHints(on) => {
            state.screen_reader_hints = on;
            state.status = format!(
                "Screen reader hints {}",
                if on { "enabled" } else { "disabled" }
            );
        }
        A11yDemoMsg::SetVolume(vol) => {
            state.volume = vol;
            state.status = format!("Volume set to {:.0}%", vol);
        }
        A11yDemoMsg::SelectSpeechRate(idx) => {
            state.speech_rate_index = idx;
            let label = match idx {
                0 => "0.75x",
                1 => "1.0x",
                2 => "1.25x",
                _ => "1.5x",
            };
            state.status = format!("Speech rate selected: {label}");
        }
        A11yDemoMsg::UpdateUserName(name) => {
            state.user_name = name;
            state.status = format!("User name updated: {}", state.user_name);
        }
    }
}

fn app(state: &A11yDemoState) -> impl View<A11yDemoState, Message = A11yDemoMsg> + use<> {
    let bg_color = if state.high_contrast {
        rgb!(0, 0, 0)
    } else {
        rgb!(248, 250, 252)
    };

    let text_color = if state.high_contrast {
        rgb!(255, 255, 255)
    } else {
        rgb!(15, 23, 42)
    };

    let card_bg = if state.high_contrast {
        rgb!(20, 20, 20)
    } else {
        rgb!(255, 255, 255)
    };

    let card_border = if state.high_contrast {
        rgb!(255, 255, 255)
    } else {
        rgb!(226, 232, 240)
    };

    let title_style = TextStyle {
        font_size: 22.0,
        font_weight: FontWeight::BOLD,
        color: text_color,
        ..Default::default()
    };

    let section_title_style = TextStyle {
        font_size: 15.0,
        font_weight: FontWeight::SEMI_BOLD,
        color: text_color,
        ..Default::default()
    };

    let body_style = TextStyle {
        font_size: 13.0,
        font_weight: FontWeight::NORMAL,
        color: text_color,
        ..Default::default()
    };

    let status_style = TextStyle {
        font_size: 13.0,
        font_weight: FontWeight::MEDIUM,
        color: if state.high_contrast {
            rgb!(134, 239, 172)
        } else {
            rgb!(37, 99, 235)
        },
        ..Default::default()
    };

    let header = column((
        text("AccessKit Accessibility Showcase").style(Style::new().set_text_style(title_style)),
        text("Explore native screen reader roles, actions, and tree inspection via AT-SPI / UI Automation / NSAccessibility.")
            .style(Style::new().set_text_style(body_style.clone())),
    ))
    .style(Style::new().gap(4.0));

    let counter_section = column((
        text("1. Accessible Buttons & Live Values")
            .style(Style::new().set_text_style(section_title_style.clone())),
        row((
            button("Decrement")
                .on_click(A11yDemoMsg::Decrement)
                .accessible_label("Decrement counter value")
                .accessible_description("Decreases the current counter count by one"),
            text(format!("Count: {}", state.counter))
                .style(Style::new().set_text_style(section_title_style.clone()))
                .accessible_label(format!("Current counter count is {}", state.counter)),
            button("Increment")
                .on_click(A11yDemoMsg::Increment)
                .accessible_label("Increment counter value")
                .accessible_description("Increases the current counter count by one"),
            button("Reset")
                .on_click(A11yDemoMsg::Reset)
                .accessible_label("Reset counter to zero"),
        ))
        .style(
            Style::new()
                .flex_direction(FlexDirection::Row)
                .align_items(AlignItems::Center)
                .gap(12.0),
        ),
    ))
    .style(
        Style::new()
            .padding(16.0)
            .corner_radius(8.0)
            .bg_color(card_bg)
            .border(1.0, card_border)
            .gap(10.0),
    );

    let toggles_section = column((
        text("2. Accessible Switches & Checkboxes")
            .style(Style::new().set_text_style(section_title_style.clone())),
        row((
            switch(state.high_contrast)
                .label("High Contrast Mode")
                .on_toggle(A11yDemoMsg::ToggleHighContrast)
                .label_color(if state.high_contrast {
                    clr!(white)
                } else {
                    clr!(black)
                }),
            checkbox(state.screen_reader_hints)
                .label("Verbose Screen Reader Hints")
                .on_toggle(A11yDemoMsg::ToggleHints)
                .label_color(if state.high_contrast {
                    clr!(white)
                } else {
                    clr!(black)
                }),
        ))
        .style(
            Style::new()
                .flex_direction(FlexDirection::Row)
                .align_items(AlignItems::Center)
                .gap(24.0),
        ),
    ))
    .style(
        Style::new()
            .padding(16.0)
            .corner_radius(8.0)
            .bg_color(card_bg)
            .border(1.0, card_border)
            .gap(10.0),
    );

    let speech_rates = vec![
        "0.75x (Relaxed)".to_string(),
        "1.0x (Normal)".to_string(),
        "1.25x (Fast)".to_string(),
        "1.5x (Very Fast)".to_string(),
    ];

    let slider_and_radio_section = column((
        text("3. Accessible Sliders & Radio Groups")
            .style(Style::new().set_text_style(section_title_style.clone())),
        column((
            text(format!("Volume: {:.0}%", state.volume))
                .style(Style::new().set_text_style(body_style.clone())),
            slider(state.volume, 0.0, 100.0)
                .step(5.0)
                .on_change(A11yDemoMsg::SetVolume)
                .style(Style::new().width(Size::Fixed(320))),
        ))
        .style(Style::new().gap(6.0)),
        divider(),
        column((
            text("Speech Rate Preset:").style(Style::new().set_text_style(body_style.clone())),
            radio_group(
                state.speech_rate_index,
                speech_rates,
                A11yDemoMsg::SelectSpeechRate,
            )
            .label_color(if state.high_contrast {
                clr!(white)
            } else {
                clr!(black)
            }),
        ))
        .style(Style::new().gap(6.0)),
    ))
    .style(
        Style::new()
            .padding(16.0)
            .corner_radius(8.0)
            .bg_color(card_bg)
            .border(1.0, card_border)
            .gap(10.0),
    );

    let input_section = column((
        text("4. Accessible Text Input (Value & Focus Actions)")
            .style(Style::new().set_text_style(section_title_style.clone())),
        adapt(
            input_text().placeholder("Enter name here...").style(
                Style::new()
                    .set_text_style(TextStyle {
                        color: if state.high_contrast {
                            clr!(white)
                        } else {
                            clr!(black)
                        },
                        ..Default::default()
                    })
                    .width(Size::Fixed(320)),
            ),
            A11yDemoState::user_name,
            A11yDemoMsg::UpdateUserName,
        ),
    ))
    .style(
        Style::new()
            .padding(16.0)
            .corner_radius(8.0)
            .bg_color(card_bg)
            .border(1.0, card_border)
            .gap(10.0),
    );

    let status_bar = row((
        text("Status:").style(Style::new().set_text_style(TextStyle {
            font_weight: FontWeight::BOLD,
            color: text_color,
            ..body_style
        })),
        text(&state.status).style(Style::new().set_text_style(status_style)),
    ))
    .style(
        Style::new()
            .flex_direction(FlexDirection::Row)
            .align_items(AlignItems::Center)
            .gap(8.0)
            .padding(12.0)
            .corner_radius(6.0)
            .bg_color(if state.high_contrast {
                rgb!(30, 30, 30)
            } else {
                rgb!(241, 245, 249)
            }),
    );

    column((
        header,
        counter_section,
        toggles_section,
        slider_and_radio_section,
        input_section,
        status_bar,
    ))
    .style(
        Style::new()
            .width(Size::Percent(100.0))
            .height(Size::Percent(100.0))
            .bg_color(bg_color)
            .padding(24.0)
            .gap(16.0),
    )
}

fn main() {
    let initial_state = A11yDemoState {
        counter: 0,
        high_contrast: false,
        screen_reader_hints: true,
        volume: 75.0,
        speech_rate_index: 1,
        user_name: "John Doe".to_string(),
        status: "Application initialized. Accessibility bridge active.".to_string(),
    };

    let window = Window::with(initial_state, update, app);
    window.present_with(
        WindowAttributes::default()
            .with_title("MTK - AccessKit Accessibility Showcase")
            .with_app_id("mtk.accessibility.test")
            .with_size(WindowDimension {
                width: 720,
                height: 800,
            }),
    );
}
