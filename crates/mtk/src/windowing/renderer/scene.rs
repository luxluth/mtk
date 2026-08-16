/// Manages the intermediate offscreen scene render target and its GPU resources.
pub struct SceneTarget {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub blit_bind_group: wgpu::BindGroup,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl SceneTarget {
    pub fn new(
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let (texture, view, blit_bind_group) = Self::create_resources(
            device,
            texture_bind_group_layout,
            sampler,
            format,
            width,
            height,
        );

        Self {
            texture,
            view,
            blit_bind_group,
            format,
            width,
            height,
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
    ) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        let (texture, view, blit_bind_group) = Self::create_resources(
            device,
            texture_bind_group_layout,
            sampler,
            self.format,
            width,
            height,
        );

        self.texture = texture;
        self.view = view;
        self.blit_bind_group = blit_bind_group;
    }

    fn create_resources(
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
        let w = width.max(1);
        let h = height.max(1);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Scene Render Target Texture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Blit Bind Group"),
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        (texture, view, blit_bind_group)
    }
}
