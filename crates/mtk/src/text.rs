use crate::colors::Color;
use crate::sys;
use crate::{Node, TextStyle};
use parley::style::{LineHeight, StyleProperty};
use parley::{
    AlignmentOptions, BreakReason, Cluster, ClusterSide, Cursor, FontContext, LayoutContext,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use swash::scale::ScaleContext;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TextLayoutCacheKey {
    pub text: String,
    pub font_size_bits: u32,
    pub font_family: String,
    pub font_weight_bits: u32,
    pub font_style: u8,
    pub color_u32: u32,
    pub wrap: bool,
    pub strikethrough: bool,
    pub selection: Option<(usize, usize)>,
    pub preedit_range: Option<(usize, usize)>,
    pub inner_w_bits: u32,
}

pub(crate) struct TextLayoutCacheEntry {
    pub layout: parley::Layout<Color>,
    pub actual_text_width: f32,
    pub actual_text_height: f32,
}

/// Holds the shared text rendering state.
pub(crate) struct TextContext {
    pub font_cx: FontContext,
    pub layout_cx: LayoutContext<Color>,
    pub scale_cx: ScaleContext,
    pub layout_cache: HashMap<TextLayoutCacheKey, Arc<TextLayoutCacheEntry>>,
}

impl TextContext {
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
            scale_cx: ScaleContext::new(),
            layout_cache: HashMap::new(),
        }
    }

    pub fn get_or_create_layout(
        &mut self,
        text: &str,
        text_style: &TextStyle,
        inner_w: f32,
        selection: Option<(usize, usize)>,
        preedit_range: Option<(usize, usize)>,
    ) -> Arc<TextLayoutCacheEntry> {
        let font_style_u8 = match text_style.font_style {
            parley::style::FontStyle::Normal => 0,
            parley::style::FontStyle::Italic => 1,
            parley::style::FontStyle::Oblique(_) => 2,
        };

        let inner_w_bits = if inner_w.is_finite() && inner_w > 0.0 {
            (inner_w * 100.0) as u32
        } else {
            u32::MAX
        };

        let key = TextLayoutCacheKey {
            text: text.to_string(),
            font_size_bits: text_style.font_size.to_bits(),
            font_family: text_style.font_family.clone(),
            font_weight_bits: text_style.font_weight.value().to_bits(),
            font_style: font_style_u8,
            color_u32: text_style.color.as_u32(),
            wrap: text_style.wrap,
            strikethrough: text_style.strikethrough,
            selection,
            preedit_range,
            inner_w_bits,
        };

        if let Some(entry) = self.layout_cache.get(&key) {
            return Arc::clone(entry);
        }

        let display_scale = 1.0;
        let quantize = true;

        let mut builder =
            self.layout_cx
                .ranged_builder(&mut self.font_cx, text, display_scale, quantize);

        builder.push_default(StyleProperty::Brush(text_style.color));
        builder.push_default(StyleProperty::FontSize(text_style.font_size));
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            text_style.line_height.resolve(),
        )));
        builder.push_default(StyleProperty::FontWeight(text_style.font_weight));
        builder.push_default(StyleProperty::FontStyle(text_style.font_style));
        builder.push_default(parley::style::FontFamily::from(
            text_style.font_family.as_str(),
        ));
        if text_style.wrap {
            builder.push_default(StyleProperty::OverflowWrap(text_style.overflow_wrap));
        }

        if text_style.strikethrough {
            builder.push_default(StyleProperty::Strikethrough(true));
        }

        if let Some((start, end)) = preedit_range {
            builder.push(StyleProperty::Underline(true), start..end);
        }

        if let Some((start, end)) = selection {
            builder.push(StyleProperty::Brush(text_style.selection_color), start..end);
        }

        let mut layout = builder.build(text);

        let max_advance = if text_style.wrap && inner_w.is_finite() && inner_w > 0.0 {
            Some(inner_w + 0.5)
        } else {
            None
        };
        layout.break_all_lines(max_advance);
        let actual_text_width = layout.width();
        let actual_text_height = layout.height();

        let entry = Arc::new(TextLayoutCacheEntry {
            layout,
            actual_text_width,
            actual_text_height,
        });

        if self.layout_cache.len() > 1000 {
            self.layout_cache.clear();
        }

        self.layout_cache.insert(key, Arc::clone(&entry));
        entry
    }
}

pub(crate) type SharedTextContext = Arc<Mutex<TextContext>>;

pub(crate) fn measure_text(
    text: &str,
    text_style: &TextStyle,
    avail_w: f32,
    _avail_h: f32,
    shared_ctx: &SharedTextContext,
) -> TextComputedOutput {
    let mut ctx_guard = shared_ctx.lock().unwrap();
    let entry = ctx_guard.get_or_create_layout(text, text_style, avail_w, None, None);

    TextComputedOutput {
        computed_width: entry.actual_text_width.ceil(),
        computed_height: entry.actual_text_height.ceil(),
        baseline_offset: entry
            .layout
            .lines()
            .next()
            .map(|l| l.metrics().ascent)
            .unwrap_or(text_style.font_size),
    }
}

pub(crate) fn hit_test_text(
    text: &str,
    text_style: &TextStyle,
    avail_w: f32,
    _avail_h: f32,
    x: f32,
    y: f32,
    shared_ctx: &SharedTextContext,
) -> usize {
    let mut text_context = shared_ctx.lock().unwrap();
    let TextContext {
        font_cx, layout_cx, ..
    } = &mut *text_context;

    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, true);

    builder.push_default(StyleProperty::FontSize(text_style.font_size));
    builder.push_default(parley::style::FontFamily::from(
        text_style.font_family.as_str(),
    ));
    builder.push_default(StyleProperty::FontWeight(text_style.font_weight));
    builder.push_default(StyleProperty::FontStyle(text_style.font_style));

    if text_style.wrap {
        builder.push_default(StyleProperty::OverflowWrap(text_style.overflow_wrap));
    }

    if text_style.strikethrough {
        builder.push_default(StyleProperty::Strikethrough(true));
    }

    let mut layout = builder.build(text);
    let max_advance = if text_style.wrap && avail_w.is_finite() && avail_w > 0.0 {
        Some(avail_w)
    } else {
        None
    };

    layout.break_all_lines(max_advance);
    layout.align(text_style.alignment, AlignmentOptions::default());

    let actual_text_width = layout.width();
    let actual_text_height = layout.height();

    let horizontal_offset = match text_style.alignment {
        parley::layout::Alignment::Center => {
            if avail_w.is_finite() && avail_w > 0.0 {
                ((avail_w - actual_text_width) / 2.0).max(0.0)
            } else {
                0.0
            }
        }
        parley::layout::Alignment::End | parley::layout::Alignment::Right => {
            if avail_w.is_finite() && avail_w > 0.0 {
                (avail_w - actual_text_width).max(0.0)
            } else {
                0.0
            }
        }
        _ => 0.0,
    };

    let vertical_offset = match text_style.vertical_alignment {
        crate::style::VerticalAlignment::Top => 0.0,
        crate::style::VerticalAlignment::Center => {
            if _avail_h.is_finite() && _avail_h > 0.0 {
                ((_avail_h - actual_text_height) / 2.0).max(0.0)
            } else {
                0.0
            }
        }
        crate::style::VerticalAlignment::Bottom => {
            if _avail_h.is_finite() && _avail_h > 0.0 {
                (_avail_h - actual_text_height).max(0.0)
            } else {
                0.0
            }
        }
    };

    let rel_x = x - horizontal_offset;
    let rel_y = y - vertical_offset;

    if let Some((cluster, side)) = Cluster::from_point(&layout, rel_x, rel_y) {
        let is_leading = side == ClusterSide::Left;
        if cluster.is_rtl() {
            if is_leading {
                cluster.text_range().end
            } else {
                cluster.text_range().start
            }
        } else {
            if is_leading || cluster.is_line_break() == Some(BreakReason::Explicit) {
                cluster.text_range().start
            } else {
                cluster.text_range().end
            }
        }
    } else {
        // If we didn't hit a cluster, let's just use from_point which gets closest
        let cursor = Cursor::from_point(&layout, rel_x, rel_y);
        cursor.index()
    }
}

pub(crate) fn get_cursor_geometry(
    text: &str,
    text_style: &TextStyle,
    avail_w: f32,
    cursor_index: usize,
    shared_ctx: &SharedTextContext,
) -> (f32, f32, f32) {
    let mut text_context = shared_ctx.lock().unwrap();
    let TextContext {
        font_cx, layout_cx, ..
    } = &mut *text_context;

    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, true);

    builder.push_default(StyleProperty::FontSize(text_style.font_size));
    builder.push_default(parley::style::FontFamily::from(
        text_style.font_family.as_str(),
    ));
    builder.push_default(StyleProperty::FontWeight(text_style.font_weight));
    builder.push_default(StyleProperty::FontStyle(text_style.font_style));

    if text_style.wrap {
        builder.push_default(StyleProperty::OverflowWrap(text_style.overflow_wrap));
    }

    if text_style.strikethrough {
        builder.push_default(StyleProperty::Strikethrough(true));
    }

    let mut layout = builder.build(text);
    let max_advance = if text_style.wrap && avail_w.is_finite() && avail_w > 0.0 {
        Some(avail_w)
    } else {
        None
    };

    layout.break_all_lines(max_advance);
    layout.align(text_style.alignment, AlignmentOptions::default());

    let actual_text_width = layout.width();
    let horizontal_offset = match text_style.alignment {
        parley::layout::Alignment::Center => {
            if avail_w.is_finite() && avail_w > 0.0 {
                ((avail_w - actual_text_width) / 2.0).max(0.0)
            } else {
                0.0
            }
        }
        parley::layout::Alignment::End | parley::layout::Alignment::Right => {
            if avail_w.is_finite() && avail_w > 0.0 {
                (avail_w - actual_text_width).max(0.0)
            } else {
                0.0
            }
        }
        _ => 0.0,
    };

    let cursor_layout =
        Cursor::from_byte_index(&layout, cursor_index, parley::layout::Affinity::Downstream);
    let geom = cursor_layout.geometry(&layout, 1.0);
    let h = (geom.y1 - geom.y0) as f32;
    (geom.x0 as f32 + horizontal_offset, geom.y0 as f32, h)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextComputedOutput {
    pub computed_width: f32,
    pub computed_height: f32,
    pub baseline_offset: f32,
}

impl Default for TextComputedOutput {
    fn default() -> Self {
        Self {
            computed_width: 0.0,
            computed_height: 0.0,
            baseline_offset: 0.0,
        }
    }
}

impl Into<sys::muTextComputedOutput> for TextComputedOutput {
    fn into(self) -> sys::muTextComputedOutput {
        sys::muTextComputedOutput {
            computed_width: self.computed_width,
            computed_height: self.computed_height,
            baseline_offset: self.baseline_offset,
        }
    }
}

type SizingFunc = Box<
    dyn Fn(
        &mut crate::Context,
        Node,
        &str,
        Option<&dyn std::any::Any>,
        f32,
        f32,
    ) -> TextComputedOutput,
>;

thread_local! {
    pub(crate) static SIZING_FUNCS: RefCell<HashMap<usize, SizingFunc>> = RefCell::new(HashMap::new());
    pub(crate) static CURRENT_CONTEXT: std::cell::Cell<*mut crate::Context> = std::cell::Cell::new(std::ptr::null_mut());
}

pub(crate) extern "C" fn text_sizing_trampoline(
    ctx: *mut sys::muContext,
    node: sys::muId,
    avail_w: f32,
    avail_h: f32,
) -> sys::muTextComputedOutput {
    let text_ptr = unsafe { sys::muse_text_get(ctx, node) };
    let (text_str, userdata_ref) = if !text_ptr.is_null() {
        let t_str = if !unsafe { (*text_ptr).data }.is_null() {
            unsafe { std::ffi::CStr::from_ptr((*text_ptr).data) }
                .to_str()
                .unwrap_or("")
        } else {
            ""
        };

        let u_ref = if !unsafe { (*text_ptr).userdata }.is_null() {
            let b = unsafe { &*((*text_ptr).userdata as *mut Box<dyn std::any::Any>) };
            Some(b.as_ref())
        } else {
            None
        };

        (t_str, u_ref)
    } else {
        ("", None)
    };

    SIZING_FUNCS.with(|funcs| {
        if let Some(func) = funcs.borrow().get(&(ctx as usize)) {
            let ctx_ptr = CURRENT_CONTEXT.with(|c| c.get());
            if !ctx_ptr.is_null() {
                let rust_ctx = unsafe { &mut *ctx_ptr };
                func(
                    rust_ctx,
                    Node(node),
                    text_str,
                    userdata_ref,
                    avail_w,
                    avail_h,
                )
                .into()
            } else {
                TextComputedOutput::default().into()
            }
        } else {
            TextComputedOutput::default().into()
        }
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRenderInfo {
    pub style: TextStyle,
    pub cursor: Option<usize>,
    pub selection: Option<(usize, usize)>,
    pub preedit_range: Option<(usize, usize)>,
}
