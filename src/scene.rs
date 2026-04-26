// Mirrors gpui::scene — the retained-mode display list of rendering primitives.
// In Zed, Scene collects primitives (Quad, Path, Shadow, Underline, StrikeThrough, Sprite)
// during the paint phase, then finish() sorts them by draw order for the renderer.
// We only implement Quad (the most important primitive — used for all rectangles, backgrounds, borders).

use crate::color::Hsla;
use crate::geometry::{Bounds, Corners, Edges};

/// Mirrors gpui::Quad — a rounded rectangle primitive.
/// Must match the Quad_ScaledPixels struct in shaders.metal exactly (#[repr(C)]).
/// The shader uses instanced rendering: one instance per Quad, vertex shader positions
/// from bounds, fragment shader applies corner radius via SDF and renders background + border.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct Quad {
    /// Draw order — higher values draw on top. Set by Scene::push_quad().
    pub order: u32,
    /// Bounds of the quad in pixel coordinates
    pub bounds: Bounds,
    /// Fill color (HSLA — shader converts to RGBA)
    pub background: Hsla,
    /// Border color (HSLA)
    pub border_color: Hsla,
    /// Corner radii (top_left, top_right, bottom_right, bottom_left)
    pub corner_radii: Corners,
    /// Border widths (top, right, bottom, left)
    pub border_widths: Edges,
}

/// Mirrors gpui::Scene — the display list that elements paint into.
/// After all elements have painted, finish() sorts primitives by order for correct rendering.
pub struct Scene {
    pub quads: Vec<Quad>,
    next_order: u32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            quads: Vec::new(),
            next_order: 0,
        }
    }

    pub fn clear(&mut self) {
        self.quads.clear();
        self.next_order = 0;
    }

    /// Push a quad into the scene, assigning it the next draw order.
    /// In Zed, this is WindowContext::paint_quad() which calls scene.push_quad().
    pub fn push_quad(&mut self, quad: Quad) {
        let order = self.next_order;
        self.next_order += 1;
        self.quads.push(Quad { order, ..quad });
    }

    /// Sort quads by draw order (mirrors Scene::finish() in Zed).
    /// In Zed, finish() also merges adjacent same-pipeline batches for efficiency.
    pub fn finish(&mut self) {
        self.quads.sort_by_key(|q| q.order);
    }
}
