use std::time::Instant;

use mtk::{
    Context, Edges, FlexDirection, Rect, Size, Style, TextComputedOutput,
    colors::Color,
    ui::{
        View,
        style::ViewStyleExt,
        widgets::{column, row, text},
    },
};

// Application state isn't strictly necessary for a static benchmark,
// but we'll include it to keep the standard signature.
pub struct AppState {}

fn app_view(_state: &AppState) -> impl View<AppState, Message = ()> {
    // Generate a 100 x 100 grid of text nodes = 10,000 nested nodes.
    // This tests the efficiency of `Vec<V>` ViewSequence implementation.
    let grid_rows: Vec<_> = (0..100)
        .map(|i| {
            let row_items: Vec<_> = (0..100)
                .map(|j| {
                    text(format!("({},{})", i, j)).style(
                        Style::new()
                            .update_constraints(|c| {
                                c.width = Size::Fixed(40);
                                c.height = Size::Fixed(20);
                                c.padding = Edges::all(2.0);
                            })
                            // Create a checkerboard pattern just for fun
                            .bg_color(if (i + j) % 2 == 0 {
                                Color::new(40, 40, 40, 255)
                            } else {
                                Color::new(60, 60, 60, 255)
                            }),
                    )
                })
                .collect();

            // Wrap the 100 items in a Row container
            row(row_items).style(Style::new().update_constraints(|c| {
                c.width = Size::Percent(1.0);
                c.height = Size::Fixed(20);
                c.flex_direction = FlexDirection::Row;
            }))
        })
        .collect();

    // Wrap the 100 Rows in a Column container
    column(grid_rows).style(Style::new().update_constraints(|c| {
        c.width = Size::Percent(1.0);
        c.height = Size::Percent(1.0);
        c.flex_direction = FlexDirection::Column;
    }))
}

fn main() {
    let mut ctx = Context::new();

    // Simple text sizing
    ctx.set_text_sizing_func(|_ctx, _node, text, _userdata, _avail_w, _avail_h| {
        TextComputedOutput {
            computed_width: text.len() as f32 * 6.0,
            computed_height: 14.0,
            baseline_offset: 12.0,
        }
    });

    let state = AppState {};

    println!("Generating 10,000 node view tree...");
    let tree_start = Instant::now();
    let view_tree = app_view(&state);
    println!("View tree generated in {:?}", tree_start.elapsed());

    println!("Building underlying C nodes...");
    let build_start = Instant::now();
    let element = view_tree.build(&mut ctx);
    println!("Built 10,000 C nodes in {:?}", build_start.elapsed());

    let root_node = view_tree.get_node(&element);
    ctx.root_attach(root_node);

    println!("Computing layout...");
    let viewport = Rect::default().w(1920.0).h(1080.0);
    let layout_start = Instant::now();
    ctx.compute_layout(viewport.w, viewport.h);

    // Also time the render list building as it involves tree traversal
    let render_start = Instant::now();
    ctx.build_render_list(viewport);

    println!("Layout computed in {:?}", layout_start.elapsed());
    println!("Render list built in {:?}", render_start.elapsed());
    println!("Total nodes rendered: {}", ctx.render_list().count());
}
