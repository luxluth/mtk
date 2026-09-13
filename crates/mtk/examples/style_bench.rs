use std::hint::black_box;
use std::time::Instant;

use mtk::colors::Color;
use mtk::style::{Style, TextStyle};
use mtk::ui::View;
use mtk::ui::style::ViewStyleExt;
use mtk::ui::widgets::{column, row, text};
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

    println!("============================================================");
}
