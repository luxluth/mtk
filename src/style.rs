use crate::sys;

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
