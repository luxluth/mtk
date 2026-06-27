use cosmic_text::FontSystem;
use cosmic_text::SwashCache;

use crate::Node;
use crate::sys;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// Holds the shared text rendering state.
pub(crate) struct TextContext {
    pub font_system: FontSystem,
    #[allow(unused)]
    pub swash_cache: SwashCache,
}

impl TextContext {
    pub fn new() -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().load_system_fonts();
        Self {
            font_system,
            swash_cache: SwashCache::new(),
        }
    }
}

pub(crate) type SharedTextContext = Arc<Mutex<TextContext>>;

pub(crate) fn measure_text(
    text: &str,
    text_style: &crate::ui::style::TextStyle,
    avail_w: f32,
    _avail_h: f32,
    shared_ctx: &SharedTextContext,
) -> TextComputedOutput {
    use cosmic_text::{Buffer, Metrics, Shaping};
    let mut ctx = shared_ctx.lock().unwrap();

    let metrics = Metrics::new(text_style.font_size, text_style.line_height);
    let mut buffer = Buffer::new(&mut ctx.font_system, metrics);

    let mut max_width = None;
    if avail_w.is_finite() && avail_w > 0.0 {
        buffer.set_size(Some(avail_w), None);
        max_width = Some(avail_w);
    } else {
        buffer.set_size(None, None);
    }

    let attrs = text_style.attrs.as_attrs();
    buffer.set_text(text, &attrs, Shaping::Advanced, Some(text_style.alignement));

    buffer.shape_until_scroll(&mut ctx.font_system, true);

    let mut measured_width = 0.0f32;
    for run in buffer.layout_runs() {
        if run.line_w > measured_width {
            measured_width = run.line_w;
        }
    }

    let measured_width = measured_width.ceil();
    let final_width = max_width.unwrap_or(measured_width);

    let measured_height = if let Some(last_run) = buffer.layout_runs().last() {
        (last_run.line_y + text_style.line_height).ceil()
    } else {
        0.0
    };

    TextComputedOutput {
        computed_width: final_width,
        computed_height: measured_height,
        baseline_offset: text_style.font_size, // Approximate baseline
    }
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

type SizingFunc =
    Box<dyn Fn(Node, &str, Option<&dyn std::any::Any>, f32, f32) -> TextComputedOutput>;

thread_local! {
    pub(crate) static SIZING_FUNCS: RefCell<HashMap<usize, SizingFunc>> = RefCell::new(HashMap::new());
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
            func(Node(node), text_str, userdata_ref, avail_w, avail_h).into()
        } else {
            TextComputedOutput::default().into()
        }
    })
}
