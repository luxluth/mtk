use mtk::image::{ImageData, ObjectFit, SvgData};
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::View;
use mtk::ui::ViewStyleExt;
use mtk::ui::widgets::{
    ScrollAxis, button, column, container, image, row, scroll_view, slider, svg, text,
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{clr, rgb, rgba};

#[derive(Clone, Debug)]
struct DemoState {
    tiger_svg: SvgData,
    procedural_image: ImageData,
    zoom_pct: f32,
    frame_width: f32,
    frame_height: f32,
    current_fit: ObjectFit,
}

#[derive(Clone, Debug)]
enum DemoMsg {
    UpdateZoom(f32),
    SetZoom(f32),
    UpdateFrameWidth(f32),
    UpdateFrameHeight(f32),
    SetDimensions(f32, f32),
    SetFit(ObjectFit),
}

fn update(state: &mut DemoState, msg: DemoMsg) {
    match msg {
        DemoMsg::UpdateZoom(z) => state.zoom_pct = z.clamp(0.25, 6.0),
        DemoMsg::SetZoom(z) => state.zoom_pct = z.clamp(0.25, 6.0),
        DemoMsg::UpdateFrameWidth(w) => state.frame_width = w.clamp(160.0, 800.0),
        DemoMsg::UpdateFrameHeight(h) => state.frame_height = h.clamp(120.0, 600.0),
        DemoMsg::SetDimensions(w, h) => {
            state.frame_width = w;
            state.frame_height = h;
        }
        DemoMsg::SetFit(fit) => state.current_fit = fit,
    }
}

fn app(state: &DemoState) -> impl View<DemoState, Message = DemoMsg> + use<> {
    let effective_w = (state.frame_width * state.zoom_pct).round() as u32;
    let effective_h = (state.frame_height * state.zoom_pct).round() as u32;

    let fit_explanation = match state.current_fit {
        ObjectFit::Contain => "Letterboxes to preserve 1:1 aspect ratio inside the box",
        ObjectFit::Cover => "Scales to fill entire box & cleanly clips excess overflow",
        ObjectFit::Fill => "Stretches/distorts to fill exact width & height",
        ObjectFit::None => "Centers at natural 200x200 pixel size with no scaling",
        ObjectFit::ScaleDown => "Scales down if box is smaller than 200x200, else natural size",
    };

    column((
        column((
            text("MTK GPU Vector SVG & Image Viewer").style(Style::new().set_text_style(
                TextStyle {
                    font_size: 22.0,
                    font_weight: FontWeight::BOLD,
                    color: rgb!(15, 23, 42),
                    ..Default::default()
                },
            )),
            text(format!(
                "Active Mode: {:?} ({}) • GPU Resolution: {}x{} px",
                state.current_fit, fit_explanation, effective_w, effective_h
            ))
            .style(Style::new().set_text_style(TextStyle {
                font_size: 13.0,
                color: rgb!(100, 116, 139),
                ..Default::default()
            })),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .padding(18.0)
                .bg_color(clr!(white))
                .border(1.0, rgb!(226, 232, 240))
                .gap(4.0),
        ),
        row((
            scroll_view(
                column((
                    text("Frame & Fit Settings").style(Style::new().set_text_style(TextStyle {
                        font_size: 15.0,
                        font_weight: FontWeight::SEMI_BOLD,
                        color: rgb!(15, 23, 42),
                        ..Default::default()
                    })),
                    column((
                        text("Aspect Ratio Presets:").style(Style::new().set_text_style(
                            TextStyle {
                                font_size: 12.0,
                                font_weight: FontWeight::SEMI_BOLD,
                                color: rgb!(71, 85, 105),
                                ..Default::default()
                            },
                        )),
                        row((
                            button("16:9 Wide").on_click(DemoMsg::SetDimensions(480.0, 270.0)),
                            button("2:1 Banner").on_click(DemoMsg::SetDimensions(500.0, 250.0)),
                        ))
                        .style(Style::new().gap(6.0)),
                        row((
                            button("1:1 Square").on_click(DemoMsg::SetDimensions(320.0, 320.0)),
                            button("4:3 Standard").on_click(DemoMsg::SetDimensions(400.0, 300.0)),
                        ))
                        .style(Style::new().gap(6.0)),
                        row((button("9:16 Tall").on_click(DemoMsg::SetDimensions(240.0, 400.0)),))
                            .style(Style::new().gap(6.0)),
                    ))
                    .style(Style::new().gap(6.0)),
                    column((
                        text("ObjectFit Mode:").style(Style::new().set_text_style(TextStyle {
                            font_size: 12.0,
                            font_weight: FontWeight::SEMI_BOLD,
                            color: rgb!(71, 85, 105),
                            ..Default::default()
                        })),
                        row((
                            button("Contain").on_click(DemoMsg::SetFit(ObjectFit::Contain)),
                            button("Cover").on_click(DemoMsg::SetFit(ObjectFit::Cover)),
                            button("Fill").on_click(DemoMsg::SetFit(ObjectFit::Fill)),
                        ))
                        .style(Style::new().gap(6.0)),
                        row((
                            button("None").on_click(DemoMsg::SetFit(ObjectFit::None)),
                            button("ScaleDown").on_click(DemoMsg::SetFit(ObjectFit::ScaleDown)),
                        ))
                        .style(Style::new().gap(6.0)),
                    ))
                    .style(Style::new().gap(6.0)),
                    column((
                        text(format!("Interactive Zoom: {:.2}x", state.zoom_pct)).style(
                            Style::new().set_text_style(TextStyle {
                                font_size: 13.0,
                                font_weight: FontWeight::SEMI_BOLD,
                                color: rgb!(15, 23, 42),
                                ..Default::default()
                            }),
                        ),
                        slider(state.zoom_pct, 0.25, 4.0)
                            .step(0.05)
                            .on_change(DemoMsg::UpdateZoom),
                        row((
                            button("0.5x").on_click(DemoMsg::SetZoom(0.5)),
                            button("1.0x").on_click(DemoMsg::SetZoom(1.0)),
                            button("2.0x").on_click(DemoMsg::SetZoom(2.0)),
                            button("4.0x").on_click(DemoMsg::SetZoom(4.0)),
                        ))
                        .style(Style::new().gap(6.0)),
                    ))
                    .style(Style::new().gap(6.0)),
                    column((
                        text(format!(
                            "Frame Base Size: {:.0} x {:.0} px",
                            state.frame_width, state.frame_height
                        ))
                        .style(Style::new().set_text_style(TextStyle {
                            font_size: 12.0,
                            color: rgb!(100, 116, 139),
                            ..Default::default()
                        })),
                        slider(state.frame_width, 160.0, 600.0)
                            .step(10.0)
                            .on_change(DemoMsg::UpdateFrameWidth),
                        slider(state.frame_height, 120.0, 450.0)
                            .step(10.0)
                            .on_change(DemoMsg::UpdateFrameHeight),
                    ))
                    .style(Style::new().gap(4.0)),
                    column((
                        text("Procedural RGBA8 Bitmap").style(Style::new().set_text_style(
                            TextStyle {
                                font_size: 12.0,
                                font_weight: FontWeight::SEMI_BOLD,
                                color: rgb!(15, 23, 42),
                                ..Default::default()
                            },
                        )),
                        image(state.procedural_image.clone())
                            .fit(state.current_fit)
                            .style(
                                Style::new()
                                    .width(Size::Fixed(200))
                                    .height(Size::Fixed(80))
                                    .corner_radius(8.0)
                                    .border(1.0, rgb!(203, 213, 225)),
                            ),
                    ))
                    .style(
                        Style::new()
                            .padding(10.0)
                            .bg_color(rgb!(248, 250, 252))
                            .border(1.0, rgb!(226, 232, 240))
                            .corner_radius(8.0)
                            .gap(6.0),
                    ),
                ))
                .style(Style::new().padding(16.0).gap(14.0)),
            )
            .axis(ScrollAxis::Vertical)
            .style(
                Style::new()
                    .width(Size::Fixed(320))
                    .height(Size::Percent(1.0))
                    .bg_color(clr!(white))
                    .border(1.0, rgb!(226, 232, 240))
                    .corner_radius(12.0),
            ),
            scroll_view(
                container((
                    column((svg(state.tiger_svg.clone()).fit(state.current_fit).style(
                        Style::new()
                            .width(Size::Fixed(effective_w))
                            .height(Size::Fixed(effective_h))
                            .corner_radius(16.0)
                            .border(1.5, rgb!(59, 130, 246))
                            .shadow(rgba!(0, 0, 0, 30), 24.0, 0.4)
                            .bg_color(clr!(white)),
                    ),))
                    .style(
                        Style::new()
                            .align_items(AlignItems::Center)
                            .justify_content(JustifyContent::Center),
                    ),
                ))
                .style(
                    Style::new()
                        .width(Size::Percent(1.0))
                        .height(Size::Percent(1.0))
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center)
                        .padding(32.0),
                ),
            )
            .axis(ScrollAxis::Both)
            .style(
                Style::new()
                    .flex_grow(1.0)
                    .height(Size::Percent(1.0))
                    .bg_color(clr!(white))
                    .corner_radius(12.0)
                    .border(1.0, rgb!(226, 232, 240)),
            ),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .flex_grow(1.0)
                .padding(16.0)
                .gap(16.0),
        ),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .bg_color(rgb!(241, 245, 249)),
    )
}

fn create_procedural_image() -> ImageData {
    let w = 128;
    let h = 64;
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);

    for y in 0..h {
        for x in 0..w {
            let r = ((x as f32 / w as f32) * 255.0) as u8;
            let g = ((y as f32 / h as f32) * 255.0) as u8;
            let b = 180u8;
            let is_checker = ((x / 8) + (y / 8)) % 2 == 0;
            let a = if is_checker { 255 } else { 200 };

            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }
    }

    ImageData::from_rgba8(w, h, pixels).unwrap()
}

fn main() {
    let tiger_svg_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/Ghostscript_Tiger.svg"
    );
    let tiger_svg = SvgData::from_file(tiger_svg_path)
        .expect("Failed to load Ghostscript_Tiger.svg from assets");

    let initial_state = DemoState {
        tiger_svg,
        procedural_image: create_procedural_image(),
        zoom_pct: 1.0,
        frame_width: 480.0,
        frame_height: 270.0,
        current_fit: ObjectFit::Contain,
    };

    let mut window = Window::with(initial_state, update, app);

    #[cfg(feature = "debugger")]
    window.enable_terminal_debugger();

    window.present_with(
        WindowAttributes::default()
            .with_decorations(true)
            .with_title("MTK Image & SVG Vector Demo")
            .with_size((1080, 780).into())
            .with_min_size(Some((800, 600).into()))
            .with_app_id("dev.mtk.image_svg"),
    );
}
