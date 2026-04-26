// Mirrors gpui::element — the core Element trait and interaction state.
// In Zed, every UI component implements Element with a three-phase lifecycle:
//   1. request_layout — register with the layout engine (Taffy), return LayoutId
//   2. prepaint — receive computed bounds, create hitboxes for mouse interaction
//   3. paint — push rendering primitives (Quad, etc.) into the Scene

use crate::geometry::{Bounds, Point};
use crate::layout::{LayoutEngine, LayoutId};
use crate::scene::Scene;

/// Element identifier for hit-testing and interaction state tracking.
pub type ElementId = usize;

/// Mirrors gpui::Interactivity — tracks hover/active state for interactive elements.
pub struct InteractionState {
    pub hovered_id: Option<ElementId>,
    pub active_id: Option<ElementId>,
    pub mouse_position: Point,
    pub hitboxes: Vec<(ElementId, Bounds)>,
}

impl InteractionState {
    pub fn new() -> Self {
        Self {
            hovered_id: None,
            active_id: None,
            mouse_position: Point::ZERO,
            hitboxes: Vec::new(),
        }
    }

    pub fn register_hitbox(&mut self, id: ElementId, bounds: Bounds) {
        self.hitboxes.push((id, bounds));
    }

    pub fn update_hover(&mut self) {
        let pos = self.mouse_position;
        self.hovered_id = None;
        for &(id, bounds) in self.hitboxes.iter().rev() {
            if bounds.contains(pos) {
                self.hovered_id = Some(id);
                break;
            }
        }
    }

    pub fn clear_frame_state(&mut self) {
        self.hitboxes.clear();
    }
}

/// Mirrors gpui::Element — the core trait for anything that can be laid out and painted.
pub trait Element {
    /// Phase 1: Register this element's style with the layout engine.
    fn request_layout(&mut self, layout_engine: &mut LayoutEngine) -> LayoutId;

    /// Phase 2: Receive computed bounds, register hitboxes for interaction.
    fn prepaint(
        &mut self,
        bounds: Bounds,
        layout_engine: &LayoutEngine,
        interaction: &mut InteractionState,
    );

    /// Phase 3: Paint rendering primitives into the Scene.
    fn paint(
        &mut self,
        bounds: Bounds,
        scene: &mut Scene,
        interaction: &InteractionState,
        layout_engine: &LayoutEngine,
    );
}
