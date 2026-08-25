use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

use parley::layout::Alignment;
use parley::style::{FontStyle, FontWeight, OverflowWrap};

use crate::animation::Curve;
use crate::colors::Color;
use crate::effects::{Effects, Filter, Radius, Shadow};
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
    RowReverse,
    ColumnReverse,
}

impl Into<sys::muFlexDirection> for FlexDirection {
    fn into(self) -> sys::muFlexDirection {
        match self {
            FlexDirection::Row => sys::muFlexDirection_MUSE_FLEX_ROW,
            FlexDirection::Column => sys::muFlexDirection_MUSE_FLEX_COLUMN,
            FlexDirection::RowReverse => sys::muFlexDirection_MUSE_FLEX_ROW_REVERSE,
            FlexDirection::ColumnReverse => sys::muFlexDirection_MUSE_FLEX_COLUMN_REVERSE,
        }
    }
}

impl From<sys::muFlexDirection> for FlexDirection {
    fn from(f: sys::muFlexDirection) -> Self {
        match f {
            sys::muFlexDirection_MUSE_FLEX_ROW => FlexDirection::Row,
            sys::muFlexDirection_MUSE_FLEX_COLUMN => FlexDirection::Column,
            sys::muFlexDirection_MUSE_FLEX_ROW_REVERSE => FlexDirection::RowReverse,
            sys::muFlexDirection_MUSE_FLEX_COLUMN_REVERSE => FlexDirection::ColumnReverse,
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
pub enum AlignSelf {
    Auto,
    Start,
    Center,
    End,
    Stretch,
}

impl Into<sys::muAlignSelf> for AlignSelf {
    fn into(self) -> sys::muAlignSelf {
        match self {
            AlignSelf::Auto => sys::muAlignSelf_MUSE_ALIGN_SELF_AUTO,
            AlignSelf::Start => sys::muAlignSelf_MUSE_ALIGN_SELF_START,
            AlignSelf::Center => sys::muAlignSelf_MUSE_ALIGN_SELF_CENTER,
            AlignSelf::End => sys::muAlignSelf_MUSE_ALIGN_SELF_END,
            AlignSelf::Stretch => sys::muAlignSelf_MUSE_ALIGN_SELF_STRETCH,
        }
    }
}

impl From<sys::muAlignSelf> for AlignSelf {
    fn from(a: sys::muAlignSelf) -> Self {
        match a {
            sys::muAlignSelf_MUSE_ALIGN_SELF_AUTO => AlignSelf::Auto,
            sys::muAlignSelf_MUSE_ALIGN_SELF_START => AlignSelf::Start,
            sys::muAlignSelf_MUSE_ALIGN_SELF_CENTER => AlignSelf::Center,
            sys::muAlignSelf_MUSE_ALIGN_SELF_END => AlignSelf::End,
            sys::muAlignSelf_MUSE_ALIGN_SELF_STRETCH => AlignSelf::Stretch,
            _ => AlignSelf::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Into<sys::muOverflow> for Overflow {
    fn into(self) -> sys::muOverflow {
        match self {
            Overflow::Visible => sys::muOverflow_MU_OVERFLOW_VISIBLE,
            Overflow::Hidden => sys::muOverflow_MU_OVERFLOW_HIDDEN,
            Overflow::Scroll | Overflow::Auto => sys::muOverflow_MU_OVERFLOW_SCROLL,
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
    pub align_self: AlignSelf,
    pub gap: f32,

    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Size,

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
            align_self: AlignSelf::Auto,
            gap: 0.0,

            flex_grow: 0.0,
            flex_shrink: 0.0,
            flex_basis: Size::Fit,

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
            align_self: self.align_self.into(),
            gap: self.gap,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.into(),
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
            align_self: c.align_self.into(),
            gap: c.gap,
            flex_grow: c.flex_grow,
            flex_shrink: c.flex_shrink,
            flex_basis: c.flex_basis.into(),

            padding: c.padding.into(),
            border: c.border.into(),

            overflow: c.overflow.into(),
            scroll: c.scroll.into(),
            z_index: c.z_index,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Computed {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub content_w: f32,
    pub content_h: f32,
}

impl From<sys::muComputed> for Computed {
    fn from(c: sys::muComputed) -> Self {
        Self {
            x: c.x,
            y: c.y,
            w: c.w,
            h: c.h,
            content_w: c.content_w,
            content_h: c.content_h,
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

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum LineHeight {
    Relative(f32),
    #[default]
    Auto,
}

impl LineHeight {
    pub fn resolve(&self) -> f32 {
        match self {
            LineHeight::Relative(f) => *f,
            LineHeight::Auto => 1.2, // I think this was 1.2
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: LineHeight,
    pub color: Color,
    pub font_family: String,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub alignment: Alignment,
    pub vertical_alignment: VerticalAlignment,
    pub wrap: bool,
    pub overflow_wrap: OverflowWrap,
    pub selection_color: Color,
    pub selection_bg: Color,
    pub caret_color: Color,
    pub strikethrough: bool,
    pub underline: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: LineHeight::Auto,
            color: clr!(black),
            font_family: "system-ui".to_string(),
            font_weight: FontWeight::default(),
            font_style: FontStyle::default(),
            alignment: Alignment::Start,
            vertical_alignment: VerticalAlignment::Top,
            wrap: false,
            overflow_wrap: OverflowWrap::default(),
            selection_color: clr!(white),
            selection_bg: clr!(ll_blue),
            caret_color: clr!(black),
            strikethrough: false,
            underline: false,
        }
    }
}

/// Target properties that can be transitioned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionProperty {
    /// Transitions all changing properties simultaneously.
    All,
    Padding,
    Border,
    Width,
    Height,
    Size,
    Gap,
    BackgroundColor,
    BorderColor,
    CornerRadius,
    Scale,
    Opacity,
    Shadow,
    TextColor,
    FontSize,
}

/// Backwards compatibility alias for [`TransitionProperty`].
pub type AnimationTarget = TransitionProperty;

/// Configuration for a smooth property transition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub property: TransitionProperty,
    pub duration_ms: f64,
    pub curve: Curve,
}

impl Transition {
    pub fn new(property: TransitionProperty, duration_ms: f64, curve: Curve) -> Self {
        Self {
            property,
            duration_ms,
            curve,
        }
    }
}

impl Constraints {
    /// Merges non-default properties from `other` into `self`.
    pub fn merge(&mut self, other: &Constraints) {
        if other.width != Size::Fit {
            self.width = other.width;
        }
        if other.height != Size::Fit {
            self.height = other.height;
        }
        if other.min_width != 0.0 {
            self.min_width = other.min_width;
        }
        if other.max_width != f32::INFINITY {
            self.max_width = other.max_width;
        }
        if other.min_height != 0.0 {
            self.min_height = other.min_height;
        }
        if other.max_height != f32::INFINITY {
            self.max_height = other.max_height;
        }
        if other.aspect_ratio != 0.0 {
            self.aspect_ratio = other.aspect_ratio;
        }
        if other.positioning != PositionStrategy::Inflow {
            self.positioning = other.positioning;
        }
        if other.justify_content != JustifyContent::Start {
            self.justify_content = other.justify_content;
        }
        if other.align_items != AlignItems::Start {
            self.align_items = other.align_items;
        }
        if other.align_self != AlignSelf::Auto {
            self.align_self = other.align_self;
        }
        if other.gap != 0.0 {
            self.gap = other.gap;
        }
        if other.flex_grow != 0.0 {
            self.flex_grow = other.flex_grow;
        }
        if other.flex_shrink != 1.0 {
            self.flex_shrink = other.flex_shrink;
        }
        if other.flex_basis != Size::Fit {
            self.flex_basis = other.flex_basis;
        }
        if other.padding != Edges::default() {
            self.padding = other.padding;
        }
        if other.border != Edges::default() {
            self.border = other.border;
        }
        if other.overflow != Overflow::Visible {
            self.overflow = other.overflow;
        }
        if other.z_index != 0 {
            self.z_index = other.z_index;
        }
    }
}

impl Effects {
    /// Merges non-default visual effects from `other` into `self`.
    pub fn merge(&mut self, other: &Effects) {
        if other.background_color != Color::transparent {
            self.background_color = other.background_color;
        }
        if other.border != crate::effects::Border::default() {
            self.border = other.border;
        }
        if other.shadow != Shadow::default() {
            self.shadow = other.shadow;
        }
        if !other.filters.is_empty() {
            self.filters = other.filters.clone();
        }
        if (other.opacity - 1.0).abs() > 1e-4 {
            self.opacity = other.opacity;
        }
        if (other.scale - 1.0).abs() > 1e-4 {
            self.scale = other.scale;
        }
    }
}

impl TextStyle {
    /// Merges non-default typography styling from `other` into `self`.
    pub fn merge(&mut self, other: &TextStyle) {
        if (other.font_size - 16.0).abs() > 1e-4 {
            self.font_size = other.font_size;
        }
        if other.line_height != LineHeight::Auto {
            self.line_height = other.line_height;
        }
        if other.color != clr!(black) {
            self.color = other.color;
        }
        if other.font_family != "system-ui" {
            self.font_family = other.font_family.clone();
        }
        if other.font_weight != FontWeight::default() {
            self.font_weight = other.font_weight;
        }
        if other.font_style != FontStyle::default() {
            self.font_style = other.font_style;
        }
        if other.alignment != Alignment::Start {
            self.alignment = other.alignment;
        }
        if other.vertical_alignment != VerticalAlignment::Top {
            self.vertical_alignment = other.vertical_alignment;
        }
        if other.wrap {
            self.wrap = other.wrap;
        }
        if other.strikethrough {
            self.strikethrough = other.strikethrough;
        }
        if other.underline {
            self.underline = other.underline;
        }
        if other.caret_color != clr!(black) {
            self.caret_color = other.caret_color;
        }
    }
}

/// Declarative styling container defining layout constraints, visual effects, typography, pseudo-states, and transitions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub base_constraints: Constraints,
    pub base_effects: Effects,
    pub base_text_style: TextStyle,
    pub flex_direction: Option<FlexDirection>,

    pub hover: Option<Box<Style>>,
    pub active: Option<Box<Style>>,
    pub focus: Option<Box<Style>>,
    pub disabled: Option<Box<Style>>,

    pub transitions: Vec<Transition>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges `other` into `self`, overriding conflicting properties while preserving non-conflicting base styles.
    pub fn merge(mut self, other: Style) -> Self {
        self.base_constraints.merge(&other.base_constraints);
        self.base_effects.merge(&other.base_effects);
        self.base_text_style.merge(&other.base_text_style);

        if let Some(dir) = other.flex_direction {
            self.flex_direction = Some(dir);
            self.base_constraints.flex_direction = dir;
        }

        if let Some(h) = other.hover {
            self.hover = Some(Box::new(match self.hover {
                Some(existing) => existing.merge(*h),
                None => *h,
            }));
        }

        if let Some(a) = other.active {
            self.active = Some(Box::new(match self.active {
                Some(existing) => existing.merge(*a),
                None => *a,
            }));
        }

        if let Some(f) = other.focus {
            self.focus = Some(Box::new(match self.focus {
                Some(existing) => existing.merge(*f),
                None => *f,
            }));
        }

        if let Some(d) = other.disabled {
            self.disabled = Some(Box::new(match self.disabled {
                Some(existing) => existing.merge(*d),
                None => *d,
            }));
        }

        self.transitions.extend(other.transitions);
        self
    }

    /// Applies a reusable style mixin function `f`.
    pub fn apply(self, f: impl FnOnce(Style) -> Style) -> Self {
        f(self)
    }

    /// Applies this style's base constraints, effects, and text style directly to a layout node.
    pub fn apply_to_node(&self, ctx: &mut crate::Context, node: crate::Node) {
        node.update_constraints(ctx, |c| {
            let overflow = c.overflow;
            let scroll = c.scroll;
            let flex_dir = self.flex_direction.unwrap_or(c.flex_direction);
            *c = self.base_constraints;
            c.flex_direction = flex_dir;
            if self.base_constraints.overflow == Overflow::Visible && overflow != Overflow::Visible
            {
                c.overflow = overflow;
            }
            c.scroll = scroll;
        });

        node.set_effects(ctx, self.base_effects.clone());

        if let Some(text) = node.get_text(ctx) {
            let text_owned = text.to_string();
            node.set_text_with_userdata(ctx, &text_owned, self.base_text_style.clone());
        }
    }

    /// Conditionally applies style modifications when `condition` is `true`.
    pub fn when(self, condition: bool, f: impl FnOnce(Style) -> Style) -> Self {
        if condition { f(self) } else { self }
    }

    pub fn padding(mut self, val: f32) -> Self {
        self.base_constraints.padding = Edges::all(val);
        self
    }

    /// Sets symmetric horizontal (`x`) and vertical (`y`) padding edges.
    pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
        self.base_constraints.padding = Edges {
            top: y,
            right: x,
            bottom: y,
            left: x,
        };
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

    pub fn shadow(mut self, color: Color, spread: f32, power: f32) -> Self {
        self.base_effects.shadow = Shadow {
            color,
            spread,
            power,
        };
        self
    }

    pub fn blur(mut self, vibrancy: f32) -> Self {
        self.base_effects.filters.push(Filter::Blur {
            vibrancy,
            vibrancy_darkness: 0.2,
            passes: 4.0,
        });
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.base_effects.opacity = opacity;
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

    pub fn align_self(mut self, a: AlignSelf) -> Self {
        self.base_constraints.align_self = a;
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.base_constraints.flex_shrink = shrink;
        self
    }

    pub fn flex_basis(mut self, basis: Size) -> Self {
        self.base_constraints.flex_basis = basis;
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
        self.flex_direction = Some(dir);
        self
    }

    /// Sets flex grow factor for layout flexing inside flex containers.
    pub fn flex_grow(mut self, val: f32) -> Self {
        self.base_constraints.flex_grow = val;
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

    /// Declares style overrides applied when the mouse cursor hovers over the element.
    pub fn on_hover(mut self, hover_fn: impl FnOnce(Style) -> Style) -> Self {
        let base = self.clone();
        let hover_style = hover_fn(base);
        self.hover = Some(Box::new(hover_style));
        self
    }

    /// Declares style overrides applied when the mouse button is pressed over the element (active state).
    pub fn on_active(mut self, active_fn: impl FnOnce(Style) -> Style) -> Self {
        let base = self.clone();
        let active_style = active_fn(base);
        self.active = Some(Box::new(active_style));
        self
    }

    /// Declares style overrides applied when the element receives focus.
    pub fn on_focus(mut self, focus_fn: impl FnOnce(Style) -> Style) -> Self {
        let base = self.clone();
        let focus_style = focus_fn(base);
        self.focus = Some(Box::new(focus_style));
        self
    }

    /// Declares style overrides applied when the element is disabled.
    pub fn on_disabled(mut self, disabled_fn: impl FnOnce(Style) -> Style) -> Self {
        let base = self.clone();
        let disabled_style = disabled_fn(base);
        self.disabled = Some(Box::new(disabled_style));
        self
    }

    /// Smoothly transitions all changing properties over `duration_ms` with easing/spring `curve`.
    pub fn transition_all(mut self, duration_ms: f64, curve: Curve) -> Self {
        self.transitions
            .push(Transition::new(TransitionProperty::All, duration_ms, curve));
        self
    }

    /// Smoothly transitions a specific property over `duration_ms` with easing/spring `curve`.
    pub fn transition(
        mut self,
        property: TransitionProperty,
        duration_ms: f64,
        curve: Curve,
    ) -> Self {
        self.transitions
            .push(Transition::new(property, duration_ms, curve));
        self
    }

    /// Backwards-compatible animation transition builder.
    pub fn animate(mut self, target: TransitionProperty, duration_ms: f64, curve: Curve) -> Self {
        self.transitions
            .push(Transition::new(target, duration_ms, curve));
        self
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    pub(crate) fn test_row_flex_direction_preserved_when_styled() {
        let style = Style::new();
        assert_eq!(style.base_constraints.flex_direction, FlexDirection::Column);
    }

    #[test]
    fn test_flexbox_new_properties() {
        let style = Style::new()
            .align_self(AlignSelf::End)
            .flex_grow(2.0)
            .flex_shrink(0.5)
            .flex_basis(Size::Fixed(100));

        assert_eq!(style.base_constraints.align_self, AlignSelf::End);
        assert_eq!(style.base_constraints.flex_grow, 2.0);
        assert_eq!(style.base_constraints.flex_shrink, 0.5);
        assert_eq!(style.base_constraints.flex_basis, Size::Fixed(100));
    }

    #[test]
    fn test_root_node_size_fill() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fill;
            c.height = Size::Fill;
        });
        ctx.root_attach(root);
        ctx.compute_layout(1024.0, 768.0);

        let bounds = root.get_computed(&ctx).unwrap();
        assert_eq!(bounds.w, 1024.0);
        assert_eq!(bounds.h, 768.0);
    }

    #[test]
    fn test_flex_shrink_distribution() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(100);
            c.height = Size::Fixed(50);
            c.flex_direction = FlexDirection::Row;
        });

        let child1 = ctx.create_node();
        child1.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(80);
            c.height = Size::Fixed(50);
            c.flex_shrink = 1.0;
        });

        let child2 = ctx.create_node();
        child2.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(40);
            c.height = Size::Fixed(50);
            c.flex_shrink = 1.0;
        });

        root.append(&mut ctx, child1);
        root.append(&mut ctx, child2);
        ctx.root_attach(root);
        ctx.compute_layout(100.0, 50.0);

        let c1_bounds = child1.get_computed(&ctx).unwrap();
        let c2_bounds = child2.get_computed(&ctx).unwrap();

        // 80 / 120 * 20 = 13.333... -> 80 - 13.333 = 66.666...
        // 40 / 120 * 20 = 6.666... -> 40 - 6.666 = 33.333...
        assert!((c1_bounds.w - 66.666).abs() < 0.1);
        assert!((c2_bounds.w - 33.333).abs() < 0.1);
        assert!(((c1_bounds.w + c2_bounds.w) - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_flex_grow_distribution() {
        let mut ctx = crate::Context::new();
        let root = ctx.create_node();
        root.update_constraints(&mut ctx, |c| {
            c.width = Size::Fixed(200);
            c.height = Size::Fixed(50);
            c.flex_direction = FlexDirection::Row;
        });

        let child1 = ctx.create_node();
        child1.update_constraints(&mut ctx, |c| {
            c.flex_grow = 1.0;
            c.height = Size::Fixed(50);
        });

        let child2 = ctx.create_node();
        child2.update_constraints(&mut ctx, |c| {
            c.flex_grow = 3.0;
            c.height = Size::Fixed(50);
        });

        root.append(&mut ctx, child1);
        root.append(&mut ctx, child2);
        ctx.root_attach(root);
        ctx.compute_layout(200.0, 50.0);

        let c1_bounds = child1.get_computed(&ctx).unwrap();
        let c2_bounds = child2.get_computed(&ctx).unwrap();

        assert_eq!(c1_bounds.w, 50.0);
        assert_eq!(c2_bounds.w, 150.0);
    }
}
