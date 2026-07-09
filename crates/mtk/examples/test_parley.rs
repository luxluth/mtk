use parley::layout::PositionedLayoutItem;
use parley::style::FontFamily;
use parley::{FontContext, LayoutContext};

fn main() {
    let mut font_cx = FontContext::new();
    let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();

    let mut builder = layout_cx.ranged_builder(&mut font_cx, "Hello\nWorld", 1.0, true);
    builder.push_default(FontFamily::from("system-ui"));

    let mut layout = builder.build("Hello\nWorld");
    layout.break_all_lines(None);
    println!(
        "Layout width: {}, height: {}",
        layout.width(),
        layout.height()
    );

    for line in layout.lines() {
        println!(
            "Line baseline: {}, height: {}, asc: {}, desc: {}",
            line.metrics().baseline,
            line.metrics().line_height,
            line.metrics().ascent,
            line.metrics().descent
        );
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                for glyph in glyph_run.glyphs() {
                    println!("Glyph x: {}, y: {}", glyph.x, glyph.y);
                }
            }
        }
    }
}
