use std::collections::HashMap;
use swash::scale::image::Content;
use swash::scale::{Render, Scaler, Source, StrikeWith};
use swash::zeno::{Format, Vector};
use wgpu::{Device, Extent3d, Queue, Texture, TextureDescriptor, TextureFormat};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub font_ptr: usize,
    pub font_size: u32,
    pub glyph_id: u16,
}

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
    pub is_color: bool,
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
            format: TextureFormat::Rgba8UnormSrgb, // Support color glyphs
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
        scaler: &mut Scaler<'_>,
        cache_key: CacheKey,
    ) -> Option<GlyphInfo> {
        if let Some(info) = self.glyphs.get(&cache_key) {
            return Some(*info);
        }

        let rendered_glyph = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(Vector::new(0.0, 0.0)) // Ignore subpixel offset for now
        .render(scaler, cache_key.glyph_id)?;

        let width = rendered_glyph.placement.width;
        let height = rendered_glyph.placement.height;

        if width == 0 || height == 0 {
            let info = GlyphInfo {
                uv_x: 0.0,
                uv_y: 0.0,
                uv_w: 0.0,
                uv_h: 0.0,
                offset_x: rendered_glyph.placement.left,
                offset_y: -rendered_glyph.placement.top,
                physical_w: 0,
                physical_h: 0,
                is_color: false,
            };
            self.glyphs.insert(cache_key, info);
            return Some(info);
        }

        // We need RGBA data for the atlas
        let (data, is_color) = match rendered_glyph.content {
            Content::Mask => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for &alpha in rendered_glyph.data.iter() {
                    rgba.push(255);
                    rgba.push(255);
                    rgba.push(255);
                    rgba.push(alpha);
                }
                (rgba, false)
            }
            Content::Color => (rendered_glyph.data.clone(), true),
            Content::SubpixelMask => {
                // Convert subpixel to alpha by taking green
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for chunk in rendered_glyph.data.chunks_exact(3) {
                    rgba.push(255);
                    rgba.push(255);
                    rgba.push(255);
                    rgba.push(chunk[1]);
                }
                (rgba, false)
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
                bytes_per_row: Some(width * 4),
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
            offset_x: rendered_glyph.placement.left,
            offset_y: -rendered_glyph.placement.top,
            physical_w: width,
            physical_h: height,
            is_color,
        };

        self.glyphs.insert(cache_key, info);
        Some(info)
    }
}
