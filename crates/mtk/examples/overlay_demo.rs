use mtk::BoxShadow;
use mtk::clr;
use mtk::colors::Color;
use mtk::rgb;
use mtk::rgba;
use mtk::style::{AlignItems, JustifyContent, Overflow, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::widgets::{button, column, row, text};
use mtk::ui::{MouseEventContext, OverlayPlacement, ViewEventExt, ViewOverlayExt, ViewStyleExt};
use mtk::windowing::{Window, WindowAttributes};

#[derive(Clone, Debug)]
struct AppState {
    popover_open: bool,
    context_menu_open: bool,
    context_menu_pos: Option<(f32, f32)>,
    last_action: String,
}

#[derive(Clone, Debug)]
enum AppMsg {
    TogglePopover,
    ClosePopover,
    OpenContextMenu((f32, f32)),
    CloseContextMenu,
    SelectOption(String),
}

fn heading(label: &'static str, size: f32) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    text::<_, AppMsg>(label).style(
        Style::new().set_text_style(
            TextStyle::new()
                .font_size(size)
                .font_weight(FontWeight::BOLD)
                .color(rgb!(15, 23, 42)),
        ),
    )
}

fn sub_heading(label: &'static str) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    text::<_, AppMsg>(label).style(
        Style::new().set_text_style(
            TextStyle::new()
                .font_size(15.0)
                .font_weight(FontWeight::BOLD)
                .color(rgb!(30, 41, 59)),
        ),
    )
}

fn menu_item(
    label: &'static str,
    action_name: &'static str,
) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    text::<_, AppMsg>(label)
        .style(
            Style::new()
                .padding_xy(12.0, 8.0)
                .width(Size::Fill)
                .corner_radius(6.0)
                .bg_color(Color::transparent)
                .text_color(rgb!(51, 65, 85))
                .font_size(13.0)
                .on_hover(|s| {
                    s.bg_color(rgb!(239, 246, 255))
                        .text_color(rgb!(37, 99, 235))
                })
                .transition_all(100.0, mtk::animation::Curve::ease_in()),
        )
        .on_click(move |_, _| Some(AppMsg::SelectOption(action_name.to_string())))
}

fn main() {
    let initial_state = AppState {
        popover_open: false,
        context_menu_open: false,
        context_menu_pos: None,
        last_action: "Ready. Click buttons or right-click to test overlays.".to_string(),
    };

    let window = Window::with(
        initial_state,
        |state: &mut AppState, msg: AppMsg| match msg {
            AppMsg::TogglePopover => {
                state.popover_open = !state.popover_open;
                state.last_action = if state.popover_open {
                    "Opened anchored flyout".to_string()
                } else {
                    "Closed anchored flyout".to_string()
                };
            }
            AppMsg::ClosePopover => {
                if state.popover_open {
                    state.popover_open = false;
                    state.last_action = "Dismissed flyout via outside-click or Escape".to_string();
                }
            }
            AppMsg::OpenContextMenu(pos) => {
                state.context_menu_open = true;
                state.context_menu_pos = Some(pos);
                state.last_action = format!("Opened context menu at ({:.1}, {:.1})", pos.0, pos.1);
            }
            AppMsg::CloseContextMenu => {
                if state.context_menu_open {
                    state.context_menu_open = false;
                    state.last_action = "Dismissed context menu".to_string();
                }
            }
            AppMsg::SelectOption(opt) => {
                state.last_action = format!("Selected action: {}", opt);
                state.popover_open = false;
                state.context_menu_open = false;
            }
        },
        |state: &AppState| {
            // Floating flyout menu (rendered unclipped over ancestor Overflow::Hidden container)
            let flyout_menu = column((
                text("Floating Options Menu").style(
                    Style::new().padding_xy(12.0, 4.0).set_text_style(
                        TextStyle::new()
                            .font_size(11.0)
                            .font_weight(FontWeight::BOLD)
                            .color(rgb!(148, 163, 184)),
                    ),
                ),
                menu_item("Profile Settings", "Profile Settings"),
                menu_item("Preferences", "Preferences"),
                menu_item("Keyboard Shortcuts", "Keyboard Shortcuts"),
                menu_item("Export Project", "Export Project"),
                menu_item("Sign Out", "Sign Out"),
            ))
            .style(
                Style::new()
                    .width(Size::Fixed(200))
                    .padding(6.0)
                    .gap(2.0)
                    .corner_radius(10.0)
                    .bg_color(clr!(white))
                    .border(1.0, rgb!(226, 232, 240))
                    .unclipped(true)
                    .box_shadow(
                        BoxShadow::new(rgba!(15, 23, 42, 35))
                            .blur(16.0)
                            .offset(0.0, 8.0),
                    ),
            );

            // Context menu anchored to right-click cursor point
            let context_menu = column((
                text("Context Actions").style(
                    Style::new().padding_xy(12.0, 4.0).set_text_style(
                        TextStyle::new()
                            .font_size(11.0)
                            .font_weight(FontWeight::BOLD)
                            .color(rgb!(148, 163, 184)),
                    ),
                ),
                menu_item("New File", "New File"),
                menu_item("Duplicate Item", "Duplicate Item"),
                menu_item("Copy Reference", "Copy Reference"),
                menu_item("Inspect Node", "Inspect Node"),
                menu_item("Delete", "Delete"),
            ))
            .style(
                Style::new()
                    .width(Size::Fixed(180))
                    .padding(6.0)
                    .gap(2.0)
                    .corner_radius(10.0)
                    .bg_color(clr!(white))
                    .border(1.0, rgb!(226, 232, 240))
                    .unclipped(true)
                    .box_shadow(
                        BoxShadow::new(rgba!(15, 23, 42, 40))
                            .blur(18.0)
                            .offset(0.0, 8.0),
                    ),
            );

            // 1. Anchored flyout nested inside an Overflow::Hidden container
            let anchored_section = column((
                sub_heading("1. Ancestor Scissor Bypass (Overflow::Hidden)"),
                text("The red-bordered box has Overflow::Hidden and height=140px. The 210px flyout renders unclipped beyond the container boundaries:")
                    .style(Style::new().width(Size::Fill).font_size(12.0).text_wrap(true).text_color(rgb!(100, 116, 139))),
                // Constrained box with Overflow::Hidden
                column((
                    text("Container (height: 140px, Overflow::Hidden)").style(
                        Style::new()
                            .font_size(11.0)
                            .text_color(rgb!(220, 38, 38)),
                    ),
                    button(if state.popover_open { "Close Flyout" } else { "Open Flyout (.overlay)" })
                        .on_click(AppMsg::TogglePopover)
                        .overlay(flyout_menu)
                        .is_open(state.popover_open)
                        .placement(OverlayPlacement::BottomStart)
                        .offset(6.0)
                        .on_dismiss(|| AppMsg::ClosePopover),
                ))
                .style(
                    Style::new()
                        .width(Size::Fixed(400))
                        .height(Size::Fixed(140))
                        .overflow(Overflow::Hidden)
                        .padding(16.0)
                        .gap(12.0)
                        .corner_radius(8.0)
                        .border(1.5, rgb!(239, 68, 68))
                        .bg_color(clr!(white))
                        .box_shadow(
                            BoxShadow::new(rgba!(0, 0, 0, 15))
                                .blur(8.0)
                                .offset(0.0, 2.0),
                        ),
                ),
            ))
            .style(Style::new().gap(8.0));

            // 2. Interactive canvas for right-click context menu
            let context_section = column((
                sub_heading("2. Cursor-Anchored Context Menu (.context_menu)"),
                text("Right-click anywhere inside the card below. The menu opens at cursor coordinates and auto-flips near window boundaries:")
                    .style(Style::new().width(Size::Fill).font_size(12.0).text_wrap(true).text_color(rgb!(100, 116, 139))),
                column((
                    text("Right-click anywhere in this card").style(
                        Style::new()
                            .font_size(13.0)
                            .text_color(rgb!(71, 85, 105)),
                    ),
                ))
                .style(
                    Style::new()
                        .width(Size::Fixed(400))
                        .height(Size::Fixed(140))
                        .corner_radius(8.0)
                        .border(1.5, rgb!(59, 130, 246))
                        .bg_color(clr!(white))
                        .padding(16.0)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center)
                        .box_shadow(
                            BoxShadow::new(rgba!(0, 0, 0, 15))
                                .blur(8.0)
                                .offset(0.0, 2.0),
                        ),
                )
                .on_right_click(|_state, mouse: MouseEventContext| {
                    Some(AppMsg::OpenContextMenu(mouse.pos()))
                })
                .context_menu(
                    state.context_menu_open,
                    state.context_menu_pos,
                    context_menu,
                )
                .on_dismiss(|| AppMsg::CloseContextMenu),
            ))
            .style(Style::new().gap(8.0));

            // Status pill
            let status_bar = row((
                text("Status:").style(
                    Style::new().set_text_style(
                        TextStyle::new()
                            .font_size(12.0)
                            .font_weight(FontWeight::BOLD)
                            .color(rgb!(37, 99, 235)),
                    ),
                ),
                text(&state.last_action)
                    .style(Style::new().font_size(12.0).text_color(rgb!(30, 41, 59))),
            ))
            .style(
                Style::new()
                    .padding_xy(16.0, 10.0)
                    .corner_radius(8.0)
                    .bg_color(clr!(white))
                    .border(1.0, rgb!(226, 232, 240))
                    .gap(8.0)
                    .align_items(AlignItems::Center)
                    .box_shadow(
                        BoxShadow::new(rgba!(0, 0, 0, 10))
                            .blur(4.0)
                            .offset(0.0, 1.0),
                    ),
            );

            // Root layout container
            column((
                heading("MTK Floating Overlays & Anchored Flyouts Demo", 20.0),
                row((anchored_section, context_section)).style(Style::new().gap(24.0)),
                status_bar,
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .height(Size::Percent(1.0))
                    .bg_color(rgb!(248, 250, 252))
                    .padding(32.0)
                    .gap(24.0),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("MTK Floating Overlays & Context Menu Demo")
        .with_size((900, 520).into());

    window.present_with(attrs);
}
