use crate::colors::Color;
use crate::sys;
use crate::{Node, TextStyle};
use parley::style::{FontStyle, LineHeight, StyleProperty};
use parley::{
    AlignmentOptions, BreakReason, Cluster, ClusterSide, Cursor, FontContext, LayoutContext,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::sync::Mutex;
use swash::scale::ScaleContext;

/// Visual styling applied to a specific sub-range of text in a rich text layout.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpanStyle {
    pub color: Option<Color>,
    pub font_weight: Option<parley::style::FontWeight>,
    pub font_style: Option<FontStyle>,
    pub font_size: Option<f32>,
    pub underline: bool,
    pub strikethrough: bool,
}

impl SpanStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.font_weight = Some(parley::style::FontWeight::BOLD);
        self
    }

    pub fn weight(mut self, weight: parley::style::FontWeight) -> Self {
        self.font_weight = Some(weight);
        self
    }

    pub fn italic(mut self) -> Self {
        self.font_style = Some(FontStyle::Italic);
        self
    }

    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.font_style = Some(style);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    pub fn strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = strikethrough;
        self
    }
}

/// A styled range of text with an optional identifier tag for interactivity.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan<Id = ()> {
    pub range: Range<usize>,
    pub style: SpanStyle,
    pub id: Option<Id>,
}

impl<Id> TextSpan<Id> {
    pub fn new(range: Range<usize>) -> Self {
        Self {
            range,
            style: SpanStyle::default(),
            id: None,
        }
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.style.font_weight = Some(parley::style::FontWeight::BOLD);
        self
    }

    pub fn weight(mut self, weight: parley::style::FontWeight) -> Self {
        self.style.font_weight = Some(weight);
        self
    }

    pub fn italic(mut self) -> Self {
        self.style.font_style = Some(FontStyle::Italic);
        self
    }

    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.style.font_style = Some(style);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.style.font_size = Some(size);
        self
    }

    pub fn underline(mut self) -> Self {
        self.style.underline = true;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.style.strikethrough = true;
        self
    }

    pub fn style(mut self, style: SpanStyle) -> Self {
        self.style = style;
        self
    }

    /// Converts this span to an untyped span for layout and rendering.
    pub fn to_untyped(&self) -> TextSpan<()> {
        TextSpan {
            range: self.range.clone(),
            style: self.style.clone(),
            id: None,
        }
    }
}

pub(crate) fn hash_spans(spans: &[TextSpan<()>]) -> u64 {
    if spans.is_empty() {
        return 0;
    }
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for span in spans {
        span.range.start.hash(&mut hasher);
        span.range.end.hash(&mut hasher);
        if let Some(c) = span.style.color {
            c.as_u32().hash(&mut hasher);
        }
        if let Some(w) = span.style.font_weight {
            w.value().to_bits().hash(&mut hasher);
        }
        if let Some(s) = span.style.font_style {
            match s {
                FontStyle::Normal => 0u8.hash(&mut hasher),
                FontStyle::Italic => 1u8.hash(&mut hasher),
                FontStyle::Oblique(_) => 2u8.hash(&mut hasher),
            }
        }
        if let Some(sz) = span.style.font_size {
            sz.to_bits().hash(&mut hasher);
        }
        span.style.underline.hash(&mut hasher);
        span.style.strikethrough.hash(&mut hasher);
    }
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextLayoutCacheKey {
    pub text: String,
    pub font_size_bits: u32,
    pub font_family: String,
    pub font_weight_bits: u32,
    pub font_style: u8,
    pub color_u32: u32,
    pub wrap: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub selection: Option<(usize, usize)>,
    pub preedit_range: Option<(usize, usize)>,
    pub inner_w_bits: u32,
    pub spans_hash: u64,
}

pub struct TextLayoutCacheEntry {
    pub layout: parley::Layout<Color>,
    pub actual_text_width: f32,
    pub actual_text_height: f32,
}

/// Holds the shared text rendering state.
pub struct TextContext {
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

    /// Registers raw font bytes (.ttf or .otf) into the Parley font collection.
    pub fn register_fonts(&mut self, font_data: Vec<u8>) {
        self.font_cx
            .collection
            .register_fonts(font_data.into(), None);
        self.layout_cache.clear();
    }

    /// Loads and registers a font file from disk.
    pub fn register_font_file(&mut self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let data = std::fs::read(path)?;
        self.register_fonts(data);
        Ok(())
    }

    pub fn get_or_create_layout(
        &mut self,
        text: &str,
        text_style: &TextStyle,
        avail_w: f32,
        selection: Option<(usize, usize)>,
        preedit_range: Option<(usize, usize)>,
        spans: &[TextSpan<()>],
    ) -> Arc<TextLayoutCacheEntry> {
        let font_style_u8 = match text_style.font_style {
            FontStyle::Normal => 0,
            FontStyle::Italic => 1,
            FontStyle::Oblique(_) => 2,
        };

        let inner_w_bits = if avail_w.is_finite() && avail_w > 0.0 {
            avail_w.to_bits()
        } else {
            u32::MAX
        };

        let spans_hash = hash_spans(spans);

        let key = TextLayoutCacheKey {
            text: text.to_string(),
            font_size_bits: text_style.font_size.to_bits(),
            font_family: text_style.font_family.clone(),
            font_weight_bits: text_style.font_weight.value().to_bits(),
            font_style: font_style_u8,
            color_u32: text_style.color.as_u32(),
            wrap: text_style.wrap,
            strikethrough: text_style.strikethrough,
            underline: text_style.underline,
            selection,
            preedit_range,
            inner_w_bits,
            spans_hash,
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

        if text_style.underline {
            builder.push_default(StyleProperty::Underline(true));
        }

        if let Some((start, end)) = preedit_range {
            builder.push(StyleProperty::Underline(true), start..end);
        }

        if let Some((start, end)) = selection {
            builder.push(StyleProperty::Brush(text_style.selection_color), start..end);
        }

        for span in spans {
            let start = span.range.start.min(text.len());
            let end = span.range.end.min(text.len());
            if start >= end {
                continue;
            }

            if let Some(color) = span.style.color {
                builder.push(StyleProperty::Brush(color), start..end);
            }
            if let Some(weight) = span.style.font_weight {
                builder.push(StyleProperty::FontWeight(weight), start..end);
            }
            if let Some(style) = span.style.font_style {
                builder.push(StyleProperty::FontStyle(style), start..end);
            }
            if let Some(size) = span.style.font_size {
                builder.push(StyleProperty::FontSize(size), start..end);
            }
            if span.style.underline {
                builder.push(StyleProperty::Underline(true), start..end);
            }
            if span.style.strikethrough {
                builder.push(StyleProperty::Strikethrough(true), start..end);
            }
        }

        let mut layout = builder.build(text);

        let max_advance = if text_style.wrap && avail_w.is_finite() && avail_w > 0.0 {
            Some(avail_w + 0.5)
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

pub type SharedTextContext = Arc<Mutex<TextContext>>;

pub fn measure_text(
    text: &str,
    text_style: &TextStyle,
    avail_w: f32,
    _avail_h: f32,
    shared_ctx: &SharedTextContext,
    spans: &[TextSpan<()>],
) -> TextComputedOutput {
    let mut ctx_guard = shared_ctx.lock().unwrap();
    let entry = ctx_guard.get_or_create_layout(text, text_style, avail_w, None, None, spans);

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

pub fn hit_test_text(
    text: &str,
    text_style: &TextStyle,
    avail_w: f32,
    avail_h: f32,
    x: f32,
    y: f32,
    shared_ctx: &SharedTextContext,
    spans: &[TextSpan<()>],
) -> usize {
    let mut ctx_guard = shared_ctx.lock().unwrap();
    let entry = ctx_guard.get_or_create_layout(text, text_style, avail_w, None, None, spans);
    let layout = &entry.layout;
    let actual_text_width = entry.actual_text_width;
    let actual_text_height = entry.actual_text_height;

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
            if avail_h.is_finite() && avail_h > 0.0 {
                ((avail_h - actual_text_height) / 2.0).max(0.0)
            } else {
                0.0
            }
        }
        crate::style::VerticalAlignment::Bottom => {
            if avail_h.is_finite() && avail_h > 0.0 {
                (avail_h - actual_text_height).max(0.0)
            } else {
                0.0
            }
        }
    };

    let rel_x = x - horizontal_offset;
    let rel_y = y - vertical_offset;

    if let Some((cluster, side)) = Cluster::from_point(layout, rel_x, rel_y) {
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
        let cursor = Cursor::from_point(layout, rel_x, rel_y);
        cursor.index()
    }
}

/// Returns the bounding rectangles (in local text coordinates `[x, y, w, h]`) of a byte range in the text.
pub fn get_range_geometry(
    text: &str,
    text_style: &TextStyle,
    avail_w: f32,
    avail_h: f32,
    range: std::ops::Range<usize>,
    shared_ctx: &SharedTextContext,
    spans: &[TextSpan<()>],
) -> Vec<[f32; 4]> {
    let mut ctx_guard = shared_ctx.lock().unwrap();
    let entry = ctx_guard.get_or_create_layout(text, text_style, avail_w, None, None, spans);
    let layout = &entry.layout;
    let actual_text_width = entry.actual_text_width;
    let actual_text_height = entry.actual_text_height;

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
            if avail_h.is_finite() && avail_h > 0.0 {
                ((avail_h - actual_text_height) / 2.0).max(0.0)
            } else {
                0.0
            }
        }
        crate::style::VerticalAlignment::Bottom => {
            if avail_h.is_finite() && avail_h > 0.0 {
                (avail_h - actual_text_height).max(0.0)
            } else {
                0.0
            }
        }
    };

    let start = range.start.min(text.len());
    let end = range.end.min(text.len());
    if start >= end {
        return Vec::new();
    }

    use parley::Selection;
    let start_cursor = Cursor::from_byte_index(layout, start, parley::layout::Affinity::Downstream);
    let end_cursor = Cursor::from_byte_index(layout, end, parley::layout::Affinity::Upstream);
    let selection_obj = Selection::new(start_cursor, end_cursor);

    let mut rects = Vec::new();
    for (bbox, _line_idx) in selection_obj.geometry(layout) {
        rects.push([
            horizontal_offset + bbox.x0 as f32,
            vertical_offset + bbox.y0 as f32,
            (bbox.x1 - bbox.x0) as f32,
            (bbox.y1 - bbox.y0) as f32,
        ]);
    }
    rects
}

pub fn get_cursor_geometry(
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

    if text_style.underline {
        builder.push_default(StyleProperty::Underline(true));
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
    pub spans: Vec<TextSpan<()>>,
}

impl Default for TextRenderInfo {
    fn default() -> Self {
        Self {
            style: TextStyle::default(),
            cursor: None,
            selection: None,
            preedit_range: None,
            spans: Vec::new(),
        }
    }
}

impl TextRenderInfo {
    pub fn new(style: TextStyle) -> Self {
        Self {
            style,
            cursor: None,
            selection: None,
            preedit_range: None,
            spans: Vec::new(),
        }
    }
}
