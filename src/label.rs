// Mirrors Zed's ui::Label at demo scale: a label owns text plus style and
// renders by delegating to the lower-level text element.

use crate::element::{Element, InteractionState};
use crate::geometry::Bounds;
use crate::layout::{LayoutEngine, LayoutId};
use crate::scene::Scene;
use crate::style::{StyleRefinement, Styled};
use crate::text::Text;

pub struct Label {
    label: String,
    style: StyleRefinement,
    text: Text,
    layout_id: Option<LayoutId>,
}

impl Label {
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();

        Self {
            text: Text::new(label.clone()),
            label,
            style: StyleRefinement::default(),
            layout_id: None,
        }
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.label = text.into();
        self.text = Text::new(self.label.clone());
    }

    fn sync_text_style(&mut self) {
        *self.text.style_refinement() = self.style.clone();
    }
}

impl Styled for Label {
    fn style_refinement(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Element for Label {
    fn request_layout(&mut self, layout_engine: &mut LayoutEngine) -> LayoutId {
        self.sync_text_style();
        let id = self.text.request_layout(layout_engine);
        self.layout_id = Some(id);
        id
    }

    fn prepaint(
        &mut self,
        bounds: Bounds,
        layout_engine: &LayoutEngine,
        interaction: &mut InteractionState,
    ) {
        self.text.prepaint(bounds, layout_engine, interaction);
    }

    fn paint(
        &mut self,
        bounds: Bounds,
        scene: &mut Scene,
        interaction: &InteractionState,
        layout_engine: &LayoutEngine,
    ) {
        self.text.paint(bounds, scene, interaction, layout_engine);
    }
}
