use std::marker::PhantomData;
use std::time::Instant;

use crate::colors::Color;
use crate::debugger::SourceLocation;
use crate::style::{Overflow, PositionStrategy, Size, Style};
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{Context, Node, rgb};

/// A progress bar widget with fluid transitions and smooth indeterminate loading mode.
pub struct ProgressBar<Msg> {
    pub(crate) progress: f32,
    pub(crate) indeterminate: bool,
    pub(crate) height: f32,
    pub(crate) fill_color: Color,
    pub(crate) track_color: Color,
    pub(crate) source_loc: Option<SourceLocation>,
    _marker: PhantomData<Msg>,
}

/// Creates a new `ProgressBar` widget with a progress value from `0.0` to `1.0`.
///
/// # Examples
/// ```rust,ignore
/// progress_bar(state.download_percent / 100.0)
/// ```
#[track_caller]
pub fn progress_bar<Msg>(progress: f32) -> ProgressBar<Msg> {
    ProgressBar {
        progress: progress.clamp(0.0, 1.0),
        indeterminate: false,
        height: 8.0,
        fill_color: rgb!(59, 130, 246),
        track_color: rgb!(226, 232, 240),
        source_loc: Some(SourceLocation::here("ProgressBar")),
        _marker: PhantomData,
    }
}

impl<Msg> ProgressBar<Msg> {
    /// Sets whether the progress bar is in indeterminate (animated loading) mode.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Sets the height of the progress bar in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets the fill color.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Sets the track background color.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }
}

pub struct ProgressBarElement {
    track_node: Node,
    fill_node: Node,
    anim_start: Instant,
}

impl<State, Msg> View<State> for ProgressBar<Msg> {
    type Element = ProgressBarElement;
    type Message = Msg;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let track_node = ctx.create_node();
        if let Some(loc) = self.source_loc {
            ctx.set_node_source(track_node, loc);
        }
        let fill_node = ctx.create_node();
        let anim_start = Instant::now();

        Style::new()
            .width(Size::Percent(1.0))
            .height(Size::Fixed(self.height as u32))
            .corner_radius(self.height / 2.0)
            .bg_color(self.track_color)
            .overflow(Overflow::Hidden)
            .apply_to_node(ctx, track_node);

        Style::new()
            .position(PositionStrategy::Absolute {
                top: 0.0,
                left: 0.0,
                bottom: 0.0,
                right: f32::NAN,
            })
            .width(Size::Percent(self.progress))
            .height(Size::Percent(1.0))
            .corner_radius(self.height / 2.0)
            .bg_color(self.fill_color)
            .apply_to_node(ctx, fill_node);

        track_node.append(ctx, fill_node);

        ProgressBarElement {
            track_node,
            fill_node,
            anim_start,
        }
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        if (self.progress - prev.progress).abs() > 1e-4 {
            element.fill_node.update_constraints(ctx, |c| {
                c.width = Size::Percent(self.progress);
            });
            ctx.request_frame();
        }

        if self.fill_color != prev.fill_color || self.track_color != prev.track_color {
            element.track_node.update_effects(ctx, |e| {
                e.background_color = self.track_color;
            });
            element.fill_node.update_effects(ctx, |e| {
                e.background_color = self.fill_color;
            });
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        element.fill_node.remove(ctx);
        ctx.destroy_node(element.fill_node);
        element.track_node.remove(ctx);
        ctx.destroy_node(element.track_node);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        element.track_node
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        _state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        if let Event::Tick { .. } = event {
            if self.indeterminate {
                let now = element.anim_start.elapsed().as_secs_f64() * 1000.0;
                let cycle = ((now / 1500.0) % 1.0) as f32;
                let track_w = element
                    .track_node
                    .get_computed(ctx)
                    .map(|c| c.w)
                    .unwrap_or(240.0);
                let bar_w = (track_w * 0.3).max(30.0);
                let max_travel = (track_w - bar_w).max(0.0);
                let t = (cycle * std::f32::consts::PI).sin();
                let cur_x = t * max_travel;

                element.fill_node.update_constraints(ctx, |c| {
                    c.positioning = PositionStrategy::Absolute {
                        top: 0.0,
                        left: cur_x,
                        bottom: 0.0,
                        right: f32::NAN,
                    };
                    c.width = Size::Fixed(bar_w as u32);
                });
                ctx.request_frame();
                return (EventResult::Handled, None);
            }
        }

        (EventResult::Ignored, None)
    }
}
