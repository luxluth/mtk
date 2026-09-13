use std::hint::black_box;
use std::time::Instant;

use mtk::colors::Color;
use mtk::style::{Style, TextStyle};
use mtk::ui::style::ViewStyleExt;
use mtk::ui::widgets::{column, row, scroll_view, text, virtual_list_count};
use mtk::ui::{Event, View};
use mtk::{Context, Rect, TextComputedOutput, rgb};

fn generate_10k_tree() -> impl View<(), Message = ()> {
    let rows: Vec<_> = (0..100)
        .map(|r| {
            let cols: Vec<_> = (0..100)
                .map(|c| {
                    text(format!("{r}:{c}")).style(
                        Style::new()
                            .padding(4.0)
                            .corner_radius(4.0)
                            .bg_color(if (r + c) % 2 == 0 {
                                rgb!(30, 41, 59)
                            } else {
                                rgb!(51, 65, 85)
                            })
                            .text_color(rgb!(248, 250, 252))
                            .font_size(12.0)
                            .on_hover(|s| s.bg_color(rgb!(99, 102, 241))),
                    )
                })
                .collect();
            row(cols)
        })
        .collect();
    column(rows)
}

fn generate_1k_scroll_view() -> impl View<(), Message = ()> {
    let sv_rows: Vec<_> = (0..1000)
        .map(|i| {
            text(format!("Row {i}")).style(
                Style::new()
                    .padding(10.0)
                    .bg_color(rgb!(30, 41, 59))
                    .text_color(rgb!(241, 245, 249)),
            )
        })
        .collect();
    scroll_view(column(sv_rows).style(Style::new().gap(5.0)))
}

fn generate_1k_virtual_list() -> impl View<(), Message = ()> {
    virtual_list_count(1000, 36.0, |idx| {
        text(format!("Row {idx}")).style(
            Style::new()
                .padding(10.0)
                .bg_color(rgb!(30, 41, 59))
                .text_color(rgb!(241, 245, 249)),
        )
    })
}

fn main() {
    println!("============================================================");
    println!("           MTK Style & Layout Performance Benchmark         ");
    println!("============================================================");
    println!("Architecture: 64-bit StyleFlags + Consolidated Interaction Store");
    println!("Style Stack Size: {} bytes", std::mem::size_of::<Style>());
    println!(
        "TextStyle Size:   {} bytes",
        std::mem::size_of::<TextStyle>()
    );
    println!("============================================================\n");

    const ITERS: usize = 2_000_000;

    // ------------------------------------------------------------------------
    // Benchmark 1: Builder Construction
    // ------------------------------------------------------------------------
    print!("1. Style Construction ({ITERS} iterations): ");
    let start = Instant::now();
    for _ in 0..ITERS {
        let s = Style::new()
            .padding(12.0)
            .corner_radius(8.0)
            .bg_color(rgb!(30, 41, 59))
            .border(1.0, rgb!(51, 65, 85))
            .text_color(rgb!(241, 245, 249))
            .font_size(14.0);
        black_box(s);
    }
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() as f64 / ITERS as f64;
    let mops = (ITERS as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
    println!("{elapsed:?} ({ns_per_op:.2} ns/op, {mops:.2} M ops/sec)");

    // ------------------------------------------------------------------------
    // Benchmark 2: Bitmask Style Merging (Base + Override)
    // ------------------------------------------------------------------------
    print!("2. Style Merging via StyleFlags ({ITERS} iterations): ");
    let base = Style::new()
        .padding(16.0)
        .corner_radius(6.0)
        .bg_color(rgb!(59, 130, 246))
        .border(1.0, rgb!(37, 99, 235))
        .text_color(Color::white)
        .font_size(14.0);

    let hover_override = Style::new()
        .bg_color(rgb!(29, 78, 216))
        .border(1.0, rgb!(30, 64, 175));

    let start = Instant::now();
    for _ in 0..ITERS {
        let merged = black_box(base.clone()).merge(black_box(hover_override.clone()));
        black_box(merged);
    }
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() as f64 / ITERS as f64;
    let mops = (ITERS as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
    println!("{elapsed:?} ({ns_per_op:.2} ns/op, {mops:.2} M ops/sec)");

    // ------------------------------------------------------------------------
    // Benchmark 3: Interactive Pseudo-State Definition & Access
    // ------------------------------------------------------------------------
    print!("3. Consolidated Pseudo-State Resolution ({ITERS} iterations): ");
    let interactive_style = Style::new()
        .bg_color(rgb!(241, 245, 249))
        .border(1.0, rgb!(203, 213, 225))
        .on_hover(|s| s.bg_color(rgb!(226, 232, 240)))
        .on_active(|s| s.scale(0.96));

    let start = Instant::now();
    for _ in 0..ITERS {
        let hover_bg = black_box(&interactive_style)
            .hover()
            .map(|h| h.base_effects.background_color);
        let active_scale = black_box(&interactive_style)
            .active()
            .map(|a| a.base_effects.scale);
        black_box((hover_bg, active_scale));
    }
    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() as f64 / ITERS as f64;
    let mops = (ITERS as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
    println!("{elapsed:?} ({ns_per_op:.2} ns/op, {mops:.2} M ops/sec)");

    // ------------------------------------------------------------------------
    // Benchmark 4: 10,000 Styled Nodes Full Lifecycle
    // ------------------------------------------------------------------------
    println!("\n------------------------------------------------------------");
    println!("4. Stress Test: 10,000 Styled Interactive Nodes in View Tree");
    println!("------------------------------------------------------------");

    let mut ctx = Context::new();
    ctx.set_text_sizing_func(|_ctx, _node, text, _userdata, _avail_w, _avail_h| {
        TextComputedOutput {
            computed_width: text.len() as f32 * 7.5,
            computed_height: 16.0,
            baseline_offset: 13.0,
        }
    });

    let tree_start = Instant::now();
    let tree = generate_10k_tree();
    let tree_time = tree_start.elapsed();
    println!("   • View Tree Generation:       {:>10.3?}", tree_time);

    let build_start = Instant::now();
    let element = tree.build(&mut ctx);
    let build_time = build_start.elapsed();
    println!("   • Building 10,000 Node Tree:  {:>10.3?}", build_time);

    let root_node = tree.get_node(&element);
    ctx.root_attach(root_node);

    let layout_start = Instant::now();
    ctx.compute_layout(1920.0, 1080.0);
    let layout_time = layout_start.elapsed();
    println!("   • Full Layout Computation:    {:>10.3?}", layout_time);

    let render_start = Instant::now();
    ctx.build_render_list(Rect::default().w(1920.0).h(1080.0));
    let render_time = render_start.elapsed();
    println!("   • Render List Building:       {:>10.3?}", render_time);

    let hit_start = Instant::now();
    let pick_iters = 100_000;
    for i in 0..pick_iters {
        let x = (i % 1920) as f32;
        let y = ((i * 7) % 1080) as f32;
        black_box(ctx.pick(x, y));
    }
    let hit_time = hit_start.elapsed();
    let ns_per_hit = hit_time.as_nanos() as f64 / pick_iters as f64;
    println!(
        "   • Quadtree Hit Testing:       {:>10.3?} ({pick_iters} hits, {ns_per_hit:.1} ns/hit)",
        hit_time
    );

    // ------------------------------------------------------------------------
    // Benchmark 5: ScrollView vs VirtualList (1,000 items)
    // ------------------------------------------------------------------------
    println!("\n------------------------------------------------------------");
    println!("5. ScrollView (Non-Virtualized) vs VirtualList (Virtualized)");
    println!("   Scenario: 1,000 styled rows in vertical viewport (800x600)");
    println!("------------------------------------------------------------");

    // ScrollView (Retained / Non-virtualized)
    let sv_start = Instant::now();
    let sv = generate_1k_scroll_view();
    let sv_tree_gen = sv_start.elapsed();

    let mut sv_ctx = Context::new();
    // Do not set dummy text_sizing_func; let it use real Parley measure_text!

    let sv_build_start = Instant::now();
    let mut sv_el = sv.build(&mut sv_ctx);
    let sv_build_time = sv_build_start.elapsed();

    let sv_node = sv.get_node(&sv_el);
    sv_ctx.root_attach(sv_node);

    let sv_layout_start = Instant::now();
    sv_ctx.compute_layout(800.0, 600.0);
    let sv_layout_time = sv_layout_start.elapsed();

    let sv_scroll_start = Instant::now();
    for _ in 0..100 {
        sv_node.update_constraints(&mut sv_ctx, |c| c.scroll.y += 10.0);
        sv_ctx.compute_layout(800.0, 600.0);
    }
    let sv_scroll_time = sv_scroll_start.elapsed() / 100;

    let sv_tick_start = Instant::now();
    const TICK_ITERS: u32 = 100;
    for _ in 0..TICK_ITERS {
        let (res, msg) = sv.handle_event(&mut sv_el, &(), Event::Tick { dt: 0.016 }, &mut sv_ctx);
        black_box((res, msg));
    }
    let sv_tick_time = sv_tick_start.elapsed() / TICK_ITERS;

    let sv_render_start = Instant::now();
    sv_ctx.build_render_list(Rect::default().w(800.0).h(600.0));
    let sv_render_time = sv_render_start.elapsed();

    let sv_mouse_start = Instant::now();
    let hit_nodes = sv_ctx.pick(400.0, 300.0);
    for _ in 0..TICK_ITERS {
        let (res, msg) = sv.handle_event(
            &mut sv_el,
            &(),
            Event::CursorMoved {
                x: 400.0,
                y: 300.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: hit_nodes.clone(),
            },
            &mut sv_ctx,
        );
        black_box((res, msg));
    }
    let sv_mouse_time = sv_mouse_start.elapsed() / TICK_ITERS;

    println!("• ScrollView (1,000 items, ~2,002 layout nodes):");
    println!("   - View Tree Generation:       {:>10.3?}", sv_tree_gen);
    println!("   - Element Build:              {:>10.3?}", sv_build_time);
    println!("   - Layout Computation (Cold):  {:>10.3?}", sv_layout_time);
    println!("   - Scroll Frame Re-layout:     {:>10.3?}", sv_scroll_time);
    println!("   - Event::Tick Dispatch/frame: {:>10.3?}", sv_tick_time);
    println!("   - Event::CursorMoved Dispatch:{:>10.3?}", sv_mouse_time);
    println!("   - Render List Culling:        {:>10.3?}", sv_render_time);

    // VirtualList (Virtualized)
    let vl_start = Instant::now();
    let vl = generate_1k_virtual_list();
    let vl_tree_gen = vl_start.elapsed();

    let mut vl_ctx = Context::new();
    vl_ctx.set_text_sizing_func(|_ctx, _node, text, _userdata, _avail_w, _avail_h| {
        TextComputedOutput {
            computed_width: text.len() as f32 * 7.5,
            computed_height: 16.0,
            baseline_offset: 13.0,
        }
    });

    let vl_build_start = Instant::now();
    let mut vl_el = vl.build(&mut vl_ctx);
    let vl_build_time = vl_build_start.elapsed();

    let vl_node = vl.get_node(&vl_el);
    vl_ctx.root_attach(vl_node);

    let vl_layout_start = Instant::now();
    vl_ctx.compute_layout(800.0, 600.0);
    let vl_layout_time = vl_layout_start.elapsed();

    let vl_tick_start = Instant::now();
    for _ in 0..TICK_ITERS {
        let (res, msg) = vl.handle_event(&mut vl_el, &(), Event::Tick { dt: 0.016 }, &mut vl_ctx);
        black_box((res, msg));
    }
    let vl_tick_time = vl_tick_start.elapsed() / TICK_ITERS;

    let vl_render_start = Instant::now();
    vl_ctx.build_render_list(Rect::default().w(800.0).h(600.0));
    let vl_render_time = vl_render_start.elapsed();

    let vl_mouse_start = Instant::now();
    let vl_hit_nodes = vl_ctx.pick(400.0, 300.0);
    for _ in 0..TICK_ITERS {
        let (res, msg) = vl.handle_event(
            &mut vl_el,
            &(),
            Event::CursorMoved {
                x: 400.0,
                y: 300.0,
                delta_x: 0.0,
                delta_y: 0.0,
                hit_nodes: vl_hit_nodes.clone(),
            },
            &mut vl_ctx,
        );
        black_box((res, msg));
    }
    let vl_mouse_time = vl_mouse_start.elapsed() / TICK_ITERS;

    println!("\n• VirtualList (1,000 items, ~20 visible layout nodes):");
    println!("   - View Tree Generation:       {:>10.3?}", vl_tree_gen);
    println!("   - Element Build:              {:>10.3?}", vl_build_time);
    println!("   - Layout Computation:         {:>10.3?}", vl_layout_time);
    println!("   - Event::Tick Dispatch/frame: {:>10.3?}", vl_tick_time);
    println!("   - Event::CursorMoved Dispatch:{:>10.3?}", vl_mouse_time);
    println!("   - Render List Culling:        {:>10.3?}", vl_render_time);

    println!("============================================================");
}
