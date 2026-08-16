use std::{collections::HashMap, sync::Arc};

use bytemuck::{Pod, Zeroable};
use parley::{Affinity, Cursor, Selection};
use wgpu::CompositeAlphaMode;
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

use crate::{TextRenderInfo, TextStyle};
use crate::{effects::Filter, render::RenderCommandKind, windowing::atlas};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ImmediateData {
    pub color: [f32; 4],
    pub pos: [f32; 2],
    pub screen_size: [f32; 2],
    pub quad_size: [f32; 2],
    pub border_radius: f32,
    pub alpha: f32,
    pub border_color: [f32; 4],
    pub shadow_color: [f32; 4],
    pub border_widths: [f32; 4],
    pub shadow_spread: f32,
    pub shadow_power: f32,
    pub vibrancy: f32,
    pub vibrancy_darkness: f32,
    pub passes: f32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
}

const _: () = assert!(std::mem::size_of::<ImmediateData>() == 128);

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TextInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv_pos: [f32; 2],
    pub uv_size: [f32; 2],
    pub color: [f32; 4],
}

pub struct CanvasGpuResource {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
}

pub struct Pipelines {
    pub solid: wgpu::RenderPipeline,
    pub text: wgpu::RenderPipeline,
    pub texture: wgpu::RenderPipeline,
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
    pub atlas: atlas::Atlas,
    pub text_bind_group_layout: wgpu::BindGroupLayout,
    pub text_bind_group: wgpu::BindGroup,
    pub text_instance_buffer: wgpu::Buffer,
    pub text_instance_capacity: usize,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_sampler: wgpu::Sampler,
    pub canvas_textures: HashMap<crate::Node, CanvasGpuResource>,
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
                apply_limit_buckets: false,
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
            color_space: wgpu::SurfaceColorSpace::Auto,
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
        let atlas = atlas::Atlas::new(&device);

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

        let texture_shader = device.create_shader_module(wgpu::include_wgsl!("texture.wgsl"));
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Canvas Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Canvas Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Canvas Texture Pipeline Layout"),
                bind_group_layouts: &[Some(&texture_bind_group_layout)],
                immediate_size: std::mem::size_of::<ImmediateData>() as u32,
            });

        let texture_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Canvas Texture Pipeline"),
            layout: Some(&texture_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &texture_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &texture_shader,
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
                texture: texture_pipeline,
            },
            atlas,
            text_bind_group_layout,
            text_bind_group,
            text_instance_buffer,
            text_instance_capacity,
            texture_bind_group_layout,
            texture_sampler,
            canvas_textures: HashMap::new(),
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

    pub fn render(&mut self, context: &crate::Context) -> Option<[f32; 4]> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                return None;
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return None;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return None;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.configure_surface();
                return None;
            }
        };

        let mut text_instances: Vec<TextInstance> = Vec::new();

        struct RenderTextData {
            glyphs: std::ops::Range<usize>,
            selections: Vec<[f32; 4]>,
            strikethroughs: Vec<[f32; 4]>,
            underlines: Vec<[f32; 4]>,
            caret: Option<[f32; 4]>,
            style: TextStyle,
            alpha: f32,
        }
        let mut text_ranges: HashMap<usize, RenderTextData> = HashMap::new();
        let mut focused_caret = None;

        {
            let mut text_ctx = context.text_context.lock().unwrap();

            for (cmd_index, cmd) in context.render_list().enumerate() {
                if cmd.kind() == RenderCommandKind::Text {
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

                        let default_style = TextStyle::default();

                        let (text_style, cursor, selection, preedit_range) = if let Some(info) =
                            node.get_text_userdata::<TextRenderInfo>(context)
                        {
                            (&info.style, info.cursor, info.selection, info.preedit_range)
                        } else if let Some(style) = node.get_text_userdata::<TextStyle>(context) {
                            (style, None, None, None)
                        } else {
                            (&default_style, None, None, None)
                        };

                        let text_ctx_ref = &mut *text_ctx;

                        let layout_entry = text_ctx_ref.get_or_create_layout(
                            text,
                            text_style,
                            inner_w,
                            selection,
                            preedit_range,
                        );
                        let layout = &layout_entry.layout;
                        let actual_text_width = layout_entry.actual_text_width;
                        let actual_text_height = layout_entry.actual_text_height;

                        let horizontal_offset = match text_style.alignment {
                            parley::layout::Alignment::Center => {
                                ((inner_w - actual_text_width) / 2.0).max(0.0)
                            }
                            parley::layout::Alignment::End | parley::layout::Alignment::Right => {
                                (inner_w - actual_text_width).max(0.0)
                            }
                            _ => 0.0,
                        };

                        let vertical_offset = match text_style.vertical_alignment {
                            crate::style::VerticalAlignment::Top => 0.0,
                            crate::style::VerticalAlignment::Center => {
                                ((inner_h - actual_text_height) / 2.0).max(0.0)
                            }
                            crate::style::VerticalAlignment::Bottom => {
                                (inner_h - actual_text_height).max(0.0)
                            }
                        };

                        let text_x = computed.x + constraints.padding.left + horizontal_offset
                            - constraints.scroll.x;
                        let text_y = computed.y + constraints.padding.top + vertical_offset
                            - constraints.scroll.y;

                        let effects = node.get_effects(context).unwrap_or_default();
                        let scale = effects.scale;

                        let cx = computed.x + computed.w / 2.0;
                        let cy = computed.y + computed.h / 2.0;

                        use parley::layout::PositionedLayoutItem;

                        for line in layout.lines() {
                            for item in line.items() {
                                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                                    continue;
                                };

                                let font_data = glyph_run.run().font();
                                let font_size = glyph_run.run().font_size();
                                let font_ptr = font_data.data.as_ref().as_ptr() as usize;
                                let brush = glyph_run.style().brush;

                                let norm_coords = glyph_run.run().normalized_coords();
                                use std::hash::{Hash, Hasher};
                                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                                norm_coords.hash(&mut hasher);
                                let coords_hash = hasher.finish();

                                let mut scaler_opt = None;

                                for glyph in glyph_run.positioned_glyphs() {
                                    let raw_x = text_x + glyph.x;
                                    let raw_y = text_y + glyph.y;
                                    let subpx =
                                        ((raw_x.fract().rem_euclid(1.0) * 4.0).round() as u8) % 4;

                                    let cache_key = atlas::CacheKey {
                                        font_ptr,
                                        font_size: (font_size * 1000.0) as u32,
                                        glyph_id: glyph.id as u16,
                                        subpx,
                                        coords_hash,
                                    };

                                    let info_opt = if let Some(info) = self.atlas.get(cache_key) {
                                        Some(info)
                                    } else {
                                        if scaler_opt.is_none() {
                                            let swash_font = swash::FontRef::from_index(
                                                font_data.data.as_ref(),
                                                font_data.index as usize,
                                            )
                                            .unwrap();

                                            scaler_opt = Some(
                                                text_ctx_ref
                                                    .scale_cx
                                                    .builder(swash_font)
                                                    .size(font_size)
                                                    .hint(true)
                                                    .normalized_coords(norm_coords)
                                                    .build(),
                                            );
                                        }

                                        self.atlas.get_or_insert(
                                            &self.queue,
                                            scaler_opt.as_mut().unwrap(),
                                            cache_key,
                                        )
                                    };

                                    if let Some(info) = info_opt {
                                        if info.physical_w == 0 || info.physical_h == 0 {
                                            continue;
                                        }

                                        let base_x = raw_x.floor();
                                        let base_y = raw_y.floor();
                                        let global_x = base_x + info.offset_x as f32;
                                        let global_y = base_y + info.offset_y as f32;

                                        let dx = global_x - cx;
                                        let dy = global_y - cy;

                                        let mut color: [f32; 4] = if info.is_color {
                                            [1.0, 1.0, 1.0, brush.a as f32 / 255.0]
                                        } else {
                                            brush.into()
                                        };
                                        color[3] *= effects.opacity;

                                        text_instances.push(TextInstance {
                                            pos: [
                                                (cx + dx * scale).round(),
                                                (cy + dy * scale).round(),
                                            ],
                                            size: [
                                                (info.physical_w as f32 * scale).round(),
                                                (info.physical_h as f32 * scale).round(),
                                            ],
                                            uv_pos: [info.uv_x, info.uv_y],
                                            uv_size: [info.uv_w, info.uv_h],
                                            color,
                                        });
                                    }
                                }
                            }
                        }

                        let mut caret_rect = None;
                        if let Some(c) = cursor {
                            let cursor_layout =
                                Cursor::from_byte_index(&layout, c, Affinity::Downstream);
                            let geom = cursor_layout.geometry(&layout, 1.0); // 1.0 width caret
                            let mut ch = (geom.y1 - geom.y0) as f32;
                            if ch <= 0.0 {
                                ch = layout.height(); // Fallback to line height
                            }
                            if ch <= 0.0 {
                                ch = text_style.font_size; // Ultimate fallback
                            }
                            caret_rect = Some([
                                text_x + geom.x0 as f32,
                                text_y + geom.y0 as f32,
                                (geom.x1 - geom.x0) as f32,
                                ch,
                            ]);
                        }

                        let mut selection_rects = Vec::new();
                        if let Some((start, end)) = selection {
                            let start_cursor =
                                Cursor::from_byte_index(&layout, start, Affinity::Downstream);
                            let end_cursor =
                                Cursor::from_byte_index(&layout, end, Affinity::Upstream);

                            let selection_obj = Selection::new(start_cursor, end_cursor);
                            for rect in selection_obj.geometry(&layout) {
                                selection_rects.push([
                                    text_x + rect.0.x0 as f32,
                                    text_y + rect.0.y0 as f32,
                                    (rect.0.x1 - rect.0.x0) as f32,
                                    (rect.0.y1 - rect.0.y0) as f32,
                                ]);
                            }
                        }

                        let mut strikethroughs = Vec::new();
                        if text_style.strikethrough {
                            for line in layout.lines() {
                                let mut line_baseline: Option<f32> = None;
                                let mut min_x: Option<f32> = None;
                                let mut line_font_size = text_style.font_size;

                                for item in line.items() {
                                    if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                                        line_font_size = glyph_run.run().font_size();
                                        for glyph in glyph_run.positioned_glyphs() {
                                            if line_baseline.is_none() {
                                                line_baseline = Some(glyph.y);
                                            }
                                            let gx = glyph.x;
                                            min_x = Some(min_x.map_or(gx, |m| m.min(gx)));
                                        }
                                    }
                                }

                                if let Some(base_y) = line_baseline {
                                    let thickness = (line_font_size * 0.08).max(1.5);
                                    let line_w = line.metrics().advance;
                                    let start_x = min_x.unwrap_or(0.0);
                                    let line_y = text_y + base_y
                                        - (line_font_size * 0.28)
                                        - (thickness * 0.5);
                                    strikethroughs.push([
                                        text_x + start_x,
                                        line_y,
                                        line_w,
                                        thickness,
                                    ]);
                                }
                            }
                        }

                        let mut underlines = Vec::new();
                        if text_style.underline {
                            for line in layout.lines() {
                                let mut line_baseline: Option<f32> = None;
                                let mut min_x: Option<f32> = None;
                                let mut line_font_size = text_style.font_size;

                                for item in line.items() {
                                    if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                                        line_font_size = glyph_run.run().font_size();
                                        for glyph in glyph_run.positioned_glyphs() {
                                            if line_baseline.is_none() {
                                                line_baseline = Some(glyph.y);
                                            }
                                            let gx = glyph.x;
                                            min_x = Some(min_x.map_or(gx, |m| m.min(gx)));
                                        }
                                    }
                                }

                                if let Some(base_y) = line_baseline {
                                    let thickness = (line_font_size * 0.08).max(1.5);
                                    let line_w = line.metrics().advance;
                                    let start_x = min_x.unwrap_or(0.0);
                                    let line_y = text_y + base_y + (line_font_size * 0.12);
                                    underlines.push([text_x + start_x, line_y, line_w, thickness]);
                                }
                            }
                        }

                        if let Some((start, end)) = preedit_range {
                            if start < end {
                                let start_cursor =
                                    Cursor::from_byte_index(&layout, start, Affinity::Downstream);
                                let end_cursor =
                                    Cursor::from_byte_index(&layout, end, Affinity::Upstream);

                                let selection_obj = Selection::new(start_cursor, end_cursor);
                                let thickness = (text_style.font_size * 0.08).max(1.5);
                                for rect in selection_obj.geometry(&layout) {
                                    let u_x = text_x + rect.0.x0 as f32;
                                    let u_y = text_y + rect.0.y1 as f32 - (thickness * 0.5);
                                    let u_w = (rect.0.x1 - rect.0.x0) as f32;
                                    let u_h = thickness;
                                    underlines.push([u_x, u_y, u_w, u_h]);
                                }
                            }
                        }

                        let end = text_instances.len() as u32;

                        if Some(cmd.node()) == context.focused_node() {
                            focused_caret = caret_rect;
                        }

                        text_ranges.insert(
                            cmd_index,
                            RenderTextData {
                                glyphs: (start as usize)..(end as usize),
                                selections: selection_rects,
                                strikethroughs,
                                underlines,
                                caret: caret_rect,
                                style: text_style.clone(),
                                alpha: effects.opacity,
                            },
                        );
                    }
                }
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

        // Retain only active canvases
        self.canvas_textures
            .retain(|node, _| context.canvases.borrow().contains_key(node));

        let any_canvas_requested_frame = std::cell::Cell::new(false);

        // Execute canvas rendering
        {
            let mut canvases = context.canvases.borrow_mut();
            for (node, canvas_data) in canvases.iter_mut() {
                if let Some(computed) = node.get_computed(context) {
                    let w = (computed.w.round() as u32).max(1);
                    let h = (computed.h.round() as u32).max(1);

                    let needs_recreate = match self.canvas_textures.get(node) {
                        Some(res) => res.width != w || res.height != h,
                        None => true,
                    };

                    if needs_recreate {
                        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("Canvas Offscreen Texture"),
                            size: wgpu::Extent3d {
                                width: w,
                                height: h,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::COPY_DST,
                            view_formats: &[],
                        });

                        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                        let bind_group =
                            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some("Canvas Bind Group"),
                                layout: &self.texture_bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(&view),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(
                                            &self.texture_sampler,
                                        ),
                                    },
                                ],
                            });

                        self.canvas_textures.insert(
                            *node,
                            CanvasGpuResource {
                                texture,
                                view,
                                bind_group,
                                width: w,
                                height: h,
                            },
                        );

                        canvas_data.width = w;
                        canvas_data.height = h;

                        match &mut canvas_data.painter {
                            crate::ui::widgets::CanvasPainterKind::Pixel(_) => {
                                canvas_data.cpu_buffer.resize((w * h) as usize, 0);
                            }
                            crate::ui::widgets::CanvasPainterKind::Wgpu(p) => {
                                if !canvas_data.initialized {
                                    p.init(
                                        &self.device,
                                        &self.queue,
                                        wgpu::TextureFormat::Rgba8UnormSrgb,
                                    );
                                    canvas_data.initialized = true;
                                }
                                p.resize(&self.device, &self.queue, w, h);
                            }
                        }
                    }

                    let gpu_res = self.canvas_textures.get(node).unwrap();
                    match &mut canvas_data.painter {
                        crate::ui::widgets::CanvasPainterKind::Pixel(p) => {
                            if canvas_data.cpu_buffer.len() != (w * h) as usize {
                                canvas_data.cpu_buffer.resize((w * h) as usize, 0);
                            }
                            let mut p_buf = crate::ui::widgets::PixelBuffer::new(
                                w,
                                h,
                                &mut canvas_data.cpu_buffer,
                                &any_canvas_requested_frame,
                            );
                            p.paint(&mut p_buf);

                            self.queue.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: &gpu_res.texture,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                bytemuck::cast_slice(&canvas_data.cpu_buffer),
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(4 * w),
                                    rows_per_image: Some(h),
                                },
                                wgpu::Extent3d {
                                    width: w,
                                    height: h,
                                    depth_or_array_layers: 1,
                                },
                            );
                        }
                        crate::ui::widgets::CanvasPainterKind::Wgpu(p) => {
                            if !canvas_data.initialized {
                                p.init(
                                    &self.device,
                                    &self.queue,
                                    wgpu::TextureFormat::Rgba8UnormSrgb,
                                );
                                canvas_data.initialized = true;
                            }
                            p.prepare(&self.device, &self.queue);

                            let mut paint_ctx = crate::ui::widgets::PaintContext {
                                device: &self.device,
                                queue: &self.queue,
                                encoder: &mut encoder,
                                target: &gpu_res.view,
                                width: w,
                                height: h,
                                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                dt: context.dt,
                                frame_requested: &any_canvas_requested_frame,
                            };
                            p.paint(&mut paint_ctx);
                        }
                    }
                }
            }
        }

        if any_canvas_requested_frame.get() {
            self.window.request_redraw();
        }

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

            /// Computes an axis-aligned integer scissor rectangle for GPU rendering from logical clip bounds,
            /// clamping bounds to the window dimensions and using sub-pixel floor/ceil boundaries.
            fn compute_scissor_rect(
                clip: crate::style::Rect,
                screen_width: u32,
                screen_height: u32,
            ) -> Option<(u32, u32, u32, u32)> {
                let max_w = screen_width as f32;
                let max_h = screen_height as f32;

                let x0 = clip.x.clamp(0.0, max_w).floor() as u32;
                let y0 = clip.y.clamp(0.0, max_h).floor() as u32;
                let x1 = (clip.x + clip.w).clamp(0.0, max_w).ceil() as u32;
                let y1 = (clip.y + clip.h).clamp(0.0, max_h).ceil() as u32;

                let cw = x1.saturating_sub(x0);
                let ch = y1.saturating_sub(y0);

                if cw > 0 && ch > 0 {
                    Some((x0, y0, cw, ch))
                } else {
                    None
                }
            }

            for (cmd_index, cmd) in context.render_list().enumerate() {
                let active_scissor = if cmd.has_clip() {
                    if let Some(rect) =
                        compute_scissor_rect(cmd.clip(), self.size.width, self.size.height)
                    {
                        render_pass.set_scissor_rect(rect.0, rect.1, rect.2, rect.3);
                        Some(rect)
                    } else {
                        continue;
                    }
                } else {
                    let default_rect = (0, 0, self.size.width.max(1), self.size.height.max(1));
                    render_pass.set_scissor_rect(
                        default_rect.0,
                        default_rect.1,
                        default_rect.2,
                        default_rect.3,
                    );
                    Some(default_rect)
                };

                if cmd.kind() == RenderCommandKind::DrawQuad {
                    let node = cmd.node();
                    let canvas_res = self.canvas_textures.get(&node);
                    if let Some(res) = canvas_res {
                        render_pass.set_pipeline(&self.pipelines.texture);
                        render_pass.set_bind_group(0, &res.bind_group, &[]);
                    } else {
                        render_pass.set_pipeline(&self.pipelines.solid);
                    }
                    let effects = context.effects.get(&node).cloned().unwrap_or_default();
                    let constraints = node.get_constraints(context).unwrap_or_default();
                    let bg_color = effects.background_color;
                    let color = if canvas_res.is_some() && bg_color.a == 0 {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        bg_color.into()
                    };

                    let computed = cmd.computed();

                    let mut vibrancy = 0.0;
                    let mut vibrancy_darkness = 0.0;
                    let mut passes = 0.0;
                    for f in &effects.filters {
                        match *f {
                            Filter::Blur {
                                vibrancy: v,
                                vibrancy_darkness: vd,
                                passes: p,
                            } => {
                                vibrancy = v;
                                vibrancy_darkness = vd;
                                passes = p;
                                break;
                            }
                        }
                    }

                    let border_c = effects.border.color;
                    let border_color = border_c.into();
                    let shadow_c = effects.shadow.color;
                    let shadow_color = shadow_c.into();

                    let immediate_data = ImmediateData {
                        color,
                        pos: [
                            computed.x + (computed.w - computed.w * effects.scale) / 2.0,
                            computed.y + (computed.h - computed.h * effects.scale) / 2.0,
                        ],
                        screen_size: [self.size.width as f32, self.size.height as f32],
                        quad_size: [computed.w * effects.scale, computed.h * effects.scale],
                        border_color,
                        shadow_color,
                        border_widths: [
                            constraints.border.top,
                            constraints.border.right,
                            constraints.border.bottom,
                            constraints.border.left,
                        ],
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
                    };
                    render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                    render_pass.draw(0..6, 0..1);
                } else if cmd.kind() == RenderCommandKind::Text {
                    // Custom text clip for Overflow::Hidden
                    let node = cmd.node();
                    let constraints = node.get_constraints(context).unwrap_or_default();
                    let mut overflow_clipped = false;
                    if constraints.overflow == crate::Overflow::Hidden {
                        let computed = cmd.computed();
                        let text_clip = crate::style::Rect {
                            x: computed.x + constraints.border.left,
                            y: computed.y + constraints.border.top,
                            w: (computed.w - constraints.border.left - constraints.border.right)
                                .max(0.0),
                            h: (computed.h - constraints.border.top - constraints.border.bottom)
                                .max(0.0),
                        };

                        let effective_clip = if cmd.has_clip() {
                            let parent_clip = cmd.clip();
                            let x1 = text_clip.x.max(parent_clip.x);
                            let y1 = text_clip.y.max(parent_clip.y);
                            let x2 = (text_clip.x + text_clip.w).min(parent_clip.x + parent_clip.w);
                            let y2 = (text_clip.y + text_clip.h).min(parent_clip.y + parent_clip.h);
                            crate::style::Rect {
                                x: x1,
                                y: y1,
                                w: (x2 - x1).max(0.0),
                                h: (y2 - y1).max(0.0),
                            }
                        } else {
                            text_clip
                        };

                        if let Some((nx, ny, nw, nh)) =
                            compute_scissor_rect(effective_clip, self.size.width, self.size.height)
                        {
                            render_pass.set_scissor_rect(nx, ny, nw, nh);
                            overflow_clipped = true;
                        } else {
                            continue;
                        }
                    }

                    if let Some(range) = text_ranges.get(&cmd_index) {
                        // A) Draw Selections
                        for sel in &range.selections {
                            let immediate_data = ImmediateData {
                                color: range.style.selection_bg.into(),
                                pos: [sel[0], sel[1]],
                                screen_size: [self.size.width as f32, self.size.height as f32],
                                quad_size: [sel[2], sel[3]],
                                border_color: [0.0; 4],
                                shadow_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                border_radius: 0.0,
                                alpha: range.alpha,
                                shadow_spread: 0.0,
                                shadow_power: 0.0,
                                vibrancy: 0.0,
                                vibrancy_darkness: 0.0,
                                passes: 0.0,
                                _pad1: 0.0,
                                _pad2: 0.0,
                                _pad3: 0.0,
                            };

                            render_pass.set_pipeline(&self.pipelines.solid);
                            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                            render_pass.draw(0..6, 0..1);
                        }

                        // B) Draw Text
                        if range.glyphs.start < range.glyphs.end {
                            render_pass.set_pipeline(&self.pipelines.text);
                            render_pass.set_bind_group(0, &self.text_bind_group, &[]);

                            // we restore screen_size push constant
                            let screen_size = [self.size.width as f32, self.size.height as f32];
                            render_pass.set_immediates(0, bytemuck::bytes_of(&screen_size));

                            render_pass
                                .draw(0..6, range.glyphs.start as u32..range.glyphs.end as u32);
                        }

                        // C) Draw Caret
                        if let Some(c_rect) = &range.caret {
                            let immediate_data = ImmediateData {
                                color: range.style.caret_color.into(),
                                pos: [c_rect[0], c_rect[1]],
                                screen_size: [self.size.width as f32, self.size.height as f32],
                                quad_size: [c_rect[2], c_rect[3]],
                                border_color: [0.0; 4],
                                shadow_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                border_radius: 0.0,
                                alpha: range.alpha,
                                shadow_spread: 0.0,
                                shadow_power: 0.0,
                                vibrancy: 0.0,
                                vibrancy_darkness: 0.0,
                                passes: 0.0,
                                _pad1: 0.0,
                                _pad2: 0.0,
                                _pad3: 0.0,
                            };

                            render_pass.set_pipeline(&self.pipelines.solid);
                            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                            render_pass.draw(0..6, 0..1);
                        }

                        // D) Draw Strikethroughs
                        for strike in &range.strikethroughs {
                            let immediate_data = ImmediateData {
                                color: range.style.color.into(),
                                pos: [strike[0], strike[1]],
                                screen_size: [self.size.width as f32, self.size.height as f32],
                                quad_size: [strike[2], strike[3]],
                                border_color: [0.0; 4],
                                shadow_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                border_radius: 0.0,
                                alpha: range.alpha,
                                shadow_spread: 0.0,
                                shadow_power: 0.0,
                                vibrancy: 0.0,
                                vibrancy_darkness: 0.0,
                                passes: 0.0,
                                _pad1: 0.0,
                                _pad2: 0.0,
                                _pad3: 0.0,
                            };

                            render_pass.set_pipeline(&self.pipelines.solid);
                            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                            render_pass.draw(0..6, 0..1);
                        }

                        // E) Draw Underlines (e.g. IME Preedit Underline)
                        for u in &range.underlines {
                            let immediate_data = ImmediateData {
                                color: range.style.color.into(),
                                pos: [u[0], u[1]],
                                screen_size: [self.size.width as f32, self.size.height as f32],
                                quad_size: [u[2], u[3]],
                                border_color: [0.0; 4],
                                shadow_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                border_radius: 0.0,
                                alpha: range.alpha,
                                shadow_spread: 0.0,
                                shadow_power: 0.0,
                                vibrancy: 0.0,
                                vibrancy_darkness: 0.0,
                                passes: 0.0,
                                _pad1: 0.0,
                                _pad2: 0.0,
                                _pad3: 0.0,
                            };

                            render_pass.set_pipeline(&self.pipelines.solid);
                            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                            render_pass.draw(0..6, 0..1);
                        }
                    }

                    if overflow_clipped {
                        if let Some(rect) = active_scissor {
                            render_pass.set_scissor_rect(rect.0, rect.1, rect.2, rect.3);
                        }
                    }
                } else if cmd.kind() == RenderCommandKind::ScrollbarV {
                    render_pass.set_pipeline(&self.pipelines.solid);
                    let node = cmd.node();
                    let computed = cmd.computed();
                    let constraints = node.get_constraints(context).unwrap_or_default();
                    let content_h = node.compute_content_height(context);
                    let max_scroll_y = (content_h - computed.h).max(0.0);

                    if max_scroll_y > 0.0 {
                        let track_top = computed.y + 4.0;
                        let track_h = (computed.h - 8.0).max(0.0);
                        let thumb_h = ((track_h / content_h) * track_h).clamp(24.0, track_h);
                        let thumb_top =
                            track_top + (constraints.scroll.y / max_scroll_y) * (track_h - thumb_h);

                        let scrollbar_data = ImmediateData {
                            color: [0.4, 0.4, 0.4, 0.85],
                            pos: [computed.x + computed.w - 10.0, thumb_top],
                            screen_size: [self.size.width as f32, self.size.height as f32],
                            quad_size: [6.0, thumb_h],
                            border_color: [0.0; 4],
                            shadow_color: [0.0; 4],
                            border_widths: [0.0; 4],
                            border_radius: 3.0,
                            alpha: 0.9,
                            shadow_spread: 0.0,
                            shadow_power: 0.0,
                            vibrancy: 0.0,
                            vibrancy_darkness: 0.0,
                            passes: 0.0,
                            _pad1: 0.0,
                            _pad2: 0.0,
                            _pad3: 0.0,
                        };
                        render_pass.set_immediates(0, bytemuck::bytes_of(&scrollbar_data));
                        render_pass.draw(0..6, 0..1);
                    }
                } else if cmd.kind() == RenderCommandKind::ScrollbarH {
                    render_pass.set_pipeline(&self.pipelines.solid);
                    let node = cmd.node();
                    let computed = cmd.computed();
                    let constraints = node.get_constraints(context).unwrap_or_default();
                    let content_w = computed.content_w.max(computed.w);
                    let max_scroll_x = (content_w - computed.w).max(0.0);

                    if max_scroll_x > 0.0 {
                        let track_left = computed.x + 4.0;
                        let track_w = (computed.w - 8.0).max(0.0);
                        let thumb_w = ((track_w / content_w) * track_w).clamp(24.0, track_w);
                        let thumb_left = track_left
                            + (constraints.scroll.x / max_scroll_x) * (track_w - thumb_w);

                        let scrollbar_data_x = ImmediateData {
                            color: [0.4, 0.4, 0.4, 0.85],
                            pos: [thumb_left, computed.y + computed.h - 10.0],
                            screen_size: [self.size.width as f32, self.size.height as f32],
                            quad_size: [thumb_w, 6.0],
                            border_color: [0.0; 4],
                            shadow_color: [0.0; 4],
                            border_widths: [0.0; 4],
                            border_radius: 3.0,
                            alpha: 0.9,
                            shadow_spread: 0.0,
                            shadow_power: 0.0,
                            vibrancy: 0.0,
                            vibrancy_darkness: 0.0,
                            passes: 0.0,
                            _pad1: 0.0,
                            _pad2: 0.0,
                            _pad3: 0.0,
                        };
                        render_pass.set_immediates(0, bytemuck::bytes_of(&scrollbar_data_x));
                        render_pass.draw(0..6, 0..1);
                    }
                }
            }

            // we draw global focus ring at Z-index Infinity
            if let Some(focused) = context.focused_node() {
                if let Some(computed) = focused.get_computed(context) {
                    let effects = focused.get_effects(context).unwrap_or_default();

                    render_pass.set_scissor_rect(
                        0,
                        0,
                        self.size.width.max(1),
                        self.size.height.max(1),
                    );

                    let ring_thickness = 2.0;
                    let immediate_data = ImmediateData {
                        color: [0.0; 4],
                        pos: [computed.x - ring_thickness, computed.y - ring_thickness],
                        screen_size: [self.size.width as f32, self.size.height as f32],
                        quad_size: [
                            computed.w + ring_thickness * 2.0,
                            computed.h + ring_thickness * 2.0,
                        ],
                        border_color: [0.0, 0.47, 1.0, 1.0], // Mac-like focus ring blue
                        shadow_color: [0.0; 4],
                        border_widths: [ring_thickness; 4],
                        border_radius: effects.border.radius.tl + ring_thickness,
                        alpha: 1.0,
                        shadow_spread: 0.0,
                        shadow_power: 0.0,
                        vibrancy: 0.0,
                        vibrancy_darkness: 0.0,
                        passes: 0.0,
                        _pad1: 0.0,
                        _pad2: 0.0,
                        _pad3: 0.0,
                    };

                    render_pass.set_pipeline(&self.pipelines.solid);
                    render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                    render_pass.draw(0..6, 0..1);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(surface_texture);

        focused_caret
    }
}
