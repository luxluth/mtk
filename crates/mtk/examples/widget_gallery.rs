use mtk::animation::Curve;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::adapter::adapt;
use mtk::ui::widgets::{
    PixelBuffer, badge, button, checkbox, chip, column, divider, dropdown, input_text,
    pixel_canvas, progress_bar, radio_group, row, scroll_view, slider, spacer, switch, text,
    text_area,
};
use mtk::ui::{View, ViewLayerExt, ViewStyleExt};
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
    pub selected_framework: usize,
    pub radio_frequency: usize,
    pub is_modal_open: bool,
    pub is_player_open: bool,
    pub tags: Vec<String>,
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
    SelectFramework(usize),
    SelectFrequency(usize),
    OpenModal(bool),
    OpenPlayer(bool),
    DeleteTag(String),
    ResetTags,
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
        GalleryMsg::SelectFramework(idx) => {
            state.selected_framework = idx;
            state.status_message = format!("Framework preset {} chosen", idx + 1);
        }
        GalleryMsg::SelectFrequency(idx) => {
            state.radio_frequency = idx;
            state.status_message = format!("Frequency preset {} chosen", idx + 1);
        }
        GalleryMsg::OpenModal(is_open) => {
            state.is_modal_open = is_open;
            state.status_message = if is_open {
                "Modal dialog opened".to_string()
            } else {
                "Modal dialog closed".to_string()
            };
        }
        GalleryMsg::OpenPlayer(is_open) => {
            state.is_player_open = is_open;
            state.status_message = if is_open {
                "Fullscreen player sheet expanded".to_string()
            } else {
                "Fullscreen player sheet closed".to_string()
            };
        }
        GalleryMsg::DeleteTag(tag_name) => {
            state.tags.retain(|t| t != &tag_name);
            state.status_message = format!("Tag '{}' removed", tag_name);
        }
        GalleryMsg::ResetTags => {
            state.tags = vec![
                "Rust".into(),
                "WGPU".into(),
                "Muse Layout".into(),
                "Parley Text".into(),
                "Flexbox Wrap".into(),
                "Layer Surfaces".into(),
                "Cross Platform".into(),
                "Zero Garbage".into(),
            ];
            state.status_message = "Tags reset".to_string();
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
            font_size: 17.0,
            font_weight: FontWeight::BOLD,
            color: title_color,
            ..Default::default()
        })),
        text(subtitle).style(Style::new().set_text_style(TextStyle {
            font_size: 13.0,
            color: sub_color,
            wrap: true,
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

    // ==========================================
    // 1. Buttons & Tooltips Card
    // ==========================================
    let buttons_card = column((
        card_header(
            "Buttons & Hover Tooltips",
            "Interactive buttons with anchored floating tooltips",
            dark,
        ),
        row((
            button("Primary Action")
                .on_click(GalleryMsg::IncrementCounter)
                .tooltip("Increments counter by 1"),
            button("Secondary")
                .secondary()
                .on_click(GalleryMsg::DecrementCounter)
                .tooltip("Decrements counter by 1"),
            button("Danger Reset")
                .danger()
                .on_click(GalleryMsg::ResetCounter)
                .tooltip("Resets counter to zero"),
            button("Disabled")
                .disabled(true)
                .tooltip("Action is disabled"),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .gap(10.0)
                .align_items(AlignItems::Center)
                .wrap(),
        ),
        text(format!("Counter: {}", state.counter)).style(Style::new().set_text_style(TextStyle {
            font_size: 14.0,
            font_weight: FontWeight::SEMI_BOLD,
            color: if dark {
                rgb!(96, 165, 250)
            } else {
                rgb!(37, 99, 235)
            },
            ..Default::default()
        })),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .padding(18.0)
            .gap(14.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3),
    );

    // ==========================================
    // 2. Overlays & Layer Surfaces Card
    // ==========================================
    let framework_options = vec![
        "MTK (Native Engine)".to_string(),
        "WGPU (GPU Accelerated)".to_string(),
        "Muse (Flexbox Engine)".to_string(),
        "Parley (Cosmic Text)".to_string(),
    ];

    let overlay_card = column((
        card_header(
            "Layer Surfaces & Overlays",
            "Fullscreen bottom sheets, modal dialogs, and select dropdowns",
            dark,
        ),
        row((
            button("Open Music Player Sheet")
                .on_click(GalleryMsg::OpenPlayer(true))
                .tooltip("Slides up expanded music player sheet"),
            button("Open Modal Dialog")
                .secondary()
                .on_click(GalleryMsg::OpenModal(true))
                .tooltip("Opens centered modal with backdrop scrim"),
        ))
        .style(Style::new().gap(10.0).wrap()),
        divider().color(border_card),
        column((
            text("Select Component Preset:").style(Style::new().set_text_style(TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::MEDIUM,
                color: text_secondary,
                ..Default::default()
            })),
            dropdown(Some(state.selected_framework), framework_options)
                .bg_color(if dark { rgb!(15, 23, 42) } else { clr!(white) })
                .text_color(text_primary)
                .border_color(border_card)
                .on_select(GalleryMsg::SelectFramework),
        ))
        .style(Style::new().width(Size::Percent(1.0)).gap(6.0)),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .padding(18.0)
            .gap(14.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3),
    );

    // ==========================================
    // 3. Selection & Radios Card
    // ==========================================
    let freq_options = vec![
        "High Performance (120 FPS)".to_string(),
        "Balanced (60 FPS)".to_string(),
        "Power Saver (30 FPS)".to_string(),
    ];

    let selection_card = column((
        card_header(
            "Selection & Radio Groups",
            "Checkboxes, animated switches, and radio groups",
            dark,
        ),
        row((
            checkbox(state.checkbox_notifications)
                .label("Push Notifications")
                .on_toggle(GalleryMsg::ToggleNotifications),
            checkbox(state.checkbox_analytics)
                .label("Telemetry Analytics")
                .on_toggle(GalleryMsg::ToggleAnalytics),
        ))
        .style(Style::new().gap(20.0).wrap()),
        row((
            switch(state.switch_dark_mode)
                .label("Dark Theme")
                .on_toggle(GalleryMsg::ToggleDarkMode),
            switch(state.switch_auto_save)
                .label("Auto-save State")
                .on_toggle(GalleryMsg::ToggleAutoSave),
        ))
        .style(Style::new().gap(24.0).wrap()),
        divider().color(border_card),
        column((
            text("Frame Rate & Sync Profile:").style(Style::new().set_text_style(TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::MEDIUM,
                color: text_secondary,
                ..Default::default()
            })),
            radio_group(
                state.radio_frequency,
                freq_options,
                GalleryMsg::SelectFrequency,
            ),
        ))
        .style(Style::new().gap(8.0)),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .padding(18.0)
            .gap(14.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3),
    );

    // ==========================================
    // 4. Progress Bars & Sliders Card
    // ==========================================
    let progress_card = column((
        card_header(
            "Progress & Sliders",
            "Smooth continuous sliders and flicker-free progress bars",
            dark,
        ),
        column((
            row((
                text("Volume Output").style(Style::new().set_text_style(TextStyle {
                    font_size: 13.0,
                    font_weight: FontWeight::MEDIUM,
                    color: text_primary,
                    ..Default::default()
                })),
                spacer(),
                text(format!("{:.0}%", state.slider_volume)).style(Style::new().set_text_style(
                    TextStyle {
                        font_size: 13.0,
                        font_weight: FontWeight::BOLD,
                        color: text_secondary,
                        ..Default::default()
                    },
                )),
            )),
            slider(state.slider_volume, 0.0, 100.0).on_change(GalleryMsg::SetVolume),
            progress_bar(state.slider_volume / 100.0),
        ))
        .style(Style::new().gap(6.0)),
        column((
            row((
                text("Indeterminate Background Sync").style(Style::new().set_text_style(
                    TextStyle {
                        font_size: 13.0,
                        font_weight: FontWeight::MEDIUM,
                        color: text_primary,
                        ..Default::default()
                    },
                )),
                spacer(),
                badge("Running").success(),
            )),
            progress_bar(0.0)
                .indeterminate(true)
                .fill_color(rgb!(168, 85, 247)),
        ))
        .style(Style::new().gap(6.0)),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .padding(18.0)
            .gap(14.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3),
    );

    // ==========================================
    // 5. Flex-Wrap Tag Cloud & Badges Card
    // ==========================================
    let tag_views: Vec<_> = state
        .tags
        .iter()
        .map(|t| {
            let t_clone = t.clone();
            chip(t)
                .bg_color(if dark {
                    rgb!(15, 23, 42)
                } else {
                    rgb!(241, 245, 249)
                })
                .text_color(if dark {
                    rgb!(226, 232, 240)
                } else {
                    rgb!(30, 41, 59)
                })
                .on_delete(move || GalleryMsg::DeleteTag(t_clone.clone()))
        })
        .collect();

    let tag_cloud_card = column((
        row((
            card_header(
                "Flex-Wrap Tag Cloud & Badges",
                "Dynamic multi-line flex wrapping and status pills",
                dark,
            ),
            spacer(),
            button("Reset Tags")
                .secondary()
                .on_click(GalleryMsg::ResetTags)
                .tooltip("Restores default tags"),
        ))
        .style(
            Style::new()
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        ),
        row((
            badge("System Ready").success(),
            badge("Optimization Active").info(),
            badge("Thermal Alert").warning(),
            badge("Degraded Mode").error(),
        ))
        .style(Style::new().gap(8.0).wrap()),
        divider().color(border_card),
        row(tag_views).style(
            Style::new()
                .width(Size::Percent(1.0))
                .gap(8.0)
                .wrap()
                .align_items(AlignItems::Center),
        ),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .padding(18.0)
            .gap(14.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3),
    );

    // ==========================================
    // 6. Text Inputs Card
    // ==========================================
    let inputs_card = column((
        card_header(
            "Text Editing & Inputs",
            "Single-line inputs and multiline text areas with undo/redo",
            dark,
        ),
        column((
            text("Developer Handle").style(Style::new().set_text_style(TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::MEDIUM,
                color: text_primary,
                ..Default::default()
            })),
            adapt(
                input_text().placeholder("Enter your username...").style(
                    Style::new()
                        .width(Size::Percent(1.0))
                        .height(Size::Fixed(38))
                        .padding_xy(12.0, 8.0)
                        .bg_color(input_bg)
                        .border(1.0, input_border)
                        .corner_radius(6.0)
                        .set_text_style(TextStyle {
                            font_size: 14.0,
                            color: text_primary,
                            caret_color,
                            ..Default::default()
                        }),
                ),
                GalleryState::username_input,
                GalleryMsg::UpdateUsername,
            ),
        ))
        .style(Style::new().gap(6.0)),
        column((
            text("Biography / Project Summary").style(Style::new().set_text_style(TextStyle {
                font_size: 13.0,
                font_weight: FontWeight::MEDIUM,
                color: text_primary,
                ..Default::default()
            })),
            adapt(
                text_area().style(
                    Style::new()
                        .width(Size::Percent(1.0))
                        .height(Size::Fixed(100))
                        .padding_xy(12.0, 8.0)
                        .bg_color(input_bg)
                        .border(1.0, input_border)
                        .corner_radius(6.0)
                        .set_text_style(TextStyle {
                            font_size: 14.0,
                            color: text_primary,
                            caret_color,
                            ..Default::default()
                        }),
                ),
                GalleryState::bio_textarea,
                GalleryMsg::UpdateBio,
            ),
        ))
        .style(Style::new().gap(6.0)),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .padding(18.0)
            .gap(14.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3),
    );

    // ==========================================
    // 7. Interactive Canvas Card
    // ==========================================
    let canvas_card = column((
        card_header(
            "Canvas Graphics & Shaders",
            "Procedural pixel buffer rendering at native frame rates",
            dark,
        ),
        row((
            pixel_canvas(|buf: &mut PixelBuffer| {
                let w = buf.width;
                let h = buf.height;
                for y in 0..h {
                    for x in 0..w {
                        let r = (x * 255 / w) as u8;
                        let g = (y * 255 / h) as u8;
                        let b = 180u8;
                        buf.set_pixel(x, y, rgb!(r, g, b).into());
                    }
                }
            })
            .style(
                Style::new()
                    .width(Size::Fixed(140))
                    .height(Size::Fixed(100))
                    .corner_radius(8.0)
                    .border(1.0, border_card),
            ),
            column((
                text("Procedural RGB Gradient").style(Style::new().set_text_style(TextStyle {
                    font_size: 14.0,
                    font_weight: FontWeight::SEMI_BOLD,
                    color: text_primary,
                    ..Default::default()
                })),
                text("Software rasterizer output directly blitted to GPU framebuffers without overhead.")
                    .style(Style::new().set_text_style(TextStyle {
                        font_size: 12.0,
                        color: text_secondary,
                        ..Default::default()
                    })),
            ))
            .style(Style::new().gap(4.0).flex_grow(1.0)),
        ))
        .style(Style::new().gap(16.0).align_items(AlignItems::Center)),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .padding(18.0)
            .gap(14.0)
            .corner_radius(10.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 10), 6.0, 0.3),
    );

    // ==========================================
    // Layout Grid & Header Banner
    // ==========================================
    let left_column = column((buttons_card, overlay_card, selection_card))
        .style(Style::new().width(Size::Percent(0.5)).gap(16.0));

    let right_column = column((progress_card, tag_cloud_card, inputs_card, canvas_card))
        .style(Style::new().width(Size::Percent(0.5)).gap(16.0));

    let header_banner = row((
        column((
            text("MTK Comprehensive Native Widget Gallery").style(Style::new().set_text_style(
                TextStyle {
                    font_size: 24.0,
                    font_weight: FontWeight::EXTRA_BOLD,
                    color: text_primary,
                    ..Default::default()
                },
            )),
            text("Showcasing multi-layer interactive surfaces, modal overlays, and next-gen controls.")
                .style(Style::new().set_text_style(TextStyle {
                    font_size: 14.0,
                    color: text_secondary,
                    ..Default::default()
                })),
        ))
        .style(Style::new().gap(4.0).flex_grow(1.0)),
        // Status Badge
        badge(&state.status_message).info(),
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

    let main_scaffold = column((scrollable_content,)).style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .bg_color(if dark {
                rgb!(15, 23, 42)
            } else {
                rgb!(248, 250, 252)
            })
            .transition_all(200.0, Curve::ease_out()),
    );

    // ==========================================
    // Layer 1: Fullscreen Music Player Sheet
    // ==========================================
    let player_sheet = column((
        row((
            badge("NOW PLAYING").info(),
            spacer(),
            button("✕ Close")
                .secondary()
                .on_click(GalleryMsg::OpenPlayer(false)),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .align_items(AlignItems::Center),
        ),
        divider().color(border_card),
        spacer(),
        column((
            // Album art representation
            column((spacer(),)).style(
                Style::new()
                    .width(Size::Fixed(240))
                    .height(Size::Fixed(240))
                    .corner_radius(16.0)
                    .bg_color(rgb!(99, 102, 241))
                    .shadow(rgba!(99, 102, 241, 120), 32.0, 0.5),
            ),
            column((
                text("Synthesis & Serenade").style(Style::new().set_text_style(TextStyle {
                    font_size: 26.0,
                    font_weight: FontWeight::BOLD,
                    color: text_primary,
                    ..Default::default()
                })),
                text("The Rustacean Symphony • 2026").style(Style::new().set_text_style(
                    TextStyle {
                        font_size: 15.0,
                        color: text_secondary,
                        ..Default::default()
                    },
                )),
                progress_bar(0.45).fill_color(rgb!(99, 102, 241)),
                row((
                    text("1:42").style(Style::new().set_text_style(TextStyle {
                        font_size: 13.0,
                        color: text_secondary,
                        ..Default::default()
                    })),
                    spacer(),
                    text("3:48").style(Style::new().set_text_style(TextStyle {
                        font_size: 13.0,
                        color: text_secondary,
                        ..Default::default()
                    })),
                ))
                .style(Style::new().width(Size::Percent(1.0))),
                row((
                    button("⏮ Previous").secondary(),
                    button("⏸ Pause"),
                    button("⏭ Next").secondary(),
                ))
                .style(Style::new().gap(12.0).align_items(AlignItems::Center)),
            ))
            .style(
                Style::new()
                    .gap(12.0)
                    .width(Size::Fixed(440))
                    .align_items(AlignItems::Center),
            ),
        ))
        .style(
            Style::new()
                .gap(24.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center)
                .width(Size::Percent(1.0)),
        ),
        spacer(),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .padding(32.0)
            .gap(18.0)
            .bg_color(if dark {
                rgb!(15, 23, 42)
            } else {
                rgb!(248, 250, 252)
            }),
    );

    // ==========================================
    // Layer 2: Modal Confirmation Dialog
    // ==========================================
    let modal_dialog = column((
        card_header(
            "Confirm Action",
            "Native modal dialog rendered on a dedicated top-level layer.",
            dark,
        ),
        text("Modals isolate keyboard focus, dim background surfaces, and dismiss on Escape or backdrop click.")
            .style(Style::new().set_text_style(TextStyle {
                font_size: 14.0,
                color: text_secondary,
                wrap: true,
                ..Default::default()
            })),
        divider().color(border_card),
        row((
            button("Cancel")
                .secondary()
                .on_click(GalleryMsg::OpenModal(false)),
            spacer(),
            button("Confirm & Close").on_click(GalleryMsg::OpenModal(false)),
        ))
        .style(Style::new().align_items(AlignItems::Center)),
    ))
    .style(
        Style::new()
            .width(Size::Fixed(440))
            .padding(24.0)
            .gap(16.0)
            .corner_radius(12.0)
            .bg_color(bg_card)
            .border(1.0, border_card)
            .shadow(rgba!(0, 0, 0, 80), 28.0, 0.7),
    );

    // Compose Layers using Chained Modifiers
    main_scaffold
        .sheet(state.is_player_open, player_sheet)
        .on_dismiss(|| GalleryMsg::OpenPlayer(false))
        .modal(state.is_modal_open, modal_dialog)
        .on_close(|| GalleryMsg::OpenModal(false))
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
        selected_framework: 0,
        radio_frequency: 1,
        is_modal_open: false,
        is_player_open: false,
        tags: vec![
            "Rust".into(),
            "WGPU".into(),
            "Muse Layout".into(),
            "Parley Text".into(),
            "Flexbox Wrap".into(),
            "Layer Surfaces".into(),
            "Cross Platform".into(),
            "Zero Garbage".into(),
        ],
    };

    let mut window = Window::with(initial_state, update, app);

    #[cfg(feature = "debugger")]
    window.enable_terminal_debugger();

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK - Comprehensive Native Widget Gallery")
            .with_size(WindowDimension {
                width: 1140,
                height: 880,
            }),
    );
}
