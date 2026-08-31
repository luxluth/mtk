pub mod cache;
pub use cache::{CacheKey, ImageCache, downscale_rgba8};

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_SVG_ID: AtomicU64 = AtomicU64::new(1);

/// Sizing and aspect-ratio behavior for images and SVGs within layout bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectFit {
    /// The image is scaled to maintain its aspect ratio while fitting within the element's content box.
    #[default]
    Contain,
    /// The image is sized to maintain its aspect ratio while filling the element's entire content box (clipping overflow).
    Cover,
    /// The image is sized to fill the element's content box, stretching or squishing if necessary.
    Fill,
    /// The image is not resized and retains its intrinsic pixel size.
    None,
    /// The image is sized as if `None` or `Contain` were specified, whichever would result in a smaller concrete object size.
    ScaleDown,
}

/// Decoded raster image data (RGBA8 format) ready for GPU upload.
#[derive(Clone)]
pub struct ImageData {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<[u8]>, // RGBA8 bytes: len = width * height * 4
}

impl std::fmt::Debug for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageData")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pixels_len", &self.pixels.len())
            .finish()
    }
}

impl ImageData {
    /// Creates an `ImageData` from a raw RGBA8 byte buffer.
    pub fn from_rgba8(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, String> {
        let expected_len = (width as usize) * (height as usize) * 4;
        if pixels.len() != expected_len {
            return Err(format!(
                "Invalid RGBA8 buffer length: expected {expected_len} bytes ({width}x{height}x4), got {} bytes",
                pixels.len()
            ));
        }
        let id = NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            id,
            width,
            height,
            pixels: pixels.into(),
        })
    }

    /// Decodes an image from encoded bytes (PNG, JPEG, WebP, etc.) using `zune-image`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let cursor = Cursor::new(bytes);
        let mut image =
            zune_image::image::Image::read(cursor, zune_core::options::DecoderOptions::default())
                .map_err(|e| format!("Failed to decode image bytes: {e:?}"))?;

        let (w, h) = image.dimensions();
        let width = w as u32;
        let height = h as u32;

        // Convert to RGBA
        image
            .convert_color(zune_core::colorspace::ColorSpace::RGBA)
            .map_err(|e| format!("Failed to convert image to RGBA: {e:?}"))?;

        let flattened = image.flatten_frames::<u8>();

        if flattened.is_empty() {
            return Err("Decoded image produced 0 frames".to_string());
        }

        let pixels = flattened[0].clone();
        let expected_len = (width as usize) * (height as usize) * 4;
        if pixels.len() != expected_len {
            return Err(format!(
                "Decoded RGBA buffer size mismatch: expected {expected_len}, got {}",
                pixels.len()
            ));
        }

        let id = NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            id,
            width,
            height,
            pixels: pixels.into(),
        })
    }

    /// Decodes an image from a file path on disk, utilizing the global in-memory texture cache.
    ///
    /// Subsequent calls with the same path will return the cached `ImageData` in $O(1)$ time,
    /// avoiding redundant disk reads and GPU texture allocations.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        ImageCache::global().get_or_load(path)
    }

    /// Decodes an image from a file path with optional thumbnail downscaling, utilizing the global texture cache.
    pub fn from_file_scaled<P: AsRef<std::path::Path>>(
        path: P,
        max_dim: Option<(u32, u32)>,
    ) -> Result<Self, String> {
        let key = CacheKey {
            path: path.as_ref().to_path_buf(),
            max_dim,
        };
        ImageCache::global().get_or_load_keyed(&key)
    }

    /// Decodes an image directly from disk without reading or writing to the global texture cache.
    pub fn from_file_uncached<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        Self::from_file_uncached_scaled(path, None)
    }

    /// Decodes an image directly from disk with optional downscaling without caching.
    pub fn from_file_uncached_scaled<P: AsRef<std::path::Path>>(
        path: P,
        max_dim: Option<(u32, u32)>,
    ) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| format!("Failed to read image file '{:?}': {e}", path.as_ref()))?;
        let full = Self::from_bytes(&bytes)?;

        if let Some((max_w, max_h)) = max_dim {
            if full.width > max_w || full.height > max_h {
                let (w, h, pixels) =
                    downscale_rgba8(full.width, full.height, &full.pixels, max_w, max_h);
                return Self::from_rgba8(w, h, pixels);
            }
        }

        Ok(full)
    }

    /// Requests background streaming and decoding for an image file without blocking the UI thread.
    pub fn load_async<P: AsRef<std::path::Path>, F>(path: P, on_complete: F)
    where
        F: FnOnce(Result<ImageData, String>) + Send + 'static,
    {
        ImageCache::global().load_async(path, Some(on_complete));
    }

    /// Requests background streaming and decoding with optional downscaling without blocking the UI thread.
    pub fn load_async_scaled<P: AsRef<std::path::Path>, F>(
        path: P,
        max_dim: Option<(u32, u32)>,
        on_complete: F,
    ) where
        F: FnOnce(Result<ImageData, String>) + Send + 'static,
    {
        let key = CacheKey {
            path: path.as_ref().to_path_buf(),
            max_dim,
        };
        ImageCache::global().load_async_keyed(key, Some(on_complete));
    }
}

use crate::colors::Color;

/// Configuration for dynamic SVG CSS styling.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgStyle {
    /// Sets CSS `currentColor` on root `<svg>`.
    pub color: Option<Color>,
    /// Sets `fill` on SVG elements.
    pub fill: Option<Color>,
    /// Sets `stroke` color on SVG elements.
    pub stroke: Option<Color>,
    /// Sets `stroke-width` in pixels.
    pub stroke_width: Option<f32>,
    /// Custom CSS stylesheet string appended to usvg options.
    pub custom_css: Option<String>,
}

impl SvgStyle {
    /// Creates a default empty `SvgStyle`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets CSS `currentColor` on root `<svg>`.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets `fill` color. Use `Color::new(0, 0, 0, 0)` for `fill: none`.
    pub fn fill(mut self, fill: Color) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Sets `stroke` color. Use `Color::new(0, 0, 0, 0)` for `stroke: none`.
    pub fn stroke(mut self, stroke: Color) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// Sets `stroke-width` in pixels.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = Some(width);
        self
    }

    /// Appends a raw custom CSS stylesheet.
    pub fn custom_css(mut self, css: impl Into<String>) -> Self {
        self.custom_css = Some(css.into());
        self
    }

    /// Compiles the style options into a CSS stylesheet for `usvg`.
    pub fn to_css(&self) -> Option<String> {
        let mut rules = String::new();

        if let Some(c) = self.color {
            rules.push_str(&format!(
                "svg {{ color: rgba({}, {}, {}, {}); }}\n",
                c.r,
                c.g,
                c.b,
                c.a as f32 / 255.0
            ));
        }

        let mut elem_rules = String::new();
        if let Some(f) = self.fill {
            if f.a == 0 {
                elem_rules.push_str("fill: none;\n");
            } else {
                elem_rules.push_str(&format!(
                    "fill: rgba({}, {}, {}, {});\n",
                    f.r,
                    f.g,
                    f.b,
                    f.a as f32 / 255.0
                ));
            }
        }
        if let Some(s) = self.stroke {
            if s.a == 0 {
                elem_rules.push_str("stroke: none;\n");
            } else {
                elem_rules.push_str(&format!(
                    "stroke: rgba({}, {}, {}, {});\n",
                    s.r,
                    s.g,
                    s.b,
                    s.a as f32 / 255.0
                ));
            }
        }
        if let Some(w) = self.stroke_width {
            elem_rules.push_str(&format!("stroke-width: {}px;\n", w));
        }

        if !elem_rules.is_empty() {
            rules.push_str(&format!("svg, * {{\n{}}}\n", elem_rules));
        }

        if let Some(ref custom) = self.custom_css {
            rules.push_str(custom);
            rules.push('\n');
        }

        if rules.is_empty() { None } else { Some(rules) }
    }
}

/// Parsed vector SVG data backed by `usvg::Tree`.
#[derive(Clone)]
pub struct SvgData {
    pub id: u64,
    pub width: f32,
    pub height: f32,
    pub tree: Arc<resvg::usvg::Tree>,
    pub source: Arc<[u8]>,
    pub style: SvgStyle,
}

impl std::fmt::Debug for SvgData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SvgData")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl SvgData {
    /// Parses an SVG from a UTF-8 string.
    pub fn from_str(svg_str: &str) -> Result<Self, String> {
        Self::from_bytes(svg_str.as_bytes())
    }

    /// Parses an SVG from a UTF-8 string with a default `currentColor` CSS value.
    pub fn from_str_with_color(svg_str: &str, color: Color) -> Result<Self, String> {
        Self::from_bytes_with_color(svg_str.as_bytes(), color)
    }

    /// Parses an SVG from a UTF-8 string with dynamic CSS styling.
    pub fn from_str_with_style(svg_str: &str, style: &SvgStyle) -> Result<Self, String> {
        Self::from_bytes_with_style(svg_str.as_bytes(), style)
    }

    /// Parses an SVG from byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let opt = resvg::usvg::Options::default();
        Self::from_bytes_with_options(bytes, opt)
    }

    /// Parses an SVG from byte slice with a default `currentColor` CSS value.
    pub fn from_bytes_with_color(bytes: &[u8], color: Color) -> Result<Self, String> {
        Self::from_bytes_with_style(bytes, &SvgStyle::new().color(color))
    }

    /// Parses an SVG from byte slice with dynamic CSS styling.
    pub fn from_bytes_with_style(bytes: &[u8], style: &SvgStyle) -> Result<Self, String> {
        let mut opt = resvg::usvg::Options::default();
        opt.style_sheet = style.to_css();
        let mut data = Self::from_bytes_with_options(bytes, opt)?;
        data.style = style.clone();
        Ok(data)
    }

    /// Parses an SVG from byte slice with explicit `usvg::Options`.
    pub fn from_bytes_with_options(
        bytes: &[u8],
        opt: resvg::usvg::Options,
    ) -> Result<Self, String> {
        let tree = resvg::usvg::Tree::from_data(bytes, &opt)
            .map_err(|e| format!("Failed to parse SVG data: {e:?}"))?;

        let size = tree.size();
        let width = size.width();
        let height = size.height();

        let id = NEXT_SVG_ID.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            id,
            width,
            height,
            tree: Arc::new(tree),
            source: bytes.into(),
            style: SvgStyle::default(),
        })
    }

    /// Parses an SVG from a file path on disk.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| format!("Failed to read SVG file '{:?}': {e}", path.as_ref()))?;
        Self::from_bytes(&bytes)
    }

    /// Parses an SVG from a file path on disk with a default `currentColor` CSS value.
    pub fn from_file_with_color<P: AsRef<std::path::Path>>(
        path: P,
        color: Color,
    ) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| format!("Failed to read SVG file '{:?}': {e}", path.as_ref()))?;
        Self::from_bytes_with_color(&bytes, color)
    }

    /// Parses an SVG from a file path on disk with dynamic CSS styling.
    pub fn from_file_with_style<P: AsRef<std::path::Path>>(
        path: P,
        style: &SvgStyle,
    ) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| format!("Failed to read SVG file '{:?}': {e}", path.as_ref()))?;
        Self::from_bytes_with_style(&bytes, style)
    }

    /// Re-parses the SVG with dynamic CSS styling.
    pub fn with_style(&self, style: &SvgStyle) -> Result<Self, String> {
        Self::from_bytes_with_style(&self.source, style)
    }

    /// Re-parses the SVG with a dynamic `currentColor`.
    pub fn with_color(&self, color: Color) -> Result<Self, String> {
        let mut style = self.style.clone();
        style.color = Some(color);
        self.with_style(&style)
    }

    /// Re-parses the SVG with a dynamic fill color.
    pub fn with_fill(&self, fill: Color) -> Result<Self, String> {
        let mut style = self.style.clone();
        style.fill = Some(fill);
        self.with_style(&style)
    }

    /// Re-parses the SVG with a dynamic stroke color and width.
    pub fn with_stroke(&self, stroke: Color, width: f32) -> Result<Self, String> {
        let mut style = self.style.clone();
        style.stroke = Some(stroke);
        style.stroke_width = Some(width);
        self.with_style(&style)
    }

    /// Re-parses the SVG with custom CSS rules.
    pub fn with_css(&self, css: impl Into<String>) -> Result<Self, String> {
        let mut style = self.style.clone();
        style.custom_css = Some(css.into());
        self.with_style(&style)
    }

    /// Rasterizes the SVG at a specific target pixel resolution `(target_w, target_h)`.
    pub fn render_to_pixmap(
        &self,
        target_w: u32,
        target_h: u32,
        fit: ObjectFit,
    ) -> Option<resvg::tiny_skia::Pixmap> {
        if target_w == 0 || target_h == 0 {
            return None;
        }

        let mut pixmap = resvg::tiny_skia::Pixmap::new(target_w, target_h)?;

        // Compute scaling transform based on ObjectFit
        let sx = target_w as f32 / self.width;
        let sy = target_h as f32 / self.height;

        let (scale_x, scale_y, offset_x, offset_y) = match fit {
            ObjectFit::Fill => (sx, sy, 0.0, 0.0),
            ObjectFit::Contain => {
                let scale = sx.min(sy);
                let rendered_w = self.width * scale;
                let rendered_h = self.height * scale;
                let dx = (target_w as f32 - rendered_w) * 0.5;
                let dy = (target_h as f32 - rendered_h) * 0.5;
                (scale, scale, dx, dy)
            }
            ObjectFit::Cover => {
                let scale = sx.max(sy);
                let rendered_w = self.width * scale;
                let rendered_h = self.height * scale;
                let dx = (target_w as f32 - rendered_w) * 0.5;
                let dy = (target_h as f32 - rendered_h) * 0.5;
                (scale, scale, dx, dy)
            }
            ObjectFit::None => {
                let dx = (target_w as f32 - self.width) * 0.5;
                let dy = (target_h as f32 - self.height) * 0.5;
                (1.0, 1.0, dx, dy)
            }
            ObjectFit::ScaleDown => {
                let scale = sx.min(sy).min(1.0);
                let rendered_w = self.width * scale;
                let rendered_h = self.height * scale;
                let dx = (target_w as f32 - rendered_w) * 0.5;
                let dy = (target_h as f32 - rendered_h) * 0.5;
                (scale, scale, dx, dy)
            }
        };

        let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y)
            .post_translate(offset_x, offset_y);

        resvg::render(&self.tree, transform, &mut pixmap.as_mut());

        Some(pixmap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_from_rgba8() {
        let pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        let image = ImageData::from_rgba8(2, 2, pixels).expect("Valid 2x2 image");
        assert_eq!(image.width, 2);
        assert_eq!(image.height, 2);
        assert_eq!(image.pixels.len(), 16);
    }

    #[test]
    fn test_current_color_behavior() {
        let svg_raw = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect width="100" height="100" fill="currentColor" />
        </svg>"#;
        let svg_red =
            SvgData::from_str_with_color(svg_raw, Color::new(255, 0, 0, 255)).expect("Valid SVG");
        let pixmap_red = svg_red
            .render_to_pixmap(100, 100, ObjectFit::Contain)
            .expect("Render");
        assert_eq!(
            (
                pixmap_red.data()[0],
                pixmap_red.data()[1],
                pixmap_red.data()[2],
                pixmap_red.data()[3]
            ),
            (255, 0, 0, 255)
        );

        let svg_blue = svg_red
            .with_color(Color::new(0, 0, 255, 255))
            .expect("Recolor");
        let pixmap_blue = svg_blue
            .render_to_pixmap(100, 100, ObjectFit::Contain)
            .expect("Render");
        assert_eq!(
            (
                pixmap_blue.data()[0],
                pixmap_blue.data()[1],
                pixmap_blue.data()[2],
                pixmap_blue.data()[3]
            ),
            (0, 0, 255, 255)
        );

        // Test multi-color SVG: fixed green rect + dynamic currentColor rect
        let multi_color_svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="0" y="0" width="50" height="100" fill="#00ff00" />
            <rect x="50" y="0" width="50" height="100" fill="currentColor" />
        </svg>"##;
        let svg = SvgData::from_str_with_color(multi_color_svg, Color::new(255, 0, 0, 255))
            .expect("Valid SVG");
        let pixmap = svg
            .render_to_pixmap(100, 100, ObjectFit::Fill)
            .expect("Render");
        // Left side is fixed green (0, 255, 0)
        assert_eq!(pixmap.pixel(25, 50).unwrap().green(), 255);
        assert_eq!(pixmap.pixel(25, 50).unwrap().red(), 0);
        // Right side is currentColor red (255, 0, 0)
        assert_eq!(pixmap.pixel(75, 50).unwrap().red(), 255);
        assert_eq!(pixmap.pixel(75, 50).unwrap().green(), 0);
    }

    #[test]
    fn test_svg_from_str_and_render() {
        let svg_str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="40" fill="red" />
        </svg>"#;
        let svg = SvgData::from_str(svg_str).expect("Valid SVG");
        assert_eq!(svg.width, 100.0);
        assert_eq!(svg.height, 100.0);

        let pixmap = svg
            .render_to_pixmap(200, 200, ObjectFit::Contain)
            .expect("Render success");
        assert_eq!(pixmap.width(), 200);
        assert_eq!(pixmap.height(), 200);
        assert!(!pixmap.data().is_empty());
    }

    #[test]
    fn test_ghostscript_tiger_svg() {
        let tiger_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/assets/Ghostscript_Tiger.svg"
        );
        let svg = SvgData::from_file(tiger_path).expect("Load Ghostscript_Tiger.svg");
        assert_eq!(svg.width, 200.0);
        assert_eq!(svg.height, 200.0);

        let pixmap = svg
            .render_to_pixmap(400, 400, ObjectFit::Cover)
            .expect("Render tiger pixmap");
        assert_eq!(pixmap.width(), 400);
        assert_eq!(pixmap.height(), 400);

        // Verify that the rendered pixmap has non-transparent pixels
        let has_colored_pixels = pixmap.data().iter().any(|&b| b > 0);
        assert!(has_colored_pixels);
    }

    #[test]
    fn test_object_fit_modes() {
        let svg_str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50">
            <rect width="100" height="50" fill="blue" />
        </svg>"#;
        let svg = SvgData::from_str(svg_str).expect("Valid SVG");

        for fit in [
            ObjectFit::Contain,
            ObjectFit::Cover,
            ObjectFit::Fill,
            ObjectFit::ScaleDown,
            ObjectFit::None,
        ] {
            let pixmap = svg.render_to_pixmap(300, 300, fit).expect("Render fit");
            assert_eq!(pixmap.width(), 300);
            assert_eq!(pixmap.height(), 300);
        }
    }

    #[test]
    fn test_svg_dynamic_styling() {
        let svg_raw = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="10" y="10" width="80" height="80" />
        </svg>"#;

        // 1. Test fill override
        let red = Color::new(255, 0, 0, 255);
        let svg_fill = SvgData::from_str(svg_raw).unwrap().with_fill(red).unwrap();
        let pixmap = svg_fill
            .render_to_pixmap(100, 100, ObjectFit::Fill)
            .unwrap();
        let center = pixmap.pixel(50, 50).unwrap();
        assert_eq!(
            (center.red(), center.green(), center.blue(), center.alpha()),
            (255, 0, 0, 255)
        );

        // 2. Test stroke override with stroke width
        let blue = Color::new(0, 0, 255, 255);
        let svg_stroke = SvgData::from_str(svg_raw)
            .unwrap()
            .with_fill(Color::new(0, 0, 0, 0))
            .unwrap()
            .with_stroke(blue, 10.0)
            .unwrap();
        let pixmap_stroke = svg_stroke
            .render_to_pixmap(100, 100, ObjectFit::Fill)
            .unwrap();
        // Border pixel (10, 50) is blue stroke
        let border = pixmap_stroke.pixel(10, 50).unwrap();
        assert_eq!(
            (border.red(), border.green(), border.blue(), border.alpha()),
            (0, 0, 255, 255)
        );
        // Center pixel (50, 50) is transparent because fill: none
        let center_empty = pixmap_stroke.pixel(50, 50).unwrap();
        assert_eq!(center_empty.alpha(), 0);

        // 3. Test comprehensive SvgStyle builder
        let style = SvgStyle::new()
            .fill(Color::new(0, 255, 0, 255))
            .stroke(Color::new(255, 255, 0, 255))
            .stroke_width(4.0);
        let svg_styled = SvgData::from_str_with_style(svg_raw, &style).unwrap();
        let pixmap_styled = svg_styled
            .render_to_pixmap(100, 100, ObjectFit::Fill)
            .unwrap();
        let styled_center = pixmap_styled.pixel(50, 50).unwrap();
        assert_eq!(
            (
                styled_center.red(),
                styled_center.green(),
                styled_center.blue(),
                styled_center.alpha()
            ),
            (0, 255, 0, 255)
        );
    }

    #[test]
    fn test_image_cache_deduplication() {
        let cache = ImageCache::new();
        let path = std::path::PathBuf::from("/virtual/test_image.png");
        let dummy = ImageData::from_rgba8(2, 2, vec![0; 16]).unwrap();

        cache.insert(path.clone(), dummy.clone());

        let retrieved = cache.get(&path).expect("Must retrieve cached image");
        assert_eq!(retrieved.id, dummy.id);
        assert_eq!(retrieved.width, 2);
        assert_eq!(retrieved.height, 2);

        cache.clear();
        assert!(cache.get(&path).is_none());
    }

    #[test]
    fn test_image_cache_lru_eviction() {
        let cache = ImageCache::new();
        // Set small limit: enough for ~2 small images (each 400 bytes pixels + ~32 bytes overhead)
        cache.set_max_bytes(1000);

        let p1 = std::path::PathBuf::from("/virtual/img1.png");
        let p2 = std::path::PathBuf::from("/virtual/img2.png");
        let p3 = std::path::PathBuf::from("/virtual/img3.png");

        let d1 = ImageData::from_rgba8(10, 10, vec![1; 400]).unwrap();
        let d2 = ImageData::from_rgba8(10, 10, vec![2; 400]).unwrap();
        let d3 = ImageData::from_rgba8(10, 10, vec![3; 400]).unwrap();

        cache.insert(p1.clone(), d1);
        cache.insert(p2.clone(), d2);

        // Access p1 so p2 becomes oldest
        let _ = cache.get(&p1);

        // Insert p3 -> should evict p2
        cache.insert(p3.clone(), d3);

        assert!(cache.get(&p1).is_some());
        assert!(cache.get(&p2).is_none(), "p2 should be evicted by LRU");
        assert!(cache.get(&p3).is_some());
        assert!(cache.current_bytes() <= 1000);
    }

    #[test]
    fn test_downscale_rgba8() {
        let original_pixels = vec![255u8; 100 * 100 * 4];
        let (w, h, scaled) = downscale_rgba8(100, 100, &original_pixels, 20, 20);

        assert_eq!(w, 20);
        assert_eq!(h, 20);
        assert_eq!(scaled.len(), 20 * 20 * 4);
        assert_eq!(scaled[0], 255);
    }

    #[test]
    fn test_downscale_diagonal_antialiasing() {
        // Draw a diagonal black/white split across a 50x50 image
        let mut original = vec![0u8; 50 * 50 * 4];
        for y in 0..50 {
            for x in 0..50 {
                let idx = (y * 50 + x) * 4;
                let val = if x >= y { 255 } else { 0 };
                original[idx] = val;
                original[idx + 1] = val;
                original[idx + 2] = val;
                original[idx + 3] = 255;
            }
        }

        // Downscale to 10x10
        let (w, h, scaled) = downscale_rgba8(50, 50, &original, 10, 10);
        assert_eq!(w, 10);
        assert_eq!(h, 10);

        // Pixels along the diagonal boundary must have intermediate anti-aliased values
        let diag_pixel_r = scaled[(5 * 10 + 5) * 4];
        assert!(
            diag_pixel_r > 0 && diag_pixel_r < 255,
            "Diagonal boundary must be smoothly anti-aliased, got: {diag_pixel_r}"
        );
    }
}
