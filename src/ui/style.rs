use std::time::{SystemTime, UNIX_EPOCH};

use cosmic_text::{Align, Attrs, AttrsOwned, Family};

use crate::animation::{AnimatedValue, Curve};
use crate::colors::Color;
use crate::effects::{Effects, Radius};
use crate::style::{Constraints, Edges};
use crate::ui::{Event, View};
use crate::{AlignItems, Context, FlexDirection, JustifyContent, Node, Overflow, Size, rgb};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationTarget {
    Padding,
    BackgroundColor,
    Scale,
}

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub attrs: AttrsOwned,
    pub alignement: Align,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: 20.0,
            attrs: AttrsOwned::new(
                &Attrs::new()
                    .color(rgb!(0, 0, 0).into())
                    .family(Family::SansSerif),
            ),
            alignement: Align::Left,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub base_constraints: Constraints,
    pub base_effects: Effects,
    pub base_text_style: TextStyle,

    pub hover_constraints: Option<Constraints>,
    pub hover_effects: Option<Effects>,
    pub hover_text_style: Option<TextStyle>,

    pub transitions: Vec<(AnimationTarget, f64, Curve)>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn padding(mut self, val: f32) -> Self {
        self.base_constraints.padding = Edges::all(val);
        self
    }

    pub fn padding_edges(mut self, edges: Edges) -> Self {
        self.base_constraints.padding = edges;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.base_constraints.border = Edges::all(width);
        self.base_effects.border.color = color;
        self
    }

    pub fn border_edges(mut self, edges: Edges, color: Color) -> Self {
        self.base_constraints.border = edges;
        self.base_effects.border.color = color;
        self
    }

    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.base_effects.border.radius = Radius::all(radius);
        self
    }

    pub fn width(mut self, size: Size) -> Self {
        self.base_constraints.width = size;
        self
    }

    pub fn height(mut self, size: Size) -> Self {
        self.base_constraints.height = size;
        self
    }

    pub fn justify_content(mut self, j: JustifyContent) -> Self {
        self.base_constraints.justify_content = j;
        self
    }

    pub fn align_items(mut self, a: AlignItems) -> Self {
        self.base_constraints.align_items = a;
        self
    }

    pub fn gap(mut self, val: f32) -> Self {
        self.base_constraints.gap = val;
        self
    }

    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.base_constraints.overflow = overflow;
        self
    }

    pub fn bg_color(mut self, color: Color) -> Self {
        self.base_effects.background_color = color;
        self
    }

    pub fn scale(mut self, s: f32) -> Self {
        self.base_effects.scale = s;
        self
    }

    pub fn flex_direction(mut self, dir: FlexDirection) -> Self {
        self.base_constraints.flex_direction = dir;
        self
    }

    pub fn set_constraints(mut self, c: Constraints) -> Self {
        self.base_constraints = c;
        self
    }

    pub fn update_constraints(mut self, f: impl FnOnce(&mut Constraints)) -> Self {
        f(&mut self.base_constraints);
        self
    }

    pub fn set_effects(mut self, e: Effects) -> Self {
        self.base_effects = e;
        self
    }

    pub fn update_effects(mut self, f: impl FnOnce(&mut Effects)) -> Self {
        f(&mut self.base_effects);
        self
    }

    pub fn set_text_style(mut self, t: TextStyle) -> Self {
        self.base_text_style = t;
        self
    }

    pub fn update_text_style(mut self, f: impl FnOnce(&mut TextStyle)) -> Self {
        f(&mut self.base_text_style);
        self
    }

    pub fn on_hover(mut self, hover_fn: impl FnOnce(Style) -> Style) -> Self {
        let hover_style = hover_fn(self.clone());
        self.hover_constraints = Some(hover_style.base_constraints);
        self.hover_effects = Some(hover_style.base_effects);
        self.hover_text_style = Some(hover_style.base_text_style);
        self
    }

    pub fn animate(mut self, target: AnimationTarget, duration_ms: f64, curve: Curve) -> Self {
        self.transitions.push((target, duration_ms, curve));
        self
    }
}

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

    fn build(&self, ctx: &mut Context) -> Self::Element {
        let child_el = self.inner.build(ctx);
        let node = self.inner.get_node(&child_el);

        node.set_constraints(ctx, self.style.base_constraints);
        node.set_effects(ctx, self.style.base_effects.clone());

        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            node.set_text_with_userdata(ctx, &text_owned, self.style.base_text_style.clone());
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

        let view_state = &mut element.1;
        let node = self.inner.get_node(&element.0);

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
            node.set_constraints(ctx, target_constraints);
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
        } else {
            final_effects.background_color = target_effects.background_color;
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
        } else {
            final_effects.scale = target_effects.scale;
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
            node.set_text_with_userdata(ctx, &text_owned, target_text_style.clone());
        }

        if is_animating {
            ctx.request_frame();
        }
    }

    fn teardown(&self, ctx: &mut Context, element: &mut Self::Element) {
        self.inner.teardown(ctx, &mut element.0);
    }

    fn get_node(&self, element: &Self::Element) -> Node {
        self.inner.get_node(&element.0)
    }

    fn message(&self, element: &mut Self::Element, state: &mut State, event: Event) {
        if let Event::CursorMoved { hit_nodes, .. } = &event {
            let node = self.inner.get_node(&element.0);
            element.1.is_hovered = hit_nodes.contains(&node);
        }

        self.inner.message(&mut element.0, state, event);
    }
}
