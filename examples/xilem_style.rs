use mtk::{
    Edges, FlexDirection, Size,
    animation::Curve,
    rgb,
    ui::{
        style::{AnimationTarget, Style, TextStyle, ViewStyleExt},
        widgets::{column, container, row, text},
    },
    windowing::{Window, WindowAttr},
};

pub struct AppState {
    pub users: Vec<String>,
    pub is_expanded: bool,
}

fn main() {
    let state = AppState {
        users: vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        is_expanded: false,
    };

    let mut window = Window::with(state, |state: &mut AppState| {
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
                        .bg_color(rgb!(40, 40, 40))
                        .on_hover(|s| {
                            s.bg_color(rgb!(60, 60, 100))
                                .update_constraints(|c| c.padding.left = 20.0)
                                .update_text_style(|t| t.font_size = 20.0)
                        })
                        .animate(AnimationTarget::Padding, 200.0, Curve::ease_out())
                        .animate(AnimationTarget::BackgroundColor, 200.0, Curve::linear()),
                )
            })
            .collect();

        column((
            text("--- User Dashboard ---").style(
                Style::new()
                    .update_constraints(|c| c.padding = Edges::all(20.0))
                    .bg_color(rgb!(20, 20, 20))
                    .set_text_style(TextStyle {
                        font_size: 24.0,
                        ..Default::default()
                    }),
            ),
            container(dynamic_user_list).style(Style::new().update_constraints(|c| {
                c.flex_direction = FlexDirection::Column;
                c.gap = 5.0;
                c.padding = Edges::all(10.0);
            })),
            row((
                text("Add User").style(
                    Style::new()
                        .update_constraints(|c| c.padding = Edges::all(15.0))
                        .bg_color(rgb!(30, 150, 50))
                        .on_hover(|s| s.bg_color(rgb!(50, 200, 80))),
                ),
                text("Clear All").style(
                    Style::new()
                        .update_constraints(|c| c.padding = Edges::all(15.0))
                        .bg_color(rgb!(150, 30, 30))
                        .on_hover(|s| s.bg_color(rgb!(200, 50, 50))),
                ),
            ))
            .style(Style::new().update_constraints(|c| c.gap = 10.0)),
        ))
        .style(Style::new().update_constraints(|c| {
            c.width = Size::Percent(1.0);
            c.height = Size::Percent(1.0);
            c.padding = Edges::all(20.0);
        }))
    });

    let attr = WindowAttr::default()
        .with_title("MTK Interactive Dashboard".to_string())
        .with_resizable(true);

    window.present_with(attr);
}
