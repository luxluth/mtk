use std::sync::{Arc, Mutex};
use std::time::Instant;

use mtk::bytemuck;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::ui::{
    Event, ViewStyleExt,
    widgets::{PaintContext, WgpuPainter, column, row, text, wgpu_canvas},
};
use mtk::wgpu;
use mtk::windowing::{Window, WindowAttributes};
use mtk::{clr, rgb, rgba};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderUniforms {
    time: f32,
    aspect: f32,
    mouse_x: f32,
    mouse_y: f32,
    resolution_x: f32,
    resolution_y: f32,
    mouse_pressed: f32,
    _pad: f32,
}

#[derive(Clone)]
struct CyberGyroidPainter {
    start_time: Instant,
    mouse_coords: Arc<Mutex<(f32, f32)>>,
    is_pressed: Arc<Mutex<bool>>,
    pipeline: Option<Arc<wgpu::RenderPipeline>>,
    bind_group: Option<Arc<wgpu::BindGroup>>,
    uniform_buffer: Option<Arc<wgpu::Buffer>>,
    width: u32,
    height: u32,
}

impl CyberGyroidPainter {
    pub fn new(mouse_coords: Arc<Mutex<(f32, f32)>>, is_pressed: Arc<Mutex<bool>>) -> Self {
        Self {
            start_time: Instant::now(),
            mouse_coords,
            is_pressed,
            pipeline: None,
            bind_group: None,
            uniform_buffer: None,
            width: 700,
            height: 480,
        }
    }
}

const RAYMARCH_SHADER: &str = include_str!("./raymarch.wgsl");

impl WgpuPainter for CyberGyroidPainter {
    fn init(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cyber Gyroid Raymarch Shader"),
            source: wgpu::ShaderSource::Wgsl(RAYMARCH_SHADER.into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Raymarch Uniform Buffer"),
            size: std::mem::size_of::<ShaderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raymarch Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raymarch Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raymarch Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cyber Gyroid Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        self.pipeline = Some(Arc::new(pipeline));
        self.bind_group = Some(Arc::new(bind_group));
        self.uniform_buffer = Some(Arc::new(uniform_buffer));
    }

    fn resize(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn prepare(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        if let Some(buf) = &self.uniform_buffer {
            let elapsed = self.start_time.elapsed().as_secs_f32();
            let (mx, my) = *self.mouse_coords.lock().unwrap();
            let is_pressed = *self.is_pressed.lock().unwrap();

            let uniforms = ShaderUniforms {
                time: elapsed,
                aspect: self.width as f32 / self.height.max(1) as f32,
                mouse_x: mx,
                mouse_y: my,
                resolution_x: self.width as f32,
                resolution_y: self.height as f32,
                mouse_pressed: if is_pressed { 1.0 } else { 0.0 },
                _pad: 0.0,
            };
            queue.write_buffer(buf, 0, bytemuck::bytes_of(&uniforms));
        }
    }

    fn paint(&mut self, ctx: &mut PaintContext) {
        {
            let mut rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Cyber Gyroid Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ctx.target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.015,
                            g: 0.015,
                            b: 0.035,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if let (Some(pipeline), Some(bind_group)) = (&self.pipeline, &self.bind_group) {
                rpass.set_pipeline(pipeline);
                rpass.set_bind_group(0, bind_group.as_ref(), &[]);
                rpass.draw(0..6, 0..1);
            }
        }

        // Keep continuous 60fps/120fps animation ticking on demand
        ctx.request_frame();
    }
}

#[derive(Clone)]
struct AppState {
    mouse_coords: Arc<Mutex<(f32, f32)>>,
    is_pressed: Arc<Mutex<bool>>,
}

#[derive(Clone, Debug)]
enum AppMsg {
    MouseMove { u: f32, v: f32 },
    MouseClick { pressed: bool },
}

fn main() {
    let mouse_coords = Arc::new(Mutex::new((0.5, 0.5)));
    let is_pressed = Arc::new(Mutex::new(false));

    let painter = CyberGyroidPainter::new(Arc::clone(&mouse_coords), Arc::clone(&is_pressed));

    let initial_state = AppState {
        mouse_coords,
        is_pressed,
    };

    let mut window = Window::with(
        initial_state,
        |state, msg: AppMsg| match msg {
            AppMsg::MouseMove { u, v } => {
                if let Ok(mut coords) = state.mouse_coords.lock() {
                    *coords = (u, v);
                }
            }
            AppMsg::MouseClick { pressed } => {
                if let Ok(mut p) = state.is_pressed.lock() {
                    *p = pressed;
                }
            }
        },
        move |_state| {
            column((
                // Header section
                row((
                    text("⚡ MTK Cybernetic Gyroid").style(
                        Style::new().padding(4.0).set_text_style(TextStyle {
                            font_size: 24.0,
                            color: clr!(white),
                            ..Default::default()
                        }),
                    ),
                    text("WGPU Raymarching Engine").style(
                        Style::new()
                            .padding_xy(8.0, 4.0)
                            .bg_color(rgba!(60, 80, 160, 180))
                            .corner_radius(8.0)
                            .set_text_style(TextStyle {
                                font_size: 11.0,
                                color: rgba!(180, 220, 255, 255),
                                ..Default::default()
                            }),
                    ),
                ))
                .style(
                    Style::new()
                        .gap(12.0)
                        .align_items(AlignItems::Center)
                        .padding_xy(0.0, 6.0),
                ),
                text("Interactive 3D raymarched volumetric distance field with Fresnel rim bloom and specular lighting.")
                    .style(
                        Style::new().padding_xy(0.0, 6.0).set_text_style(TextStyle {
                            font_size: 13.0,
                            color: rgba!(160, 175, 210, 220),
                            ..Default::default()
                        }),
                    ),
                // Custom WGPU Canvas
                wgpu_canvas(painter.clone())
                    .on_event(|_state, event, details| match event {
                        Event::CursorMoved { .. } => Some(AppMsg::MouseMove {
                            u: details.uv_x,
                            v: details.uv_y,
                        }),
                        Event::MouseInput { pressed, .. } => {
                            Some(AppMsg::MouseClick { pressed })
                        }
                        _ => None,
                    })
                    .style(
                        Style::new()
                            .width(Size::Fixed(720))
                            .height(Size::Fixed(460))
                            .corner_radius(24.0)
                            .border(2.0, rgba!(80, 140, 255, 180)),
                    ),
                // Footer instructions
                text("✨ Drag cursor across the canvas to orbit the 3D scene in real-time").style(
                    Style::new().padding_xy(0.0, 8.0).set_text_style(TextStyle {
                        font_size: 12.0,
                        color: rgba!(130, 150, 190, 180),
                        ..Default::default()
                    }),
                ),
            ))
            .style(
                Style::new()
                    .bg_color(rgb!(12, 14, 22))
                    .padding(24.0)
                    .width(Size::Percent(1.0))
                    .height(Size::Percent(1.0))
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::Center),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK 3D Cybernetic Gyroid Raymarching Canvas")
        .with_size((860, 680).into());

    window.present_with(attrs);
}
