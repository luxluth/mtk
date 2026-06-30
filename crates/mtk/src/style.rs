use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

use parley::layout::Alignment;
use parley::style::{FontStyle, FontWeight, OverflowWrap};

use crate::animation::Curve;
use crate::colors::Color;
use crate::effects::{Effects, Radius};
use crate::{clr, sys};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    /// The element's size is a fraction of its parent's size
    Percent(f32),
    /// The element has a hardcoded size
    Fixed(u32),
    /// The element consumes all remaining available space inside the parent
    /// after other siblings are measured
    Fill,
    /// The element shrinks to tightly wrap its internal contents or children
    Fit,
}

impl Into<sys::muSize> for Size {
    fn into(self) -> sys::muSize {
        match self {
            Size::Percent(percent) => sys::muSize {
                kind: sys::muSizeKind_MU_PERCENT,
                __bindgen_anon_1: sys::muSize__bindgen_ty_1 { percent },
            },
            Size::Fixed(px) => sys::muSize {
                kind: sys::muSizeKind_MU_FIXED,
                __bindgen_anon_1: sys::muSize__bindgen_ty_1 { px },
            },
            Size::Fill => sys::muSize {
                kind: sys::muSizeKind_MU_FILL,
                __bindgen_anon_1: sys::muSize__bindgen_ty_1 { fill: true },
            },
            Size::Fit => sys::muSize {
                kind: sys::muSizeKind_MU_FIT,
                __bindgen_anon_1: sys::muSize__bindgen_ty_1 { fit: true },
            },
        }
    }
}

impl From<sys::muSize> for Size {
    fn from(s: sys::muSize) -> Self {
        match s.kind {
            sys::muSizeKind_MU_PERCENT => Size::Percent(unsafe { s.__bindgen_anon_1.percent }),
            sys::muSizeKind_MU_FIXED => Size::Fixed(unsafe { s.__bindgen_anon_1.px }),
            sys::muSizeKind_MU_FILL => Size::Fill,
            sys::muSizeKind_MU_FIT => Size::Fit,
            _ => Size::Fit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edges {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Default for Edges {
    fn default() -> Self {
        Self::all(0.0)
    }
}

impl Edges {
    pub fn all(v: f32) -> Self {
        Self {
            top: v,
            bottom: v,
            left: v,
            right: v,
        }
    }
    pub fn lr(v: f32) -> Self {
        Self {
            top: 0.0,
            bottom: 0.0,
            left: v,
            right: v,
        }
    }
    pub fn tb(v: f32) -> Self {
        Self {
            top: v,
            bottom: v,
            left: 0.0,
            right: 0.0,
        }
    }
}

impl Into<sys::muEdges> for Edges {
    fn into(self) -> sys::muEdges {
        sys::muEdges {
            top: self.top,
            bottom: self.bottom,
            left: self.left,
            right: self.right,
        }
    }
}

impl From<sys::muEdges> for Edges {
    fn from(e: sys::muEdges) -> Self {
        Self {
            top: e.top,
            bottom: e.bottom,
            left: e.left,
            right: e.right,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Into<sys::muVector2> for Vector2 {
    fn into(self) -> sys::muVector2 {
        sys::muVector2 {
            x: self.x,
            y: self.y,
        }
    }
}

impl From<sys::muVector2> for Vector2 {
    fn from(v: sys::muVector2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<f32> for Vector2 {
    fn from(value: f32) -> Self {
        Vector2 { x: value, y: value }
    }
}

impl From<(f32, f32)> for Vector2 {
    fn from((x, y): (f32, f32)) -> Self {
        Vector2 { x, y }
    }
}

impl Add<Vector2> for Vector2 {
    type Output = Vector2;

    fn add(self, rhs: Vector2) -> Self::Output {
        Vector2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Vector2> for Vector2 {
    type Output = Vector2;

    fn sub(self, rhs: Vector2) -> Self::Output {
        Vector2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add<f32> for Vector2 {
    type Output = Vector2;

    fn add(self, rhs: f32) -> Self::Output {
        self + Vector2::from(rhs)
    }
}

impl Sub<f32> for Vector2 {
    type Output = Vector2;

    fn sub(self, rhs: f32) -> Self::Output {
        self - Vector2::from(rhs)
    }
}

impl AddAssign<Vector2> for Vector2 {
    fn add_assign(&mut self, rhs: Vector2) {
        *self = *self + rhs;
    }
}

impl AddAssign<f32> for Vector2 {
    fn add_assign(&mut self, rhs: f32) {
        *self = *self + rhs;
    }
}

impl SubAssign<Vector2> for Vector2 {
    fn sub_assign(&mut self, rhs: Vector2) {
        *self = *self - rhs;
    }
}

impl SubAssign<f32> for Vector2 {
    fn sub_assign(&mut self, rhs: f32) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vector2 {
    type Output = Vector2;

    fn mul(self, rhs: f32) -> Self::Output {
        Vector2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl MulAssign<f32> for Vector2 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlexDirection {
    Row,
    Column,
}

impl Into<sys::muFlexDirection> for FlexDirection {
    fn into(self) -> sys::muFlexDirection {
        match self {
            FlexDirection::Row => sys::muFlexDirection_MUSE_FLEX_ROW,
            FlexDirection::Column => sys::muFlexDirection_MUSE_FLEX_COLUMN,
        }
    }
}

impl From<sys::muFlexDirection> for FlexDirection {
    fn from(f: sys::muFlexDirection) -> Self {
        match f {
            sys::muFlexDirection_MUSE_FLEX_ROW => FlexDirection::Row,
            sys::muFlexDirection_MUSE_FLEX_COLUMN => FlexDirection::Column,
            _ => FlexDirection::Column,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Into<sys::muJustifyContent> for JustifyContent {
    fn into(self) -> sys::muJustifyContent {
        match self {
            JustifyContent::Start => sys::muJustifyContent_MUSE_JUSTIFY_START,
            JustifyContent::Center => sys::muJustifyContent_MUSE_JUSTIFY_CENTER,
            JustifyContent::End => sys::muJustifyContent_MUSE_JUSTIFY_END,
            JustifyContent::SpaceBetween => sys::muJustifyContent_MUSE_JUSTIFY_SPACE_BETWEEN,
            JustifyContent::SpaceAround => sys::muJustifyContent_MUSE_JUSTIFY_SPACE_AROUND,
            JustifyContent::SpaceEvenly => sys::muJustifyContent_MUSE_JUSTIFY_SPACE_EVENLY,
        }
    }
}

impl From<sys::muJustifyContent> for JustifyContent {
    fn from(j: sys::muJustifyContent) -> Self {
        match j {
            sys::muJustifyContent_MUSE_JUSTIFY_START => JustifyContent::Start,
            sys::muJustifyContent_MUSE_JUSTIFY_CENTER => JustifyContent::Center,
            sys::muJustifyContent_MUSE_JUSTIFY_END => JustifyContent::End,
            sys::muJustifyContent_MUSE_JUSTIFY_SPACE_BETWEEN => JustifyContent::SpaceBetween,
            sys::muJustifyContent_MUSE_JUSTIFY_SPACE_AROUND => JustifyContent::SpaceAround,
            sys::muJustifyContent_MUSE_JUSTIFY_SPACE_EVENLY => JustifyContent::SpaceEvenly,
            _ => JustifyContent::Start,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
}

impl Into<sys::muAlignItems> for AlignItems {
    fn into(self) -> sys::muAlignItems {
        match self {
            AlignItems::Start => sys::muAlignItems_MUSE_ALIGN_START,
            AlignItems::Center => sys::muAlignItems_MUSE_ALIGN_CENTER,
            AlignItems::End => sys::muAlignItems_MUSE_ALIGN_END,
            AlignItems::Stretch => sys::muAlignItems_MUSE_ALIGN_STRETCH,
        }
    }
}

impl From<sys::muAlignItems> for AlignItems {
    fn from(a: sys::muAlignItems) -> Self {
        match a {
            sys::muAlignItems_MUSE_ALIGN_START => AlignItems::Start,
            sys::muAlignItems_MUSE_ALIGN_CENTER => AlignItems::Center,
            sys::muAlignItems_MUSE_ALIGN_END => AlignItems::End,
            sys::muAlignItems_MUSE_ALIGN_STRETCH => AlignItems::Stretch,
            _ => AlignItems::Start,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
}

impl Into<sys::muOverflow> for Overflow {
    fn into(self) -> sys::muOverflow {
        match self {
            Overflow::Visible => sys::muOverflow_MU_OVERFLOW_VISIBLE,
            Overflow::Hidden => sys::muOverflow_MU_OVERFLOW_HIDDEN,
            Overflow::Scroll => sys::muOverflow_MU_OVERFLOW_SCROLL,
        }
    }
}

impl From<sys::muOverflow> for Overflow {
    fn from(o: sys::muOverflow) -> Self {
        match o {
            sys::muOverflow_MU_OVERFLOW_VISIBLE => Overflow::Visible,
            sys::muOverflow_MU_OVERFLOW_HIDDEN => Overflow::Hidden,
            sys::muOverflow_MU_OVERFLOW_SCROLL => Overflow::Scroll,
            _ => Overflow::Visible,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PositionStrategy {
    Inflow,
    Absolute {
        top: f32,
        left: f32,
        bottom: f32,
        right: f32,
    },
}

impl Into<sys::muPositionStrategy> for PositionStrategy {
    fn into(self) -> sys::muPositionStrategy {
        match self {
            PositionStrategy::Inflow => sys::muPositionStrategy {
                strategy: sys::muPositionStrategyKind_MUSE_POSITION_STRATEGY_INFLOW,
                __bindgen_anon_1: sys::muPositionStrategy__bindgen_ty_1 {
                    absolute: sys::muPositionStrategy__bindgen_ty_1__bindgen_ty_1 {
                        top: f32::NAN,
                        left: f32::NAN,
                        bottom: f32::NAN,
                        right: f32::NAN,
                    },
                },
            },
            PositionStrategy::Absolute {
                top,
                left,
                bottom,
                right,
            } => sys::muPositionStrategy {
                strategy: sys::muPositionStrategyKind_MUSE_POSITION_STRATEGY_ABSOLUTE,
                __bindgen_anon_1: sys::muPositionStrategy__bindgen_ty_1 {
                    absolute: sys::muPositionStrategy__bindgen_ty_1__bindgen_ty_1 {
                        top,
                        left,
                        bottom,
                        right,
                    },
                },
            },
        }
    }
}

impl From<sys::muPositionStrategy> for PositionStrategy {
    fn from(p: sys::muPositionStrategy) -> Self {
        match p.strategy {
            sys::muPositionStrategyKind_MUSE_POSITION_STRATEGY_INFLOW => PositionStrategy::Inflow,
            sys::muPositionStrategyKind_MUSE_POSITION_STRATEGY_ABSOLUTE => {
                let abs = unsafe { p.__bindgen_anon_1.absolute };
                PositionStrategy::Absolute {
                    top: abs.top,
                    left: abs.left,
                    bottom: abs.bottom,
                    right: abs.right,
                }
            }
            _ => PositionStrategy::Inflow,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    pub width: Size,
    pub height: Size,
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub aspect_ratio: f32,

    pub positioning: PositionStrategy,
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub gap: f32,

    pub padding: Edges,
    pub border: Edges,

    pub overflow: Overflow,
    pub scroll: Vector2,
    pub z_index: i32,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            width: Size::Fit,
            height: Size::Fit,
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            aspect_ratio: 0.0,

            positioning: PositionStrategy::Inflow,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Start,
            gap: 0.0,

            padding: Edges::default(),
            border: Edges::default(),

            overflow: Overflow::Visible,
            scroll: Vector2 { x: 0.0, y: 0.0 },
            z_index: 0,
        }
    }
}

impl Into<sys::muConstraints> for Constraints {
    fn into(self) -> sys::muConstraints {
        sys::muConstraints {
            dimension: sys::muConstraints__bindgen_ty_1 {
                width: self.width.into(),
                height: self.height.into(),
                min_width: self.min_width,
                max_width: self.max_width,
                min_height: self.min_height,
                max_height: self.max_height,
                aspect_ratio: self.aspect_ratio,
            },
            positioning: self.positioning.into(),
            flex_direction: self.flex_direction.into(),
            justify_content: self.justify_content.into(),
            align_items: self.align_items.into(),
            gap: self.gap,
            padding: self.padding.into(),
            border: self.border.into(),
            overflow: self.overflow.into(),
            scroll: self.scroll.into(),
            z_index: self.z_index,
        }
    }
}

impl From<sys::muConstraints> for Constraints {
    fn from(c: sys::muConstraints) -> Self {
        Self {
            width: c.dimension.width.into(),
            height: c.dimension.height.into(),
            min_width: c.dimension.min_width,
            max_width: c.dimension.max_width,
            min_height: c.dimension.min_height,
            max_height: c.dimension.max_height,
            aspect_ratio: c.dimension.aspect_ratio,

            positioning: c.positioning.into(),
            flex_direction: c.flex_direction.into(),
            justify_content: c.justify_content.into(),
            align_items: c.align_items.into(),
            gap: c.gap,

            padding: c.padding.into(),
            border: c.border.into(),

            overflow: c.overflow.into(),
            scroll: c.scroll.into(),
            z_index: c.z_index,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Computed {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl From<sys::muComputed> for Computed {
    fn from(c: sys::muComputed) -> Self {
        Self {
            x: c.x,
            y: c.y,
            w: c.w,
            h: c.h,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

macro_rules! with {
    ($name:ident, $value:expr) => {
        pub fn $name(mut self, value: f32) -> Self {
            self.$name = value;
            self
        }
    };
}

impl Rect {
    with!(x, f32);
    with!(y, f32);
    with!(w, f32);
    with!(h, f32);
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }
}

impl Into<sys::muRect> for Rect {
    fn into(self) -> sys::muRect {
        sys::muRect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

impl From<sys::muRect> for Rect {
    fn from(r: sys::muRect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub color: Color,
    pub font_family: String,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub alignment: Alignment,
    pub wrap: bool,
    pub overflow_wrap: OverflowWrap,
    pub selection_color: Color,
    pub selection_bg: Color,
    pub caret_color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: 20.0,
            color: clr!(black),
            font_family: "system-ui".to_string(),
            font_weight: FontWeight::default(),
            font_style: FontStyle::default(),
            alignment: Alignment::Start,
            wrap: false,
            overflow_wrap: OverflowWrap::Normal,
            selection_color: clr!(white),
            selection_bg: clr!(ll_blue),
            caret_color: clr!(black),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationTarget {
    Padding,
    BackgroundColor,
    Scale,
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

    pub fn corner_radius_precise(mut self, radius: Radius) -> Self {
        self.base_effects.border.radius = radius;
        self
    }

    pub fn z_index(mut self, z_index: i32) -> Self {
        self.base_constraints.z_index = z_index;
        self
    }

    pub fn absolute(mut self, left: f32, top: f32) -> Self {
        self.base_constraints.positioning = PositionStrategy::Absolute {
            left,
            top,
            right: f32::NAN,
            bottom: f32::NAN,
        };
        self
    }

    pub fn position(mut self, positioning: PositionStrategy) -> Self {
        self.base_constraints.positioning = positioning;
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
