use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;

/// Immediate push constant data for solid and texture shader pipelines.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ImmediateData {
    pub color: [f32; 4],
    pub pos: [f32; 2],
    pub screen_size: [f32; 2],
    pub quad_size: [f32; 2],
    pub alpha: f32,
    pub _pad0: f32,
    pub border_radii: [f32; 4], // tl, tr, br, bl
    pub border_color: [f32; 4],
    pub shadow_color: [f32; 4],
    pub border_widths: [f32; 4], // top, right, bottom, left
    pub shadow_spread: f32,
    pub shadow_power: f32,
    pub vibrancy: f32,
    pub vibrancy_darkness: f32,
    pub passes: f32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
}

/// GPU Render Pipelines and associated Bind Group Layouts.
pub struct Pipelines {
    pub solid: wgpu::RenderPipeline,
    pub text: wgpu::RenderPipeline,
    pub texture: wgpu::RenderPipeline,

    pub solid_bind_group_layout: wgpu::BindGroupLayout,
    pub dummy_solid_bind_group: wgpu::BindGroup,
    pub text_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_sampler: wgpu::Sampler,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        // 1. Shaders
        let solid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Solid Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/solid.wgsl"))),
        });

        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/text.wgsl"))),
        });

        let texture_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/texture.wgsl"))),
        });

        // 2. Bind Group Layouts
        let solid_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Solid Pipeline Bind Group Layout (Frosted Texture & Sampler)"),
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
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

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // 3. Fallback dummy texture for solid pipeline when no blur is sampled
        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dummy Solid Texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_solid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dummy Solid Bind Group"),
            layout: &solid_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&dummy_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
            ],
        });

        // 4. Pipeline Layouts
        let solid_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Solid Pipeline Layout"),
                bind_group_layouts: &[Some(&solid_bind_group_layout)],
                immediate_size: std::mem::size_of::<ImmediateData>() as u32,
            });

        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[Some(&text_bind_group_layout)],
            immediate_size: 8, // pc.screen_size: vec2<f32>
        });

        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Texture Pipeline Layout"),
                bind_group_layouts: &[Some(&texture_bind_group_layout)],
                immediate_size: std::mem::size_of::<ImmediateData>() as u32,
            });

        // 5. Render Pipelines
        let solid = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Solid Pipeline"),
            layout: Some(&solid_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &solid_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &solid_shader,
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let text = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let texture = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Texture Pipeline"),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            solid,
            text,
            texture,
            solid_bind_group_layout,
            dummy_solid_bind_group,
            text_bind_group_layout,
            texture_bind_group_layout,
            texture_sampler,
        }
    }
}
