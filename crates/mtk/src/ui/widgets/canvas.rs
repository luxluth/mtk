//! Canvas widgets and painter abstractions for 2D software pixel buffers and custom WGPU pipelines.
//!
//! The canvas element allows developers to render arbitrary visual content inside the MTK
//! layout hierarchy. MTK supports two modes of canvas painting:
//!
//! 1. [`PixelPainter`]: Software-based direct pixel manipulation using a raw `[u32]` color buffer.
//! 2. [`WgpuPainter`]: Hardware-accelerated GPU rendering with full access to `wgpu` render/compute
//!    pipelines, shaders, uniform buffers, and offscreen render targets.
//!
//! Because canvases render into offscreen GPU textures, MTK seamlessly composites them with
//! native SDF rounded corners ([`border_radius`](crate::style::BorderRadius)), opacity, borders,
//! scale, and scissor clipping without requiring extra boilerplate from the painter.

use std::cell::Cell;
use std::marker::PhantomData;

use crate::{
    Color, Context, Node,
    ui::{Event, View, event::EventResult},
};

/// A CPU-side pixel buffer representing the drawing area of a [`PixelPainter`].
///
/// ### Pixel Format: **RGBA8**
///
/// The image buffer uses standard **RGBA8** layout (8 bits per channel: Red, Green, Blue, Alpha).
/// Each pixel in the slice is laid out as 4 consecutive bytes `[R, G, B, A]`.
///
/// When accessing raw `u32` values:
/// * On native little-endian architectures, `0xAABBGGRR` maps directly to bytes `[R, G, B, A]`.
/// * You can use [`Color`](crate::colors::Color) directly with [`set_pixel_with_color`](PixelBuffer::set_pixel_with_color),
///   [`get_pixel_by_color`](PixelBuffer::get_pixel_by_color), [`fill_with_color`](PixelBuffer::fill_with_color), etc.
/// * You can also construct raw RGBA `u32` values with [`PixelBuffer::rgba`](PixelBuffer::rgba) or [`PixelBuffer::rgb`](PixelBuffer::rgb).
pub struct PixelBuffer<'a> {
    /// Physical canvas width in pixels.
    pub width: u32,
    /// Physical canvas height in pixels.
    pub height: u32,
    /// Mutable slice of 32-bit pixel values with length `width * height` in RGBA8 format.
    pub pixels: &'a mut [u32],
    pub(crate) frame_requested: &'a Cell<bool>,
}

impl<'a> PixelBuffer<'a> {
    /// Packs `(r, g, b, a)` components into a 32-bit integer matching the buffer's native RGBA8 layout.
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
        u32::from_ne_bytes([r, g, b, a])
    }

    /// Packs `(r, g, b)` components with full opacity (`a = 255`) into a 32-bit RGBA8 integer.
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
        Self::rgba(r, g, b, 255)
    }

    /// Creates a new `PixelBuffer` wrapping a slice of pixels.
    #[inline]
    pub fn new(
        width: u32,
        height: u32,
        pixels: &'a mut [u32],
        frame_requested: &'a Cell<bool>,
    ) -> Self {
        Self {
            width,
            height,
            pixels,
            frame_requested,
        }
    }

    /// Schedules a redraw for the next frame. Call this if your pixel canvas contains continuous animations.
    #[inline]
    pub fn request_frame(&self) {
        self.frame_requested.set(true);
    }

    /// Sets the color of a single pixel at `(x, y)` using a raw 32-bit RGBA8 value. Out-of-bounds coordinates are ignored.
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            if index < self.pixels.len() {
                self.pixels[index] = color;
            }
        }
    }

    /// Sets the color of a single pixel at `(x, y)` using MTK's [`Color`](crate::colors::Color). Out-of-bounds coordinates are ignored.
    #[inline]
    pub fn set_pixel_with_color(&mut self, x: u32, y: u32, color: Color) {
        self.set_pixel(x, y, color.to_rgba_u32());
    }

    /// Alias for [`set_pixel_with_color`](PixelBuffer::set_pixel_with_color).
    #[inline]
    pub fn set_pixel_color(&mut self, x: u32, y: u32, color: Color) {
        self.set_pixel_with_color(x, y, color);
    }

    /// Gets the raw 32-bit RGBA8 color of a single pixel at `(x, y)`. Returns `None` if out of bounds.
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<u32> {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.pixels.get(index).copied()
        } else {
            None
        }
    }

    /// Gets the decoded [`Color`](crate::colors::Color) of a single pixel at `(x, y)`. Returns `None` if out of bounds.
    #[inline]
    pub fn get_pixel_by_color(&self, x: u32, y: u32) -> Option<Color> {
        self.get_pixel(x, y).map(Color::from_rgba_u32)
    }

    /// Alias for [`get_pixel_by_color`](PixelBuffer::get_pixel_by_color).
    #[inline]
    pub fn get_pixel_color(&self, x: u32, y: u32) -> Option<Color> {
        self.get_pixel_by_color(x, y)
    }

    /// Fills the entire buffer with a single solid raw RGBA8 value.
    #[inline]
    pub fn fill(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Fills the entire buffer with a single solid [`Color`](crate::colors::Color).
    #[inline]
    pub fn fill_with_color(&mut self, color: Color) {
        self.fill(color.to_rgba_u32());
    }

    /// Alias for [`fill_with_color`](PixelBuffer::fill_with_color).
    #[inline]
    pub fn fill_color(&mut self, color: Color) {
        self.fill_with_color(color);
    }

    /// Clears the entire buffer to transparent black (`0x00000000` / `Color::transparent`).
    #[inline]
    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Fills a rectangular region with a raw RGBA8 `color`. Coordinates are clamped to the buffer bounds.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        let x_start = x.max(0) as u32;
        let y_start = y.max(0) as u32;
        let x_end = ((x + w as i32).max(0) as u32).min(self.width);
        let y_end = ((y + h as i32).max(0) as u32).min(self.height);

        for row in y_start..y_end {
            let row_offset = (row * self.width) as usize;
            for col in x_start..x_end {
                let idx = row_offset + col as usize;
                if idx < self.pixels.len() {
                    self.pixels[idx] = color;
                }
            }
        }
    }

    /// Fills a rectangular region with MTK's [`Color`](crate::colors::Color). Coordinates are clamped to buffer bounds.
    #[inline]
    pub fn fill_rect_with_color(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        self.fill_rect(x, y, w, h, color.to_rgba_u32());
    }

    /// Alias for [`fill_rect_with_color`](PixelBuffer::fill_rect_with_color).
    #[inline]
    pub fn fill_rect_color(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        self.fill_rect_with_color(x, y, w, h, color);
    }

    /// Blits a rectangular slice of 32-bit RGBA8 pixel data onto the canvas at `(dst_x, dst_y)`.
    pub fn blit(&mut self, src: &[u32], src_w: u32, src_h: u32, dst_x: i32, dst_y: i32) {
        for row in 0..src_h {
            let target_y = dst_y + row as i32;
            if target_y < 0 || target_y >= self.height as i32 {
                continue;
            }
            for col in 0..src_w {
                let target_x = dst_x + col as i32;
                if target_x < 0 || target_x >= self.width as i32 {
                    continue;
                }
                let src_idx = (row * src_w + col) as usize;
                let dst_idx = (target_y as u32 * self.width + target_x as u32) as usize;
                if src_idx < src.len() && dst_idx < self.pixels.len() {
                    self.pixels[dst_idx] = src[src_idx];
                }
            }
        }
    }

    /// Blits a rectangular slice of [`Color`](crate::colors::Color) onto the canvas at `(dst_x, dst_y)`.
    #[inline]
    pub fn blit_colors(&mut self, src: &[Color], src_w: u32, src_h: u32, dst_x: i32, dst_y: i32) {
        let src_u32 = bytemuck::cast_slice::<Color, u32>(src);
        self.blit(src_u32, src_w, src_h, dst_x, dst_y);
    }

    /// Blits raw RGBA byte slices onto the canvas at `(dst_x, dst_y)`.
    #[inline]
    pub fn blit_bytes(&mut self, src_bytes: &[u8], src_w: u32, src_h: u32, dst_x: i32, dst_y: i32) {
        let src_u32 = bytemuck::cast_slice::<u8, u32>(src_bytes);
        self.blit(src_u32, src_w, src_h, dst_x, dst_y);
    }

    /// Reinterprets the underlying pixel buffer as a slice of [`Color`](crate::colors::Color).
    #[inline]
    pub fn as_colors(&self) -> &[Color] {
        bytemuck::cast_slice(self.pixels)
    }

    /// Reinterprets the underlying pixel buffer as a mutable slice of [`Color`](crate::colors::Color).
    #[inline]
    pub fn as_colors_mut(&mut self) -> &mut [Color] {
        bytemuck::cast_slice_mut(self.pixels)
    }

    /// Reinterprets the underlying pixel buffer as raw RGBA bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.pixels)
    }

    /// Reinterprets the underlying pixel buffer as mutable raw RGBA bytes.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        bytemuck::cast_slice_mut(self.pixels)
    }
}

/// Trait implemented by software rasterizers and pixel painters.
pub trait PixelPainter: 'static {
    /// Renders pixel data into the provided CPU `buffer`.
    fn paint(&mut self, buffer: &mut PixelBuffer);
}

impl<F> PixelPainter for F
where
    F: FnMut(&mut PixelBuffer) + 'static,
{
    fn paint(&mut self, buffer: &mut PixelBuffer) {
        (self)(buffer);
    }
}

/// Execution context passed to [`WgpuPainter::paint`].
pub struct PaintContext<'a> {
    /// The WGPU Device handle.
    pub device: &'a wgpu::Device,
    /// The WGPU Queue handle for submitting writes and commands.
    pub queue: &'a wgpu::Queue,
    /// The active Command Encoder for recording render and compute passes.
    pub encoder: &'a mut wgpu::CommandEncoder,
    /// The offscreen TextureView target corresponding to this canvas element.
    pub target: &'a wgpu::TextureView,
    /// Physical canvas width in pixels.
    pub width: u32,
    /// Physical canvas height in pixels.
    pub height: u32,
    /// The texture format of `target` (typically `Rgba8UnormSrgb`).
    pub format: wgpu::TextureFormat,
    /// Elapsed delta time in seconds since the previous frame tick.
    pub dt: f32,
    pub(crate) frame_requested: &'a Cell<bool>,
}

impl<'a> PaintContext<'a> {
    /// Schedules a redraw for the next frame. Call this if your canvas contains continuous animations or physics.
    #[inline]
    pub fn request_frame(&self) {
        self.frame_requested.set(true);
    }
}

/// Trait implemented by GPU painters with custom WGPU render pipelines, shaders, and passes.
pub trait WgpuPainter: 'static {
    /// Called once when the painter is initialized. Create pipelines, bind groups, and static buffers here.
    fn init(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) {
        let _ = (device, queue, format);
    }

    /// Called whenever the canvas element layout size changes.
    fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        let _ = (device, queue, width, height);
    }

    /// Called before `paint` to upload uniforms or staging buffers.
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let _ = (device, queue);
    }

    /// Records GPU draw commands targeting `ctx.target`.
    fn paint(&mut self, ctx: &mut PaintContext);
}

impl<F> WgpuPainter for F
where
    F: FnMut(&mut PaintContext) + 'static,
{
    fn paint(&mut self, ctx: &mut PaintContext) {
        (self)(ctx);
    }
}

/// Internal representation of canvas painter types stored in [`Context`].
pub enum CanvasPainterKind {
    /// Software CPU pixel painter.
    Pixel(Box<dyn PixelPainter>),
    /// Hardware WGPU GPU painter.
    Wgpu(Box<dyn WgpuPainter>),
}

/// Internal state stored per canvas node.
pub struct CanvasData {
    /// The attached painter instance.
    pub painter: CanvasPainterKind,
    /// Whether `init` has been called on the painter.
    pub initialized: bool,
    /// CPU buffer memory cached between frames for `PixelPainter`.
    pub cpu_buffer: Vec<u32>,
    /// Last known physical width in pixels.
    pub width: u32,
    /// Last known physical height in pixels.
    pub height: u32,
}

/// Interaction event information passed to canvas event handlers.
#[derive(Clone, Copy, Debug)]
pub struct CanvasEventDetails {
    /// Local horizontal pixel position relative to top-left of canvas.
    pub local_x: f32,
    /// Local vertical pixel position relative to top-left of canvas.
    pub local_y: f32,
    /// Normalized horizontal position `0.0..=1.0`.
    pub uv_x: f32,
    /// Normalized vertical position `0.0..=1.0`.
    pub uv_y: f32,
}

/// A declarative UI element that renders custom 2D/3D graphics via an attached painter.
pub struct Canvas<State, Msg> {
    painter_fn: Box<dyn Fn() -> CanvasPainterKind>,
    on_event_fn: Option<Box<dyn Fn(&State, Event, CanvasEventDetails) -> Option<Msg>>>,
    _marker: PhantomData<(State, Msg)>,
}

/// Creates a new software pixel canvas driven by a [`PixelPainter`] or closure `FnMut(&mut PixelBuffer)`.
///
/// # Examples
/// ```rust,ignore
/// pixel_canvas(|buf| {
///     buf.fill(0xFF181825);
///     buf.set_pixel(10, 10, 0xFFFFFFFF);
/// })
/// ```
pub fn pixel_canvas<P, State, Msg>(painter: P) -> Canvas<State, Msg>
where
    P: PixelPainter + Clone,
{
    Canvas {
        painter_fn: Box::new(move || CanvasPainterKind::Pixel(Box::new(painter.clone()))),
        on_event_fn: None,
        _marker: PhantomData,
    }
}

/// Creates a new GPU canvas driven by a [`WgpuPainter`] or closure `FnMut(&mut PaintContext)`.
///
/// # Examples
/// ```rust,ignore
/// wgpu_canvas(|ctx| {
///     let _pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
///         label: Some("Canvas Pass"),
///         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
///             view: ctx.target,
///             resolve_target: None,
///             ops: wgpu::Operations {
///                 load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
///                 store: wgpu::StoreOp::Store,
///             },
///         })],
///         depth_stencil_attachment: None,
///         timestamp_writes: None,
///         occlusion_query_set: None,
///     });
/// })
/// ```
pub fn wgpu_canvas<P, State, Msg>(painter: P) -> Canvas<State, Msg>
where
    P: WgpuPainter + Clone,
{
    Canvas {
        painter_fn: Box::new(move || CanvasPainterKind::Wgpu(Box::new(painter.clone()))),
        on_event_fn: None,
        _marker: PhantomData,
    }
}

impl<State, Msg> Canvas<State, Msg> {
    /// Attaches an interactive event handler mapping canvas interactions into messages.
    pub fn on_event<F>(mut self, handler: F) -> Self
    where
        F: Fn(&State, Event, CanvasEventDetails) -> Option<Msg> + 'static,
    {
        self.on_event_fn = Some(Box::new(handler));
        self
    }
}

impl<State: 'static, Msg: 'static> View<State> for Canvas<State, Msg> {
    type Element = Node;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let node = ctx.create_node();
        let painter = (self.painter_fn)();
        ctx.canvases.borrow_mut().insert(
            node,
            CanvasData {
                painter,
                initialized: false,
                cpu_buffer: Vec::new(),
                width: 0,
                height: 0,
            },
        );
        node
    }

    fn rebuild(&self, _prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let mut canvases = ctx.canvases.borrow_mut();
        if let Some(canvas_data) = canvases.get_mut(element) {
            match (&mut canvas_data.painter, (self.painter_fn)()) {
                (CanvasPainterKind::Pixel(_), new_p @ CanvasPainterKind::Pixel(_)) => {
                    canvas_data.painter = new_p;
                }
                (CanvasPainterKind::Wgpu(_), CanvasPainterKind::Wgpu(_)) => {
                    // Retain the initialized GPU pipeline, bind groups, and buffers for the active canvas
                }
                (_, new_p) => {
                    canvas_data.painter = new_p;
                    canvas_data.initialized = false;
                }
            }
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        ctx.canvases.borrow_mut().remove(element);
        element.remove(ctx);
        ctx.destroy_node(*element);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        *element
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if let Some(on_event) = &self.on_event_fn {
            let (cursor_x, cursor_y, is_hit) = match &event {
                Event::CursorMoved { x, y, hit_nodes } => (*x, *y, hit_nodes.contains(element)),
                Event::MouseInput {
                    x, y, hit_nodes, ..
                } => (*x, *y, hit_nodes.contains(element)),
                Event::MouseWheel { hit_nodes, .. } => (0.0, 0.0, hit_nodes.contains(element)),
                _ => (0.0, 0.0, false),
            };

            if is_hit {
                if let Some(computed) = element.get_computed(ctx) {
                    let local_x = (cursor_x - computed.x).max(0.0);
                    let local_y = (cursor_y - computed.y).max(0.0);
                    let uv_x = if computed.w > 0.0 {
                        (local_x / computed.w).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let uv_y = if computed.h > 0.0 {
                        (local_y / computed.h).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };

                    let details = CanvasEventDetails {
                        local_x,
                        local_y,
                        uv_x,
                        uv_y,
                    };

                    if let Some(msg) = (on_event)(state, event, details) {
                        return (EventResult::Handled, Some(msg));
                    }
                }
            }
        }
        (EventResult::Ignored, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_buffer_operations() {
        let mut data = vec![0u32; 100]; // 10x10
        let frame_requested = Cell::new(false);
        let mut buf = PixelBuffer::new(10, 10, &mut data, &frame_requested);

        buf.set_pixel(2, 3, 0xFF112233);
        assert_eq!(buf.get_pixel(2, 3), Some(0xFF112233));
        assert_eq!(buf.get_pixel(0, 0), Some(0));
        assert_eq!(buf.get_pixel(10, 10), None);

        buf.fill(0xFFAAAAAA);
        assert_eq!(buf.get_pixel(0, 0), Some(0xFFAAAAAA));
        assert_eq!(buf.get_pixel(9, 9), Some(0xFFAAAAAA));

        buf.clear();
        assert_eq!(buf.get_pixel(5, 5), Some(0));

        buf.fill_rect(2, 2, 4, 4, 0xFFEEFF00);
        assert_eq!(buf.get_pixel(2, 2), Some(0xFFEEFF00));
        assert_eq!(buf.get_pixel(5, 5), Some(0xFFEEFF00));
        assert_eq!(buf.get_pixel(6, 6), Some(0));

        let src = [0xFF010203, 0xFF040506, 0xFF070809, 0xFF0A0B0C];
        buf.blit(&src, 2, 2, 0, 0);
        assert_eq!(buf.get_pixel(0, 0), Some(0xFF010203));
        assert_eq!(buf.get_pixel(1, 0), Some(0xFF040506));
        assert_eq!(buf.get_pixel(0, 1), Some(0xFF070809));
        assert_eq!(buf.get_pixel(1, 1), Some(0xFF0A0B0C));
    }

    #[test]
    fn test_pixel_buffer_color_operations() {
        let mut data = vec![0u32; 100]; // 10x10
        let frame_requested = Cell::new(false);
        let mut buf = PixelBuffer::new(10, 10, &mut data, &frame_requested);

        let red = Color::new(255, 0, 0, 255);
        let blue = Color::new(0, 0, 255, 255);
        let green = Color::new(0, 255, 0, 255);

        buf.fill_with_color(red);
        assert_eq!(buf.get_pixel_by_color(0, 0), Some(red));
        assert_eq!(buf.get_pixel_by_color(9, 9), Some(red));

        buf.set_pixel_with_color(4, 5, blue);
        assert_eq!(buf.get_pixel_by_color(4, 5), Some(blue));
        assert_eq!(buf.get_pixel_by_color(4, 6), Some(red));

        buf.fill_rect_with_color(1, 1, 3, 3, green);
        assert_eq!(buf.get_pixel_by_color(1, 1), Some(green));
        assert_eq!(buf.get_pixel_by_color(3, 3), Some(green));
        assert_eq!(buf.get_pixel_by_color(4, 4), Some(red));

        let color_src = [blue, green, red, blue];
        buf.blit_colors(&color_src, 2, 2, 0, 0);
        assert_eq!(buf.get_pixel_by_color(0, 0), Some(blue));
        assert_eq!(buf.get_pixel_by_color(1, 0), Some(green));
        assert_eq!(buf.get_pixel_by_color(0, 1), Some(red));
        assert_eq!(buf.get_pixel_by_color(1, 1), Some(blue));

        // Test as_colors slice view
        let colors = buf.as_colors();
        assert_eq!(colors.len(), 100);
        assert_eq!(colors[0], blue);
    }

    #[test]
    fn test_canvas_view_lifecycle() {
        let mut ctx = Context::new();
        let canvas_widget = pixel_canvas::<_, (), ()>(|buf: &mut PixelBuffer| {
            buf.fill_with_color(Color::green);
        });

        let element = canvas_widget.build(&mut ctx);
        assert!(ctx.canvases.borrow().contains_key(&element));

        canvas_widget.teardown(&mut ctx, &mut { element });
        assert!(!ctx.canvases.borrow().contains_key(&element));
    }
}
