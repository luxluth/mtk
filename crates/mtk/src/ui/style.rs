use std::time::{SystemTime, UNIX_EPOCH};

use crate::animation::AnimatedValue;
use crate::colors::Color;
use crate::style::Edges;
use crate::ui::event::EventResult;
use crate::ui::{Event, View};
use crate::{AnimationTarget, Context, Node, Overflow, Style, TextRenderInfo};

pub struct StyledView<V> {
    pub(crate) inner: V,
    pub(crate) style: Style,
}

pub trait ViewStyleExt: Sized {
    fn style(self, style: Style) -> StyledView<Self>;
}

impl<V> ViewStyleExt for V {
    fn style(self, style: Style) -> StyledView<Self> {
        StyledView { inner: self, style }
    }
}

#[derive(Default)]
pub struct StyledViewState {
    pub is_hovered: bool,
    pub padding_anim: Option<AnimatedValue<f32>>,
    pub color_anim: Option<AnimatedValue<Color>>,
    pub scale_anim: Option<AnimatedValue<f32>>,
}

impl StyledViewState {
    pub fn new() -> Self {
        Self {
            is_hovered: false,
            padding_anim: None,
            color_anim: None,
            scale_anim: None,
        }
    }
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * 1000.0
}

impl<State, V: View<State>> View<State> for StyledView<V> {
    type Element = (V::Element, StyledViewState);
    type Message = V::Message;

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let child_el = self.inner.build(ctx);
        let node = self.inner.get_node(&child_el);

        node.update_constraints(ctx, |c| {
            let overflow = c.overflow;
            let scroll = c.scroll;
            *c = self.style.base_constraints;

            // ScrollView sets Overflow::Scroll before StyledView runs, preserve it if we didn't explicitly override it (assuming default Visible means no override for ScrollView)
            if self.style.base_constraints.overflow == Overflow::Visible
                && overflow == Overflow::Scroll
            {
                c.overflow = overflow;
            }
            c.scroll = scroll;
        });

        node.set_effects(ctx, self.style.base_effects.clone());

        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            if let Some(mut info) = node.get_text_userdata::<TextRenderInfo>(ctx).cloned() {
                info.style = self.style.base_text_style.clone();
                node.set_text_with_userdata(ctx, &text_owned, info);
            } else {
                node.set_text_with_userdata(ctx, &text_owned, self.style.base_text_style.clone());
            }
        }

        let mut view_state = StyledViewState::new();

        for (target, _, _) in &self.style.transitions {
            match target {
                AnimationTarget::Padding => {
                    view_state.padding_anim =
                        Some(AnimatedValue::new(self.style.base_constraints.padding.top));
                }
                AnimationTarget::BackgroundColor => {
                    view_state.color_anim =
                        Some(AnimatedValue::new(self.style.base_effects.background_color));
                }
                AnimationTarget::Scale => {
                    view_state.scale_anim = Some(AnimatedValue::new(self.style.base_effects.scale));
                }
            }
        }

        (child_el, view_state)
    }

    fn rebuild(&self, prev: &Self, ctx: &mut Context, element: &mut Self::Element) {
        // Rebuild child
        self.inner.rebuild(&prev.inner, ctx, &mut element.0);
        let node = self.inner.get_node(&element.0);
        self.apply_style(ctx, &mut element.1, node);
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.0);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.0)
    }

    fn handle_event(
        &self,
        element: &mut Self::Element,
        state: &State,
        event: Event,
        ctx: &mut Context,
    ) -> (EventResult, Option<Self::Message>) {
        let mut hover_changed = false;

        if let Event::CursorMoved { hit_nodes, .. } = &event {
            let node = self.inner.get_node(&element.0);
            let newly_hovered = hit_nodes.contains(&node);
            if element.1.is_hovered != newly_hovered {
                element.1.is_hovered = newly_hovered;
                hover_changed = true;
            }
        }

        let is_tick = matches!(event, Event::Tick { .. });

        if hover_changed || is_tick {
            let node = self.inner.get_node(&element.0);
            self.apply_style(ctx, &mut element.1, node);

            if hover_changed {
                ctx.request_frame();
            }
        }

        self.inner.handle_event(&mut element.0, state, event, ctx)
    }
}

impl<V> StyledView<V> {
    fn apply_style(&self, ctx: &mut Context, view_state: &mut StyledViewState, node: Node) {
        let target_constraints = if view_state.is_hovered {
            self.style
                .hover_constraints
                .unwrap_or(self.style.base_constraints)
        } else {
            self.style.base_constraints
        };

        let target_effects = if view_state.is_hovered {
            self.style
                .hover_effects
                .as_ref()
                .unwrap_or(&self.style.base_effects)
        } else {
            &self.style.base_effects
        };

        let mut is_animating = false;

        if let Some(padding_anim) = &mut view_state.padding_anim {
            let transition = self
                .style
                .transitions
                .iter()
                .find(|t| t.0 == AnimationTarget::Padding);

            if let Some((_, duration, curve)) = transition {
                let target_pad = target_constraints.padding.top;
                padding_anim.set_target(target_pad, now_ms(), *duration, *curve);
            }

            if padding_anim.tick(now_ms()) {
                is_animating = true;
                node.update_constraints(ctx, |c| c.padding = Edges::all(padding_anim.current));
            } else {
                node.update_constraints(ctx, |c| {
                    c.padding = Edges::all(target_constraints.padding.top)
                });
            }
        } else {
            node.update_constraints(ctx, |c| {
                let overflow = c.overflow;
                let scroll = c.scroll;
                *c = target_constraints;

                if target_constraints.overflow == crate::style::Overflow::Visible
                    && overflow == crate::style::Overflow::Scroll
                {
                    c.overflow = overflow;
                }
                c.scroll = scroll;
            });
        }

        let mut final_effects = target_effects.clone();

        if let Some(color_anim) = &mut view_state.color_anim {
            let transition = self
                .style
                .transitions
                .iter()
                .find(|t| t.0 == AnimationTarget::BackgroundColor);

            if let Some((_, duration, curve)) = transition {
                let target_color = target_effects.background_color;
                color_anim.set_target(target_color, now_ms(), *duration, *curve);
            }

            if color_anim.tick(now_ms()) {
                is_animating = true;
                final_effects.background_color = color_anim.current;
            } else {
                final_effects.background_color = target_effects.background_color;
            }
        }

        if let Some(scale_anim) = &mut view_state.scale_anim {
            let transition = self
                .style
                .transitions
                .iter()
                .find(|t| t.0 == AnimationTarget::Scale);

            if let Some((_, duration, curve)) = transition {
                let target_scale = target_effects.scale;
                scale_anim.set_target(target_scale, now_ms(), *duration, *curve);
            }

            if scale_anim.tick(now_ms()) {
                is_animating = true;
                final_effects.scale = scale_anim.current;
            } else {
                final_effects.scale = target_effects.scale;
            }
        }

        node.set_effects(ctx, final_effects);

        let target_text_style = if view_state.is_hovered {
            self.style
                .hover_text_style
                .as_ref()
                .unwrap_or(&self.style.base_text_style)
        } else {
            &self.style.base_text_style
        };

        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            if let Some(mut info) = node.get_text_userdata::<TextRenderInfo>(ctx).cloned() {
                info.style = target_text_style.clone();
                node.set_text_with_userdata(ctx, &text_owned, info);
            } else {
                node.set_text_with_userdata(ctx, &text_owned, target_text_style.clone());
            }
        }

        if is_animating {
            ctx.request_frame();
        }
    }
}
