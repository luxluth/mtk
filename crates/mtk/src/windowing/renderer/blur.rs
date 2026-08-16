use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BlurParams {
    pub src_size: [f32; 2],
    pub dst_size: [f32; 2],
    pub offset: f32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
}

const _: () = assert!(std::mem::size_of::<BlurParams>() == 32);

pub struct PyramidLevel {
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

pub struct BlurPipeline {
    pub downsample_pipeline: wgpu::ComputePipeline,
    pub upsample_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub linear_sampler: wgpu::Sampler,
    pub pyramid: Vec<PyramidLevel>,
    pub width: u32,
    pub height: u32,
}

impl BlurPipeline {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/blur.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blur Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blur Compute Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: std::mem::size_of::<BlurParams>() as u32,
        });

        let downsample_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Dual Kawase Downsample Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_downsample"),
                compilation_options: Default::default(),
                cache: None,
            });

        let upsample_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Dual Kawase Upsample Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_upsample"),
            compilation_options: Default::default(),
            cache: None,
        });

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blur Linear Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let mut instance = Self {
            downsample_pipeline,
            upsample_pipeline,
            bind_group_layout,
            linear_sampler,
            pyramid: Vec::new(),
            width: 0,
            height: 0,
        };

        instance.resize(device, width.max(1), height.max(1));
        instance
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height && !self.pyramid.is_empty() {
            return;
        }
        self.width = width.max(1);
        self.height = height.max(1);
        self.pyramid.clear();

        // 3 pyramid levels (1/2, 1/4, 1/8)
        let mut cur_w = (self.width / 2).max(1);
        let mut cur_h = (self.height / 2).max(1);

        for i in 0..3 {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Blur Pyramid Level {}", i)),
                size: wgpu::Extent3d {
                    width: cur_w,
                    height: cur_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            self.pyramid.push(PyramidLevel {
                texture,
                view,
                width: cur_w,
                height: cur_h,
            });

            cur_w = (cur_w / 2).max(1);
            cur_h = (cur_h / 2).max(1);
        }
    }

    pub fn execute<'a>(
        &'a self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &'a wgpu::TextureView,
        src_w: u32,
        src_h: u32,
    ) -> &'a wgpu::TextureView {
        if self.pyramid.len() < 3 {
            return src_view;
        }

        // Pass 0: Source (full resolution) -> Level 0 (1/2)
        {
            let dst = &self.pyramid[0];
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blur Pass 0 BindGroup"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&dst.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                ],
            });

            let params = BlurParams {
                src_size: [src_w as f32, src_h as f32],
                dst_size: [dst.width as f32, dst.height as f32],
                offset: 1.0,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Blur Downsample Pass 0"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.downsample_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_immediates(0, bytemuck::bytes_of(&params));
            pass.dispatch_workgroups((dst.width + 7) / 8, (dst.height + 7) / 8, 1);
        }

        // Pass 1: Level 0 (1/2) -> Level 1 (1/4)
        {
            let src = &self.pyramid[0];
            let dst = &self.pyramid[1];
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blur Pass 1 BindGroup"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&dst.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                ],
            });

            let params = BlurParams {
                src_size: [src.width as f32, src.height as f32],
                dst_size: [dst.width as f32, dst.height as f32],
                offset: 1.0,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Blur Downsample Pass 1"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.downsample_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_immediates(0, bytemuck::bytes_of(&params));
            pass.dispatch_workgroups((dst.width + 7) / 8, (dst.height + 7) / 8, 1);
        }

        // Pass 2: Level 1 (1/4) -> Level 2 (1/8)
        {
            let src = &self.pyramid[1];
            let dst = &self.pyramid[2];
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blur Pass 2 BindGroup"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&dst.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                ],
            });

            let params = BlurParams {
                src_size: [src.width as f32, src.height as f32],
                dst_size: [dst.width as f32, dst.height as f32],
                offset: 2.0,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Blur Downsample Pass 2"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.downsample_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_immediates(0, bytemuck::bytes_of(&params));
            pass.dispatch_workgroups((dst.width + 7) / 8, (dst.height + 7) / 8, 1);
        }

        // Pass 3: Level 2 (1/8) -> Level 1 (1/4) (Upsample)
        {
            let src = &self.pyramid[2];
            let dst = &self.pyramid[1];
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blur Pass 3 BindGroup"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&dst.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                ],
            });

            let params = BlurParams {
                src_size: [src.width as f32, src.height as f32],
                dst_size: [dst.width as f32, dst.height as f32],
                offset: 2.0,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Blur Upsample Pass 3"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.upsample_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_immediates(0, bytemuck::bytes_of(&params));
            pass.dispatch_workgroups((dst.width + 7) / 8, (dst.height + 7) / 8, 1);
        }

        // Pass 4: Level 1 (1/4) -> Level 0 (1/2) (Upsample)
        {
            let src = &self.pyramid[1];
            let dst = &self.pyramid[0];
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blur Pass 4 BindGroup"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&dst.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                ],
            });

            let params = BlurParams {
                src_size: [src.width as f32, src.height as f32],
                dst_size: [dst.width as f32, dst.height as f32],
                offset: 1.0,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            };

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Blur Upsample Pass 4"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.upsample_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_immediates(0, bytemuck::bytes_of(&params));
            pass.dispatch_workgroups((dst.width + 7) / 8, (dst.height + 7) / 8, 1);
        }

        &self.pyramid[0].view
    }
}
