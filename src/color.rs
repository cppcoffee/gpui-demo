// Mirrors gpui::color — HSLA is Zed's primary color representation.
// The shader uses hsla_to_rgba to convert at render time.

/// Mirrors gpui::Hsla — HSLA color used throughout the element tree.
/// Must match the Hsla struct in shaders.metal exactly (#[repr(C)]).
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

/// Mirrors gpui::Rgba — RGBA color for final output.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
#[allow(dead_code)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Same algorithm as hsla_to_rgba in Zed's shaders.metal (line 889).
#[allow(dead_code)]
pub fn hsla_to_rgba(hsla: Hsla) -> Rgba {
    let h = hsla.h * 6.0;
    let s = hsla.s;
    let l = hsla.l;
    let a = hsla.a;

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h >= 0.0 && h < 1.0 {
        (c, x, 0.0)
    } else if h >= 1.0 && h < 2.0 {
        (x, c, 0.0)
    } else if h >= 2.0 && h < 3.0 {
        (0.0, c, x)
    } else if h >= 3.0 && h < 4.0 {
        (0.0, x, c)
    } else if h >= 4.0 && h < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Rgba {
        r: r + m,
        g: g + m,
        b: b + m,
        a,
    }
}

impl Hsla {
    pub fn transparent() -> Self {
        Self {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 0.0,
        }
    }

    /// Create from RGB values (0-255) — convenience for defining colors
    #[allow(dead_code)]
    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        let rgba = Rgba {
            r: r / 255.0,
            g: g / 255.0,
            b: b / 255.0,
            a: 1.0,
        };
        rgba_to_hsla(rgba)
    }
}

#[allow(dead_code)]
fn rgba_to_hsla(rgba: Rgba) -> Hsla {
    let r = rgba.r;
    let g = rgba.g;
    let b = rgba.b;
    let a = rgba.a;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if max == min {
        return Hsla {
            h: 0.0,
            s: 0.0,
            l,
            a,
        };
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    Hsla { h, s, l, a }
}
