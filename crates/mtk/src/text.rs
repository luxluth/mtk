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

/// Holds the shared text rendering state.
pub(crate) struct TextContext {
    pub font_cx: FontContext,
    pub layout_cx: LayoutContext<Color>,
    pub scale_cx: ScaleContext,
}

impl TextContext {
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
            scale_cx: ScaleContext::new(),
        }
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
    let ctx = &mut *ctx_guard;

    let display_scale = 1.0;
    let quantize = true;

    let mut builder = ctx
        .layout_cx
        .ranged_builder(&mut ctx.font_cx, text, display_scale, quantize);

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

    let mut layout = builder.build(text);

    let max_advance = if text_style.wrap && avail_w.is_finite() && avail_w > 0.0 {
        Some(avail_w)
    } else {
        None
    };

    layout.break_all_lines(max_advance);
    layout.align(text_style.alignment, AlignmentOptions::default());

    let measured_width = layout.width().ceil();
    let measured_height = layout.height().ceil();

    TextComputedOutput {
        computed_width: measured_width,
        computed_height: measured_height,
        baseline_offset: text_style.font_size, // Approximate baseline
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

    let mut layout = builder.build(text);
    let max_advance = if text_style.wrap && avail_w.is_finite() && avail_w > 0.0 {
        Some(avail_w)
    } else {
        None
    };

    layout.break_all_lines(max_advance);
    layout.align(text_style.alignment, AlignmentOptions::default());

    if let Some((cluster, side)) = Cluster::from_point(&layout, x, y) {
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
        let cursor = Cursor::from_point(&layout, x, y);
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

    let mut layout = builder.build(text);
    let max_advance = if text_style.wrap && avail_w.is_finite() && avail_w > 0.0 {
        Some(avail_w)
    } else {
        None
    };

    layout.break_all_lines(max_advance);
    layout.align(text_style.alignment, AlignmentOptions::default());

    let cursor_layout =
        Cursor::from_byte_index(&layout, cursor_index, parley::layout::Affinity::Downstream);
    let geom = cursor_layout.geometry(&layout, 2.0);
    let h = (geom.y1 - geom.y0) as f32;
    (geom.x0 as f32, geom.y0 as f32, h)
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

#[derive(Clone, Debug)]
pub struct TextRenderInfo {
    pub style: TextStyle,
    pub cursor: Option<usize>,
    pub selection: Option<(usize, usize)>,
    pub preedit_range: Option<(usize, usize)>,
}
