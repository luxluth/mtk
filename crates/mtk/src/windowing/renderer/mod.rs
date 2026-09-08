pub(crate) mod atlas;
pub(crate) mod blur;
pub(crate) mod pipelines;
pub(crate) mod quad_batch;
pub(crate) mod scene;
pub(crate) mod svg_cache;
pub(crate) mod text_batch;

use self::atlas::Atlas;
use self::blur::BlurPipeline;
use self::pipelines::{ImmediateData, Pipelines};
use self::quad_batch::{QuadBatch, QuadInstance, SolidPushConstants};
use self::scene::SceneTarget;
use self::svg_cache::{SvgRasterPool, SvgRasterRequest};
use self::text_batch::{RenderTextData, TextBatch};
use crate::effects::Filter;
use crate::render::RenderCommandKind;
use crate::style::ScrollbarVisibility;
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

pub struct ImageGpuResource {
    pub texture: wgpu::Texture,
    pub _view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub image_id: u64,
    pub _width: u32,
    pub _height: u32,
}

pub struct SvgGpuResource {
    pub texture: wgpu::Texture,
    pub _view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
    pub svg_id: u64,
    pub width: u32,
    pub height: u32,
    pub fit: crate::image::ObjectFit,
}

pub struct Renderer {
    _instance: wgpu::Instance,
    window: Arc<dyn Window>,
    pub surface: wgpu::Surface<'static>,
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
    pub quad_batch: QuadBatch,
    pub canvas_textures: HashMap<crate::Node, CanvasGpuResource>,
    pub image_textures: HashMap<crate::Node, ImageGpuResource>,
    pub svg_textures: HashMap<crate::Node, SvgGpuResource>,
    pub(crate) svg_pool: SvgRasterPool,
    pub(crate) svg_generations: HashMap<crate::Node, u64>,
    pub(crate) svg_requested_sizes: HashMap<crate::Node, (u64, u32, u32, crate::image::ObjectFit)>,
    pub(crate) next_svg_gen: u64,
}

fn generate_rgba8_mipmaps(width: u32, height: u32, base_pixels: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut mips = Vec::new();
    let mut cur_w = width;
    let mut cur_h = height;
    let mut cur_pixels = base_pixels.to_vec();

    while cur_w > 1 || cur_h > 1 {
        let next_w = (cur_w / 2).max(1);
        let next_h = (cur_h / 2).max(1);
        let mut next_pixels = vec![0u8; (next_w * next_h * 4) as usize];

        for y in 0..next_h {
            for x in 0..next_w {
                let src_x0 = x * 2;
                let src_x1 = (src_x0 + 1).min(cur_w - 1);
                let src_y0 = y * 2;
                let src_y1 = (src_y0 + 1).min(cur_h - 1);

                let p00_idx = ((src_y0 * cur_w + src_x0) * 4) as usize;
                let p10_idx = ((src_y0 * cur_w + src_x1) * 4) as usize;
                let p01_idx = ((src_y1 * cur_w + src_x0) * 4) as usize;
                let p11_idx = ((src_y1 * cur_w + src_x1) * 4) as usize;

                let dst_idx = ((y * next_w + x) * 4) as usize;

                for c in 0..4 {
                    let sum = cur_pixels[p00_idx + c] as u32
                        + cur_pixels[p10_idx + c] as u32
                        + cur_pixels[p01_idx + c] as u32
                        + cur_pixels[p11_idx + c] as u32;
                    next_pixels[dst_idx + c] = ((sum + 2) / 4) as u8;
                }
            }
        }

        mips.push((next_w, next_h, next_pixels.clone()));
        cur_w = next_w;
        cur_h = next_h;
        cur_pixels = next_pixels;
    }

    mips
}

impl Renderer {
    pub async fn new(display: OwnedDisplayHandle, window: Arc<dyn Window>) -> Self {
        let size = window.surface_size();

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

        if cap.present_modes.contains(&wgpu::PresentMode::AutoVsync) {
            config.present_mode = wgpu::PresentMode::AutoVsync;
        } else if cap.present_modes.contains(&wgpu::PresentMode::FifoRelaxed) {
            config.present_mode = wgpu::PresentMode::FifoRelaxed;
        } else if cap.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            config.present_mode = wgpu::PresentMode::Mailbox;
        }

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
        let quad_batch = QuadBatch::new(&device);

        let svg_pool = SvgRasterPool::new(Arc::clone(&window));

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
            quad_batch,
            canvas_textures: HashMap::new(),
            image_textures: HashMap::new(),
            svg_textures: HashMap::new(),
            svg_pool,
            svg_generations: HashMap::new(),
            svg_requested_sizes: HashMap::new(),
            next_svg_gen: 0,
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            if self.config.width == size.width && self.config.height == size.height {
                return;
            }
            self.size = size;
            self.config.width = size.width;
            self.config.height = size.height;
            self.configure_surface();
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

        // 2. Update and paint active canvas, image, and svg widgets
        self.update_canvas_textures(context, &mut encoder);
        self.update_image_textures(context);
        self.update_svg_textures(context);

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
            let mut quad_instances = Vec::new();
            let mut batches_pass1 = Vec::new();
            let mut batches_pass3 = Vec::new();

            prepare_command_slice(
                context.render_list().enumerate().take(split_idx),
                self.size.width,
                self.size.height,
                &text_ranges,
                &self.canvas_textures,
                &self.image_textures,
                &self.svg_textures,
                context,
                &mut quad_instances,
                &mut batches_pass1,
            );

            prepare_command_slice(
                context.render_list().enumerate().skip(split_idx),
                self.size.width,
                self.size.height,
                &text_ranges,
                &self.canvas_textures,
                &self.image_textures,
                &self.svg_textures,
                context,
                &mut quad_instances,
                &mut batches_pass3,
            );

            prepare_debug_highlight(context, &mut quad_instances, &mut batches_pass3);

            self.quad_batch
                .ensure_capacity(&self.device, quad_instances.len());
            self.quad_batch.upload(&self.queue, &quad_instances);

            // Ensure offscreen scene target and blur pyramid match window size
            self.scene.resize(
                &self.device,
                &self.pipelines.texture_bind_group_layout,
                &self.pipelines.texture_sampler,
                self.size.width,
                self.size.height,
            );
            self.blur
                .resize(&self.device, self.size.width, self.size.height);

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

                execute_draw_batches(
                    &mut scene_pass,
                    &batches_pass1,
                    self.size.width,
                    self.size.height,
                    &self.pipelines,
                    &self.pipelines.dummy_solid_bind_group,
                    &self.text_batch.bind_group,
                    &self.quad_batch.buffer,
                    &self.canvas_textures,
                    &self.image_textures,
                    &self.svg_textures,
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
                    alpha: 1.0,
                    _pad0: 0.0,
                    border_radii: [0.0; 4],
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

                execute_draw_batches(
                    &mut surface_pass,
                    &batches_pass3,
                    self.size.width,
                    self.size.height,
                    &self.pipelines,
                    &blurred_solid_bind_group,
                    &self.text_batch.bind_group,
                    &self.quad_batch.buffer,
                    &self.canvas_textures,
                    &self.image_textures,
                    &self.svg_textures,
                );
            }
        } else {
            // SINGLE-PASS FAST PATH FOR NON-BLURRED SCENES //
            let mut quad_instances = Vec::new();
            let mut batches = Vec::new();

            prepare_command_slice(
                context.render_list().enumerate(),
                self.size.width,
                self.size.height,
                &text_ranges,
                &self.canvas_textures,
                &self.image_textures,
                &self.svg_textures,
                context,
                &mut quad_instances,
                &mut batches,
            );

            prepare_debug_highlight(context, &mut quad_instances, &mut batches);

            self.quad_batch
                .ensure_capacity(&self.device, quad_instances.len());
            self.quad_batch.upload(&self.queue, &quad_instances);

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

            execute_draw_batches(
                &mut surface_pass,
                &batches,
                self.size.width,
                self.size.height,
                &self.pipelines,
                &self.pipelines.dummy_solid_bind_group,
                &self.text_batch.bind_group,
                &self.quad_batch.buffer,
                &self.canvas_textures,
                &self.image_textures,
                &self.svg_textures,
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
        self.canvas_textures.retain(|node, res| {
            let keep = context.canvases.borrow().contains_key(node);
            if !keep {
                res.texture.destroy();
            }
            keep
        });

        let any_canvas_requested_frame = std::cell::Cell::new(false);

        let scale_factor = context.scale_factor.max(0.1);
        let mut canvases = context.canvases.borrow_mut();
        for (node, canvas_data) in canvases.iter_mut() {
            if let Some(computed) = node.get_computed(context) {
                let w = ((computed.w * scale_factor).round() as u32).max(1);
                let h = ((computed.h * scale_factor).round() as u32).max(1);

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

                    if let Some(old) = self.canvas_textures.insert(
                        *node,
                        CanvasGpuResource {
                            texture,
                            view,
                            bind_group,
                            width: w,
                            height: h,
                        },
                    ) {
                        old.texture.destroy();
                    }

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
                            scale_factor,
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
                            scale_factor,
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

    fn update_image_textures(&mut self, context: &crate::Context) {
        self.image_textures.retain(|node, res| {
            let keep = context.images.borrow().contains_key(node);
            if !keep {
                res.texture.destroy();
            }
            keep
        });

        self.upload_image_texture(context);
    }

    fn upload_image_texture(&mut self, context: &crate::Context) {
        let images = context.images.borrow();
        for (node, (image_data, _fit)) in images.iter() {
            let needs_upload = match self.image_textures.get(node) {
                Some(res) => res.image_id != image_data.id,
                None => true,
            };

            if needs_upload {
                let w = image_data.width.max(1);
                let h = image_data.height.max(1);
                let mipmaps = generate_rgba8_mipmaps(w, h, &image_data.pixels);
                let mip_level_count = 1 + mipmaps.len() as u32;

                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Image Offscreen Texture"),
                    size: wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });

                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &image_data.pixels,
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

                for (idx, (mip_w, mip_h, mip_data)) in mipmaps.iter().enumerate() {
                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: (idx + 1) as u32,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        mip_data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * mip_w),
                            rows_per_image: Some(*mip_h),
                        },
                        wgpu::Extent3d {
                            width: *mip_w,
                            height: *mip_h,
                            depth_or_array_layers: 1,
                        },
                    );
                }

                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Image Bind Group"),
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

                if let Some(old) = self.image_textures.insert(
                    *node,
                    ImageGpuResource {
                        texture,
                        _view: view,
                        bind_group,
                        image_id: image_data.id,
                        _width: w,
                        _height: h,
                    },
                ) {
                    old.texture.destroy();
                }
            }
        }
    }

    fn upload_svg_texture(
        &mut self,
        node: crate::Node,
        svg_id: u64,
        width: u32,
        height: u32,
        fit: crate::image::ObjectFit,
        pixmap: resvg::tiny_skia::Pixmap,
    ) {
        if let Some(existing) = self.svg_textures.get_mut(&node) {
            if existing.width == width && existing.height == height {
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &existing.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    pixmap.data(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * width),
                        rows_per_image: Some(height),
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
                existing.svg_id = svg_id;
                existing.fit = fit;
                return;
            }
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SVG Rendered Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixmap.data(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SVG Bind Group"),
            layout: &self.pipelines.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.pipelines.texture_sampler),
                },
            ],
        });

        if let Some(old) = self.svg_textures.insert(
            node,
            SvgGpuResource {
                texture,
                _view: view,
                bind_group,
                svg_id,
                width,
                height,
                fit,
            },
        ) {
            old.texture.destroy();
        }
    }

    fn update_svg_textures(&mut self, context: &crate::Context) {
        self.svg_textures.retain(|node, res| {
            let keep = context.svgs.borrow().contains_key(node);
            if !keep {
                res.texture.destroy();
            }
            keep
        });
        self.svg_generations
            .retain(|node, _| context.svgs.borrow().contains_key(node));
        self.svg_requested_sizes
            .retain(|node, _| context.svgs.borrow().contains_key(node));

        while let Some(res) = self.svg_pool.try_recv() {
            if let Some(&expected_gen) = self.svg_generations.get(&res.node) {
                if expected_gen == res.generation {
                    if let Some(pixmap) = res.pixmap {
                        self.upload_svg_texture(
                            res.node,
                            res.svg_id,
                            res.target_w,
                            res.target_h,
                            res.fit,
                            pixmap,
                        );
                    }
                }
            }
        }

        let scale_factor = context.scale_factor.max(0.1);
        let svgs = context.svgs.borrow();
        for (node, (svg_data, fit)) in svgs.iter() {
            if let Some(computed) = node.get_computed(context) {
                let target_w = ((computed.w * scale_factor).round() as u32).max(1);
                let target_h = ((computed.h * scale_factor).round() as u32).max(1);

                match self.svg_textures.get(node) {
                    None => {
                        if let Some(pixmap) = svg_data.render_to_pixmap(target_w, target_h, *fit) {
                            self.upload_svg_texture(
                                *node,
                                svg_data.id,
                                target_w,
                                target_h,
                                *fit,
                                pixmap,
                            );
                            self.svg_requested_sizes
                                .insert(*node, (svg_data.id, target_w, target_h, *fit));
                        }
                    }
                    Some(res) => {
                        let needs_update = res.svg_id != svg_data.id
                            || res.fit != *fit
                            || res.width != target_w
                            || res.height != target_h;

                        if needs_update {
                            let requested = self.svg_requested_sizes.get(node).copied();
                            if requested != Some((svg_data.id, target_w, target_h, *fit)) {
                                self.next_svg_gen += 1;
                                let generation = self.next_svg_gen;
                                self.svg_generations.insert(*node, generation);
                                self.svg_requested_sizes
                                    .insert(*node, (svg_data.id, target_w, target_h, *fit));

                                self.svg_pool.schedule(SvgRasterRequest {
                                    node: *node,
                                    svg_data: svg_data.clone(),
                                    target_w,
                                    target_h,
                                    fit: *fit,
                                    generation,
                                });
                            }
                        }
                    }
                }
            }
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

#[inline]
pub(crate) fn compute_effective_opacity(context: &crate::Context, mut node: crate::Node) -> f32 {
    let mut alpha = 1.0;
    loop {
        if let Some(eff) = context.effects.get(&node) {
            alpha *= eff.opacity;
        }
        if let Some(parent) = node.parent(context) {
            node = parent;
        } else {
            break;
        }
    }
    alpha
}

#[inline]
pub(crate) fn compute_effective_scale(context: &crate::Context, mut node: crate::Node) -> f32 {
    let mut scale = 1.0;
    loop {
        if let Some(eff) = context.effects.get(&node) {
            scale *= eff.scale;
        }
        if let Some(parent) = node.parent(context) {
            node = parent;
        } else {
            break;
        }
    }
    scale
}

#[inline]
pub(crate) fn transform_point_ancestors(
    context: &crate::Context,
    mut node: crate::Node,
    mut pt: (f32, f32),
) -> (f32, f32) {
    let scale_factor = context.scale_factor.max(0.1);
    while let Some(parent) = node.parent(context) {
        if let Some(computed) = parent.get_computed(context) {
            let eff = context.effects.get(&parent).cloned().unwrap_or_default();
            let s = eff.scale;
            if (s - 1.0).abs() > 1e-4 {
                let cx = (computed.x + computed.w / 2.0) * scale_factor;
                let cy = (computed.y + computed.h / 2.0) * scale_factor;
                pt.0 = cx + (pt.0 - cx) * s;
                pt.1 = cy + (pt.1 - cy) * s;
            }
        }
        node = parent;
    }
    pt
}

#[inline]
pub(crate) fn transform_node_point(
    context: &crate::Context,
    node: crate::Node,
    pt: (f32, f32),
) -> (f32, f32) {
    let scale_factor = context.scale_factor.max(0.1);
    let mut curr_pt = pt;
    if let Some(computed) = node.get_computed(context) {
        let eff = context.effects.get(&node).cloned().unwrap_or_default();
        let s = eff.scale;
        if (s - 1.0).abs() > 1e-4 {
            let cx = (computed.x + computed.w / 2.0) * scale_factor;
            let cy = (computed.y + computed.h / 2.0) * scale_factor;
            curr_pt.0 = cx + (curr_pt.0 - cx) * s;
            curr_pt.1 = cy + (curr_pt.1 - cy) * s;
        }
    }
    transform_point_ancestors(context, node, curr_pt)
}

enum DrawBatch {
    SolidQuads {
        start: u32,
        count: u32,
    },
    TextGlyphs {
        start: u32,
        count: u32,
        clip: Option<crate::style::Rect>,
    },
    CanvasTexture {
        node: crate::Node,
        immediate: ImmediateData,
        clip: Option<crate::style::Rect>,
    },
    ImageTexture {
        node: crate::Node,
        immediate: ImmediateData,
        clip: Option<crate::style::Rect>,
    },
    SvgTexture {
        node: crate::Node,
        immediate: ImmediateData,
        clip: Option<crate::style::Rect>,
    },
}

#[inline]
fn push_solid_quad(
    quad_instances: &mut Vec<QuadInstance>,
    draw_batches: &mut Vec<DrawBatch>,
    instance: QuadInstance,
) {
    let instance_idx = quad_instances.len() as u32;
    quad_instances.push(instance);
    if let Some(DrawBatch::SolidQuads { count, .. }) = draw_batches.last_mut() {
        *count += 1;
    } else {
        draw_batches.push(DrawBatch::SolidQuads {
            start: instance_idx,
            count: 1,
        });
    }
}

fn prepare_command_slice<'a, I>(
    commands: I,
    screen_width: u32,
    screen_height: u32,
    text_ranges: &HashMap<usize, RenderTextData>,
    canvas_textures: &HashMap<crate::Node, CanvasGpuResource>,
    image_textures: &HashMap<crate::Node, ImageGpuResource>,
    svg_textures: &HashMap<crate::Node, SvgGpuResource>,
    context: &crate::Context,
    quad_instances: &mut Vec<QuadInstance>,
    draw_batches: &mut Vec<DrawBatch>,
) where
    I: Iterator<Item = (usize, crate::render::RenderCommand<'a>)>,
{
    let screen_w_f32 = screen_width as f32;
    let screen_h_f32 = screen_height as f32;
    let scale_factor = context.scale_factor.max(0.1);

    for (cmd_index, cmd) in commands {
        let clip_rect = if cmd.has_clip() {
            let c = cmd.clip();
            let cx = c.x * scale_factor;
            let cy = c.y * scale_factor;
            let cw = c.w * scale_factor;
            let ch = c.h * scale_factor;
            if cw <= 0.0
                || ch <= 0.0
                || cx >= screen_w_f32
                || cy >= screen_h_f32
                || cx + cw <= 0.0
                || cy + ch <= 0.0
            {
                continue;
            }
            [cx, cy, cw, ch]
        } else {
            [-1.0, -1.0, -1.0, -1.0]
        };

        if cmd.kind() == RenderCommandKind::DrawQuad {
            let node = cmd.node();
            let computed = cmd.computed();
            let constraints = node.get_constraints(context).unwrap_or_default();
            let effects = context.effects.get(&node).cloned().unwrap_or_default();
            let total_scale = compute_effective_scale(context, node);

            let cx = (computed.x + computed.w / 2.0) * scale_factor;
            let cy = (computed.y + computed.h / 2.0) * scale_factor;
            let (transformed_cx, transformed_cy) =
                transform_point_ancestors(context, node, (cx, cy));

            let scaled_w = computed.w * total_scale * scale_factor;
            let scaled_h = computed.h * total_scale * scale_factor;
            let scaled_x = transformed_cx - scaled_w / 2.0;
            let scaled_y = transformed_cy - scaled_h / 2.0;

            let border_widths = [
                constraints.border.top * total_scale * scale_factor,
                constraints.border.right * total_scale * scale_factor,
                constraints.border.bottom * total_scale * scale_factor,
                constraints.border.left * total_scale * scale_factor,
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

            let effective_alpha = compute_effective_opacity(context, node);

            let border_radii = [
                effects.border.radius.tl * total_scale * scale_factor,
                effects.border.radius.tr * total_scale * scale_factor,
                effects.border.radius.br * total_scale * scale_factor,
                effects.border.radius.bl * total_scale * scale_factor,
            ];

            if canvas_textures.contains_key(&node) {
                let immediate = ImmediateData {
                    color: [1.0, 1.0, 1.0, effective_alpha],
                    pos: [scaled_x, scaled_y],
                    screen_size: [screen_w_f32, screen_h_f32],
                    quad_size: [scaled_w, scaled_h],
                    alpha: effective_alpha,
                    _pad0: 0.0,
                    border_radii,
                    border_color: effects.border.color.into(),
                    shadow_color: effects.shadow.color.into(),
                    border_widths,
                    shadow_spread: effects.shadow.spread * total_scale * scale_factor,
                    shadow_power: effects.shadow.power,
                    vibrancy,
                    vibrancy_darkness: 0.0,
                    passes,
                    _pad1: 0.0,
                    _pad2: 0.0,
                    _pad3: 0.0,
                };
                draw_batches.push(DrawBatch::CanvasTexture {
                    node,
                    immediate,
                    clip: if cmd.has_clip() {
                        let c = cmd.clip();
                        Some(crate::style::Rect {
                            x: c.x * scale_factor,
                            y: c.y * scale_factor,
                            w: c.w * scale_factor,
                            h: c.h * scale_factor,
                        })
                    } else {
                        None
                    },
                });
            } else if image_textures.contains_key(&node) {
                let immediate = ImmediateData {
                    color: [1.0, 1.0, 1.0, effective_alpha],
                    pos: [scaled_x, scaled_y],
                    screen_size: [screen_w_f32, screen_h_f32],
                    quad_size: [scaled_w, scaled_h],
                    alpha: effective_alpha,
                    _pad0: 0.0,
                    border_radii,
                    border_color: effects.border.color.into(),
                    shadow_color: effects.shadow.color.into(),
                    border_widths,
                    shadow_spread: effects.shadow.spread * total_scale * scale_factor,
                    shadow_power: effects.shadow.power,
                    vibrancy,
                    vibrancy_darkness: 0.0,
                    passes,
                    _pad1: 0.0,
                    _pad2: 0.0,
                    _pad3: 0.0,
                };
                draw_batches.push(DrawBatch::ImageTexture {
                    node,
                    immediate,
                    clip: if cmd.has_clip() {
                        let c = cmd.clip();
                        Some(crate::style::Rect {
                            x: c.x * scale_factor,
                            y: c.y * scale_factor,
                            w: c.w * scale_factor,
                            h: c.h * scale_factor,
                        })
                    } else {
                        None
                    },
                });
            } else if svg_textures.contains_key(&node) {
                let immediate = ImmediateData {
                    color: [1.0, 1.0, 1.0, effective_alpha],
                    pos: [scaled_x, scaled_y],
                    screen_size: [screen_w_f32, screen_h_f32],
                    quad_size: [scaled_w, scaled_h],
                    alpha: effective_alpha,
                    _pad0: 0.0,
                    border_radii,
                    border_color: effects.border.color.into(),
                    shadow_color: effects.shadow.color.into(),
                    border_widths,
                    shadow_spread: effects.shadow.spread * total_scale * scale_factor,
                    shadow_power: effects.shadow.power,
                    vibrancy,
                    vibrancy_darkness: 0.0,
                    passes,
                    _pad1: 0.0,
                    _pad2: 0.0,
                    _pad3: 0.0,
                };
                draw_batches.push(DrawBatch::SvgTexture {
                    node,
                    immediate,
                    clip: if cmd.has_clip() {
                        let c = cmd.clip();
                        Some(crate::style::Rect {
                            x: c.x * scale_factor,
                            y: c.y * scale_factor,
                            w: c.w * scale_factor,
                            h: c.h * scale_factor,
                        })
                    } else {
                        None
                    },
                });
            } else {
                let quad = QuadInstance {
                    pos: [scaled_x, scaled_y],
                    quad_size: [scaled_w, scaled_h],
                    color: effects.background_color.into(),
                    border_radii,
                    border_color: effects.border.color.into(),
                    border_widths,
                    shadow_color: effects.shadow.color.into(),
                    shadow_params: [
                        effects.shadow.spread * total_scale * scale_factor,
                        effects.shadow.power,
                        effective_alpha,
                        0.0,
                    ],
                    effects: [vibrancy, 0.0, passes, 0.0],
                    clip_rect,
                };
                push_solid_quad(quad_instances, draw_batches, quad);
            }

            if Some(node) == context.focused_node() {
                let should_render = if context.modal_layer.state.visible {
                    if let Some(modal_root) = context.modal_layer.state.root_node {
                        node.is_descendant_of(context, modal_root) || node == modal_root
                    } else {
                        false
                    }
                } else if let Some(inter) = context
                    .intermediate_layers
                    .iter()
                    .rfind(|l| l.state.visible && l.blocking)
                {
                    if let Some(inter_root) = inter.state.root_node {
                        node.is_descendant_of(context, inter_root) || node == inter_root
                    } else {
                        false
                    }
                } else {
                    true
                };

                if should_render {
                    let ring_thickness = 2.0 * scale_factor;
                    let ring_radii = [
                        border_radii[0] + ring_thickness,
                        border_radii[1] + ring_thickness,
                        border_radii[2] + ring_thickness,
                        border_radii[3] + ring_thickness,
                    ];
                    let ring_quad = QuadInstance {
                        pos: [scaled_x - ring_thickness, scaled_y - ring_thickness],
                        quad_size: [
                            scaled_w + ring_thickness * 2.0,
                            scaled_h + ring_thickness * 2.0,
                        ],
                        color: [0.0; 4],
                        border_radii: ring_radii,
                        border_color: [0.0, 0.47, 1.0, 1.0],
                        border_widths: [ring_thickness; 4],
                        shadow_color: [0.0; 4],
                        shadow_params: [0.0, 0.0, effective_alpha, 0.0],
                        effects: [0.0; 4],
                        clip_rect,
                    };
                    push_solid_quad(quad_instances, draw_batches, ring_quad);
                }
            }
        } else if cmd.kind() == RenderCommandKind::Text {
            let text_clip = if cmd.has_clip() {
                let c = cmd.clip();
                Some(crate::style::Rect {
                    x: c.x * scale_factor,
                    y: c.y * scale_factor,
                    w: c.w * scale_factor,
                    h: c.h * scale_factor,
                })
            } else {
                None
            };

            if let Some(range) = text_ranges.get(&cmd_index) {
                for s_rect in &range.selections {
                    let s_quad = QuadInstance {
                        pos: [s_rect[0] * scale_factor, s_rect[1] * scale_factor],
                        quad_size: [s_rect[2] * scale_factor, s_rect[3] * scale_factor],
                        color: range.style.selection_bg.into(),
                        border_radii: [0.0; 4],
                        border_color: [0.0; 4],
                        border_widths: [0.0; 4],
                        shadow_color: [0.0; 4],
                        shadow_params: [0.0, 0.0, range.alpha, 0.0],
                        effects: [0.0; 4],
                        clip_rect,
                    };
                    push_solid_quad(quad_instances, draw_batches, s_quad);
                }

                let glyph_start = range.glyphs.start as u32;
                let glyph_end = range.glyphs.end as u32;
                let count = glyph_end.saturating_sub(glyph_start);
                if count > 0 {
                    if let Some(DrawBatch::TextGlyphs {
                        start: prev_start,
                        count: prev_count,
                        clip: prev_clip,
                    }) = draw_batches.last_mut()
                    {
                        if *prev_clip == text_clip && *prev_start + *prev_count == glyph_start {
                            *prev_count += count;
                        } else {
                            draw_batches.push(DrawBatch::TextGlyphs {
                                start: glyph_start,
                                count,
                                clip: text_clip,
                            });
                        }
                    } else {
                        draw_batches.push(DrawBatch::TextGlyphs {
                            start: glyph_start,
                            count,
                            clip: text_clip,
                        });
                    }
                }

                if let Some(c_rect) = range.caret {
                    let c_quad = QuadInstance {
                        pos: [c_rect[0] * scale_factor, c_rect[1] * scale_factor],
                        quad_size: [c_rect[2] * scale_factor, c_rect[3] * scale_factor],
                        color: range.style.caret_color.into(),
                        border_radii: [0.0; 4],
                        border_color: [0.0; 4],
                        border_widths: [0.0; 4],
                        shadow_color: [0.0; 4],
                        shadow_params: [0.0, 0.0, range.alpha, 0.0],
                        effects: [0.0; 4],
                        clip_rect,
                    };
                    push_solid_quad(quad_instances, draw_batches, c_quad);
                }

                for st_rect in &range.strikethroughs {
                    let st_quad = QuadInstance {
                        pos: [st_rect[0] * scale_factor, st_rect[1] * scale_factor],
                        quad_size: [st_rect[2] * scale_factor, st_rect[3] * scale_factor],
                        color: range.style.color.into(),
                        border_radii: [0.0; 4],
                        border_color: [0.0; 4],
                        border_widths: [0.0; 4],
                        shadow_color: [0.0; 4],
                        shadow_params: [0.0, 0.0, range.alpha, 0.0],
                        effects: [0.0; 4],
                        clip_rect,
                    };
                    push_solid_quad(quad_instances, draw_batches, st_quad);
                }

                for un_rect in &range.underlines {
                    let un_quad = QuadInstance {
                        pos: [un_rect[0] * scale_factor, un_rect[1] * scale_factor],
                        quad_size: [un_rect[2] * scale_factor, un_rect[3] * scale_factor],
                        color: range.style.color.into(),
                        border_radii: [0.0; 4],
                        border_color: [0.0; 4],
                        border_widths: [0.0; 4],
                        shadow_color: [0.0; 4],
                        shadow_params: [0.0, 0.0, range.alpha, 0.0],
                        effects: [0.0; 4],
                        clip_rect,
                    };
                    push_solid_quad(quad_instances, draw_batches, un_quad);
                }
            }
        } else if cmd.kind() == RenderCommandKind::ScrollbarV {
            let node = cmd.node();
            let scrollbar_style = node.get_scrollbar_style(context).unwrap_or_default();
            if scrollbar_style.visibility == ScrollbarVisibility::Never {
                return;
            }

            let computed = cmd.computed();
            let constraints = node.get_constraints(context).unwrap_or_default();
            let content_h = node.compute_content_height(context).max(computed.h) * scale_factor;
            let computed_h_scaled = computed.h * scale_factor;

            let should_render = match scrollbar_style.visibility {
                ScrollbarVisibility::Always => true,
                ScrollbarVisibility::Auto => content_h > computed_h_scaled + 0.5,
                ScrollbarVisibility::Never => false,
            };

            if should_render {
                let padding_top = (constraints.padding.top + constraints.border.top) * scale_factor;
                let padding_bottom =
                    (constraints.padding.bottom + constraints.border.bottom) * scale_factor;
                let track_h = (computed_h_scaled - padding_top - padding_bottom).max(0.0);
                if track_h > 0.0 {
                    let min_thumb_len = scrollbar_style.min_thumb_len * scale_factor;
                    let ratio = (computed_h_scaled / content_h).clamp(0.0, 1.0);
                    let thumb_h = (track_h * ratio).clamp(min_thumb_len.min(track_h), track_h);
                    let max_scroll_y = (content_h - computed_h_scaled).max(0.0);
                    let scroll_pct = if max_scroll_y > 0.0 {
                        ((constraints.scroll.y * scale_factor) / max_scroll_y).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let thumb_w = scrollbar_style.width * scale_factor;
                    let margin = scrollbar_style.margin * scale_factor;
                    let gap = scrollbar_style.gap * scale_factor;
                    let thumb_x = (computed.x + computed.w - constraints.border.right)
                        * scale_factor
                        - thumb_w
                        - margin;
                    let track_top = computed.y * scale_factor + padding_top;
                    let track_bottom = track_top + track_h;
                    let thumb_y = track_top + scroll_pct * (track_h - thumb_h);

                    let border_radii = [
                        scrollbar_style.radius.tl * scale_factor,
                        scrollbar_style.radius.tr * scale_factor,
                        scrollbar_style.radius.br * scale_factor,
                        scrollbar_style.radius.bl * scale_factor,
                    ];

                    // Material 3 segmented track quads
                    if let Some(track_color) = scrollbar_style.track_color {
                        let track_rgba: [f32; 4] = track_color.into();

                        // Top track segment: [track_top .. thumb_y - gap]
                        let top_track_h = (thumb_y - track_top - gap).max(0.0);
                        if top_track_h > 0.5 {
                            let top_track_quad = QuadInstance {
                                pos: [thumb_x, track_top],
                                quad_size: [thumb_w, top_track_h],
                                color: track_rgba,
                                border_radii,
                                border_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                shadow_color: [0.0; 4],
                                shadow_params: [0.0, 0.0, 1.0, 0.0],
                                effects: [0.0; 4],
                                clip_rect,
                            };
                            push_solid_quad(quad_instances, draw_batches, top_track_quad);
                        }

                        // Bottom track segment: [thumb_y + thumb_h + gap .. track_bottom]
                        let bot_track_y = thumb_y + thumb_h + gap;
                        let bot_track_h = (track_bottom - bot_track_y).max(0.0);
                        if bot_track_h > 0.5 {
                            let bot_track_quad = QuadInstance {
                                pos: [thumb_x, bot_track_y],
                                quad_size: [thumb_w, bot_track_h],
                                color: track_rgba,
                                border_radii,
                                border_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                shadow_color: [0.0; 4],
                                shadow_params: [0.0, 0.0, 1.0, 0.0],
                                effects: [0.0; 4],
                                clip_rect,
                            };
                            push_solid_quad(quad_instances, draw_batches, bot_track_quad);
                        }
                    }

                    // Draggable Thumb
                    let thumb_quad = QuadInstance {
                        pos: [thumb_x, thumb_y],
                        quad_size: [thumb_w, thumb_h],
                        color: scrollbar_style.thumb_color.into(),
                        border_radii,
                        border_color: [0.0; 4],
                        border_widths: [0.0; 4],
                        shadow_color: [0.0; 4],
                        shadow_params: [0.0, 0.0, 1.0, 0.0],
                        effects: [0.0; 4],
                        clip_rect,
                    };
                    push_solid_quad(quad_instances, draw_batches, thumb_quad);
                }
            }
        } else if cmd.kind() == RenderCommandKind::ScrollbarH {
            let node = cmd.node();
            let scrollbar_style = node.get_scrollbar_style(context).unwrap_or_default();
            if scrollbar_style.visibility == ScrollbarVisibility::Never {
                return;
            }

            let computed = cmd.computed();
            let constraints = node.get_constraints(context).unwrap_or_default();
            let content_w = computed.content_w.max(computed.w) * scale_factor;
            let computed_w_scaled = computed.w * scale_factor;

            let should_render = match scrollbar_style.visibility {
                ScrollbarVisibility::Always => true,
                ScrollbarVisibility::Auto => content_w > computed_w_scaled + 0.5,
                ScrollbarVisibility::Never => false,
            };

            if should_render {
                let padding_left =
                    (constraints.padding.left + constraints.border.left) * scale_factor;
                let padding_right =
                    (constraints.padding.right + constraints.border.right) * scale_factor;
                let track_w = (computed_w_scaled - padding_left - padding_right).max(0.0);
                if track_w > 0.0 {
                    let min_thumb_len = scrollbar_style.min_thumb_len * scale_factor;
                    let ratio = (computed_w_scaled / content_w).clamp(0.0, 1.0);
                    let thumb_w = (track_w * ratio).clamp(min_thumb_len.min(track_w), track_w);
                    let max_scroll_x = (content_w - computed_w_scaled).max(0.0);
                    let scroll_pct = if max_scroll_x > 0.0 {
                        ((constraints.scroll.x * scale_factor) / max_scroll_x).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let thumb_h = scrollbar_style.width * scale_factor;
                    let margin = scrollbar_style.margin * scale_factor;
                    let gap = scrollbar_style.gap * scale_factor;
                    let track_left = computed.x * scale_factor + padding_left;
                    let track_right = track_left + track_w;
                    let thumb_x = track_left + scroll_pct * (track_w - thumb_w);
                    let thumb_y = (computed.y + computed.h - constraints.border.bottom)
                        * scale_factor
                        - thumb_h
                        - margin;

                    let border_radii = [
                        scrollbar_style.radius.tl * scale_factor,
                        scrollbar_style.radius.tr * scale_factor,
                        scrollbar_style.radius.br * scale_factor,
                        scrollbar_style.radius.bl * scale_factor,
                    ];

                    // Material 3 segmented track quads
                    if let Some(track_color) = scrollbar_style.track_color {
                        let track_rgba: [f32; 4] = track_color.into();

                        // Left track segment: [track_left .. thumb_x - gap]
                        let left_track_w = (thumb_x - track_left - gap).max(0.0);
                        if left_track_w > 0.5 {
                            let left_track_quad = QuadInstance {
                                pos: [track_left, thumb_y],
                                quad_size: [left_track_w, thumb_h],
                                color: track_rgba,
                                border_radii,
                                border_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                shadow_color: [0.0; 4],
                                shadow_params: [0.0, 0.0, 1.0, 0.0],
                                effects: [0.0; 4],
                                clip_rect,
                            };
                            push_solid_quad(quad_instances, draw_batches, left_track_quad);
                        }

                        // Right track segment: [thumb_x + thumb_w + gap .. track_right]
                        let right_track_x = thumb_x + thumb_w + gap;
                        let right_track_w = (track_right - right_track_x).max(0.0);
                        if right_track_w > 0.5 {
                            let right_track_quad = QuadInstance {
                                pos: [right_track_x, thumb_y],
                                quad_size: [right_track_w, thumb_h],
                                color: track_rgba,
                                border_radii,
                                border_color: [0.0; 4],
                                border_widths: [0.0; 4],
                                shadow_color: [0.0; 4],
                                shadow_params: [0.0, 0.0, 1.0, 0.0],
                                effects: [0.0; 4],
                                clip_rect,
                            };
                            push_solid_quad(quad_instances, draw_batches, right_track_quad);
                        }
                    }

                    // Draggable Thumb
                    let thumb_quad = QuadInstance {
                        pos: [thumb_x, thumb_y],
                        quad_size: [thumb_w, thumb_h],
                        color: scrollbar_style.thumb_color.into(),
                        border_radii,
                        border_color: [0.0; 4],
                        border_widths: [0.0; 4],
                        shadow_color: [0.0; 4],
                        shadow_params: [0.0, 0.0, 1.0, 0.0],
                        effects: [0.0; 4],
                        clip_rect,
                    };
                    push_solid_quad(quad_instances, draw_batches, thumb_quad);
                }
            }
        }
    }
}

fn prepare_debug_highlight(
    context: &crate::Context,
    quad_instances: &mut Vec<QuadInstance>,
    draw_batches: &mut Vec<DrawBatch>,
) {
    let Some(highlighted) = context.highlight_node else {
        return;
    };

    let scale_factor = context.scale_factor.max(0.1);
    if let Some(computed) = highlighted.get_computed(context) {
        let effects = highlighted.get_effects(context).unwrap_or_default();
        let outline_thickness = 2.0 * scale_factor;
        let highlight_quad = QuadInstance {
            pos: [computed.x * scale_factor, computed.y * scale_factor],
            quad_size: [computed.w * scale_factor, computed.h * scale_factor],
            color: [0.06, 0.72, 0.95, 0.15],
            border_radii: [
                effects.border.radius.tl * scale_factor,
                effects.border.radius.tr * scale_factor,
                effects.border.radius.br * scale_factor,
                effects.border.radius.bl * scale_factor,
            ],
            border_color: [0.06, 0.72, 0.95, 0.9],
            border_widths: [outline_thickness; 4],
            shadow_color: [0.0; 4],
            shadow_params: [0.0, 0.0, 1.0, 0.0],
            effects: [0.0; 4],
            clip_rect: [-1.0, -1.0, -1.0, -1.0],
        };
        push_solid_quad(quad_instances, draw_batches, highlight_quad);
    }
}

fn execute_draw_batches<'a>(
    render_pass: &mut wgpu::RenderPass<'a>,
    batches: &[DrawBatch],
    screen_width: u32,
    screen_height: u32,
    pipelines: &'a Pipelines,
    solid_bind_group: &'a wgpu::BindGroup,
    text_bind_group: &'a wgpu::BindGroup,
    quad_buffer: &'a wgpu::Buffer,
    canvas_textures: &'a HashMap<crate::Node, CanvasGpuResource>,
    image_textures: &'a HashMap<crate::Node, ImageGpuResource>,
    svg_textures: &'a HashMap<crate::Node, SvgGpuResource>,
) {
    let default_rect = (0, 0, screen_width.max(1), screen_height.max(1));
    render_pass.set_scissor_rect(
        default_rect.0,
        default_rect.1,
        default_rect.2,
        default_rect.3,
    );
    let mut current_scissor_is_default = true;

    for batch in batches {
        match batch {
            DrawBatch::SolidQuads { start, count } => {
                if !current_scissor_is_default {
                    render_pass.set_scissor_rect(
                        default_rect.0,
                        default_rect.1,
                        default_rect.2,
                        default_rect.3,
                    );
                    current_scissor_is_default = true;
                }
                render_pass.set_pipeline(&pipelines.solid);
                render_pass.set_bind_group(0, solid_bind_group, &[]);
                render_pass.set_vertex_buffer(0, quad_buffer.slice(..));
                let push_constants = SolidPushConstants {
                    screen_size: [screen_width as f32, screen_height as f32],
                };
                render_pass.set_immediates(0, bytemuck::bytes_of(&push_constants));
                render_pass.draw(0..6, *start..(*start + *count));
            }
            DrawBatch::TextGlyphs { start, count, clip } => {
                if let Some(c) = clip {
                    if let Some((nx, ny, nw, nh)) =
                        compute_scissor_rect(*c, screen_width, screen_height)
                    {
                        render_pass.set_scissor_rect(nx, ny, nw, nh);
                        current_scissor_is_default = false;
                    } else {
                        continue;
                    }
                } else if !current_scissor_is_default {
                    render_pass.set_scissor_rect(
                        default_rect.0,
                        default_rect.1,
                        default_rect.2,
                        default_rect.3,
                    );
                    current_scissor_is_default = true;
                }
                render_pass.set_pipeline(&pipelines.text);
                render_pass.set_bind_group(0, text_bind_group, &[]);
                let screen_size = [screen_width as f32, screen_height as f32];
                render_pass.set_immediates(0, bytemuck::bytes_of(&screen_size));
                render_pass.draw(0..6, *start..(*start + *count));
            }
            DrawBatch::CanvasTexture {
                node,
                immediate,
                clip,
            } => {
                if let Some(c) = clip {
                    if let Some((nx, ny, nw, nh)) =
                        compute_scissor_rect(*c, screen_width, screen_height)
                    {
                        render_pass.set_scissor_rect(nx, ny, nw, nh);
                        current_scissor_is_default = false;
                    }
                } else if !current_scissor_is_default {
                    render_pass.set_scissor_rect(
                        default_rect.0,
                        default_rect.1,
                        default_rect.2,
                        default_rect.3,
                    );
                    current_scissor_is_default = true;
                }
                if let Some(canvas_res) = canvas_textures.get(node) {
                    render_pass.set_pipeline(&pipelines.texture);
                    render_pass.set_bind_group(0, &canvas_res.bind_group, &[]);
                    render_pass.set_immediates(0, bytemuck::bytes_of(immediate));
                    render_pass.draw(0..6, 0..1);
                }
            }
            DrawBatch::ImageTexture {
                node,
                immediate,
                clip,
            } => {
                if let Some(c) = clip {
                    if let Some((nx, ny, nw, nh)) =
                        compute_scissor_rect(*c, screen_width, screen_height)
                    {
                        render_pass.set_scissor_rect(nx, ny, nw, nh);
                        current_scissor_is_default = false;
                    }
                } else if !current_scissor_is_default {
                    render_pass.set_scissor_rect(
                        default_rect.0,
                        default_rect.1,
                        default_rect.2,
                        default_rect.3,
                    );
                    current_scissor_is_default = true;
                }
                if let Some(image_res) = image_textures.get(node) {
                    render_pass.set_pipeline(&pipelines.texture);
                    render_pass.set_bind_group(0, &image_res.bind_group, &[]);
                    render_pass.set_immediates(0, bytemuck::bytes_of(immediate));
                    render_pass.draw(0..6, 0..1);
                }
            }
            DrawBatch::SvgTexture {
                node,
                immediate,
                clip,
            } => {
                if let Some(c) = clip {
                    if let Some((nx, ny, nw, nh)) =
                        compute_scissor_rect(*c, screen_width, screen_height)
                    {
                        render_pass.set_scissor_rect(nx, ny, nw, nh);
                        current_scissor_is_default = false;
                    }
                } else if !current_scissor_is_default {
                    render_pass.set_scissor_rect(
                        default_rect.0,
                        default_rect.1,
                        default_rect.2,
                        default_rect.3,
                    );
                    current_scissor_is_default = true;
                }
                if let Some(svg_res) = svg_textures.get(node) {
                    render_pass.set_pipeline(&pipelines.texture);
                    render_pass.set_bind_group(0, &svg_res.bind_group, &[]);
                    render_pass.set_immediates(0, bytemuck::bytes_of(immediate));
                    render_pass.draw(0..6, 0..1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Context;
    use crate::style::Size;

    #[test]
    fn test_hierarchical_scale_cascading() {
        let mut ctx = Context::new();

        let parent = ctx.create_node();
        parent.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(100);
            c.height = Size::Fixed(100);
        });
        parent.update_effects(&mut ctx, |e| {
            e.scale = 0.5;
        });

        let child = ctx.create_node();
        child.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(20);
            c.height = Size::Fixed(20);
        });
        child.update_effects(&mut ctx, |e| {
            e.scale = 0.8;
        });

        parent.append(&mut ctx, child);
        ctx.root_attach(parent);
        ctx.compute_layout(100.0, 100.0);

        // Effective scale compounds: 0.5 * 0.8 = 0.4
        let parent_scale = compute_effective_scale(&ctx, parent);
        let child_scale = compute_effective_scale(&ctx, child);
        assert!((parent_scale - 0.5).abs() < 1e-4);
        assert!((child_scale - 0.4).abs() < 1e-4);

        // Transform child center point relative to parent scale
        // Parent center is (50, 50). Child is at (0, 0) relative to parent -> center (10, 10).
        // (10 - 50) * 0.5 + 50 = -20 + 50 = 30.
        let child_center = (10.0, 10.0);
        let transformed = transform_point_ancestors(&ctx, child, child_center);
        assert!((transformed.0 - 30.0).abs() < 1e-4);
        assert!((transformed.1 - 30.0).abs() < 1e-4);
    }

    #[test]
    fn test_batched_quad_collection_and_analytical_scissoring() {
        let mut ctx = Context::new();

        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(800);
            c.height = Size::Fixed(600);
            c.flex_direction = crate::FlexDirection::Column;
        });

        for i in 0..10 {
            let child = ctx.create_node();
            child.update_constraints(&mut ctx, |c| {
                c.width = Size::Fixed(100);
                c.height = Size::Fixed(40);
            });
            child.update_effects(&mut ctx, |e| {
                e.background_color = if i % 2 == 0 {
                    crate::colors::Color::red
                } else {
                    crate::colors::Color::blue
                };
            });
            root.append(&mut ctx, child);
        }

        ctx.root_attach(root);
        ctx.compute_layout(800.0, 600.0);
        ctx.build_render_list(crate::style::Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        });

        let mut quad_instances = Vec::new();
        let mut draw_batches = Vec::new();
        let dummy_text_ranges = HashMap::new();
        let dummy_canvas = HashMap::new();
        let dummy_images = HashMap::new();
        let dummy_svgs = HashMap::new();

        prepare_command_slice(
            ctx.render_list().enumerate(),
            800,
            600,
            &dummy_text_ranges,
            &dummy_canvas,
            &dummy_images,
            &dummy_svgs,
            &ctx,
            &mut quad_instances,
            &mut draw_batches,
        );

        // 1 root + 10 children = 11 quads
        assert_eq!(quad_instances.len(), 11);
        // All 11 consecutive solid quads MUST be coalesced into a single draw batch!
        assert_eq!(draw_batches.len(), 1);
        match &draw_batches[0] {
            DrawBatch::SolidQuads { start, count } => {
                assert_eq!(*start, 0);
                assert_eq!(*count, 11);
            }
            _ => panic!("Expected DrawBatch::SolidQuads"),
        }
    }

    #[test]
    fn test_analytical_scissoring_preserves_single_batch() {
        let mut ctx = Context::new();

        let scroll_container = ctx.create_node();
        scroll_container.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(300);
            c.height = Size::Fixed(200);
            c.overflow = crate::Overflow::Scroll;
        });

        for _ in 0..5 {
            let item = ctx.create_node();
            item.update_constraints(&mut ctx, |c| {
                c.width = Size::Fixed(100);
                c.height = Size::Fixed(50);
            });
            scroll_container.append(&mut ctx, item);
        }

        ctx.root_attach(scroll_container);
        ctx.compute_layout(800.0, 600.0);
        ctx.build_render_list(crate::style::Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        });

        let mut quad_instances = Vec::new();
        let mut draw_batches = Vec::new();
        let dummy_text_ranges = HashMap::new();
        let dummy_canvas = HashMap::new();
        let dummy_images = HashMap::new();
        let dummy_svgs = HashMap::new();

        prepare_command_slice(
            ctx.render_list().enumerate(),
            800,
            600,
            &dummy_text_ranges,
            &dummy_canvas,
            &dummy_images,
            &dummy_svgs,
            &ctx,
            &mut quad_instances,
            &mut draw_batches,
        );

        // All quads (scroll container + 5 clipped items) must be in ONE single draw batch!
        assert_eq!(draw_batches.len(), 1);
        match &draw_batches[0] {
            DrawBatch::SolidQuads { count, .. } => {
                assert_eq!(*count, 6);
            }
            _ => panic!("Expected DrawBatch::SolidQuads"),
        }

        // Children must have non-negative clip_rect matching the scroll container clip
        for instance in &quad_instances[1..] {
            assert!(instance.clip_rect[2] > 0.0);
            assert!(instance.clip_rect[3] > 0.0);
            assert_eq!(instance.clip_rect[2], 300.0);
            assert_eq!(instance.clip_rect[3], 200.0);
        }
    }

    #[test]
    fn test_text_batch_receives_scroll_container_clip() {
        let mut ctx = Context::new();

        let scroll_container = ctx.create_node();
        scroll_container.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(300);
            c.height = Size::Fixed(200);
            c.overflow = crate::Overflow::Scroll;
        });

        let text_node = ctx.create_node();
        text_node.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(150);
            c.height = Size::Fixed(30);
        });
        text_node.set_text(&mut ctx, "Hello Scroll");
        scroll_container.append(&mut ctx, text_node);

        ctx.root_attach(scroll_container);
        ctx.compute_layout(800.0, 600.0);
        ctx.build_render_list(crate::style::Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        });

        // Verify that the text RenderCommand has clip assigned from its parent scrollview
        let text_cmd = ctx
            .render_list()
            .enumerate()
            .find(|(_, cmd)| cmd.kind() == RenderCommandKind::Text);
        assert!(text_cmd.is_some());
        let (cmd_idx, cmd) = text_cmd.unwrap();
        assert!(cmd.has_clip());
        assert_eq!(cmd.clip().w, 300.0);
        assert_eq!(cmd.clip().h, 200.0);

        let mut quad_instances = Vec::new();
        let mut draw_batches = Vec::new();
        let mut text_ranges = HashMap::new();
        text_ranges.insert(
            cmd_idx,
            RenderTextData {
                glyphs: 0..12,
                selections: Vec::new(),
                strikethroughs: Vec::new(),
                underlines: Vec::new(),
                caret: None,
                style: Default::default(),
                alpha: 1.0,
            },
        );
        let dummy_canvas = HashMap::new();
        let dummy_images = HashMap::new();
        let dummy_svgs = HashMap::new();

        prepare_command_slice(
            ctx.render_list().enumerate(),
            800,
            600,
            &text_ranges,
            &dummy_canvas,
            &dummy_images,
            &dummy_svgs,
            &ctx,
            &mut quad_instances,
            &mut draw_batches,
        );

        // Find the TextGlyphs batch and ensure clip matches the scroll container
        let text_batch = draw_batches
            .iter()
            .find(|b| matches!(b, DrawBatch::TextGlyphs { .. }));
        assert!(text_batch.is_some());
        match text_batch.unwrap() {
            DrawBatch::TextGlyphs { clip, count, .. } => {
                assert_eq!(*count, 12);
                assert!(clip.is_some());
                let clip_rect = clip.unwrap();
                assert_eq!(clip_rect.w, 300.0);
                assert_eq!(clip_rect.h, 200.0);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_scale_factor_scaling_quads_and_clips() {
        let mut ctx = Context::new();
        ctx.scale_factor = 1.25;

        let container = ctx.create_node();
        container.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(100);
            c.border = crate::style::Edges {
                top: 2.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            };
            c.overflow = crate::Overflow::Scroll;
        });

        let text_node = ctx.create_node();
        text_node.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(100);
            c.height = Size::Fixed(20);
        });
        text_node.set_text(&mut ctx, "Scaled Text");
        container.append(&mut ctx, text_node);

        ctx.root_attach(container);
        ctx.compute_layout(800.0, 600.0);
        ctx.build_render_list(crate::style::Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        });

        let text_cmd = ctx
            .render_list()
            .enumerate()
            .find(|(_, cmd)| cmd.kind() == RenderCommandKind::Text);
        assert!(text_cmd.is_some());
        let (cmd_idx, _) = text_cmd.unwrap();

        let mut quad_instances = Vec::new();
        let mut draw_batches = Vec::new();
        let mut text_ranges = HashMap::new();
        text_ranges.insert(
            cmd_idx,
            RenderTextData {
                glyphs: 0..11,
                selections: vec![[10.0, 5.0, 50.0, 15.0]],
                strikethroughs: Vec::new(),
                underlines: Vec::new(),
                caret: Some([20.0, 5.0, 2.0, 15.0]),
                style: Default::default(),
                alpha: 1.0,
            },
        );
        let dummy_canvas = HashMap::new();
        let dummy_images = HashMap::new();
        let dummy_svgs = HashMap::new();

        prepare_command_slice(
            ctx.render_list().enumerate(),
            1000,
            750,
            &text_ranges,
            &dummy_canvas,
            &dummy_images,
            &dummy_svgs,
            &ctx,
            &mut quad_instances,
            &mut draw_batches,
        );

        assert!(!quad_instances.is_empty());
        let container_quad = quad_instances[0];
        assert_eq!(container_quad.quad_size, [250.0, 125.0]);
        assert_eq!(container_quad.border_widths, [2.5, 2.5, 2.5, 2.5]);

        let text_batch = draw_batches
            .iter()
            .find(|b| matches!(b, DrawBatch::TextGlyphs { .. }));
        assert!(text_batch.is_some());
        if let DrawBatch::TextGlyphs { clip, .. } = text_batch.unwrap() {
            let clip_rect = clip.unwrap();
            assert_eq!(clip_rect.w, 245.0); // (200 - 2*2) * 1.25
            assert_eq!(clip_rect.h, 120.0); // (100 - 2*2) * 1.25
        }

        let selection_quad = quad_instances
            .iter()
            .find(|q| q.quad_size == [50.0 * 1.25, 15.0 * 1.25]);
        assert!(selection_quad.is_some());
        assert_eq!(selection_quad.unwrap().pos, [12.5, 6.25]);

        let caret_quad = quad_instances
            .iter()
            .find(|q| q.quad_size == [2.0 * 1.25, 15.0 * 1.25]);
        assert!(caret_quad.is_some());
        assert_eq!(caret_quad.unwrap().pos, [25.0, 6.25]);
    }
}
