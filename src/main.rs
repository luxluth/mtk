use std::time::Instant;

use mtk::{
    AlignItems, Context, Edges, FlexDirection, JustifyContent, Rect, Size, TextComputedOutput,
};

#[derive(Debug)]
#[allow(dead_code)]
struct FontStyle {
    size: f32,
    family: &'static str,
}

fn main() {
    let mut ctx = Context::new();

    ctx.set_text_sizing_func(|_node, text, userdata, _avail_w, _avail_h| {
        // Retrieve the font size from the userdata if available
        let font_size = if let Some(any_data) = userdata {
            if let Some(font) = any_data.downcast_ref::<FontStyle>() {
                font.size
            } else {
                16.0
            }
        } else {
            16.0
        };

        TextComputedOutput {
            computed_width: text.len() as f32 * (font_size * 0.6), // rough approximation
            computed_height: font_size * 1.2,
            baseline_offset: font_size * 1.0,
        }
    });

    let root = ctx.create_node();
    ctx.root_attach(root);

    root.update_constraints(&mut ctx, |c| {
        c.width = Size::Percent(1.0);
        c.height = Size::Percent(1.0);
        c.flex_direction = FlexDirection::Column;
        c.justify_content = JustifyContent::Center;
        c.align_items = AlignItems::Center;
        c.gap = 20.0;
        c.padding = Edges::all(10.0);
    });

    let title = ctx.create_node();
    title.update_constraints(&mut ctx, |_| {});
    title.set_text_with_userdata(
        &mut ctx,
        "Welcome to Muse Gui Toolkit",
        FontStyle {
            size: 32.0,
            family: "Inter",
        },
    );
    root.append(&mut ctx, title);

    let button_row = ctx.create_node();
    button_row.update_constraints(&mut ctx, |c| {
        c.width = Size::Fill;
        c.height = Size::Fit;
        c.flex_direction = FlexDirection::Row;
        c.justify_content = JustifyContent::Center;
        c.gap = 15.0;
    });
    root.append(&mut ctx, button_row);

    for label in &["Cancel", "Confirm"] {
        let btn = ctx.create_node();
        btn.update_constraints(&mut ctx, |c| {
            c.width = Size::Fit;
            c.height = Size::Fixed(40);
            c.padding = Edges::lr(20.0);
            c.justify_content = JustifyContent::Center;
            c.align_items = AlignItems::Center;
        });

        let text = ctx.create_node();
        text.update_constraints(&mut ctx, |_| {});
        text.set_text(&mut ctx, label);
        btn.append(&mut ctx, text);

        button_row.append(&mut ctx, btn);
    }

    print!("Computing layout..");
    let start = Instant::now();
    let viewport = Rect::default().w(800.0).h(600.0);
    ctx.compute_layout(viewport.w, viewport.h);

    ctx.build_render_list(viewport);

    println!("{:?}", start.elapsed());

    println!("Render Commands:");
    for cmd in ctx.render_list() {
        let kind = cmd.kind();
        let computed = cmd.computed();

        print!(
            "- {:?} at (x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1})",
            kind, computed.x, computed.y, computed.w, computed.h
        );

        if kind == mtk::render::RenderCommandKind::Text {
            if let Some(text) = cmd.node().get_text(&ctx) {
                print!(" [Text: {:?}]", text);
            }
        }
        println!();
    }
}
