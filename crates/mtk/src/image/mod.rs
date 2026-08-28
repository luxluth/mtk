//! Image and SVG vector data structures, decoding, and fitting algorithms.

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

    /// Decodes an image from a file path on disk.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| format!("Failed to read image file '{:?}': {e}", path.as_ref()))?;
        Self::from_bytes(&bytes)
    }
}

use crate::colors::Color;

/// Parsed vector SVG data backed by `usvg::Tree`.
#[derive(Clone)]
pub struct SvgData {
    pub id: u64,
    pub width: f32,
    pub height: f32,
    pub tree: Arc<resvg::usvg::Tree>,
    pub source: Arc<[u8]>,
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

    /// Parses an SVG from byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let opt = resvg::usvg::Options::default();
        Self::from_bytes_with_options(bytes, opt)
    }

    /// Parses an SVG from byte slice with a default `currentColor` CSS value.
    pub fn from_bytes_with_color(bytes: &[u8], color: Color) -> Result<Self, String> {
        let mut opt = resvg::usvg::Options::default();
        opt.style_sheet = Some(format!(
            "svg {{ color: rgba({}, {}, {}, {}); }}",
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0
        ));
        Self::from_bytes_with_options(bytes, opt)
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

    /// Re-parses the SVG with a dynamic `currentColor`.
    pub fn with_color(&self, color: Color) -> Result<Self, String> {
        Self::from_bytes_with_color(&self.source, color)
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
}
