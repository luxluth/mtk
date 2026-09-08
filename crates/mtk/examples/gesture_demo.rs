use mtk::style::{
    AlignItems, FlexDirection, JustifyContent, PositionStrategy, Size, Style, TextStyle,
};
use mtk::ui::View;
use mtk::ui::{
    DragContext, DragPhase, FocusableExt, KeyEventContext, KineticTracker, ViewEventExt,
    ViewStyleExt,
    widgets::{button, column, row, text},
};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{Color, rgb, rgba, winit};

#[derive(Clone)]
struct State {
    // 1. Draggable modular synth node
    node_pos: (f32, f32),
    node_dragging: bool,
    node_delta: (f32, f32),
    node_total: (f32, f32),

    // 2. Continuous rotary knob (relative drag)
    dial_angle: f32,
    dial_dragging: bool,
    dial_delta: f32,

    // 3. Kinetic physics arena
    puck_pos: (f32, f32),
    puck_vel: (f32, f32),
    puck_dragging: bool,
    puck_bounces: u32,
    kinetic_tracker: KineticTracker,

    // 4. Keyboard command deck
    active_keys: Vec<String>,
    last_key: String,
    last_physical: String,
    last_modifiers: String,
    drone_pos: (f32, f32),
    global_burst_count: u32,
}

#[derive(Clone, Debug)]
enum Message {
    NodeDrag(DragContext),
    DialScrub(DragContext),
    PuckDrag(DragContext),
    PhysicsTick(f32),
    KeyDown(KeyEventContext),
    KeyUp(KeyEventContext),
    GlobalKey(KeyEventContext),
    ResetAll,
}

const ARENA_WIDTH: f32 = 420.0;
const ARENA_HEIGHT: f32 = 150.0;
const PUCK_SIZE: f32 = 36.0;

fn card_panel(s: Style) -> Style {
    s.bg_color(rgb!(255, 255, 255))
        .border(1.0, rgb!(226, 232, 240))
        .corner_radius(14.0)
        .padding(18.0)
        .shadow(rgba!(15, 23, 42, 20), 14.0, 0.15)
}

fn badge_pill(s: Style, bg: Color, fg: Color) -> Style {
    s.bg_color(bg)
        .corner_radius(999.0)
        .padding_xy(10.0, 4.0)
        .set_text_style(TextStyle {
            font_size: 11.0,
            color: fg,
            ..Default::default()
        })
}

fn desc_text(label: &str) -> impl View<State, Message = Message> {
    text(label).style(
        Style::new()
            .width(Size::Fill)
            .text_wrap(true)
            .set_text_style(TextStyle {
                font_size: 12.0,
                color: rgb!(100, 116, 139),
                wrap: true,
                ..Default::default()
            }),
    )
}

fn keycap(label: &str, is_active: bool) -> impl View<State, Message = Message> {
    let (bg, border, fg) = if is_active {
        (rgb!(16, 185, 129), rgb!(5, 150, 105), rgb!(255, 255, 255))
    } else {
        (rgb!(241, 245, 249), rgb!(203, 213, 225), rgb!(51, 65, 85))
    };

    text(label).style(
        Style::new()
            .padding_xy(12.0, 7.0)
            .corner_radius(6.0)
            .bg_color(bg)
            .border(1.5, border)
            .shadow(
                if is_active {
                    rgba!(16, 185, 129, 60)
                } else {
                    rgba!(0, 0, 0, 0)
                },
                8.0,
                0.2,
            )
            .set_text_style(TextStyle {
                font_size: 12.0,
                color: fg,
                ..Default::default()
            }),
    )
}

fn main() {
    let initial_state = State {
        node_pos: (24.0, 24.0),
        node_dragging: false,
        node_delta: (0.0, 0.0),
        node_total: (0.0, 0.0),

        dial_angle: 45.0,
        dial_dragging: false,
        dial_delta: 0.0,

        puck_pos: (180.0, 55.0),
        puck_vel: (0.0, 0.0),
        puck_dragging: false,
        puck_bounces: 0,
        kinetic_tracker: KineticTracker::new(2.8),

        active_keys: Vec::new(),
        last_key: "None".to_string(),
        last_physical: "None".to_string(),
        last_modifiers: "None".to_string(),
        drone_pos: (0.0, 0.0),
        global_burst_count: 0,
    };

    let window = Window::with(
        initial_state,
        |state, msg: Message| match msg {
            Message::NodeDrag(ctx) => {
                state.node_dragging = ctx.phase != DragPhase::End;
                state.node_delta = ctx.delta;
                state.node_total = ctx.total_delta;
                if ctx.phase == DragPhase::Move {
                    state.node_pos.0 = (state.node_pos.0 + ctx.delta.0).clamp(0.0, 240.0);
                    state.node_pos.1 = (state.node_pos.1 + ctx.delta.1).clamp(0.0, 90.0);
                }
            }
            Message::DialScrub(ctx) => {
                state.dial_dragging = ctx.phase != DragPhase::End;
                state.dial_delta = ctx.delta.0;
                state.dial_angle = (state.dial_angle + ctx.delta.0 * 0.75).rem_euclid(360.0);
            }
            Message::PuckDrag(ctx) => match ctx.phase {
                DragPhase::Start => {
                    state.puck_dragging = true;
                    state.puck_vel = (0.0, 0.0);
                    state
                        .kinetic_tracker
                        .on_press(ctx.current_pos.0, ctx.current_pos.1);
                }
                DragPhase::Move => {
                    state.puck_dragging = true;
                    state
                        .kinetic_tracker
                        .on_move(ctx.current_pos.0, ctx.current_pos.1);
                    state.puck_pos.0 =
                        (state.puck_pos.0 + ctx.delta.0).clamp(0.0, ARENA_WIDTH - PUCK_SIZE);
                    state.puck_pos.1 =
                        (state.puck_pos.1 + ctx.delta.1).clamp(0.0, ARENA_HEIGHT - PUCK_SIZE);
                }
                DragPhase::End => {
                    state.puck_dragging = false;
                    let launch = state.kinetic_tracker.on_release();
                    state.puck_vel = launch;
                }
            },
            Message::PhysicsTick(dt) => {
                if !state.puck_dragging && state.kinetic_tracker.is_active() {
                    let max_x = ARENA_WIDTH - PUCK_SIZE;
                    let max_y = ARENA_HEIGHT - PUCK_SIZE;

                    if let Some((dx, dy)) = state.kinetic_tracker.update(dt) {
                        state.puck_pos.0 += dx;
                        state.puck_pos.1 += dy;

                        let cur_vel = state.kinetic_tracker.velocity();
                        let mut bounce_vx = cur_vel.0;
                        let mut bounce_vy = cur_vel.1;
                        let mut bounced = false;

                        if state.puck_pos.0 <= 0.0 {
                            state.puck_pos.0 = 0.0;
                            bounce_vx = -bounce_vx * 0.85;
                            bounced = true;
                        } else if state.puck_pos.0 >= max_x {
                            state.puck_pos.0 = max_x;
                            bounce_vx = -bounce_vx * 0.85;
                            bounced = true;
                        }

                        if state.puck_pos.1 <= 0.0 {
                            state.puck_pos.1 = 0.0;
                            bounce_vy = -bounce_vy * 0.85;
                            bounced = true;
                        } else if state.puck_pos.1 >= max_y {
                            state.puck_pos.1 = max_y;
                            bounce_vy = -bounce_vy * 0.85;
                            bounced = true;
                        }

                        if bounced {
                            state.puck_bounces += 1;
                            state.kinetic_tracker.set_velocity(bounce_vx, bounce_vy);
                        }

                        state.puck_vel = state.kinetic_tracker.velocity();
                    } else {
                        state.puck_vel = (0.0, 0.0);
                    }
                }
            }
            Message::KeyDown(k) => {
                let key_id = match &k.logical_key {
                    winit::keyboard::Key::Character(c) => c.to_ascii_lowercase(),
                    winit::keyboard::Key::Named(named) => format!("{:?}", named),
                    _ => format!("{:?}", k.logical_key),
                };
                state.last_key = match &k.logical_key {
                    winit::keyboard::Key::Character(c) => c.to_uppercase(),
                    winit::keyboard::Key::Named(named) => format!("{:?}", named),
                    _ => format!("{:?}", k.logical_key),
                };
                state.last_physical = format!("{:?}", k.physical_key);
                state.last_modifiers = format!(
                    "Ctrl: {} | Shift: {} | Alt: {}",
                    k.modifiers.control_key(),
                    k.modifiers.shift_key(),
                    k.modifiers.alt_key()
                );

                if !state.active_keys.contains(&key_id) {
                    state.active_keys.push(key_id);
                }

                if k.key_matches("ArrowLeft") {
                    state.drone_pos.0 = (state.drone_pos.0 - 12.0).clamp(-80.0, 80.0);
                } else if k.key_matches("ArrowRight") {
                    state.drone_pos.0 = (state.drone_pos.0 + 12.0).clamp(-80.0, 80.0);
                } else if k.key_matches("ArrowUp") {
                    state.drone_pos.1 = (state.drone_pos.1 - 12.0).clamp(-40.0, 40.0);
                } else if k.key_matches("ArrowDown") {
                    state.drone_pos.1 = (state.drone_pos.1 + 12.0).clamp(-40.0, 40.0);
                }
            }
            Message::KeyUp(k) => {
                let key_id = match &k.logical_key {
                    winit::keyboard::Key::Character(c) => c.to_ascii_lowercase(),
                    winit::keyboard::Key::Named(named) => format!("{:?}", named),
                    _ => format!("{:?}", k.logical_key),
                };
                state.active_keys.retain(|x| x != &key_id);
            }
            Message::GlobalKey(k) => {
                if k.is_escape() || (k.key_matches("r") && k.with_ctrl()) {
                    state.node_pos = (24.0, 24.0);
                    state.dial_angle = 45.0;
                    state.puck_pos = (180.0, 55.0);
                    state.puck_vel = (0.0, 0.0);
                    state.drone_pos = (0.0, 0.0);
                    state.active_keys.clear();
                    state.last_key = "Reset Triggered".to_string();
                } else if k.key_matches("g") {
                    state.global_burst_count += 1;
                    state.last_key = "Sonic Pulse [G]".to_string();
                }
            }
            Message::ResetAll => {
                state.node_pos = (24.0, 24.0);
                state.dial_angle = 45.0;
                state.puck_pos = (180.0, 55.0);
                state.puck_vel = (0.0, 0.0);
                state.drone_pos = (0.0, 0.0);
                state.active_keys.clear();
                state.global_burst_count = 0;
            }
        },
        |state| {
            let angle_deg = state.dial_angle;
            let cutoff_hz = 20.0 * (1000.0_f32).powf(angle_deg / 360.0);
            let knob_needle = match ((angle_deg + 22.5) / 45.0) as u32 % 8 {
                0 => "↑",
                1 => "↗",
                2 => "→",
                3 => "↘",
                4 => "↓",
                5 => "↙",
                6 => "←",
                _ => "↖",
            };

            let led_blocks = (angle_deg / 30.0) as usize;
            let led_meter: String = (0..12)
                .map(|i| if i < led_blocks { '▮' } else { '▯' })
                .collect();

            let speed = (state.puck_vel.0.powi(2) + state.puck_vel.1.powi(2)).sqrt();
            let puck_color = if state.puck_dragging {
                rgb!(225, 29, 72)
            } else if speed > 100.0 {
                rgb!(234, 88, 12)
            } else {
                rgb!(14, 165, 233)
            };

            let has_key = |name: &str| {
                state
                    .active_keys
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(name))
            };

            column((
                // Header Bar
                row((
                    column((
                        row((
                            text("●").style(
                                Style::new().set_text_style(TextStyle {
                                    font_size: 16.0,
                                    color: rgb!(16, 185, 129),
                                    ..Default::default()
                                }),
                            ),
                            text("MTK Interaction & Gesture Primitives").style(
                                Style::new().set_text_style(TextStyle {
                                    font_size: 20.0,
                                    color: rgb!(15, 23, 42),
                                    font_weight: parley::style::FontWeight::BOLD,
                                    ..Default::default()
                                }),
                            ),
                            text("Phase 3 Active").style(
                                Style::new().apply(|s| badge_pill(s, rgb!(238, 242, 255), rgb!(79, 70, 229))),
                            ),
                        ))
                        .style(Style::new().gap(10.0).align_items(AlignItems::Center)),
                        desc_text(
                            "Pointer capture, locked-cursor infinite scrubbing, kinetic momentum simulation, and keyboard navigation.",
                        ),
                    ))
                    .style(Style::new().gap(4.0)),
                    row((
                        text(&format!("Global Pulses: {}", state.global_burst_count)).style(
                            Style::new().apply(|s| badge_pill(s, rgb!(254, 226, 226), rgb!(220, 38, 38))),
                        ),
                        button("Reset All [Esc]").on_click(Message::ResetAll),
                    ))
                    .style(Style::new().gap(12.0).align_items(AlignItems::Center)),
                ))
                .style(
                    Style::new()
                        .width(Size::Fill)
                        .justify_content(JustifyContent::SpaceBetween)
                        .align_items(AlignItems::Center)
                        .padding_xy(0.0, 4.0),
                ),

                // Top Grid: 1. Modular Synth Drag + 2. Rotary Audio Dial
                row((
                    // CARD 1: Modular Node Drag
                    column((
                        row((
                            text("1. MODULAR DSP NODE").style(
                                Style::new().set_text_style(TextStyle {
                                    font_size: 14.0,
                                    color: rgb!(30, 41, 59),
                                    font_weight: parley::style::FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }),
                            ),
                            text(if state.node_dragging { "CAPTURED" } else { "IDLE" }).style(
                                Style::new().apply(|s| {
                                    if state.node_dragging {
                                        badge_pill(s, rgb!(224, 242, 254), rgb!(2, 132, 199))
                                    } else {
                                        badge_pill(s, rgb!(241, 245, 249), rgb!(100, 116, 139))
                                    }
                                }),
                            ),
                        ))
                        .style(Style::new().width(Size::Fill).justify_content(JustifyContent::SpaceBetween)),

                        desc_text(
                            "Click and drag the DSP card. Pointer capture guarantees continuous tracking even outside bounds.",
                        ),

                        // 2D Workspace Sandbox
                        column((
                            // Floating Draggable Card
                            column((
                                row((
                                    text("⚡ RES-FILTER").style(
                                        Style::new().set_text_style(TextStyle {
                                            font_size: 13.0,
                                            color: rgb!(15, 23, 42),
                                            font_weight: parley::style::FontWeight::BOLD,
                                            ..Default::default()
                                        }),
                                    ),
                                    text("● AUDIO OUT").style(
                                        Style::new().set_text_style(TextStyle {
                                            font_size: 10.0,
                                            color: rgb!(22, 163, 74),
                                            ..Default::default()
                                        }),
                                    ),
                                ))
                                .style(Style::new().width(Size::Fill).justify_content(JustifyContent::SpaceBetween)),

                                text(&format!("Offset: ({:.0}, {:.0}) px", state.node_pos.0, state.node_pos.1))
                                    .style(
                                        Style::new().set_text_style(TextStyle {
                                            font_size: 11.0,
                                            color: rgb!(71, 85, 105),
                                            ..Default::default()
                                        }),
                                    ),

                                row((
                                    text("Moog Ladder 24dB").style(
                                        Style::new().apply(|s| badge_pill(s, rgb!(238, 242, 255), rgb!(67, 56, 202))),
                                    ),
                                )),
                            ))
                            .style(
                                Style::new()
                                    .width(Size::Fixed(175))
                                    .padding(12.0)
                                    .gap(6.0)
                                    .corner_radius(10.0)
                                    .bg_color(if state.node_dragging {
                                        rgb!(238, 242, 255)
                                    } else {
                                        rgb!(255, 255, 255)
                                    })
                                    .border(
                                        1.5,
                                        if state.node_dragging {
                                            rgb!(79, 70, 229)
                                        } else {
                                            rgb!(203, 213, 225)
                                        },
                                    )
                                    .shadow(
                                        if state.node_dragging {
                                            rgba!(79, 70, 229, 60)
                                        } else {
                                            rgba!(15, 23, 42, 20)
                                        },
                                        if state.node_dragging { 20.0 } else { 8.0 },
                                        0.2,
                                    )
                                    .position(PositionStrategy::Absolute {
                                        left: state.node_pos.0,
                                        top: state.node_pos.1,
                                        right: f32::NAN,
                                        bottom: f32::NAN,
                                    }),
                            )
                            .on_drag(|_, ctx| Some(Message::NodeDrag(ctx))),
                        ))
                        .style(
                            Style::new()
                                .width(Size::Fill)
                                .height(Size::Fixed(165))
                                .corner_radius(10.0)
                                .bg_color(rgb!(248, 250, 252))
                                .border(1.0, rgb!(226, 232, 240))
                                .padding(8.0),
                        ),
                    ))
                    .style(card_panel(Style::new()).width(Size::Percent(0.5)).gap(12.0)),

                    // CARD 2: Rotary Synthesizer Dial
                    column((
                        row((
                            text("2. INFINITE ROTARY DIAL").style(
                                Style::new().set_text_style(TextStyle {
                                    font_size: 14.0,
                                    color: rgb!(30, 41, 59),
                                    font_weight: parley::style::FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }),
                            ),
                            text(if state.dial_dragging { "CURSOR LOCKED" } else { "IDLE" }).style(
                                Style::new().apply(|s| {
                                    if state.dial_dragging {
                                        badge_pill(s, rgb!(254, 243, 199), rgb!(180, 83, 9))
                                    } else {
                                        badge_pill(s, rgb!(241, 245, 249), rgb!(100, 116, 139))
                                    }
                                }),
                            ),
                        ))
                        .style(Style::new().width(Size::Fill).justify_content(JustifyContent::SpaceBetween)),

                        desc_text(
                            "Locks and hides OS cursor during drag. Accumulates unlimited raw hardware mouse deltas.",
                        ),

                        // Interactive Rotary Knob Display
                        row((
                            // Knob Circle
                            column((
                                text(knob_needle).style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 32.0,
                                        color: if state.dial_dragging {
                                            rgb!(217, 119, 6)
                                        } else {
                                            rgb!(51, 65, 85)
                                        },
                                        ..Default::default()
                                    }),
                                ),
                            ))
                            .style(
                                Style::new()
                                    .width(Size::Fixed(90))
                                    .height(Size::Fixed(90))
                                    .corner_radius(45.0)
                                    .align_items(AlignItems::Center)
                                    .justify_content(JustifyContent::Center)
                                    .bg_color(if state.dial_dragging {
                                        rgb!(254, 243, 199)
                                    } else {
                                        rgb!(255, 255, 255)
                                    })
                                    .border(
                                        2.5,
                                        if state.dial_dragging {
                                            rgb!(217, 119, 6)
                                        } else {
                                            rgb!(203, 213, 225)
                                        },
                                    )
                                    .shadow(
                                        if state.dial_dragging {
                                            rgba!(217, 119, 6, 60)
                                        } else {
                                            rgba!(15, 23, 42, 25)
                                        },
                                        14.0,
                                        0.2,
                                    ),
                            )
                            .on_drag_relative(|_, ctx| Some(Message::DialScrub(ctx))),

                            // Dial Telemetry Readout
                            column((
                                text(&format!("CUTOFF: {:.0} Hz", cutoff_hz)).style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 16.0,
                                        color: rgb!(180, 83, 9),
                                        font_weight: parley::style::FontWeight::BOLD,
                                        ..Default::default()
                                    }),
                                ),
                                text(&format!("Rotation: {:.1}°", angle_deg)).style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 12.0,
                                        color: rgb!(71, 85, 105),
                                        ..Default::default()
                                    }),
                                ),
                                text(&led_meter).style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 13.0,
                                        color: rgb!(217, 119, 6),
                                        ..Default::default()
                                    }),
                                ),
                                text("Drag horizontally to scrub").style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 11.0,
                                        color: rgb!(148, 163, 184),
                                        ..Default::default()
                                    }),
                                ),
                            ))
                            .style(Style::new().gap(4.0)),
                        ))
                        .style(
                            Style::new()
                                .width(Size::Fill)
                                .height(Size::Fixed(165))
                                .corner_radius(10.0)
                                .bg_color(rgb!(248, 250, 252))
                                .border(1.0, rgb!(226, 232, 240))
                                .padding(18.0)
                                .gap(20.0)
                                .align_items(AlignItems::Center),
                        ),
                    ))
                    .style(card_panel(Style::new()).width(Size::Percent(0.5)).gap(12.0)),
                ))
                .style(
                    Style::new()
                        .width(Size::Fill)
                        .gap(16.0)
                        .flex_direction(FlexDirection::Row)
                        .align_items(AlignItems::Stretch),
                ),

                // Bottom Grid: 3. Kinetic Physics Arena + 4. Keyboard Command Deck
                row((
                    // CARD 3: Kinetic Physics Arena
                    column((
                        row((
                            text("3. KINETIC MOMENTUM ARENA").style(
                                Style::new().set_text_style(TextStyle {
                                    font_size: 14.0,
                                    color: rgb!(30, 41, 59),
                                    font_weight: parley::style::FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }),
                            ),
                            text(&format!("Bounces: {}", state.puck_bounces)).style(
                                Style::new().apply(|s| badge_pill(s, rgb!(254, 226, 226), rgb!(220, 38, 38))),
                            ),
                        ))
                        .style(Style::new().width(Size::Fill).justify_content(JustifyContent::SpaceBetween)),

                        desc_text(
                            "Fling the puck with speed. Demonstrates real-time exponential velocity decay and wall reflections.",
                        ),

                        // Walled Arena Sandbox
                        column((
                            // Kinetic Puck
                            column((
                                text("●").style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 16.0,
                                        color: rgb!(255, 255, 255),
                                        ..Default::default()
                                    }),
                                ),
                            ))
                            .style(
                                Style::new()
                                    .width(Size::Fixed(PUCK_SIZE as u32))
                                    .height(Size::Fixed(PUCK_SIZE as u32))
                                    .corner_radius(PUCK_SIZE / 2.0)
                                    .align_items(AlignItems::Center)
                                    .justify_content(JustifyContent::Center)
                                    .bg_color(puck_color)
                                    .border(2.0, rgb!(255, 255, 255))
                                    .shadow(puck_color, 14.0, 0.4)
                                    .position(PositionStrategy::Absolute {
                                        left: state.puck_pos.0,
                                        top: state.puck_pos.1,
                                        right: f32::NAN,
                                        bottom: f32::NAN,
                                    }),
                            )
                            .on_drag(|_, ctx| Some(Message::PuckDrag(ctx))),

                            // Bottom Telemetry Bar inside arena
                            row((
                                text(&format!("Velocity: {:.0} px/s", speed)).style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 11.0,
                                        color: rgb!(71, 85, 105),
                                        font_weight: parley::style::FontWeight::SEMI_BOLD,
                                        ..Default::default()
                                    }),
                                ),
                                text(&format!("Vx: {:.0}, Vy: {:.0}", state.puck_vel.0, state.puck_vel.1)).style(
                                    Style::new().set_text_style(TextStyle {
                                        font_size: 11.0,
                                        color: rgb!(100, 116, 139),
                                        ..Default::default()
                                    }),
                                ),
                            ))
                            .style(
                                Style::new()
                                    .width(Size::Fill)
                                    .justify_content(JustifyContent::SpaceBetween)
                                    .position(PositionStrategy::Absolute {
                                        left: 12.0,
                                        bottom: 8.0,
                                        right: 12.0,
                                        top: f32::NAN,
                                    }),
                            ),
                        ))
                        .style(
                            Style::new()
                                .width(Size::Fill)
                                .height(Size::Fixed(165))
                                .corner_radius(10.0)
                                .bg_color(rgb!(248, 250, 252))
                                .border(1.0, rgb!(226, 232, 240))
                                .padding(8.0),
                        ),
                    ))
                    .style(card_panel(Style::new()).width(Size::Percent(0.5)).gap(12.0)),

                    // CARD 4: Keyboard Command Deck (Now Focusable!)
                    column((
                        row((
                            text("4. KEYBOARD COMMAND DECK").style(
                                Style::new().set_text_style(TextStyle {
                                    font_size: 14.0,
                                    color: rgb!(30, 41, 59),
                                    font_weight: parley::style::FontWeight::SEMI_BOLD,
                                    ..Default::default()
                                }),
                            ),
                            text(if state.active_keys.is_empty() {
                                "FOCUSABLE"
                            } else {
                                "KEYS ACTIVE"
                            })
                            .style(
                                Style::new().apply(|s| badge_pill(s, rgb!(209, 250, 229), rgb!(5, 150, 105))),
                            ),
                        ))
                        .style(Style::new().width(Size::Fill).justify_content(JustifyContent::SpaceBetween)),

                        desc_text(
                            "Click the deck to focus. Type to light up matrix keys, and use Arrow keys to steer vector drone.",
                        ),

                        // Interactive Command Matrix Deck
                        column((
                            // Row 1 keys
                            row((
                                keycap("Q", has_key("q")),
                                keycap("W", has_key("w") || has_key("ArrowUp")),
                                keycap("E", has_key("e")),
                                keycap("R", has_key("r")),
                                keycap("▲", has_key("ArrowUp")),
                            ))
                            .style(Style::new().gap(8.0)),

                            // Row 2 keys
                            row((
                                keycap("A", has_key("a") || has_key("ArrowLeft")),
                                keycap("S", has_key("s") || has_key("ArrowDown")),
                                keycap("D", has_key("d") || has_key("ArrowRight")),
                                keycap("F", has_key("f")),
                                keycap("◀", has_key("ArrowLeft")),
                                keycap("▼", has_key("ArrowDown")),
                                keycap("▶", has_key("ArrowRight")),
                            ))
                            .style(Style::new().gap(8.0)),

                            // Telemetry HUD
                            row((
                                text(&format!("Key: {}", state.last_key)).style(
                                    Style::new().apply(|s| badge_pill(s, rgb!(241, 245, 249), rgb!(51, 65, 85))),
                                ),
                                text(&format!("Drone: ({:.0}, {:.0})", state.drone_pos.0, state.drone_pos.1)).style(
                                    Style::new().apply(|s| badge_pill(s, rgb!(224, 242, 254), rgb!(2, 132, 199))),
                                ),
                            ))
                            .style(Style::new().gap(8.0).align_items(AlignItems::Center)),
                        ))
                        .style(
                            Style::new()
                                .width(Size::Fill)
                                .height(Size::Fixed(165))
                                .corner_radius(10.0)
                                .bg_color(rgb!(248, 250, 252))
                                .border(1.0, rgb!(226, 232, 240))
                                .padding(14.0)
                                .gap(10.0)
                                .justify_content(JustifyContent::Center),
                        ),
                    ))
                    .focusable()
                    .style(
                        card_panel(Style::new())
                            .width(Size::Percent(0.5))
                            .gap(12.0)
                            .on_focus(|s| {
                                s.border_color(rgb!(59, 130, 246))
                                    .shadow(rgba!(59, 130, 246, 50), 16.0, 0.25)
                            }),
                    )
                    .on_key_down(|_, k| Some(Message::KeyDown(k)))
                    .on_key_up(|_, k| Some(Message::KeyUp(k))),
                ))
                .style(
                    Style::new()
                        .width(Size::Fill)
                        .gap(16.0)
                        .flex_direction(FlexDirection::Row)
                        .align_items(AlignItems::Stretch),
                ),
            ))
            .style(
                Style::new()
                    .width(Size::Fill)
                    .height(Size::Fill)
                    .padding(24.0)
                    .gap(18.0)
                    .bg_color(rgb!(241, 245, 249)),
            )
            .on_global_key_down(|_, k| Some(Message::GlobalKey(k)))
            .on_tick(|state, dt| {
                if !state.puck_dragging && state.kinetic_tracker.is_active() {
                    Some(Message::PhysicsTick(dt))
                } else {
                    None
                }
            })
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK Workstation — Interaction & Gesture Primitives")
        .with_size((1040, 740).into())
        .with_decorations(true);

    window.present_with(attrs);
}
