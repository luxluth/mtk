pub(crate) mod atlas;
pub(crate) mod blur;
pub(crate) mod pipelines;
pub(crate) mod scene;
pub(crate) mod text_batch;

use self::atlas::Atlas;
use self::blur::BlurPipeline;
use self::pipelines::{ImmediateData, Pipelines};
use self::scene::SceneTarget;
use self::text_batch::{RenderTextData, TextBatch};
use crate::effects::Filter;
use crate::render::RenderCommandKind;
use std::collections::HashMap;
use std::sync::Arc;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

pub struct CanvasGpuResource {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
}

pub struct Renderer<'w> {
    _instance: wgpu::Instance,
    window: Arc<Window>,
    pub surface: wgpu::Surface<'w>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface_format: wgpu::TextureFormat,
    pub pipelines: Pipelines,
    pub atlas: Atlas,
    pub blur: BlurPipeline,
    pub scene: SceneTarget,
    pub text_batch: TextBatch,
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
                label: Some("Primary GPU Device"),
                required_features: wgpu::Features::IMMEDIATES,
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::default(),
                ..Default::default()
            })
            .await
            .unwrap();

        let mut cap = surface.get_capabilities(&adapter);
        let surface_format = cap
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(cap.formats[0]);

        cap.alpha_modes.sort_by_key(|mode| match mode {
            wgpu::CompositeAlphaMode::PreMultiplied => 0,
            wgpu::CompositeAlphaMode::PostMultiplied => 1,
            wgpu::CompositeAlphaMode::Inherit => 2,
            wgpu::CompositeAlphaMode::Auto => 3,
            wgpu::CompositeAlphaMode::Opaque => 4,
        });

        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .unwrap();
        config.view_formats.push(surface_format.add_srgb_suffix());
        config.alpha_mode = cap.alpha_modes[0];
        config.desired_maximum_frame_latency = 2;
        config.format = surface_format;

        surface.configure(&device, &config);

        let pipelines = Pipelines::new(&device, &config);
        let atlas = Atlas::new(&device);
        let blur = BlurPipeline::new(&device, size.width, size.height);
        let scene = SceneTarget::new(
            &device,
            &pipelines.texture_bind_group_layout,
            &pipelines.texture_sampler,
            surface_format,
            size.width,
            size.height,
        );
        let text_batch = TextBatch::new(
            &device,
            &pipelines.text_bind_group_layout,
            &atlas.view,
            &atlas.sampler,
        );

        Self {
            _instance: instance,
            window,
            surface,
            device,
            queue,
            config,
            size,
            surface_format,
            pipelines,
            atlas,
            blur,
            scene,
            text_batch,
            canvas_textures: HashMap::new(),
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.size = size;
            self.config.width = size.width;
            self.config.height = size.height;
            self.configure_surface();

            self.scene.resize(
                &self.device,
                &self.pipelines.texture_bind_group_layout,
                &self.pipelines.texture_sampler,
                size.width,
                size.height,
            );
            self.blur.resize(&self.device, size.width, size.height);
        }
    }

    fn configure_surface(&self) {
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self, context: &crate::Context) -> Option<[f32; 4]> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return None;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.configure_surface();
                return None;
            }
            _ => {
                return None;
            }
        };

        // 1. Prepare text batch and extract focused caret
        let (text_ranges, focused_caret) = self.text_batch.prepare(
            &self.device,
            &self.queue,
            &mut self.atlas,
            &self.pipelines.text_bind_group_layout,
            context,
        );

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

        // 2. Update and paint active canvas widgets
        self.update_canvas_textures(context, &mut encoder);

        // 3. Determine if multi-pass background blur is required
        let first_vibrancy_index = context.render_list().enumerate().find_map(|(idx, cmd)| {
            if cmd.kind() == RenderCommandKind::DrawQuad {
                let node = cmd.node();
                let effects = context.effects.get(&node).cloned().unwrap_or_default();
                for f in &effects.filters {
                    let Filter::Blur { vibrancy, .. } = *f;
                    if vibrancy > 0.0 {
                        return Some(idx);
                    }
                }
            }
            None
        });

        if let Some(split_idx) = first_vibrancy_index {
            // MULTI-PASS FROSTED GLASS PIPELINE //

            // Pass 1: Render background scene into offscreen scene target
            {
                let mut scene_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Scene Background Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.scene.view,
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

                let commands = context.render_list().enumerate().take(split_idx);
                render_command_slice(
                    &mut scene_pass,
                    commands,
                    self.size.width,
                    self.size.height,
                    &self.pipelines,
                    &self.pipelines.dummy_solid_bind_group,
                    &self.text_batch.bind_group,
                    &text_ranges,
                    &self.canvas_textures,
                    context,
                );
            }

            // Pass 2: 5-pass Dual Kawase compute blur on background scene
            let blurred_view = self.blur.execute(
                &self.device,
                &mut encoder,
                &self.scene.view,
                self.size.width,
                self.size.height,
            );

            let blurred_solid_bind_group =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Blurred Solid Bind Group"),
                    layout: &self.pipelines.solid_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(blurred_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                &self.pipelines.texture_sampler,
                            ),
                        },
                    ],
                });

            // Pass 3: Blit sharp scene, render frosted glass quads and foreground onto surface
            {
                let mut surface_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Surface Foreground Pass"),
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

                // Blit unblurred background scene onto surface
                let screen_w = self.size.width as f32;
                let screen_h = self.size.height as f32;
                let blit_immediate = ImmediateData {
                    color: [1.0, 1.0, 1.0, 1.0],
                    pos: [0.0, 0.0],
                    screen_size: [screen_w, screen_h],
                    quad_size: [screen_w, screen_h],
                    border_radius: 0.0,
                    alpha: 1.0,
                    border_color: [0.0; 4],
                    shadow_color: [0.0; 4],
                    border_widths: [0.0; 4],
                    shadow_spread: 0.0,
                    shadow_power: 0.0,
                    vibrancy: 0.0,
                    vibrancy_darkness: 0.0,
                    passes: 0.0,
                    _pad1: 0.0,
                    _pad2: 0.0,
                    _pad3: 0.0,
                };
                surface_pass.set_scissor_rect(
                    0,
                    0,
                    self.size.width.max(1),
                    self.size.height.max(1),
                );
                surface_pass.set_pipeline(&self.pipelines.texture);
                surface_pass.set_bind_group(0, &self.scene.blit_bind_group, &[]);
                surface_pass.set_immediates(0, bytemuck::bytes_of(&blit_immediate));
                surface_pass.draw(0..6, 0..1);

                let commands = context.render_list().enumerate().skip(split_idx);
                render_command_slice(
                    &mut surface_pass,
                    commands,
                    self.size.width,
                    self.size.height,
                    &self.pipelines,
                    &blurred_solid_bind_group,
                    &self.text_batch.bind_group,
                    &text_ranges,
                    &self.canvas_textures,
                    context,
                );

                render_focus_ring(
                    &mut surface_pass,
                    self.size.width,
                    self.size.height,
                    &self.pipelines,
                    &blurred_solid_bind_group,
                    context,
                );
            }
        } else {
            // SINGLE-PASS FAST PATH FOR NON-BLURRED SCENES //
            let mut surface_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Surface Pass"),
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

            let commands = context.render_list().enumerate();
            render_command_slice(
                &mut surface_pass,
                commands,
                self.size.width,
                self.size.height,
                &self.pipelines,
                &self.pipelines.dummy_solid_bind_group,
                &self.text_batch.bind_group,
                &text_ranges,
                &self.canvas_textures,
                context,
            );

            render_focus_ring(
                &mut surface_pass,
                self.size.width,
                self.size.height,
                &self.pipelines,
                &self.pipelines.dummy_solid_bind_group,
                context,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(surface_texture);

        focused_caret
    }

    fn update_canvas_textures(
        &mut self,
        context: &crate::Context,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Retain only active canvases
        self.canvas_textures
            .retain(|node, _| context.canvases.borrow().contains_key(node));

        let any_canvas_requested_frame = std::cell::Cell::new(false);

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

                    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Canvas Bind Group"),
                        layout: &self.pipelines.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(
                                    &self.pipelines.texture_sampler,
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
                            encoder,
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

        if any_canvas_requested_frame.get() {
            self.window.request_redraw();
        }
    }
}

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

fn render_command_slice<'a, I>(
    render_pass: &mut wgpu::RenderPass<'a>,
    commands: I,
    screen_width: u32,
    screen_height: u32,
    pipelines: &'a Pipelines,
    solid_bind_group: &'a wgpu::BindGroup,
    text_bind_group: &'a wgpu::BindGroup,
    text_ranges: &'a HashMap<usize, RenderTextData>,
    canvas_textures: &'a HashMap<crate::Node, CanvasGpuResource>,
    context: &crate::Context,
) where
    I: Iterator<Item = (usize, crate::render::RenderCommand<'a>)>,
{
    for (cmd_index, cmd) in commands {
        let _active_scissor = if cmd.has_clip() {
            if let Some(rect) = compute_scissor_rect(cmd.clip(), screen_width, screen_height) {
                render_pass.set_scissor_rect(rect.0, rect.1, rect.2, rect.3);
                Some(rect)
            } else {
                continue;
            }
        } else {
            let default_rect = (0, 0, screen_width.max(1), screen_height.max(1));
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
            let computed = cmd.computed();
            let constraints = node.get_constraints(context).unwrap_or_default();
            let effects = context.effects.get(&node).cloned().unwrap_or_default();

            let border_widths = [
                constraints.border.top,
                constraints.border.right,
                constraints.border.bottom,
                constraints.border.left,
            ];

            let mut vibrancy = 0.0;
            let mut passes = 0.0;
            for f in &effects.filters {
                match *f {
                    Filter::Blur {
                        vibrancy: v,
                        passes: p,
                        ..
                    } => {
                        vibrancy = v;
                        passes = p;
                    }
                }
            }

            let mut immediate_data = ImmediateData {
                color: effects.background_color.into(),
                pos: [computed.x, computed.y],
                screen_size: [screen_width as f32, screen_height as f32],
                quad_size: [computed.w, computed.h],
                border_radius: effects.border.radius.tl,
                alpha: effects.opacity,
                border_color: effects.border.color.into(),
                shadow_color: effects.shadow.color.into(),
                border_widths,
                shadow_spread: effects.shadow.spread,
                shadow_power: effects.shadow.power,
                vibrancy,
                vibrancy_darkness: 0.0,
                passes,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            if let Some(canvas_res) = canvas_textures.get(&node) {
                render_pass.set_pipeline(&pipelines.texture);
                render_pass.set_bind_group(0, &canvas_res.bind_group, &[]);
                immediate_data.color = [1.0, 1.0, 1.0, 1.0];
                render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                render_pass.draw(0..6, 0..1);
            } else {
                render_pass.set_pipeline(&pipelines.solid);
                render_pass.set_bind_group(0, solid_bind_group, &[]);
                render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                render_pass.draw(0..6, 0..1);
            }
        } else if cmd.kind() == RenderCommandKind::Text {
            let node = cmd.node();
            let constraints = node.get_constraints(context).unwrap_or_default();
            let mut overflow_clipped = false;
            if constraints.overflow == crate::Overflow::Hidden {
                let computed = cmd.computed();
                let text_clip = crate::style::Rect {
                    x: computed.x + constraints.border.left,
                    y: computed.y + constraints.border.top,
                    w: (computed.w - constraints.border.left - constraints.border.right).max(0.0),
                    h: (computed.h - constraints.border.top - constraints.border.bottom).max(0.0),
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
                    compute_scissor_rect(effective_clip, screen_width, screen_height)
                {
                    render_pass.set_scissor_rect(nx, ny, nw, nh);
                    overflow_clipped = true;
                } else {
                    continue;
                }
            }

            if let Some(range) = text_ranges.get(&cmd_index) {
                // A) Draw selections
                if !range.selections.is_empty() {
                    render_pass.set_pipeline(&pipelines.solid);
                    render_pass.set_bind_group(0, solid_bind_group, &[]);
                    for s_rect in &range.selections {
                        let immediate_data = ImmediateData {
                            color: range.style.selection_bg.into(),
                            pos: [s_rect[0], s_rect[1]],
                            screen_size: [screen_width as f32, screen_height as f32],
                            quad_size: [s_rect[2], s_rect[3]],
                            border_radius: 0.0,
                            alpha: range.alpha,
                            border_color: [0.0; 4],
                            shadow_color: [0.0; 4],
                            border_widths: [0.0; 4],
                            shadow_spread: 0.0,
                            shadow_power: 0.0,
                            vibrancy: 0.0,
                            vibrancy_darkness: 0.0,
                            passes: 0.0,
                            _pad1: 0.0,
                            _pad2: 0.0,
                            _pad3: 0.0,
                        };
                        render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                        render_pass.draw(0..6, 0..1);
                    }
                }

                // B) Draw Glyphs
                render_pass.set_pipeline(&pipelines.text);
                render_pass.set_bind_group(0, text_bind_group, &[]);
                let screen_size = [screen_width as f32, screen_height as f32];
                render_pass.set_immediates(0, bytemuck::bytes_of(&screen_size));
                let glyph_start = range.glyphs.start as u32;
                let glyph_end = range.glyphs.end as u32;
                let count = glyph_end.saturating_sub(glyph_start);
                if count > 0 {
                    render_pass.draw(0..6, glyph_start..glyph_end);
                }

                // C) Draw Caret
                if let Some(c_rect) = range.caret {
                    let immediate_data = ImmediateData {
                        color: range.style.caret_color.into(),
                        pos: [c_rect[0], c_rect[1]],
                        screen_size: [screen_width as f32, screen_height as f32],
                        quad_size: [c_rect[2], c_rect[3]],
                        border_color: [0.0; 4],
                        border_radius: 0.0,
                        alpha: range.alpha,
                        shadow_color: [0.0; 4],
                        border_widths: [0.0; 4],
                        shadow_spread: 0.0,
                        shadow_power: 0.0,
                        vibrancy: 0.0,
                        vibrancy_darkness: 0.0,
                        passes: 0.0,
                        _pad1: 0.0,
                        _pad2: 0.0,
                        _pad3: 0.0,
                    };
                    render_pass.set_pipeline(&pipelines.solid);
                    render_pass.set_bind_group(0, solid_bind_group, &[]);
                    render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                    render_pass.draw(0..6, 0..1);
                }

                // D) Draw Strikethroughs
                if !range.strikethroughs.is_empty() {
                    render_pass.set_pipeline(&pipelines.solid);
                    render_pass.set_bind_group(0, solid_bind_group, &[]);
                    for st_rect in &range.strikethroughs {
                        let immediate_data = ImmediateData {
                            color: range.style.color.into(),
                            pos: [st_rect[0], st_rect[1]],
                            screen_size: [screen_width as f32, screen_height as f32],
                            quad_size: [st_rect[2], st_rect[3]],
                            border_radius: 0.0,
                            alpha: range.alpha,
                            border_color: [0.0; 4],
                            shadow_color: [0.0; 4],
                            border_widths: [0.0; 4],
                            shadow_spread: 0.0,
                            shadow_power: 0.0,
                            vibrancy: 0.0,
                            vibrancy_darkness: 0.0,
                            passes: 0.0,
                            _pad1: 0.0,
                            _pad2: 0.0,
                            _pad3: 0.0,
                        };
                        render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                        render_pass.draw(0..6, 0..1);
                    }
                }

                // E) Draw Underlines
                if !range.underlines.is_empty() {
                    render_pass.set_pipeline(&pipelines.solid);
                    render_pass.set_bind_group(0, solid_bind_group, &[]);
                    for un_rect in &range.underlines {
                        let immediate_data = ImmediateData {
                            color: range.style.color.into(),
                            pos: [un_rect[0], un_rect[1]],
                            screen_size: [screen_width as f32, screen_height as f32],
                            quad_size: [un_rect[2], un_rect[3]],
                            border_radius: 0.0,
                            alpha: range.alpha,
                            border_color: [0.0; 4],
                            shadow_color: [0.0; 4],
                            border_widths: [0.0; 4],
                            shadow_spread: 0.0,
                            shadow_power: 0.0,
                            vibrancy: 0.0,
                            vibrancy_darkness: 0.0,
                            passes: 0.0,
                            _pad1: 0.0,
                            _pad2: 0.0,
                            _pad3: 0.0,
                        };
                        render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
                        render_pass.draw(0..6, 0..1);
                    }
                }
            }

            if overflow_clipped {
                if let Some(rect) = compute_scissor_rect(cmd.clip(), screen_width, screen_height) {
                    render_pass.set_scissor_rect(rect.0, rect.1, rect.2, rect.3);
                }
            }
        } else if cmd.kind() == RenderCommandKind::ScrollbarV {
            let sb_rect = cmd.computed();
            let immediate_data = ImmediateData {
                color: [0.4, 0.4, 0.4, 0.6],
                pos: [sb_rect.x, sb_rect.y],
                screen_size: [screen_width as f32, screen_height as f32],
                quad_size: [sb_rect.w, sb_rect.h],
                border_radius: sb_rect.w / 2.0,
                alpha: 1.0,
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                border_widths: [0.0; 4],
                shadow_spread: 0.0,
                shadow_power: 0.0,
                vibrancy: 0.0,
                vibrancy_darkness: 0.0,
                passes: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            render_pass.set_pipeline(&pipelines.solid);
            render_pass.set_bind_group(0, solid_bind_group, &[]);
            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
            render_pass.draw(0..6, 0..1);
        } else if cmd.kind() == RenderCommandKind::ScrollbarH {
            let sb_rect = cmd.computed();
            let immediate_data = ImmediateData {
                color: [0.4, 0.4, 0.4, 0.6],
                pos: [sb_rect.x, sb_rect.y],
                screen_size: [screen_width as f32, screen_height as f32],
                quad_size: [sb_rect.w, sb_rect.h],
                border_radius: sb_rect.h / 2.0,
                alpha: 1.0,
                border_color: [0.0; 4],
                shadow_color: [0.0; 4],
                border_widths: [0.0; 4],
                shadow_spread: 0.0,
                shadow_power: 0.0,
                vibrancy: 0.0,
                vibrancy_darkness: 0.0,
                passes: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            render_pass.set_pipeline(&pipelines.solid);
            render_pass.set_bind_group(0, solid_bind_group, &[]);
            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
            render_pass.draw(0..6, 0..1);
        }
    }
}

fn render_focus_ring<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    screen_width: u32,
    screen_height: u32,
    pipelines: &'a Pipelines,
    solid_bind_group: &'a wgpu::BindGroup,
    context: &crate::Context,
) {
    if let Some(focused) = context.focused_node() {
        if let Some(computed) = focused.get_computed(context) {
            let effects = focused.get_effects(context).unwrap_or_default();

            render_pass.set_scissor_rect(0, 0, screen_width.max(1), screen_height.max(1));

            let ring_thickness = 2.0;
            let immediate_data = ImmediateData {
                color: [0.0; 4],
                pos: [computed.x - ring_thickness, computed.y - ring_thickness],
                screen_size: [screen_width as f32, screen_height as f32],
                quad_size: [
                    computed.w + ring_thickness * 2.0,
                    computed.h + ring_thickness * 2.0,
                ],
                border_color: [0.0, 0.47, 1.0, 1.0], // Accessible blue focus ring
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

            render_pass.set_pipeline(&pipelines.solid);
            render_pass.set_bind_group(0, solid_bind_group, &[]);
            render_pass.set_immediates(0, bytemuck::bytes_of(&immediate_data));
            render_pass.draw(0..6, 0..1);
        }
    }
}
