use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::style::{Edges, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node};

/// A container widget that insets its child by the window's safe area insets (e.g. status bar, camera notch, navigation bar).
pub struct SafeArea<Child> {
    pub(crate) child: Child,
    pub(crate) top: bool,
    pub(crate) bottom: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
    pub(crate) bg_color: Option<Color>,
    pub(crate) source_loc: Option<SourceLocation>,
}

/// Creates a new `SafeArea` widget that insets its child based on the current window safe area.
#[track_caller]
pub fn safe_area<Child>(child: Child) -> SafeArea<Child> {
    SafeArea {
        child,
        top: true,
        bottom: true,
        left: true,
        right: true,
        bg_color: None,
        source_loc: Some(SourceLocation::here("SafeArea")),
    }
}

impl<Child> SafeArea<Child> {
    /// Enable or disable safe area inset on the top edge (e.g. status bar, camera cutout).
    pub fn top(mut self, enabled: bool) -> Self {
        self.top = enabled;
        self
    }

    /// Enable or disable safe area inset on the bottom edge (e.g. navigation bar, home indicator).
    pub fn bottom(mut self, enabled: bool) -> Self {
        self.bottom = enabled;
        self
    }

    /// Enable or disable safe area inset on the left edge.
    pub fn left(mut self, enabled: bool) -> Self {
        self.left = enabled;
        self
    }

    /// Enable or disable safe area inset on the right edge.
    pub fn right(mut self, enabled: bool) -> Self {
        self.right = enabled;
        self
    }

    /// Sets the background color of the safe area container, filling the entire window including the inset regions.
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }
}

impl<State, Child: View<State>> View<State> for SafeArea<Child> {
    type Element = (Node, Child::Element);
    type Message = Child::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let parent = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(parent, loc);
        }

        let insets = ctx.safe_area;
        let padding = Edges {
            top: if self.top { insets.top } else { 0.0 },
            bottom: if self.bottom { insets.bottom } else { 0.0 },
            left: if self.left { insets.left } else { 0.0 },
            right: if self.right { insets.right } else { 0.0 },
        };

        let mut style = Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Percent(1.0))
            .padding_edges(padding);

        if let Some(bg) = self.bg_color {
            style = style.bg_color(bg);
        }

        style.apply_to_node(ctx, parent);

        let child_el = self.child.build(ctx);
        parent.append(ctx, self.child.get_node(&child_el));
        (parent, child_el)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        let insets = ctx.safe_area;
        let padding = Edges {
            top: if self.top { insets.top } else { 0.0 },
            bottom: if self.bottom { insets.bottom } else { 0.0 },
            left: if self.left { insets.left } else { 0.0 },
            right: if self.right { insets.right } else { 0.0 },
        };

        element.0.update_constraints(ctx, |c| {
            c.padding = padding;
        });

        if self.bg_color != prev.bg_color {
            if let Some(bg) = self.bg_color {
                element.0.update_effects(ctx, |e| {
                    e.background_color = bg;
                });
            }
        }

        self.child.rebuild(&prev.child, ctx, &mut element.1);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.child.teardown(ctx, &mut element.1);
        element.0.remove(ctx);
        ctx.destroy_node(element.0);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.0
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        self.child.handle_event(&mut element.1, state, event, ctx)
    }
}
