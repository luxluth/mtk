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
    pub shadow_params: [f32; 4], // shadow_spread, shadow_power, alpha, _pad
    pub effects: [f32; 4],       // vibrancy, vibrancy_darkness, passes, _pad
    pub clip_rect: [f32; 4],     // clip_x, clip_y, clip_w, clip_h (w<=0 or h<=0 disables clipping)
}

impl QuadInstance {
    pub const ATTRIBS: [wgpu::VertexAttribute; 10] = [
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
        // 7: shadow_params
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 96,
            shader_location: 7,
        },
        // 8: effects
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 112,
            shader_location: 8,
        },
        // 9: clip_rect
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 128,
            shader_location: 9,
        },
    ];

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
        assert_eq!(std::mem::size_of::<QuadInstance>(), 144);
        assert_eq!(std::mem::align_of::<QuadInstance>(), 4);
        assert_eq!(std::mem::size_of::<SolidPushConstants>(), 8);

        let desc = QuadInstance::desc();
        assert_eq!(desc.array_stride, 144);
        assert_eq!(desc.step_mode, wgpu::VertexStepMode::Instance);
        assert_eq!(desc.attributes.len(), 10);

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
    }
}
