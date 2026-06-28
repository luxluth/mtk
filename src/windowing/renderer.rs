use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::CompositeAlphaMode;
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

use crate::{colors::Color, effects::Filter};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ImmediateData {
    pub color: [f32; 4],
    pub pos: [f32; 2],

    pub screen_size: [f32; 2],

    pub quad_size: [f32; 2],
    pub src_offset: [f32; 2],
    pub src_size: [f32; 2],

    pub border_radius: f32,
    pub alpha: f32,
    pub shadow_spread: f32,
    pub shadow_power: f32,
    pub vibrancy: f32,
    pub vibrancy_darkness: f32,
    pub passes: f32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
    pub border_widths: [f32; 4],
    pub border_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TextInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv_pos: [f32; 2],
    pub uv_size: [f32; 2],
    pub color: [f32; 4],
}

pub struct Pipelines {
    pub solid: wgpu::RenderPipeline,
    pub text: wgpu::RenderPipeline,
}

pub struct Renderer<'w> {
    instance: wgpu::Instance,
    window: Arc<Window>,
    pub surface: wgpu::Surface<'w>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface_format: wgpu::TextureFormat,
    pub pipelines: Pipelines,
    pub atlas: crate::windowing::atlas::Atlas,
    pub text_bind_group_layout: wgpu::BindGroupLayout,
    pub text_bind_group: wgpu::BindGroup,
    pub text_instance_buffer: wgpu::Buffer,
    pub text_instance_capacity: usize,
}

impl<'w> Renderer<'w> {
    pub async fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display),
        ));

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::IMMEDIATES,
                required_limits: wgpu::Limits {
                    max_immediate_size: 128,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: *surface_caps
                .alpha_modes
                .iter()
                .find(|mode| **mode == CompositeAlphaMode::PreMultiplied)
                .unwrap_or(&CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let shader = device.create_shader_module(wgpu::include_wgsl!("solid.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Solid Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: std::mem::size_of::<ImmediateData>() as u32,
        });

        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Solid Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let text_shader = device.create_shader_module(wgpu::include_wgsl!("text.wgsl"));
        let atlas = crate::windowing::atlas::Atlas::new(&device);

        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(&text_bind_group_layout)],
            immediate_size: 8, // Push constant for screen_size: vec2<f32>
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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

        let text_instance_capacity = 1024;
        let text_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Instance Buffer"),
            size: (std::mem::size_of::<TextInstance>() * text_instance_capacity)
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout: &text_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: text_instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&atlas.sampler),
                },
            ],
        });

        Self {
            instance,
            window,
            surface,
            device,
            queue,
            config,
            size,
            surface_format,
            pipelines: Pipelines {
                solid: solid_pipeline,
                text: text_pipeline,
            },
            atlas,
            text_bind_group_layout,
            text_bind_group,
            text_instance_buffer,
            text_instance_capacity,
        }
    }

    pub(crate) fn resize(&mut self, physical_size: PhysicalSize<u32>) {
        if physical_size.width > 0 && physical_size.height > 0 {
            self.size = physical_size;
            self.config.width = physical_size.width;
            self.config.height = physical_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn configure_surface(&self) {
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self, context: &crate::Context) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.configure_surface();
                return;
            }
        };

        let mut text_instances = Vec::new();
        let mut text_ranges = std::collections::HashMap::new();

        {
            let mut text_ctx = context.text_context.lock().unwrap();
            let mut cmd_index = 0;

            for cmd in context.render_list() {
                if cmd.kind() == crate::render::RenderCommandKind::Text {
                    let start = text_instances.len() as u32;
                    let node = cmd.node();
                    if let Some(text) = node.get_text(context) {
                        let computed = cmd.computed();
                        let constraints = node.get_constraints(context).unwrap_or_default();

                        let inner_w =
                            (computed.w - constraints.padding.left - constraints.padding.right)
                                .max(0.0);
                        let inner_h =
                            (computed.h - constraints.padding.top - constraints.padding.bottom)
                                .max(0.0);

                        let default_style = crate::ui::style::TextStyle::default();
                        let text_style = node
                            .get_text_userdata::<crate::ui::style::TextStyle>(context)
                            .unwrap_or(&default_style);

                        let text_ctx_ref = &mut *text_ctx;
                        use cosmic_text::{Buffer, Metrics, Shaping};
                        let mut buffer = Buffer::new(
                            &mut text_ctx_ref.font_system,
                            Metrics::new(text_style.font_size, text_style.line_height),
                        );
                        buffer.set_size(Some(inner_w), Some(inner_h));
                        buffer.set_text(
                            text,
                            &text_style.attrs.as_attrs(),
                            Shaping::Advanced,
                            Some(text_style.alignement),
                        );
                        buffer.shape_until_scroll(&mut text_ctx_ref.font_system, true);

                        let actual_text_height =
                            buffer.layout_runs().count() as f32 * text_style.line_height;
                        let vertical_offset = ((inner_h - actual_text_height) / 2.0).max(0.0);

                        let text_x = computed.x + constraints.padding.left;
                        let text_y = computed.y + constraints.padding.top + vertical_offset;

                        let effects = node.get_effects(context).unwrap_or_default();
                        let scale = effects.scale;

                        let cx = computed.x + computed.w / 2.0;
                        let cy = computed.y + computed.h / 2.0;

                        let default_color: Color = text_style
                            .attrs
                            .as_attrs()
                            .color_opt
                            .map_or(Color::black, |c| Color::from(c));

                        for run in buffer.layout_runs() {
                            for glyph in run.glyphs.iter() {
                                let physical_glyph =
                                    glyph.physical((text_x, text_y + run.line_y), 1.0);
                                if let Some(info) = self.atlas.get_or_insert(
                                    &self.queue,
                                    &mut text_ctx_ref.swash_cache,
                                    &mut text_ctx_ref.font_system,
                                    physical_glyph.cache_key,
                                ) {
                                    let dx = physical_glyph.x as f32 + info.offset_x as f32 - cx;
                                    let dy = physical_glyph.y as f32 + info.offset_y as f32 - cy;

                                    text_instances.push(TextInstance {
                                        pos: [(cx + dx * scale).round(), (cy + dy * scale).round()],
                                        size: [
                                            info.physical_w as f32 * scale,
                                            info.physical_h as f32 * scale,
                                        ],
                                        uv_pos: [info.uv_x, info.uv_y],
                                        uv_size: [info.uv_w, info.uv_h],
                                        color: default_color.into(),
                                    });
                                }
                            }
                        }
                    }
                    let end = text_instances.len() as u32;
                    text_ranges.insert(cmd_index, start..end);
                }
                cmd_index += 1;
            }
        }

        if !text_instances.is_empty() {
            if text_instances.len() > self.text_instance_capacity {
                self.text_instance_capacity = (text_instances.len() * 2).max(1024);
                self.text_instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Text Instance Buffer"),
                    size: (std::mem::size_of::<TextInstance>() * self.text_instance_capacity)
                        as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.text_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Text Bind Group"),
                    layout: &self.text_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.text_instance_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&self.atlas.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.atlas.sampler),
                        },
                    ],
                });
            }
            self.queue.write_buffer(
                &self.text_instance_buffer,
                0,
                bytemuck::cast_slice(&text_instances),
            );
        }

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let mut cmd_index = 0;
            for cmd in context.render_list() {
                if cmd.kind() == crate::render::RenderCommandKind::DrawQuad {
                    render_pass.set_pipeline(&self.pipelines.solid);
                    let node = cmd.node();
                    let effects = context.effects.get(&node).cloned().unwrap_or_default();
                    let constraints = node.get_constraints(context).unwrap_or_default();
                    let bg_color = effects.background_color;
                    let color = bg_color.into();

                    let computed = cmd.computed();

                    let mut vibrancy = 0.0;
                    let mut vibrancy_darkness = 0.0;
                    let mut passes = 0.0;
                    for f in &effects.filters {
                        match f {
                            Filter::Blur {
                                vibrancy: v,
                                vibrancy_darkness: vd,
                                passes: p,
                            } => {
                                vibrancy = *v;
                                vibrancy_darkness = *vd;
                                passes = *p;
                                break;
                            }
                        }
                    }

                    let border_c = effects.border.color;
                    let border_color = border_c.into();

                    let immediate_data = ImmediateData {
                        color,
                        border_color,
                        pos: [
                            computed.x + (computed.w - computed.w * effects.scale) / 2.0,
                            computed.y + (computed.h - computed.h * effects.scale) / 2.0,
                        ],
                        screen_size: [self.size.width as f32, self.size.height as f32],
                        quad_size: [computed.w * effects.scale, computed.h * effects.scale],
                        src_offset: [0.0, 0.0],
                        src_size: [0.0, 0.0],
                        border_radius: effects.border.radius.tl,
                        alpha: effects.opacity,
                        shadow_spread: effects.shadow.spread,
                        shadow_power: effects.shadow.power,
                        vibrancy,
                        vibrancy_darkness,
                        passes,
                        _pad1: 0.0,
                        _pad2: 0.0,
                        _pad3: 0.0,
                        border_widths: [
                            constraints.border.top,
                            constraints.border.right,
                            constraints.border.bottom,
                            constraints.border.left,
                        ],
                    };

                    render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                    render_pass.draw(0..6, 0..1);
                } else if cmd.kind() == crate::render::RenderCommandKind::Text {
                    if let Some(range) = text_ranges.get(&cmd_index) {
                        if range.start < range.end {
                            render_pass.set_pipeline(&self.pipelines.text);
                            render_pass.set_bind_group(0, &self.text_bind_group, &[]);
                            let screen_size = [self.size.width as f32, self.size.height as f32];
                            render_pass.set_immediates(0, bytemuck::bytes_of(&screen_size));
                            render_pass.draw(0..6, range.start..range.end);
                        }
                    }
                }
                cmd_index += 1;
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        surface_texture.present();
    }
}
