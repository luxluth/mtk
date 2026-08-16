use std::sync::{Arc, Mutex};
use std::time::Instant;

use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::ui::{
    Event, ViewStyleExt,
    widgets::{column, pixel_canvas, text},
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{Color, clr, rgb, rgba};

#[derive(Clone)]
struct AppState {
    draw_points: Arc<Mutex<Vec<(u32, u32, Color)>>>,
    start_time: Instant,
}

#[derive(Clone, Debug)]
enum AppMsg {
    DrawPixel { x: u32, y: u32 },
}

fn main() {
    let initial_state = AppState {
        draw_points: Arc::new(Mutex::new(Vec::new())),
        start_time: Instant::now(),
    };

    let mut window = Window::with(
        initial_state,
        |state, msg: AppMsg| match msg {
            AppMsg::DrawPixel { x, y } => {
                let mut pts = state.draw_points.lock().unwrap();
                pts.push((x, y, rgba!(0, 255, 204, 255))); // Teal drawing color
            }
        },
        |state| {
            let pts_clone = Arc::clone(&state.draw_points);
            let start = state.start_time;

            column((
                text("MTK Software PixelPainter Canvas").style(
                    Style::new().padding(10.0).set_text_style(TextStyle {
                        font_size: 20.0,
                        color: clr!(white),
                        ..Default::default()
                    }),
                ),
                text("Move mouse on the canvas to draw!").style(
                    Style::new().padding(5.0).set_text_style(TextStyle {
                        font_size: 13.0,
                        color: rgba!(200, 200, 220, 200),
                        ..Default::default()
                    }),
                ),
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
                        for &(px, py, color) in points.iter() {
                            buf.fill_rect_with_color(px as i32 - 4, py as i32 - 4, 8, 8, color);
                        }
                    }

                    // Request next frame to keep plasma animating smoothly
                    buf.request_frame();
                })
                .on_event(|_state, event, details| match event {
                    Event::CursorMoved { .. } | Event::MouseInput { pressed: true, .. } => {
                        Some(AppMsg::DrawPixel {
                            x: details.local_x.round() as u32,
                            y: details.local_y.round() as u32,
                        })
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
        .with_size((800, 600).into());

    window.present_with(attrs);
}
