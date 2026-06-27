use std::time::Instant;

use mtk::{
    Context, Edges, FlexDirection, Rect, Size, TextComputedOutput,
    animation::Curve,
    colors::Color,
    ui::{
        View,
        style::{AnimationTarget, Style, ViewStyleExt},
        widgets::{column, container, row, text},
    },
};

pub struct AppState {
    pub users: Vec<String>,
    pub is_expanded: bool,
}

fn app_view(state: &AppState) -> impl View<AppState> {
    // We can map our dynamic data into a Vec of Views
    let dynamic_user_list: Vec<_> = state
        .users
        .iter()
        .map(|name| {
            text(format!("User: {}", name)).style(
                Style::new()
                    .update_constraints(|c| {
                        c.padding = Edges::all(10.0);
                        c.width = Size::Fill;
                    })
                    .bg_color(Color::new(40, 40, 40, 255))
                    // When hovering over a user, highlight them and shift them slightly!
                    .on_hover(|s| {
                        s.bg_color(Color::new(60, 60, 100, 255))
                            .update_constraints(|c| c.padding.left = 20.0)
                    })
                    // Smoothly animate the padding and background color
                    .animate(AnimationTarget::Padding, 200.0, Curve::ease_out())
                    .animate(AnimationTarget::BackgroundColor, 200.0, Curve::linear()),
            )
        })
        .collect();

    // 3. Compose the static layout using heterogeneous Tuples
    column((
        // Header
        text("--- User Dashboard ---").style(
            Style::new()
                .update_constraints(|c| c.padding = Edges::all(20.0))
                .bg_color(Color::new(20, 20, 20, 255)),
        ),
        // Dynamic List
        container(dynamic_user_list).style(Style::new().update_constraints(|c| {
            c.flex_direction = FlexDirection::Column;
            c.gap = 5.0;
            c.padding = Edges::all(10.0);
        })),
        // Footer Toolbar (Row)
        row((
            text("Add User").style(
                Style::new()
                    .update_constraints(|c| c.padding = Edges::all(15.0))
                    .bg_color(Color::new(30, 150, 50, 255))
                    .on_hover(|s| s.bg_color(Color::new(50, 200, 80, 255))),
            ),
            text("Clear All").style(
                Style::new()
                    .update_constraints(|c| c.padding = Edges::all(15.0))
                    .bg_color(Color::new(150, 30, 30, 255))
                    .on_hover(|s| s.bg_color(Color::new(200, 50, 50, 255))),
            ),
        ))
        .style(Style::new().update_constraints(|c| c.gap = 10.0)),
    ))
    .style(Style::new().update_constraints(|c| {
        c.width = Size::Percent(1.0);
        c.height = Size::Percent(1.0);
        c.padding = Edges::all(20.0);
    }))
}

fn main() {
    let mut ctx = Context::new();

    ctx.set_text_sizing_func(
        |_node, text, _userdata, _avail_w, _avail_h| TextComputedOutput {
            computed_width: text.len() as f32 * 10.0,
            computed_height: 20.0,
            baseline_offset: 16.0,
        },
    );

    // Create our initial state
    let state = AppState {
        users: vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        is_expanded: false,
    };

    println!("Building UI Tree...");

    // 1. Build the view tree
    let view_tree = app_view(&state);

    // 2. Execute the build phase (this pushes nodes to your C backend!)
    let element = view_tree.build(&mut ctx);

    // We attach the root node to the context
    use mtk::ui::View;
    let root_node = view_tree.get_node(&element);
    ctx.root_attach(root_node);

    // 3. Compute layout
    let viewport = Rect::default().w(800.0).h(600.0);
    let start = Instant::now();
    ctx.compute_layout(viewport.w, viewport.h);
    ctx.build_render_list(viewport);

    println!("Computed layout in {:?}", start.elapsed());
    println!("Total render commands: {}", ctx.render_list().count());
}
