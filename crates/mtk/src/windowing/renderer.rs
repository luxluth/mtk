use std::{collections::HashMap, sync::Arc};

use bytemuck::{Pod, Zeroable};
use parley::{Affinity, AlignmentOptions, Cursor, Selection};
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
    pub atlas: atlas::Atlas,
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
            caret: Option<[f32; 4]>,
            style: TextStyle,
        }
        let mut text_ranges: HashMap<usize, RenderTextData> = HashMap::new();
        let mut focused_caret = None;

        {
            let mut text_ctx = context.text_context.lock().unwrap();
            let mut cmd_index = 0;

            for cmd in context.render_list() {
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

                        use parley::style::{LineHeight, StyleProperty};

                        let display_scale = 1.0;
                        let quantize = true;

                        let mut builder = text_ctx_ref.layout_cx.ranged_builder(
                            &mut text_ctx_ref.font_cx,
                            text,
                            display_scale,
                            quantize,
                        );

                        builder.push_default(StyleProperty::Brush(text_style.color));
                        builder.push_default(StyleProperty::FontSize(text_style.font_size));
                        builder.push_default(StyleProperty::LineHeight(
                            LineHeight::FontSizeRelative(text_style.line_height.resolve()),
                        ));
                        builder.push_default(StyleProperty::FontWeight(text_style.font_weight));
                        builder.push_default(StyleProperty::FontStyle(text_style.font_style));
                        builder.push_default(parley::style::FontFamily::from(
                            text_style.font_family.as_str(),
                        ));
                        if text_style.wrap {
                            builder.push_default(StyleProperty::OverflowWrap(
                                text_style.overflow_wrap,
                            ));
                        }

                        if let Some((start, end)) = preedit_range {
                            // Style the preedit text with an underline to show it is being composed
                            builder.push(StyleProperty::Underline(true), start..end);
                        }

                        if let Some((start, end)) = selection {
                            builder
                                .push(StyleProperty::Brush(text_style.selection_color), start..end);
                        }

                        let mut layout = builder.build(text);

                        let max_advance = if text_style.wrap && inner_w > 0.0 {
                            Some(inner_w)
                        } else {
                            None
                        };
                        layout.break_all_lines(max_advance);
                        layout.align(text_style.alignment, AlignmentOptions::default());

                        let actual_text_height = layout.height();
                        let vertical_offset = match text_style.vertical_alignment {
                            crate::style::VerticalAlignment::Top => 0.0,
                            crate::style::VerticalAlignment::Center => {
                                ((inner_h - actual_text_height) / 2.0).max(0.0)
                            }
                            crate::style::VerticalAlignment::Bottom => {
                                (inner_h - actual_text_height).max(0.0)
                            }
                        };

                        let text_x = computed.x + constraints.padding.left - constraints.scroll.x;
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
                                let swash_font = swash::FontRef::from_index(
                                    font_data.data.as_ref(),
                                    font_data.index as usize,
                                )
                                .unwrap();
                                let font_size = glyph_run.run().font_size();
                                let font_ptr = font_data.data.as_ref().as_ptr() as usize;
                                let brush = glyph_run.style().brush;

                                let mut scaler = text_ctx_ref
                                    .scale_cx
                                    .builder(swash_font)
                                    .size(font_size)
                                    .hint(true)
                                    .normalized_coords(glyph_run.run().normalized_coords())
                                    .build();

                                for glyph in glyph_run.positioned_glyphs() {
                                    let cache_key = atlas::CacheKey {
                                        font_ptr,
                                        font_size: (font_size * 1000.0) as u32,
                                        glyph_id: glyph.id as u16,
                                    };

                                    if let Some(info) = self.atlas.get_or_insert(
                                        &self.queue,
                                        &mut scaler,
                                        cache_key,
                                    ) {
                                        let global_x = text_x + glyph.x + info.offset_x as f32;
                                        // `glyph.y` is already positioned by `positioned_glyphs()`
                                        let global_y = text_y + glyph.y + info.offset_y as f32;

                                        let dx = global_x - cx;
                                        let dy = global_y - cy;

                                        let color = if info.is_color {
                                            [1.0, 1.0, 1.0, brush.a as f32 / 255.0]
                                        } else {
                                            brush.into()
                                        };

                                        text_instances.push(TextInstance {
                                            pos: [
                                                (cx + dx * scale).round(),
                                                (cy + dy * scale).round(),
                                            ],
                                            size: [
                                                info.physical_w as f32 * scale,
                                                info.physical_h as f32 * scale,
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
                            let geom = cursor_layout.geometry(&layout, 2.0); // 2.0 width caret
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

                        let end = text_instances.len() as u32;

                        if Some(cmd.node()) == context.focused_node() {
                            focused_caret = caret_rect;
                        }

                        text_ranges.insert(
                            cmd_index,
                            RenderTextData {
                                glyphs: (start as usize)..(end as usize),
                                selections: selection_rects,
                                caret: caret_rect,
                                style: text_style.clone(),
                            },
                        );
                    }
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
                if cmd.has_clip() {
                    let clip = cmd.clip();
                    let x = clip.x.max(0.0) as u32;
                    let y = clip.y.max(0.0) as u32;
                    let w = clip.w.max(0.0) as u32;
                    let h = clip.h.max(0.0) as u32;

                    let max_w = self.size.width.max(1);
                    let max_h = self.size.height.max(1);

                    let cx = x.min(max_w);
                    let cy = y.min(max_h);
                    let cw = w.min(max_w.saturating_sub(cx));
                    let ch = h.min(max_h.saturating_sub(cy));

                    if cw == 0 || ch == 0 {
                        cmd_index += 1;
                        continue;
                    }

                    render_pass.set_scissor_rect(cx, cy, cw, ch);
                } else {
                    render_pass.set_scissor_rect(
                        0,
                        0,
                        self.size.width.max(1),
                        self.size.height.max(1),
                    );
                }

                if cmd.kind() == RenderCommandKind::DrawQuad {
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
                } else if cmd.kind() == RenderCommandKind::Text {
                    // Custom text clip for Overflow::Hidden
                    let node = cmd.node();
                    let constraints = node.get_constraints(context).unwrap_or_default();
                    if constraints.overflow == crate::Overflow::Hidden {
                        let computed = cmd.computed();
                        let cx = (computed.x + constraints.border.left).max(0.0) as u32;
                        let cy = (computed.y + constraints.border.top).max(0.0) as u32;
                        let cw = (computed.w - constraints.border.left - constraints.border.right)
                            .max(0.0) as u32;
                        let ch = (computed.h - constraints.border.top - constraints.border.bottom)
                            .max(0.0) as u32;

                        let mut nx = cx;
                        let mut ny = cy;
                        let mut nw = cw;
                        let mut nh = ch;

                        if cmd.has_clip() {
                            let clip = cmd.clip();
                            let ccx = clip.x.max(0.0) as u32;
                            let ccy = clip.y.max(0.0) as u32;
                            let ccw = clip.w.max(0.0) as u32;
                            let cch = clip.h.max(0.0) as u32;

                            let max_x = (nx + nw).min(ccx + ccw);
                            let max_y = (ny + nh).min(ccy + cch);
                            nx = nx.max(ccx);
                            ny = ny.max(ccy);
                            nw = max_x.saturating_sub(nx);
                            nh = max_y.saturating_sub(ny);
                        }

                        let max_w = self.size.width.max(1);
                        let max_h = self.size.height.max(1);
                        nx = nx.min(max_w);
                        ny = ny.min(max_h);
                        nw = nw.min(max_w.saturating_sub(nx));
                        nh = nh.min(max_h.saturating_sub(ny));

                        if nw == 0 || nh == 0 {
                            cmd_index += 1;
                            continue;
                        }

                        render_pass.set_scissor_rect(nx, ny, nw, nh);
                    }

                    if let Some(range) = text_ranges.get(&cmd_index) {
                        // A) Draw Selections
                        for sel in &range.selections {
                            let immediate_data = ImmediateData {
                                color: range.style.selection_bg.into(),
                                border_color: [0.0; 4],
                                pos: [sel[0], sel[1]],
                                screen_size: [self.size.width as f32, self.size.height as f32],
                                quad_size: [sel[2], sel[3]],
                                src_offset: [0.0, 0.0],
                                src_size: [0.0, 0.0],
                                border_radius: 0.0,
                                alpha: 1.0,
                                shadow_spread: 0.0,
                                shadow_power: 0.0,
                                vibrancy: 0.0,
                                vibrancy_darkness: 0.0,
                                passes: 0.0,
                                _pad1: 0.0,
                                _pad2: 0.0,
                                _pad3: 0.0,
                                border_widths: [0.0; 4],
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
                            // println!("DRAWING CARET: {:?}", c_rect);
                            let immediate_data = ImmediateData {
                                color: range.style.caret_color.into(),
                                border_color: [0.0; 4],
                                pos: [c_rect[0], c_rect[1]],
                                screen_size: [self.size.width as f32, self.size.height as f32],
                                quad_size: [c_rect[2], c_rect[3]],
                                src_offset: [0.0, 0.0],
                                src_size: [0.0, 0.0],
                                border_radius: 0.0,
                                alpha: 1.0,
                                shadow_spread: 0.0,
                                shadow_power: 0.0,
                                vibrancy: 0.0,
                                vibrancy_darkness: 0.0,
                                passes: 0.0,
                                _pad1: 0.0,
                                _pad2: 0.0,
                                _pad3: 0.0,
                                border_widths: [0.0; 4],
                            };

                            render_pass.set_pipeline(&self.pipelines.solid);
                            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                            render_pass.draw(0..6, 0..1);
                        }
                    }
                }
                cmd_index += 1;
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
                        // TODO: make this customizable
                        border_color: [0.0, 0.47, 1.0, 1.0], // Mac-like focus ring blue
                        pos: [computed.x - ring_thickness, computed.y - ring_thickness],
                        screen_size: [self.size.width as f32, self.size.height as f32],
                        quad_size: [
                            computed.w + ring_thickness * 2.0,
                            computed.h + ring_thickness * 2.0,
                        ],
                        src_offset: [0.0, 0.0],
                        src_size: [0.0, 0.0],
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
                        border_widths: [ring_thickness; 4],
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
