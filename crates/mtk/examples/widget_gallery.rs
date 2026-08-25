use mtk::animation::Curve;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::adapter::adapt;
use mtk::ui::widgets::{
    PixelBuffer, button, checkbox, column, input_text, pixel_canvas, row, scroll_view, slider,
    switch, text, text_area,
};
use mtk::ui::{View, ViewAdaptExt, ViewStyleExt};
use mtk::windowing::{Window, WindowAttributes, WindowDimension};
use mtk::{Lens, clr, rgb, rgba};

#[derive(Clone, Debug, Lens)]
pub struct GalleryState {
    pub counter: i32,
    pub checkbox_notifications: bool,
    pub checkbox_analytics: bool,
    pub switch_dark_mode: bool,
    pub switch_auto_save: bool,
    pub slider_volume: f32,
    pub slider_brightness: f32,
    pub username_input: String,
    pub bio_textarea: String,
    pub status_message: String,
}

#[derive(Clone, Debug)]
pub enum GalleryMsg {
    IncrementCounter,
    DecrementCounter,
    ResetCounter,
    ToggleNotifications(bool),
    ToggleAnalytics(bool),
    ToggleDarkMode(bool),
    ToggleAutoSave(bool),
    SetVolume(f32),
    SetBrightness(f32),
    UpdateUsername(String),
    UpdateBio(String),
}

fn update(state: &mut GalleryState, msg: GalleryMsg) {
    match msg {
        GalleryMsg::IncrementCounter => {
            state.counter += 1;
            state.status_message = format!("Counter incremented to {}", state.counter);
        }
        GalleryMsg::DecrementCounter => {
            state.counter -= 1;
            state.status_message = format!("Counter decremented to {}", state.counter);
        }
        GalleryMsg::ResetCounter => {
            state.counter = 0;
            state.status_message = "Counter reset to 0".to_string();
        }
        GalleryMsg::ToggleNotifications(val) => {
            state.checkbox_notifications = val;
            state.status_message =
                format!("Notifications {}", if val { "enabled" } else { "disabled" });
        }
        GalleryMsg::ToggleAnalytics(val) => {
            state.checkbox_analytics = val;
            state.status_message =
                format!("Analytics {}", if val { "enabled" } else { "disabled" });
        }
        GalleryMsg::ToggleDarkMode(val) => {
            state.switch_dark_mode = val;
            state.status_message =
                format!("Dark mode {}", if val { "enabled" } else { "disabled" });
        }
        GalleryMsg::ToggleAutoSave(val) => {
            state.switch_auto_save = val;
            state.status_message =
                format!("Auto-save {}", if val { "enabled" } else { "disabled" });
        }
        GalleryMsg::SetVolume(val) => {
            state.slider_volume = val;
        }
        GalleryMsg::SetBrightness(val) => {
            state.slider_brightness = val;
        }
        GalleryMsg::UpdateUsername(txt) => {
            state.username_input = txt;
        }
        GalleryMsg::UpdateBio(txt) => {
            state.bio_textarea = txt;
        }
    }
}

fn card_header(
    title: &str,
    subtitle: &str,
    dark_mode: bool,
) -> impl View<GalleryState, Message = GalleryMsg> + use<> {
    let title_color = if dark_mode {
        rgb!(248, 250, 252)
    } else {
        rgb!(15, 23, 42)
    };
    let sub_color = if dark_mode {
        rgb!(148, 163, 184)
    } else {
        rgb!(100, 116, 139)
    };

    column((
        text(title).style(Style::new().set_text_style(TextStyle {
            font_size: 18.0,
            font_weight: FontWeight::BOLD,
            color: title_color,
            ..Default::default()
        })),
        text(subtitle).style(Style::new().set_text_style(TextStyle {
            font_size: 13.0,
            color: sub_color,
            ..Default::default()
        })),
    ))
    .style(Style::new().gap(2.0))
}

fn app(state: &GalleryState) -> impl View<GalleryState, Message = GalleryMsg> + use<> {
    let dark = state.switch_dark_mode;

    let bg_card = if dark { rgb!(30, 41, 59) } else { clr!(white) };
    let border_card = if dark {
        rgb!(51, 65, 85)
    } else {
        rgb!(226, 232, 240)
    };
    let text_primary = if dark {
        rgb!(248, 250, 252)
    } else {
        rgb!(15, 23, 42)
    };
    let text_secondary = if dark {
        rgb!(148, 163, 184)
    } else {
        rgb!(100, 116, 139)
    };
    let input_bg = if dark {
        rgb!(15, 23, 42)
    } else {
        rgb!(248, 250, 252)
    };
    let input_border = if dark {
        rgb!(71, 85, 105)
    } else {
        rgb!(203, 213, 225)
    };

    let caret_color = if dark {
        rgb!(96, 165, 250)
    } else {
        rgb!(37, 99, 235)
    };

    let count_label = format!("Current Count: {}", state.counter);
    let volume_label = format!("Volume: {:.0}%", state.slider_volume);
    let brightness_label = format!("Brightness (Stepped 10%): {:.0}%", state.slider_brightness);

    // Left Column: Interactive Inputs, Buttons, Toggles
    let left_column = column((
        // 1. Buttons & Counter
        column((
            card_header(
                "Buttons & Actions",
                "Interactive buttons with hover scale animations and disabled state.",
                dark,
            ),
            column((
                text(count_label).style(Style::new().set_text_style(TextStyle {
                    font_size: 15.0,
                    font_weight: FontWeight::SEMI_BOLD,
                    color: text_primary,
                    ..Default::default()
                })),
                row((
                    button("Increment").on_click(GalleryMsg::IncrementCounter),
                    button("Decrement")
                        .on_click(GalleryMsg::DecrementCounter)
                        .style(
                            Style::new()
                                .bg_color(if dark {
                                    rgb!(51, 65, 85)
                                } else {
                                    rgb!(241, 245, 249)
                                })
                                .border(1.0, border_card)
                                .set_text_style(TextStyle {
                                    font_size: 14.0,
                                    font_weight: FontWeight::SEMI_BOLD,
                                    color: text_primary,
                                    ..Default::default()
                                }),
                        ),
                    button("Reset").on_click(GalleryMsg::ResetCounter).style(
                        Style::new()
                            .bg_color(if dark {
                                rgb!(69, 10, 10)
                            } else {
                                rgb!(254, 242, 242)
                            })
                            .border(
                                1.0,
                                if dark {
                                    rgb!(153, 27, 27)
                                } else {
                                    rgb!(254, 202, 202)
                                },
                            )
                            .set_text_style(TextStyle {
                                font_size: 14.0,
                                font_weight: FontWeight::SEMI_BOLD,
                                color: if dark {
                                    rgb!(252, 165, 165)
                                } else {
                                    rgb!(220, 38, 38)
                                },
                                ..Default::default()
                            }),
                    ),
                    button("Disabled").disabled(true),
                ))
                .style(Style::new().gap(8.0).align_items(AlignItems::Center)),
            ))
            .style(Style::new().gap(10.0)),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .bg_color(bg_card)
                .border(1.0, border_card)
                .corner_radius(10.0)
                .padding(18.0)
                .gap(14.0)
                .shadow(rgba!(0, 0, 0, 12), 8.0, 0.4),
        ),
        // 2. Checkboxes & Switches
        column((
            card_header(
                "Toggles & Checkboxes",
                "Accessible binary toggles with smooth animated transitions.",
                dark,
            ),
            row((
                // Checkboxes
                column((
                    checkbox(state.checkbox_notifications)
                        .label("Enable Notifications")
                        .on_toggle(GalleryMsg::ToggleNotifications),
                    checkbox(state.checkbox_analytics)
                        .label("Share Usage Analytics")
                        .on_toggle(GalleryMsg::ToggleAnalytics),
                    checkbox(true).label("Locked Policy").disabled(true),
                ))
                .style(Style::new().gap(10.0).flex_grow(1.0)),
                // Switches
                column((
                    row((
                        switch(state.switch_dark_mode).on_toggle(GalleryMsg::ToggleDarkMode),
                        text("Dark Mode").style(Style::new().set_text_style(TextStyle {
                            font_size: 14.0,
                            color: text_primary,
                            ..Default::default()
                        })),
                    ))
                    .style(Style::new().gap(8.0).align_items(AlignItems::Center)),
                    row((
                        switch(state.switch_auto_save).on_toggle(GalleryMsg::ToggleAutoSave),
                        text("Auto-Save").style(Style::new().set_text_style(TextStyle {
                            font_size: 14.0,
                            color: text_primary,
                            ..Default::default()
                        })),
                    ))
                    .style(Style::new().gap(8.0).align_items(AlignItems::Center)),
                    row((
                        switch(false).disabled(true),
                        text("Offline Mode (Disabled)").style(Style::new().set_text_style(
                            TextStyle {
                                font_size: 14.0,
                                color: text_secondary,
                                ..Default::default()
                            },
                        )),
                    ))
                    .style(Style::new().gap(8.0).align_items(AlignItems::Center)),
                ))
                .style(Style::new().gap(10.0).flex_grow(1.0)),
            ))
            .style(Style::new().gap(16.0).align_items(AlignItems::Center)),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .bg_color(bg_card)
                .border(1.0, border_card)
                .corner_radius(10.0)
                .padding(18.0)
                .gap(14.0)
                .shadow(rgba!(0, 0, 0, 12), 8.0, 0.4),
        ),
        // 3. Sliders
        column((
            card_header(
                "Range Sliders",
                "Continuous and stepped slider controls with drag hit-testing.",
                dark,
            ),
            column((
                column((
                    text(volume_label).style(Style::new().set_text_style(TextStyle {
                        font_size: 13.0,
                        font_weight: FontWeight::MEDIUM,
                        color: text_primary,
                        ..Default::default()
                    })),
                    slider(state.slider_volume, 0.0, 100.0)
                        .width(Size::Percent(1.0))
                        .on_change(GalleryMsg::SetVolume),
                ))
                .style(Style::new().gap(4.0)),
                column((
                    text(brightness_label).style(Style::new().set_text_style(TextStyle {
                        font_size: 13.0,
                        font_weight: FontWeight::MEDIUM,
                        color: text_primary,
                        ..Default::default()
                    })),
                    slider(state.slider_brightness, 0.0, 100.0)
                        .step(10.0)
                        .width(Size::Percent(1.0))
                        .on_change(GalleryMsg::SetBrightness),
                ))
                .style(Style::new().gap(4.0)),
            ))
            .style(Style::new().gap(14.0)),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .bg_color(bg_card)
                .border(1.0, border_card)
                .corner_radius(10.0)
                .padding(18.0)
                .gap(14.0)
                .shadow(rgba!(0, 0, 0, 12), 8.0, 0.4),
        ),
    ))
    .style(
        Style::new()
            .width(Size::Percent(0.5))
            .gap(16.0)
            .flex_grow(1.0),
    );

    // Right Column: Text Inputs, Textarea, Pixel Canvas Graphics
    let vol_ratio = state.slider_volume / 100.0;
    let bright_ratio = state.slider_brightness / 100.0;

    let right_column = column((
        // 4. Text Inputs & Undo/Redo Editor
        column((
            card_header(
                "Text Fields & Multi-Line Editor",
                "Native single-line and multi-line text input with multi-level Undo/Redo (Ctrl+Z/Ctrl+Y).",
                dark,
            ),
            column((
                column((
                    text("Username (Single Line)").style(Style::new().set_text_style(TextStyle {
                        font_size: 13.0,
                        font_weight: FontWeight::MEDIUM,
                        color: text_primary,
                        ..Default::default()
                    })),
                    adapt(
                        input_text()
                            .placeholder("Enter username...")
                            .text_style(TextStyle {
                                font_size: 14.0,
                                color: text_primary,
                                caret_color,
                                ..Default::default()
                            })
                            .style(
                                Style::new()
                                    .width(Size::Percent(1.0))
                                    .height(Size::Fixed(38))
                                    .padding(10.0)
                                    .bg_color(input_bg)
                                    .border(1.0, input_border)
                                    .corner_radius(6.0),
                            ),
                        GalleryState::username_input,
                        GalleryMsg::UpdateUsername,
                    ),
                ))
                .style(Style::new().gap(4.0)),
                column((
                    text("Bio / Notes (Multi-Line TextArea)").style(Style::new().set_text_style(
                        TextStyle {
                            font_size: 13.0,
                            font_weight: FontWeight::MEDIUM,
                            color: text_primary,
                            ..Default::default()
                        },
                    )),
                    text_area()
                        .captures_tab(false)
                        .style(
                            Style::new()
                                .width(Size::Percent(1.0))
                                .height(Size::Fixed(90))
                                .padding(10.0)
                                .bg_color(input_bg)
                                .border(1.0, input_border)
                                .corner_radius(6.0)
                                .set_text_style(TextStyle {
                                    font_size: 14.0,
                                    color: text_primary,
                                    caret_color,
                                    wrap: true,
                                    ..Default::default()
                                }),
                        )
                        .adapt(GalleryState::bio_textarea, GalleryMsg::UpdateBio),
                ))
                .style(Style::new().gap(4.0)),
            ))
            .style(Style::new().gap(12.0)),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .bg_color(bg_card)
                .border(1.0, border_card)
                .corner_radius(10.0)
                .padding(18.0)
                .gap(14.0)
                .shadow(rgba!(0, 0, 0, 12), 8.0, 0.4),
        ),
        // 5. Procedural Pixel Canvas
        column((
            card_header(
                "Pixel Buffer Canvas",
                "Hardware-accelerated dynamic pixel canvas driven by slider parameters.",
                dark,
            ),
            pixel_canvas(move |buf: &mut PixelBuffer| {
                let w = buf.width as usize;
                let h = buf.height as usize;

                for y in 0..h {
                    for x in 0..w {
                        let nx = x as f32 / w.max(1) as f32;
                        let ny = y as f32 / h.max(1) as f32;

                        let wave = ((nx * 10.0 * vol_ratio + ny * 6.0).sin() * 0.5 + 0.5)
                            * bright_ratio;
                        let r = (nx * 200.0 * bright_ratio + wave * 55.0).clamp(0.0, 255.0) as u8;
                        let g = (wave * 220.0).clamp(0.0, 255.0) as u8;
                        let b = (ny * 255.0 * vol_ratio + 50.0).clamp(0.0, 255.0) as u8;

                        buf.set_pixel_color(x as u32, y as u32, rgba!(r, g, b, 255));
                    }
                }
            })
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .height(Size::Fixed(120))
                    .corner_radius(8.0)
                    .border(1.0, input_border),
            ),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .bg_color(bg_card)
                .border(1.0, border_card)
                .corner_radius(10.0)
                .padding(18.0)
                .gap(14.0)
                .shadow(rgba!(0, 0, 0, 12), 8.0, 0.4),
        ),
    ))
    .style(
        Style::new()
            .width(Size::Percent(0.5))
            .gap(16.0)
            .flex_grow(1.0),
    );

    // Top Header Banner
    let header_banner = row((
        column((
            text("MTK Native Widget Gallery").style(Style::new().set_text_style(TextStyle {
                font_size: 24.0,
                font_weight: FontWeight::EXTRA_BOLD,
                color: text_primary,
                ..Default::default()
            })),
            text("Explore all native UI primitives, responsive layouts, and interactive controls.")
                .style(Style::new().set_text_style(TextStyle {
                    font_size: 14.0,
                    color: text_secondary,
                    ..Default::default()
                })),
        ))
        .style(Style::new().gap(4.0).flex_grow(1.0)),
        // Status Badge
        text(&state.status_message).style(
            Style::new()
                .padding_xy(12.0, 6.0)
                .corner_radius(6.0)
                .bg_color(if dark {
                    rgb!(15, 23, 42)
                } else {
                    rgb!(241, 245, 249)
                })
                .border(
                    1.0,
                    if dark {
                        rgb!(51, 65, 85)
                    } else {
                        rgb!(203, 213, 225)
                    },
                )
                .set_text_style(TextStyle {
                    font_size: 12.0,
                    font_weight: FontWeight::MEDIUM,
                    color: if dark {
                        rgb!(147, 197, 253)
                    } else {
                        rgb!(37, 99, 235)
                    },
                    ..Default::default()
                }),
        ),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::SpaceBetween)
            .padding_xy(20.0, 16.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3)
            .transition_all(200.0, Curve::ease_out()),
    );

    // Main 2-column grid inside Scroll View
    let content_grid = row((left_column, right_column)).style(
        Style::new()
            .width(Size::Percent(1.0))
            .gap(16.0)
            .align_items(AlignItems::Start),
    );

    let scrollable_content = scroll_view(
        column((header_banner, content_grid)).style(
            Style::new()
                .width(Size::Percent(1.0))
                .padding(20.0)
                .gap(16.0),
        ),
    )
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0)),
    );

    // Root Container
    column((scrollable_content,)).style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .bg_color(if dark {
                rgb!(15, 23, 42)
            } else {
                rgb!(248, 250, 252)
            })
            .transition_all(200.0, Curve::ease_out()),
    )
}

fn main() {
    let initial_state = GalleryState {
        counter: 0,
        checkbox_notifications: true,
        checkbox_analytics: false,
        switch_dark_mode: false,
        switch_auto_save: true,
        slider_volume: 75.0,
        slider_brightness: 50.0,
        username_input: "alex_developer".to_string(),
        bio_textarea: "Building high performance native user interfaces with MTK!".to_string(),
        status_message: "Ready".to_string(),
    };

    let mut window = Window::with(initial_state, update, app);

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK - Comprehensive Native Widget Gallery")
            .with_size(WindowDimension {
                width: 1080,
                height: 840,
            }),
    );
}
