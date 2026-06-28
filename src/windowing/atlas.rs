use cosmic_text::CacheKey;
use std::collections::HashMap;
use wgpu::{Device, Extent3d, Queue, Texture, TextureDescriptor, TextureFormat};

pub struct Atlas {
    pub texture: Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,

    pub size: u32,

    glyphs: HashMap<CacheKey, GlyphInfo>,

    next_x: u32,
    next_y: u32,
    row_height: u32,
}

#[derive(Clone, Copy)]
pub struct GlyphInfo {
    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_w: f32,
    pub uv_h: f32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub physical_w: u32,
    pub physical_h: u32,
}

impl Atlas {
    pub fn new(device: &Device) -> Self {
        let size = 2048;

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Text Atlas"),
            size: Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size,
            glyphs: HashMap::new(),
            next_x: 0,
            next_y: 0,
            row_height: 0,
        }
    }

    pub fn get_or_insert(
        &mut self,
        queue: &Queue,
        swash_cache: &mut cosmic_text::SwashCache,
        font_system: &mut cosmic_text::FontSystem,
        cache_key: CacheKey,
    ) -> Option<GlyphInfo> {
        if let Some(info) = self.glyphs.get(&cache_key) {
            return Some(*info);
        }

        let image = swash_cache.get_image_uncached(font_system, cache_key)?;

        let width = image.placement.width;
        let height = image.placement.height;

        if width == 0 || height == 0 {
            let info = GlyphInfo {
                uv_x: 0.0,
                uv_y: 0.0,
                uv_w: 0.0,
                uv_h: 0.0,
                offset_x: image.placement.left,
                offset_y: -image.placement.top,
                physical_w: 0,
                physical_h: 0,
            };
            self.glyphs.insert(cache_key, info);
            return Some(info);
        }

        // NOTE: We convert data if necessary (we assume R8Unorm for simplicity, if color, we extract A or convert to grayscale)
        let data = match image.content {
            cosmic_text::SwashContent::Mask => image.data.clone(),
            cosmic_text::SwashContent::Color => {
                // If it's color (like emojis), we convert to just alpha or intensity for now.
                // TODO: Normally we'd want an RGBA atlas, but for now we fallback to R8Unorm.
                let mut mask = Vec::with_capacity((width * height) as usize);
                for chunk in image.data.chunks_exact(4) {
                    mask.push(chunk[3]); // Take alpha
                }
                mask
            }
            cosmic_text::SwashContent::SubpixelMask => {
                // NOTE: we ignore subpixels for simplicity
                let mut mask = Vec::with_capacity((width * height) as usize);
                for chunk in image.data.chunks_exact(3) {
                    mask.push(chunk[1]); // take green
                }
                mask
            }
        };

        if self.next_x + width > self.size {
            self.next_x = 0;
            self.next_y += self.row_height + 1;
            self.row_height = 0;
        }

        if self.next_y + height > self.size {
            return None; // Out of atlas memory
        }

        let x = self.next_x;
        let y = self.next_y;

        self.next_x += width + 1;
        self.row_height = self.row_height.max(height);

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let info = GlyphInfo {
            uv_x: x as f32 / self.size as f32,
            uv_y: y as f32 / self.size as f32,
            uv_w: width as f32 / self.size as f32,
            uv_h: height as f32 / self.size as f32,
            offset_x: image.placement.left,
            offset_y: -image.placement.top,
            physical_w: width,
            physical_h: height,
        };

        self.glyphs.insert(cache_key, info);
        Some(info)
    }
}
