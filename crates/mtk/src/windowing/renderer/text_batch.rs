use super::atlas::{Atlas, CacheKey};
use crate::TextRenderInfo;
use crate::render::RenderCommandKind;
use crate::style::TextStyle;
use bytemuck::{Pod, Zeroable};
use parley::layout::{Affinity, PositionedLayoutItem};
use parley::{Cursor, Selection};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Individual glyph GPU instance payload.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TextInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv_pos: [f32; 2],
    pub uv_size: [f32; 2],
    pub color: [f32; 4],
}

/// Metadata and sub-rectangles for a single text command.
pub struct RenderTextData {
    pub glyphs: std::ops::Range<usize>,
    pub selections: Vec<[f32; 4]>,
    pub strikethroughs: Vec<[f32; 4]>,
    pub underlines: Vec<[f32; 4]>,
    pub caret: Option<[f32; 4]>,
    pub style: TextStyle,
    pub alpha: f32,
}

/// Manages glyph instance generation, text decorations (carets, selections, underlines),
/// and the GPU instance storage buffer.
pub struct TextBatch {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub capacity: usize,
}

impl TextBatch {
    pub fn new(
        device: &wgpu::Device,
        text_bind_group_layout: &wgpu::BindGroupLayout,
        atlas_view: &wgpu::TextureView,
        atlas_sampler: &wgpu::Sampler,
    ) -> Self {
        let capacity = 1024;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Instance Storage Buffer"),
            size: (capacity * std::mem::size_of::<TextInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout: text_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(atlas_sampler),
                },
            ],
        });

        Self {
            buffer,
            bind_group,
            capacity,
        }
    }

    /// Iterates over text commands in `context`, performs glyph layout and caching,
    /// uploads glyph instances to the GPU storage buffer, and returns the range map.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: &mut Atlas,
        text_bind_group_layout: &wgpu::BindGroupLayout,
        context: &crate::Context,
    ) -> (HashMap<usize, RenderTextData>, Option<[f32; 4]>) {
        let mut text_instances = Vec::new();
        let mut text_ranges = HashMap::new();
        let mut focused_caret = None;

        {
            let mut text_ctx = context.text_context.lock().unwrap();

            for (cmd_index, cmd) in context.render_list().enumerate() {
                if cmd.kind() != RenderCommandKind::Text {
                    continue;
                }

                let start = text_instances.len() as u32;
                let node = cmd.node();
                let Some(text) = node.get_text(context) else {
                    continue;
                };

                let computed = cmd.computed();
                let constraints = node.get_constraints(context).unwrap_or_default();

                let inner_w =
                    (computed.w - constraints.padding.left - constraints.padding.right).max(0.0);
                let inner_h =
                    (computed.h - constraints.padding.top - constraints.padding.bottom).max(0.0);

                let default_style = TextStyle::default();
                let (text_style, cursor, selection, preedit_range, spans) =
                    if let Some(info) = node.get_text_userdata::<TextRenderInfo>(context) {
                        (
                            &info.style,
                            info.cursor,
                            info.selection,
                            info.preedit_range,
                            &info.spans[..],
                        )
                    } else if let Some(style) = node.get_text_userdata::<TextStyle>(context) {
                        (style, None, None, None, &[][..])
                    } else {
                        (&default_style, None, None, None, &[][..])
                    };

                let text_ctx_ref = &mut *text_ctx;
                let layout_entry = text_ctx_ref.get_or_create_layout(
                    text,
                    text_style,
                    inner_w,
                    selection,
                    preedit_range,
                    spans,
                );
                let layout = &layout_entry.layout;
                let actual_text_width = layout_entry.actual_text_width;
                let actual_text_height = layout_entry.actual_text_height;

                let horizontal_offset = match text_style.alignment {
                    parley::layout::Alignment::Center => {
                        ((inner_w - actual_text_width) / 2.0).max(0.0)
                    }
                    parley::layout::Alignment::End | parley::layout::Alignment::Right => {
                        (inner_w - actual_text_width).max(0.0)
                    }
                    _ => 0.0,
                };

                let vertical_offset = match text_style.vertical_alignment {
                    crate::style::VerticalAlignment::Top => 0.0,
                    crate::style::VerticalAlignment::Center => {
                        ((inner_h - actual_text_height) / 2.0).max(0.0)
                    }
                    crate::style::VerticalAlignment::Bottom => {
                        (inner_h - actual_text_height).max(0.0)
                    }
                };

                let text_x = computed.x + constraints.padding.left + horizontal_offset
                    - constraints.scroll.x;
                let text_y =
                    computed.y + constraints.padding.top + vertical_offset - constraints.scroll.y;

                let total_scale = super::compute_effective_scale(context, node);

                // 1. Glyphs extraction
                for line in layout.lines() {
                    for item in line.items() {
                        let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                            continue;
                        };

                        let font_data = glyph_run.run().font();
                        let font_size = glyph_run.run().font_size();
                        let font_ptr = font_data.data.as_ref().as_ptr() as usize;
                        let brush = glyph_run.style().brush;

                        let norm_coords = glyph_run.run().normalized_coords();
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        norm_coords.hash(&mut hasher);
                        let coords_hash = hasher.finish();

                        let mut scaler_opt = None;

                        for glyph in glyph_run.positioned_glyphs() {
                            let raw_x = text_x + glyph.x;
                            let raw_y = text_y + glyph.y;
                            let subpx = ((raw_x.fract().rem_euclid(1.0) * 4.0).round() as u8) % 4;

                            let cache_key = CacheKey {
                                font_ptr,
                                font_size: (font_size * 1000.0) as u32,
                                glyph_id: glyph.id as u16,
                                subpx,
                                coords_hash,
                            };

                            let info_opt = if let Some(info) = atlas.get(cache_key) {
                                Some(info)
                            } else {
                                if scaler_opt.is_none() {
                                    let swash_font = swash::FontRef::from_index(
                                        font_data.data.as_ref(),
                                        font_data.index as usize,
                                    )
                                    .unwrap();

                                    scaler_opt = Some(
                                        text_ctx_ref
                                            .scale_cx
                                            .builder(swash_font)
                                            .size(font_size)
                                            .hint(true)
                                            .normalized_coords(norm_coords)
                                            .build(),
                                    );
                                }

                                atlas.get_or_insert(queue, scaler_opt.as_mut().unwrap(), cache_key)
                            };

                            if let Some(info) = info_opt {
                                if info.physical_w == 0 || info.physical_h == 0 {
                                    continue;
                                }

                                let base_x = raw_x.floor();
                                let base_y = raw_y.floor();
                                let global_x = base_x + info.offset_x as f32;
                                let global_y = base_y + info.offset_y as f32;

                                let (transformed_x, transformed_y) = super::transform_node_point(
                                    context,
                                    node,
                                    (global_x, global_y),
                                );

                                let mut color: [f32; 4] = if info.is_color {
                                    [1.0, 1.0, 1.0, brush.a as f32 / 255.0]
                                } else {
                                    brush.into()
                                };
                                color[3] *= super::compute_effective_opacity(context, node);

                                text_instances.push(TextInstance {
                                    pos: [transformed_x.round(), transformed_y.round()],
                                    size: [
                                        (info.physical_w as f32 * total_scale).round(),
                                        (info.physical_h as f32 * total_scale).round(),
                                    ],
                                    uv_pos: [info.uv_x, info.uv_y],
                                    uv_size: [info.uv_w, info.uv_h],
                                    color,
                                });
                            }
                        }
                    }
                }

                // 2. Caret geometry
                let mut caret_rect = None;
                if let Some(c) = cursor {
                    let cursor_layout = Cursor::from_byte_index(layout, c, Affinity::Downstream);
                    let geom = cursor_layout.geometry(layout, 1.0);
                    let mut ch = (geom.y1 - geom.y0) as f32;
                    if ch <= 0.0 {
                        ch = layout.height();
                    }
                    if ch <= 0.0 {
                        ch = text_style.font_size;
                    }
                    caret_rect = Some([
                        text_x + geom.x0 as f32,
                        text_y + geom.y0 as f32,
                        (geom.x1 - geom.x0) as f32,
                        ch,
                    ]);
                }

                // 3. Selection geometry
                let mut selection_rects = Vec::new();
                if let Some((start, end)) = selection {
                    let start_cursor = Cursor::from_byte_index(layout, start, Affinity::Downstream);
                    let end_cursor = Cursor::from_byte_index(layout, end, Affinity::Upstream);

                    let selection_obj = Selection::new(start_cursor, end_cursor);
                    for rect in selection_obj.geometry(layout) {
                        selection_rects.push([
                            text_x + rect.0.x0 as f32,
                            text_y + rect.0.y0 as f32,
                            (rect.0.x1 - rect.0.x0) as f32,
                            (rect.0.y1 - rect.0.y0) as f32,
                        ]);
                    }
                }

                // 4. Strikethrough geometry
                let mut strikethroughs = Vec::new();
                if text_style.strikethrough {
                    for line in layout.lines() {
                        let mut line_baseline: Option<f32> = None;
                        let mut min_x: Option<f32> = None;
                        let mut line_font_size = text_style.font_size;

                        for item in line.items() {
                            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                                line_font_size = glyph_run.run().font_size();
                                for glyph in glyph_run.positioned_glyphs() {
                                    if line_baseline.is_none() {
                                        line_baseline = Some(glyph.y);
                                    }
                                    let gx = glyph.x;
                                    min_x = Some(min_x.map_or(gx, |m| m.min(gx)));
                                }
                            }
                        }

                        if let Some(base_y) = line_baseline {
                            let thickness = (line_font_size * 0.08).max(1.5);
                            let line_w = line.metrics().advance;
                            let start_x = min_x.unwrap_or(0.0);
                            let line_y =
                                text_y + base_y - (line_font_size * 0.28) - (thickness * 0.5);
                            strikethroughs.push([text_x + start_x, line_y, line_w, thickness]);
                        }
                    }
                }

                // 5. Underline geometry
                let mut underlines = Vec::new();
                if text_style.underline {
                    for line in layout.lines() {
                        let mut line_baseline: Option<f32> = None;
                        let mut min_x: Option<f32> = None;
                        let mut line_font_size = text_style.font_size;

                        for item in line.items() {
                            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                                line_font_size = glyph_run.run().font_size();
                                for glyph in glyph_run.positioned_glyphs() {
                                    if line_baseline.is_none() {
                                        line_baseline = Some(glyph.y);
                                    }
                                    let gx = glyph.x;
                                    min_x = Some(min_x.map_or(gx, |m| m.min(gx)));
                                }
                            }
                        }

                        if let Some(base_y) = line_baseline {
                            let thickness = (line_font_size * 0.08).max(1.5);
                            let line_w = line.metrics().advance;
                            let start_x = min_x.unwrap_or(0.0);
                            let line_y = text_y + base_y + (line_font_size * 0.12);
                            underlines.push([text_x + start_x, line_y, line_w, thickness]);
                        }
                    }
                }

                // 6. Preedit underline geometry
                if let Some((start, end)) = preedit_range
                    && start < end
                {
                    let start_cursor = Cursor::from_byte_index(layout, start, Affinity::Downstream);
                    let end_cursor = Cursor::from_byte_index(layout, end, Affinity::Upstream);

                    let selection_obj = Selection::new(start_cursor, end_cursor);
                    let thickness = (text_style.font_size * 0.08).max(1.5);
                    for rect in selection_obj.geometry(layout) {
                        let u_x = text_x + rect.0.x0 as f32;
                        let u_y = text_y + rect.0.y1 as f32 - (thickness * 0.5);
                        let u_w = (rect.0.x1 - rect.0.x0) as f32;
                        let u_h = thickness;
                        underlines.push([u_x, u_y, u_w, u_h]);
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
                        strikethroughs,
                        underlines,
                        caret: caret_rect,
                        style: text_style.clone(),
                        alpha: super::compute_effective_opacity(context, node),
                    },
                );
            }
        }

        // Upload instances to GPU buffer (reallocating if capacity exceeded)
        if !text_instances.is_empty() {
            if text_instances.len() > self.capacity {
                self.capacity = (text_instances.len() * 2).max(1024);
                self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Text Instance Storage Buffer"),
                    size: (self.capacity * std::mem::size_of::<TextInstance>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Text Bind Group"),
                    layout: text_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.buffer.as_entire_binding(),
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
            }

            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&text_instances));
        }

        (text_ranges, focused_caret)
    }
}
