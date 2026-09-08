use std::sync::{Arc, Mutex};
use std::time::Instant;

use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::ui::{
    Event, ViewStyleExt,
    widgets::{button, column, pixel_canvas, row, text},
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{Color, clr, rgb, rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolMode {
    Paint,
    Eraser,
}

#[derive(Clone)]
struct AppState {
    draw_points: Arc<Mutex<Vec<(u32, u32, u32, Color)>>>,
    start_time: Instant,
    last_pressure: f32,
    last_tilt: Option<(f32, f32)>,
    is_eraser: bool,
    is_stylus: bool,
    is_mouse_down: bool,
    tool_mode: ToolMode,
}

#[derive(Clone, Debug)]
enum AppMsg {
    SetToolMode(ToolMode),
    MouseDown {
        x: u32,
        y: u32,
        radius: u32,
    },
    MouseMove {
        x: u32,
        y: u32,
        radius: u32,
    },
    MouseUp,
    StylusStroke {
        x: u32,
        y: u32,
        radius: u32,
        pressure: f32,
        tilt: Option<(f32, f32)>,
        is_eraser: bool,
    },
}

fn main() {
    let initial_state = AppState {
        draw_points: Arc::new(Mutex::new(Vec::new())),
        start_time: Instant::now(),
        last_pressure: 0.0,
        last_tilt: None,
        is_eraser: false,
        is_stylus: false,
        is_mouse_down: false,
        tool_mode: ToolMode::Paint,
    };

    let window = Window::with(
        initial_state,
        |state, msg: AppMsg| match msg {
            AppMsg::SetToolMode(mode) => {
                state.tool_mode = mode;
                if mode == ToolMode::Eraser {
                    state.is_eraser = true;
                } else if !state.is_stylus {
                    state.is_eraser = false;
                }
            }
            AppMsg::MouseDown { x, y, radius } => {
                state.is_stylus = false;
                state.is_mouse_down = true;
                let mut pts = state.draw_points.lock().unwrap();
                if state.tool_mode == ToolMode::Eraser {
                    let r_sq = (radius * radius) as i32;
                    pts.retain(|(px, py, ..)| {
                        let dx = *px as i32 - x as i32;
                        let dy = *py as i32 - y as i32;
                        dx * dx + dy * dy > r_sq
                    });
                } else {
                    pts.push((x, y, radius, rgba!(0, 255, 204, 255)));
                }
            }
            AppMsg::MouseMove { x, y, radius } => {
                if state.is_mouse_down && !state.is_stylus {
                    let mut pts = state.draw_points.lock().unwrap();
                    if state.tool_mode == ToolMode::Eraser {
                        let r_sq = (radius * radius) as i32;
                        pts.retain(|(px, py, ..)| {
                            let dx = *px as i32 - x as i32;
                            let dy = *py as i32 - y as i32;
                            dx * dx + dy * dy > r_sq
                        });
                    } else {
                        pts.push((x, y, radius, rgba!(0, 255, 204, 255)));
                    }
                }
            }
            AppMsg::MouseUp => {
                state.is_mouse_down = false;
            }
            AppMsg::StylusStroke {
                x,
                y,
                radius,
                pressure,
                tilt,
                is_eraser,
            } => {
                state.is_stylus = true;
                state.last_pressure = pressure;
                state.last_tilt = tilt;
                let effective_eraser = is_eraser || state.tool_mode == ToolMode::Eraser;
                state.is_eraser = effective_eraser;
                if pressure > 0.0 {
                    let mut pts = state.draw_points.lock().unwrap();
                    if effective_eraser {
                        let r_sq = (radius * radius) as i32;
                        pts.retain(|(px, py, ..)| {
                            let dx = *px as i32 - x as i32;
                            let dy = *py as i32 - y as i32;
                            dx * dx + dy * dy > r_sq
                        });
                    } else {
                        pts.push((x, y, radius, rgba!(0, 255, 204, 255)));
                    }
                }
            }
        },
        |state| {
            let pts_clone = Arc::clone(&state.draw_points);
            let start = state.start_time;
            let status_info = if state.is_stylus {
                format!(
                    "Stylus Telemetry | Mode: {:?} | Pressure: {:.2} | Tilt: {:?} | Eraser: {}",
                    state.tool_mode, state.last_pressure, state.last_tilt, state.is_eraser
                )
            } else {
                format!(
                    "Mode: {:?} | Click and drag on canvas, or draw with tablet stylus!",
                    state.tool_mode
                )
            };

            let paint_btn = button("🖌️ Paint Mode")
                .on_click(AppMsg::SetToolMode(ToolMode::Paint))
                .style(
                    Style::new()
                        .padding_xy(16.0, 8.0)
                        .corner_radius(8.0)
                        .bg_color(if state.tool_mode == ToolMode::Paint {
                            rgba!(0, 200, 160, 255)
                        } else {
                            rgba!(45, 45, 65, 200)
                        })
                        .border(
                            1.5,
                            if state.tool_mode == ToolMode::Paint {
                                rgba!(0, 255, 204, 255)
                            } else {
                                rgba!(70, 70, 95, 180)
                            },
                        ),
                );

            let eraser_btn = button("🧹 Eraser Mode")
                .on_click(AppMsg::SetToolMode(ToolMode::Eraser))
                .style(
                    Style::new()
                        .padding_xy(16.0, 8.0)
                        .corner_radius(8.0)
                        .bg_color(if state.tool_mode == ToolMode::Eraser {
                            rgba!(235, 75, 110, 255)
                        } else {
                            rgba!(45, 45, 65, 200)
                        })
                        .border(
                            1.5,
                            if state.tool_mode == ToolMode::Eraser {
                                rgba!(255, 120, 150, 255)
                            } else {
                                rgba!(70, 70, 95, 180)
                            },
                        ),
                );

            column((
                text("MTK Software PixelPainter Canvas").style(
                    Style::new().padding(10.0).set_text_style(TextStyle {
                        font_size: 20.0,
                        color: clr!(white),
                        ..Default::default()
                    }),
                ),
                row((paint_btn, eraser_btn)).style(Style::new().gap(12.0).padding_xy(0.0, 6.0)),
                text(status_info).style(Style::new().padding(5.0).set_text_style(TextStyle {
                    font_size: 13.0,
                    color: rgba!(200, 200, 220, 200),
                    ..Default::default()
                })),
                pixel_canvas(move |buf: &mut mtk::ui::widgets::PixelBuffer| {
                    let elapsed = start.elapsed().as_secs_f32();

                    // 1. Procedural plasma background using Color
                    for y in 0..buf.height {
                        let y_f = y as f32;
                        for x in 0..buf.width {
                            let x_f = x as f32;
                            let v1 = (x_f * 0.05 + elapsed * 2.0).sin();
                            let v2 = (y_f * 0.05 + elapsed * 1.5).cos();
                            let v3 = ((x_f + y_f) * 0.03 + elapsed).sin();
                            let val = ((v1 + v2 + v3 + 3.0) / 6.0 * 255.0) as u32;

                            let r = ((val * 2) % 255) as u8;
                            let g = ((val + 50) % 255) as u8;
                            let b = ((255 - val) % 255) as u8;

                            buf.set_pixel_with_color(x, y, Color::new(r, g, b, 255));
                        }
                    }

                    // 2. Overlay user drawn points with fill_rect_with_color
                    if let Ok(points) = pts_clone.lock() {
                        for &(px, py, radius, color) in points.iter() {
                            buf.fill_rect_with_color(
                                px as i32 - radius as i32,
                                py as i32 - radius as i32,
                                radius * 2,
                                radius * 2,
                                color,
                            );
                        }
                    }

                    // Request next frame to keep plasma animating smoothly
                    buf.request_frame();
                })
                .on_event(|state: &AppState, event, details| match event {
                    Event::StylusInput {
                        pressure,
                        tilt,
                        is_eraser,
                        ..
                    } => {
                        let effective_eraser = is_eraser || state.tool_mode == ToolMode::Eraser;
                        let base_radius = if effective_eraser { 26.0 } else { 18.0 };
                        Some(AppMsg::StylusStroke {
                            x: details.pixel_x.round() as u32,
                            y: details.pixel_y.round() as u32,
                            radius: (pressure * base_radius * details.scale_factor)
                                .max(2.0 * details.scale_factor)
                                .round() as u32,
                            pressure,
                            tilt,
                            is_eraser: effective_eraser,
                        })
                    }
                    Event::MouseInput { pressed: true, .. } => {
                        let radius = if state.tool_mode == ToolMode::Eraser {
                            (16.0 * details.scale_factor).round() as u32
                        } else {
                            (5.0 * details.scale_factor).round() as u32
                        };
                        Some(AppMsg::MouseDown {
                            x: details.pixel_x.round() as u32,
                            y: details.pixel_y.round() as u32,
                            radius,
                        })
                    }
                    Event::MouseInput { pressed: false, .. } => Some(AppMsg::MouseUp),
                    Event::CursorMoved { .. } => {
                        if state.is_mouse_down && !state.is_stylus {
                            let radius = if state.tool_mode == ToolMode::Eraser {
                                (16.0 * details.scale_factor).round() as u32
                            } else {
                                (5.0 * details.scale_factor).round() as u32
                            };
                            Some(AppMsg::MouseMove {
                                x: details.pixel_x.round() as u32,
                                y: details.pixel_y.round() as u32,
                                radius,
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .style(
                    Style::new()
                        .width(Size::Fixed(600))
                        .height(Size::Fixed(400))
                        .corner_radius(20.0)
                        .border(2.0, rgba!(150, 150, 255, 180)),
                ),
            ))
            .style(
                Style::new()
                    .bg_color(rgb!(24, 24, 37))
                    .padding(20.0)
                    .width(Size::Percent(1.0))
                    .height(Size::Percent(1.0))
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::Center),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK Pixel Canvas Demo")
        .with_size((800, 640).into());

    window.present_with(attrs);
}
