// Mirrors gpui::geometry — core geometric types used throughout the framework.
// In Zed, these are generic over units (Pixels, ScaledPixels, DevicePixels).
// We use f32 directly for simplicity.

/// Mirrors gpui::geometry::Point
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Mirrors gpui::geometry::Size
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// Mirrors gpui::geometry::Bounds — origin + size, the layout result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Bounds {
    pub origin: Point,
    pub size: Size,
}

/// Mirrors gpui::geometry::Corners — used for corner_radii
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

/// Mirrors gpui::geometry::Edges — used for margin, padding, border_widths
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };
}

impl Size {
    pub const ZERO: Size = Size {
        width: 0.0,
        height: 0.0,
    };
}

impl Bounds {
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x <= self.origin.x + self.size.width
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.size.height
    }
}

impl Corners {
    pub fn uniform(r: f32) -> Self {
        Self {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        }
    }
}

impl Edges {
    pub fn uniform(v: f32) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    #[allow(dead_code)]
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    #[allow(dead_code)]
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}
