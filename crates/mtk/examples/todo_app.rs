use mtk::{
    AlignItems, AlignSelf, Lens, Size, Style, TextStyle, clr, hsl, text_property,
    ui::{
        EventKind, View, ViewAdaptExt, ViewEventExt, ViewStyleExt,
        memoize::memoize,
        widgets::{column, input_text, row, scroll_view, text},
    },
    windowing::{Window, WindowAttributes},
};

#[derive(Clone, Debug, PartialEq)]
pub struct TodoItem {
    pub id: usize,
    pub title: String,
    pub completed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    All,
    Active,
    Completed,
}

#[derive(Lens)]
pub struct TodoState {
    pub new_input: String,
    pub todos: Vec<TodoItem>,
    pub filter: FilterKind,
    pub next_id: usize,
}

#[derive(Clone, Debug)]
pub enum TodoMsg {
    UpdateInput(String),
    AddTodo,
    ToggleTodo(usize),
    DeleteTodo(usize),
    SetFilter(FilterKind),
    ClearCompleted,
}

fn update(state: &mut TodoState, msg: TodoMsg) {
    match msg {
        TodoMsg::UpdateInput(text) => {
            state.new_input = text;
        }
        TodoMsg::AddTodo => {
            let trimmed = state.new_input.trim();
            if !trimmed.is_empty() {
                state.todos.push(TodoItem {
                    id: state.next_id,
                    title: trimmed.to_string(),
                    completed: false,
                });
                state.next_id += 1;
                state.new_input.clear();
            }
        }
        TodoMsg::ToggleTodo(id) => {
            if let Some(todo) = state.todos.iter_mut().find(|t| t.id == id) {
                todo.completed = !todo.completed;
            }
        }
        TodoMsg::DeleteTodo(id) => {
            state.todos.retain(|t| t.id != id);
        }
        TodoMsg::SetFilter(filter) => {
            state.filter = filter;
        }
        TodoMsg::ClearCompleted => {
            state.todos.retain(|t| !t.completed);
        }
    }
}

fn filter_button(
    label: &str,
    kind: FilterKind,
    current: FilterKind,
) -> impl View<TodoState, Message = TodoMsg> + use<> {
    let is_active = kind == current;
    let bg = if is_active {
        hsl!(222.2, 0.47, 0.11) // High contrast Slate 900
    } else {
        hsl!(220.0, 0.14, 0.93) // Crisp Slate 100
    };
    let fg = if is_active {
        clr!(white)
    } else {
        hsl!(222.2, 0.47, 0.18) // High contrast Slate 800
    };

    text(label)
        .style(
            Style::new()
                .padding_xy(12.0, 6.0)
                .bg_color(bg)
                .corner_radius(6.0)
                .set_text_style(TextStyle {
                    font_size: 13.0,
                    color: fg,
                    font_weight: text_property::FontWeight::BOLD,
                    alignment: text_property::Alignment::Center,
                    vertical_alignment: mtk::style::VerticalAlignment::Center,
                    ..Default::default()
                }),
        )
        .on_event(EventKind::Click, move |_| Some(TodoMsg::SetFilter(kind)))
}

fn todo_item_view(todo: &TodoItem) -> impl View<TodoState, Message = TodoMsg> + use<> {
    let id = todo.id;
    let completed = todo.completed;

    let check_icon = if completed { "✓" } else { "" };
    let text_color = if completed {
        hsl!(215.4, 0.16, 0.55) // Slate 400 with high contrast
    } else {
        hsl!(222.2, 0.84, 0.05) // Slate 950 High Contrast Text
    };

    let checkbox_bg = if completed {
        hsl!(222.2, 0.47, 0.11) // Slate 900
    } else {
        clr!(white)
    };

    let checkbox_border = if completed {
        hsl!(222.2, 0.47, 0.11)
    } else {
        hsl!(214.3, 0.32, 0.72) // High contrast Slate 400 border
    };

    row((
        // Custom Checkbox Button
        text(check_icon)
            .style(
                Style::new()
                    .width(Size::Fixed(20))
                    .height(Size::Fixed(20))
                    .bg_color(checkbox_bg)
                    .border(1.5, checkbox_border)
                    .corner_radius(5.0)
                    .set_text_style(TextStyle {
                        font_size: 12.0,
                        color: clr!(white),
                        font_weight: text_property::FontWeight::BOLD,
                        alignment: text_property::Alignment::Center,
                        vertical_alignment: mtk::style::VerticalAlignment::Center,
                        ..Default::default()
                    }),
            )
            .on_event(EventKind::Click, move |_| Some(TodoMsg::ToggleTodo(id))),
        // Todo Item Title
        text(todo.title.clone()).style(
            Style::new()
                .flex_grow(1.0)
                .padding_xy(10.0, 4.0)
                .set_text_style(TextStyle {
                    font_size: 14.0,
                    color: text_color,
                    vertical_alignment: mtk::style::VerticalAlignment::Center,
                    strikethrough: completed,
                    ..Default::default()
                }),
        ),
        // Delete Action Button
        text("✕")
            .style(
                Style::new()
                    .align_self(AlignSelf::Center)
                    .padding_xy(8.0, 4.0)
                    .set_text_style(TextStyle {
                        font_size: 13.0,
                        color: hsl!(215.4, 0.16, 0.50), // High contrast slate icon
                        font_weight: text_property::FontWeight::BOLD,
                        alignment: text_property::Alignment::Center,
                        vertical_alignment: mtk::style::VerticalAlignment::Center,
                        ..Default::default()
                    }),
            )
            .on_event(EventKind::Click, move |_| Some(TodoMsg::DeleteTodo(id))),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .align_items(AlignItems::Center)
            .bg_color(clr!(white))
            .corner_radius(8.0)
            .border(1.0, hsl!(214.3, 0.32, 0.88)) // High Contrast Inset Border
            .padding_xy(12.0, 8.0),
    )
}

fn app(state: &TodoState) -> impl View<TodoState, Message = TodoMsg> + use<> {
    let active_count = state.todos.iter().filter(|t| !t.completed).count();
    let completed_count = state.todos.iter().filter(|t| t.completed).count();
    let count_str = format!(
        "{} item{} left",
        active_count,
        if active_count <= 1 { "" } else { "s" }
    );

    let filtered_items: Vec<_> = state
        .todos
        .iter()
        .filter(|t| match state.filter {
            FilterKind::All => true,
            FilterKind::Active => !t.completed,
            FilterKind::Completed => t.completed,
        })
        .map(|t| memoize(t.clone(), todo_item_view))
        .collect();

    let has_completed = completed_count > 0;

    column((
        // Header Row: App Title + Counter Badge
        row((
            text("Tasks").style(Style::new().align_self(AlignSelf::Center).set_text_style(
                TextStyle {
                    font_size: 26.0,
                    font_weight: text_property::FontWeight::BOLD,
                    color: hsl!(222.2, 0.84, 0.05), // Slate 950 High Contrast
                    vertical_alignment: mtk::style::VerticalAlignment::Center,
                    ..Default::default()
                },
            )),
            text(count_str).style(Style::new().align_self(AlignSelf::Center).set_text_style(
                TextStyle {
                    font_size: 13.0,
                    color: hsl!(215.4, 0.16, 0.40), // Slate 600 Badge
                    vertical_alignment: mtk::style::VerticalAlignment::Center,
                    ..Default::default()
                },
            )),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .align_items(AlignItems::Center)
                .gap(12.0),
        ),
        // Input Control Bar: [Input Field (flex_grow: 1)] + [Primary Action Button]
        row((
            input_text()
                .placeholder("Add a new task...")
                .text_style(TextStyle {
                    font_size: 14.0,
                    color: hsl!(222.2, 0.84, 0.05),
                    ..Default::default()
                })
                .style(
                    Style::new()
                        .flex_grow(1.0)
                        .height(Size::Fixed(42))
                        .padding(12.0)
                        .bg_color(clr!(white))
                        .border(1.5, hsl!(214.3, 0.32, 0.85)) // High Contrast Border
                        .corner_radius(8.0),
                )
                .adapt(TodoState::new_input, TodoMsg::UpdateInput)
                .on_event(EventKind::Submit, |_| Some(TodoMsg::AddTodo)),
            text("Add Task")
                .style(
                    Style::new()
                        .width(Size::Percent(0.22))
                        .height(Size::Fixed(42))
                        .bg_color(hsl!(222.2, 0.47, 0.11)) // Slate 900 Primary Fill
                        .corner_radius(8.0)
                        .set_text_style(TextStyle {
                            font_size: 14.0,
                            color: clr!(white),
                            font_weight: text_property::FontWeight::BOLD,
                            alignment: text_property::Alignment::Center,
                            vertical_alignment: mtk::style::VerticalAlignment::Center,
                            ..Default::default()
                        }),
                )
                .on_event(EventKind::Click, |_| Some(TodoMsg::AddTodo)),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .gap(10.0)
                .align_items(AlignItems::Center),
        ),
        // Explicitly Delimited Scroll Container with Inset Background and High Contrast Border
        scroll_view(
            column(filtered_items)
                .style(Style::new().width(Size::Percent(1.0)).padding(8.0).gap(6.0)),
        )
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .flex_grow(1.0)
                .bg_color(hsl!(220.0, 0.14, 0.96)) // Distinct Inset Slate 100 Background
                .border(1.5, hsl!(214.3, 0.32, 0.85)) // High Contrast Delimitation Border
                .corner_radius(8.0),
        ),
        // Footer Bar: Filter Controls & Clear Completed Action
        row((
            // Filter Pills
            row((
                filter_button("All", FilterKind::All, state.filter),
                filter_button("Active", FilterKind::Active, state.filter),
                filter_button("Completed", FilterKind::Completed, state.filter),
            ))
            .style(Style::new().gap(6.0)),
            // Clear Completed Link
            if has_completed {
                Some(
                    text("Clear Completed")
                        .style(
                            Style::new()
                                .padding_xy(10.0, 6.0)
                                .set_text_style(TextStyle {
                                    font_size: 13.0,
                                    color: hsl!(0.0, 0.84, 0.55), // High Contrast Red 500
                                    font_weight: text_property::FontWeight::BOLD,
                                    vertical_alignment: mtk::style::VerticalAlignment::Center,
                                    ..Default::default()
                                }),
                        )
                        .on_event(EventKind::Click, |_| Some(TodoMsg::ClearCompleted)),
                )
            } else {
                None
            },
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .align_items(AlignItems::Center)
                .padding_xy(10.0, 8.0)
                .border(1.0, hsl!(214.3, 0.32, 0.88))
                .corner_radius(8.0)
                .bg_color(hsl!(220.0, 0.14, 0.97)),
        ),
    ))
    .style(
        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .padding(24.0)
            .gap(16.0)
            .bg_color(clr!(white))
            .border(1.5, hsl!(214.3, 0.32, 0.85)) // High Contrast Card Border
            .corner_radius(12.0),
    )
}

fn main() {
    env_logger::init();

    let initial_state = TodoState {
        new_input: String::new(),
        todos: vec![
            TodoItem {
                id: 1,
                title: "Build MTK framework features".to_string(),
                completed: true,
            },
            TodoItem {
                id: 2,
                title: "Implement Todo app example".to_string(),
                completed: false,
            },
            TodoItem {
                id: 3,
                title: "Analyze missing MTK capabilities".to_string(),
                completed: false,
            },
        ],
        filter: FilterKind::All,
        next_id: 4,
    };

    let mut window = Window::with(initial_state, update, app);

    #[cfg(feature = "debugger")]
    window.enable_terminal_debugger();

    window.present_with(
        WindowAttributes::default()
            .with_decorations(true)
            .with_title("MTK Todo App Example")
            .with_size((600, 700).into())
            .with_min_size(Some((490, 800).into()))
            .with_app_id("dev.mtk.todo"),
    );
}
