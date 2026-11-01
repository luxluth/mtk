use bytemuck::{Pod, Zeroable};

/// GPU vertex attribute layout and per-instance data for solid quads.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],
    pub quad_size: [f32; 2],
    pub color: [f32; 4],
    pub border_radii: [f32; 4], // tl, tr, br, bl
    pub border_color: [f32; 4],
    pub border_widths: [f32; 4], // top, right, bottom, left
    pub shadow_color: [f32; 4],
    pub shadow_offset: [f32; 4], // offset_x, offset_y, blur_radius, spread_radius
    pub shadow_params: [f32; 4], // inset (1.0 or 0.0), _pad0, _pad1, alpha
    pub effects: [f32; 4],       // vibrancy, vibrancy_darkness, passes, _pad
    pub clip_rect: [f32; 4],     // clip_x, clip_y, clip_w, clip_h (w<=0 or h<=0 disables clipping)
    pub gradient_color: [f32; 4], // stop 1 color
    pub gradient_color2: [f32; 4], // stop 2 color
    pub gradient_params: [f32; 4], // type (0=solid, 1=linear, 2=radial), angle/center_x, center_y, radius
    pub gradient_stops: [f32; 4],  // stop0_pos, stop1_pos, stop2_pos, num_stops
}

impl QuadInstance {
    pub const ATTRIBS: [wgpu::VertexAttribute; 15] = [
        // 0: pos
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        },
        // 1: quad_size
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 8,
            shader_location: 1,
        },
        // 2: color
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 16,
            shader_location: 2,
        },
        // 3: border_radii
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 32,
            shader_location: 3,
        },
        // 4: border_color
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 48,
            shader_location: 4,
        },
        // 5: border_widths
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 64,
            shader_location: 5,
        },
        // 6: shadow_color
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 80,
            shader_location: 6,
        },
        // 7: shadow_offset
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 96,
            shader_location: 7,
        },
        // 8: shadow_params
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 112,
            shader_location: 8,
        },
        // 9: effects
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 128,
            shader_location: 9,
        },
        // 10: clip_rect
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 144,
            shader_location: 10,
        },
        // 11: gradient_color
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 160,
            shader_location: 11,
        },
        // 12: gradient_color2
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 176,
            shader_location: 12,
        },
        // 13: gradient_params
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 192,
            shader_location: 13,
        },
        // 14: gradient_stops
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 208,
            shader_location: 14,
        },
    ];

    pub fn solid(
        pos: [f32; 2],
        quad_size: [f32; 2],
        color: [f32; 4],
        border_radii: [f32; 4],
        border_color: [f32; 4],
        border_widths: [f32; 4],
        shadow_color: [f32; 4],
        shadow_offset: [f32; 4],
        shadow_params: [f32; 4],
        effects: [f32; 4],
        clip_rect: [f32; 4],
    ) -> Self {
        Self {
            pos,
            quad_size,
            color,
            border_radii,
            border_color,
            border_widths,
            shadow_color,
            shadow_offset,
            shadow_params,
            effects,
            clip_rect,
            gradient_color: [0.0; 4],
            gradient_color2: [0.0; 4],
            gradient_params: [0.0; 4],
            gradient_stops: [0.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Push constant containing screen dimensions for vertex NDC transformation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SolidPushConstants {
    pub screen_size: [f32; 2],
}

/// Manages dynamic GPU vertex buffer allocation for batched quads.
pub struct QuadBatch {
    pub buffer: wgpu::Buffer,
    pub capacity: usize,
}

impl QuadBatch {
    pub fn new(device: &wgpu::Device) -> Self {
        let capacity = 1024;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Vertex Buffer"),
            size: (capacity * std::mem::size_of::<QuadInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { buffer, capacity }
    }

    pub fn ensure_capacity(&mut self, device: &wgpu::Device, count: usize) {
        if count > self.capacity {
            self.capacity = (count * 2).max(1024);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Quad Instance Vertex Buffer"),
                size: (self.capacity * std::mem::size_of::<QuadInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
    }

    pub fn upload(&self, queue: &wgpu::Queue, instances: &[QuadInstance]) {
        if !instances.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(instances));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quad_instance_layout() {
        assert_eq!(std::mem::size_of::<QuadInstance>(), 224);
        assert_eq!(std::mem::align_of::<QuadInstance>(), 4);
        assert_eq!(std::mem::size_of::<SolidPushConstants>(), 8);

        let desc = QuadInstance::desc();
        assert_eq!(desc.array_stride, 224);
        assert_eq!(desc.step_mode, wgpu::VertexStepMode::Instance);
        assert_eq!(desc.attributes.len(), 15);

        // Verify that attributes are strictly contiguous and non-overlapping
        assert_eq!(desc.attributes[0].offset, 0);
        assert_eq!(desc.attributes[1].offset, 8);
        assert_eq!(desc.attributes[2].offset, 16);
        assert_eq!(desc.attributes[3].offset, 32);
        assert_eq!(desc.attributes[4].offset, 48);
        assert_eq!(desc.attributes[5].offset, 64);
        assert_eq!(desc.attributes[6].offset, 80);
        assert_eq!(desc.attributes[7].offset, 96);
        assert_eq!(desc.attributes[8].offset, 112);
        assert_eq!(desc.attributes[9].offset, 128);
        assert_eq!(desc.attributes[10].offset, 144);
        assert_eq!(desc.attributes[11].offset, 160);
        assert_eq!(desc.attributes[12].offset, 176);
        assert_eq!(desc.attributes[13].offset, 192);
        assert_eq!(desc.attributes[14].offset, 208);
    }
}
