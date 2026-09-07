use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

use super::sparse_set::NodeId;

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

    pub fn top(mut self, v: f32) -> Self {
        self.top = v;
        self
    }
    pub fn bottom(mut self, v: f32) -> Self {
        self.bottom = v;
        self
    }
    pub fn right(mut self, v: f32) -> Self {
        self.right = v;
        self
    }
    pub fn left(mut self, v: f32) -> Self {
        self.left = v;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
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

impl Add<f32> for Vector2 {
    type Output = Vector2;

    fn add(self, rhs: f32) -> Self::Output {
        self + Vector2::from(rhs)
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlignSelf {
    Auto,
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
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

impl IntoPositionStrategy for PositionStrategy {
    fn into_strategy(self) -> PositionStrategy {
        self
    }
}

impl IntoPositionStrategy for AbsoluteBuilder {
    fn into_strategy(self) -> PositionStrategy {
        self.build()
    }
}

pub trait IntoPositionStrategy {
    fn into_strategy(self) -> PositionStrategy;
}

#[derive(Default)]
pub struct AbsoluteBuilder {
    top: Option<f32>,
    left: Option<f32>,
    bottom: Option<f32>,
    right: Option<f32>,
}

macro_rules! builder_part {
    ($what:ident) => {
        pub fn $what(mut self, value: f32) -> Self {
            self.$what = Some(value);
            self
        }
    };
}

impl AbsoluteBuilder {
    builder_part!(top);
    builder_part!(left);
    builder_part!(bottom);
    builder_part!(right);

    pub fn build(self) -> PositionStrategy {
        PositionStrategy::Absolute {
            top: self.top.unwrap_or(f32::NAN),
            left: self.left.unwrap_or(f32::NAN),
            bottom: self.bottom.unwrap_or(f32::NAN),
            right: self.right.unwrap_or(f32::NAN),
        }
    }
}

impl PositionStrategy {
    pub fn absolute() -> AbsoluteBuilder {
        AbsoluteBuilder::default()
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
    pub flex_wrap: FlexWrap,
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
            flex_wrap: FlexWrap::NoWrap,
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
        if other.flex_shrink != 0.0 {
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Computed {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub content_w: f32,
    pub content_h: f32,
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Hierarchy {
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline_offset: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CachedTextMeasurement {
    pub avail_w: f32,
    pub avail_h: f32,
    pub metrics: TextMetrics,
}

#[derive(Default)]
pub struct TextNode {
    pub content: String,
    pub userdata: Option<Box<dyn std::any::Any>>,
    pub cache: Option<CachedTextMeasurement>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderCommandKind {
    DrawQuad,
    Text,
    ScrollbarV,
    ScrollbarH,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderCommand {
    pub node: NodeId,
    pub kind: RenderCommandKind,
    pub computed: Computed,
    pub clip: Rect,
    pub z_index: i32,
    pub has_clip: bool,
}
