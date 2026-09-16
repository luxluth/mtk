use mtk::animation::Spring;
use mtk::style::{AlignItems, JustifyContent, Size, Style, TextStyle};
use mtk::text_property::FontWeight;
use mtk::ui::event::{EventKind, ViewEventExt as _};
use mtk::ui::morph::MorphViewExt;
use mtk::ui::transition::PageTransition;
use mtk::ui::widgets::{button, column, router, row, text};
use mtk::ui::{View, ViewStyleExt};
use mtk::windowing::{Window, WindowAttributes, WindowDimension};
use mtk::{BoxShadow, clr, rgb, rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Gallery,
    Details(usize),
}

#[derive(Clone, Debug)]
pub struct CardData {
    pub id: usize,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub description: &'static str,
    pub color_card: mtk::Color,
    pub color_hero: mtk::Color,
}

const ITEMS: [CardData; 3] = [
    CardData {
        id: 0,
        title: "Aurora Engine",
        subtitle: "High-Performance Vector Graphics",
        description: "A modern, GPU-accelerated rendering architecture designed for rich desktop interfaces with sub-millisecond frame times and analytical scissoring.",
        color_card: rgb!(79, 70, 229),
        color_hero: rgb!(67, 56, 202),
    },
    CardData {
        id: 1,
        title: "Nebula Shaders",
        subtitle: "Realtime Procedural Effects",
        description: "Composable multi-pass shader pipelines supporting frosted glass, directional blurs, analytical drop shadows, and responsive morph animations.",
        color_card: rgb!(219, 39, 119),
        color_hero: rgb!(190, 24, 93),
    },
    CardData {
        id: 2,
        title: "Emerald Core",
        subtitle: "Zero-Cost Reactive State",
        description: "Ultra-lightweight state management and memoized view hierarchies that bring desktop applications the speed of native C with declarative ergonomics.",
        color_card: rgb!(5, 150, 105),
        color_hero: rgb!(4, 120, 87),
    },
];

#[derive(Clone, Debug)]
pub struct AppState {
    pub screen: Screen,
}

#[derive(Clone, Debug)]
pub enum AppMsg {
    OpenDetails(usize),
    BackToGallery,
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::OpenDetails(idx) => {
            state.screen = Screen::Details(idx);
        }
        AppMsg::BackToGallery => {
            state.screen = Screen::Gallery;
        }
    }
}

fn render_gallery() -> impl View<AppState, Message = AppMsg> + use<> {
    let header = column((
        text::<_, AppMsg>("MTK Shared Element Transitions").style(
            Style::new().font_size(28.0).set_text_style(TextStyle {
                color: rgb!(15, 23, 42),
                font_weight: FontWeight::BOLD,
                ..Default::default()
            }),
        ),
        text::<_, AppMsg>(
            "Click any card to observe the spring-driven morph animation across page routes",
        )
        .style(Style::new().font_size(14.0).set_text_style(TextStyle {
            color: rgb!(100, 116, 139),
            ..Default::default()
        })),
    ))
    .style(Style::new().align_items(AlignItems::Center).gap(6.0));

    let mut card_views = Vec::new();
    for item in &ITEMS {
        let card_id = item.id;
        let morph_id = format!("morph-card-{}", card_id);

        let card_content = column((
            text::<_, AppMsg>(item.title).style(Style::new().font_size(18.0).set_text_style(
                TextStyle {
                    color: clr!(white),
                    font_weight: FontWeight::BOLD,
                    ..Default::default()
                },
            )),
            text::<_, AppMsg>(item.subtitle).style(
                Style::new()
                    .width(Size::Fill)
                    .font_size(12.0)
                    .set_text_style(TextStyle {
                        color: rgba!(255, 255, 255, 220),
                        wrap: true,
                        ..Default::default()
                    }),
            ),
        ))
        .style(
            Style::new()
                .width(Size::Fixed(240))
                .height(Size::Fixed(160))
                .padding(20.0)
                .bg_color(item.color_card)
                .corner_radius(14.0)
                .box_shadow(
                    BoxShadow::drop(rgba!(0, 0, 0, 45))
                        .offset(0.0, 4.0)
                        .blur(10.0),
                )
                .justify_content(JustifyContent::SpaceBetween),
        )
        .morph(morph_id)
        .spring(Spring::gentle())
        .on_event(EventKind::Click, move |_| {
            Some(AppMsg::OpenDetails(card_id))
        });

        card_views.push(card_content);
    }

    let cards_row = row(card_views).style(
        Style::new()
            .gap(24.0)
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center),
    );

    column((header, cards_row)).style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .gap(28.0)
            .bg_color(rgb!(248, 250, 252)),
    )
}

fn render_details(idx: usize) -> impl View<AppState, Message = AppMsg> + use<> {
    let item = &ITEMS[idx.min(ITEMS.len() - 1)];
    let morph_id = format!("morph-card-{}", item.id);

    let hero_banner = column((
        row((
            button("< Back").on_click(AppMsg::BackToGallery).style(
                Style::new()
                    .padding_xy(12.0, 6.0)
                    .bg_color(rgba!(255, 255, 255, 50))
                    .corner_radius(8.0)
                    .set_text_style(TextStyle {
                        color: clr!(white),
                        font_weight: FontWeight::BOLD,
                        font_size: 13.0,
                        ..Default::default()
                    }),
            ),
            text::<_, AppMsg>(item.subtitle).style(Style::new().font_size(13.0).set_text_style(
                TextStyle {
                    color: rgba!(255, 255, 255, 230),
                    ..Default::default()
                },
            )),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .justify_content(JustifyContent::SpaceBetween)
                .align_items(AlignItems::Center),
        ),
        column((
            text::<_, AppMsg>(item.title).style(Style::new().font_size(32.0).set_text_style(
                TextStyle {
                    color: clr!(white),
                    font_weight: FontWeight::BOLD,
                    ..Default::default()
                },
            )),
            text::<_, AppMsg>(item.description).style(
                Style::new()
                    .width(Size::Fill)
                    .font_size(15.0)
                    .set_text_style(TextStyle {
                        color: rgba!(255, 255, 255, 235),
                        wrap: true,
                        ..Default::default()
                    }),
            ),
        ))
        .style(Style::new().width(Size::Percent(1.0)).gap(10.0)),
    ))
    .style(
        Style::new()
            .width(Size::Fixed(760))
            .height(Size::Fixed(320))
            .padding(28.0)
            .bg_color(item.color_hero)
            .corner_radius(26.0)
            .box_shadow(
                BoxShadow::drop(rgba!(0, 0, 0, 65))
                    .offset(0.0, 12.0)
                    .blur(28.0),
            )
            .justify_content(JustifyContent::SpaceBetween),
    )
    .morph(morph_id)
    .spring(Spring::gentle());

    let actions = row((button("Return to Gallery")
        .on_click(AppMsg::BackToGallery)
        .style(
            Style::new()
                .padding_xy(20.0, 10.0)
                .bg_color(rgb!(15, 23, 42))
                .corner_radius(10.0)
                .set_text_style(TextStyle {
                    color: clr!(white),
                    font_weight: FontWeight::MEDIUM,
                    font_size: 14.0,
                    ..Default::default()
                }),
        ),))
    .style(Style::new().justify_content(JustifyContent::Center));

    column((hero_banner, actions)).style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center)
            .gap(20.0)
            .bg_color(rgb!(248, 250, 252)),
    )
}

fn app(state: &AppState) -> impl View<AppState, Message = AppMsg> + use<> {
    let view = match state.screen {
        Screen::Gallery => mtk::ui::boxed::BoxedView::new(render_gallery()),
        Screen::Details(idx) => mtk::ui::boxed::BoxedView::new(render_details(idx)),
    };

    router(state.screen, view).transition(PageTransition::fade().duration_secs(1.))
}

fn main() {
    let initial_state = AppState {
        screen: Screen::Gallery,
    };

    #[cfg(feature = "debugger")]
    let mut window = Window::with(initial_state, update, app);

    #[cfg(not(feature = "debugger"))]
    let window = Window::with(initial_state, update, app);

    #[cfg(feature = "debugger")]
    window.enable_terminal_debugger();

    window.present_with(
        WindowAttributes::default()
            .with_decorations(true)
            .with_title("MTK - Shared Element Morph Transitions Demo")
            .with_size(WindowDimension {
                width: 1000,
                height: 650,
            }),
    );
}
