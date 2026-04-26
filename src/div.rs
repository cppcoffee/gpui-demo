// Mirrors gpui::elements::div — the workhorse container element.

use crate::element::{Element, ElementId, InteractionState};
use crate::geometry::Bounds;
use crate::layout::{LayoutEngine, LayoutId};
use crate::scene::{Quad, Scene};
use crate::style::{Style, StyleRefinement, Styled};

pub struct Div {
    pub style: StyleRefinement,
    pub children: Vec<Box<dyn Element>>,
    pub element_id: Option<ElementId>,
    pub hover_style: Option<StyleRefinement>,
    pub active_style: Option<StyleRefinement>,
    #[allow(dead_code)]
    pub on_click: Option<Box<dyn FnMut()>>,
    layout_id: Option<LayoutId>,
    child_layout_ids: Vec<LayoutId>,
}

impl Div {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
            element_id: None,
            hover_style: None,
            active_style: None,
            on_click: None,
            layout_id: None,
            child_layout_ids: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn id(mut self, id: ElementId) -> Self {
        self.element_id = Some(id);
        self
    }

    pub fn child(mut self, child: impl Element + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    #[allow(dead_code)]
    pub fn hover_style(mut self, style: StyleRefinement) -> Self {
        self.hover_style = Some(style);
        self
    }

    #[allow(dead_code)]
    pub fn active_style(mut self, style: StyleRefinement) -> Self {
        self.active_style = Some(style);
        self
    }

    #[allow(dead_code)]
    pub fn on_click(mut self, handler: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    fn resolved_style(&self, interaction: &InteractionState) -> Style {
        let mut style = self.style.resolve();

        if let (Some(hover), Some(id)) = (&self.hover_style, self.element_id) {
            if interaction.hovered_id == Some(id) {
                let hover_resolved = hover.resolve();
                if hover_resolved.background.a > 0.0 {
                    style.background = hover_resolved.background;
                }
                if hover_resolved.border_color.a > 0.0 {
                    style.border_color = hover_resolved.border_color;
                }
            }
        }

        if let (Some(active), Some(id)) = (&self.active_style, self.element_id) {
            if interaction.active_id == Some(id) {
                let active_resolved = active.resolve();
                if active_resolved.background.a > 0.0 {
                    style.background = active_resolved.background;
                }
                if active_resolved.border_color.a > 0.0 {
                    style.border_color = active_resolved.border_color;
                }
            }
        }

        style
    }
}

impl Styled for Div {
    fn style_refinement(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Element for Div {
    fn request_layout(&mut self, layout_engine: &mut LayoutEngine) -> LayoutId {
        let child_ids: Vec<LayoutId> = self
            .children
            .iter_mut()
            .map(|child| child.request_layout(layout_engine))
            .collect();

        self.child_layout_ids = child_ids.clone();

        let style = self.style.resolve();
        let id = layout_engine.add_node(style, child_ids);
        self.layout_id = Some(id);
        id
    }

    fn prepaint(
        &mut self,
        bounds: Bounds,
        layout_engine: &LayoutEngine,
        interaction: &mut InteractionState,
    ) {
        if let Some(id) = self.element_id {
            interaction.register_hitbox(id, bounds);
        }

        // Prepaint children with their computed bounds from the layout engine
        for (i, child) in self.children.iter_mut().enumerate() {
            if i < self.child_layout_ids.len() {
                let child_bounds = layout_engine.bounds(self.child_layout_ids[i]);
                child.prepaint(child_bounds, layout_engine, interaction);
            }
        }
    }

    fn paint(
        &mut self,
        bounds: Bounds,
        scene: &mut Scene,
        interaction: &InteractionState,
        layout_engine: &LayoutEngine,
    ) {
        let style = self.resolved_style(interaction);

        if style.background.a > 0.0 || style.border_color.a > 0.0 {
            scene.push_quad(Quad {
                order: 0,
                bounds,
                background: style.background,
                border_color: style.border_color,
                corner_radii: style.corner_radii,
                border_widths: style.border_widths,
            });
        }

        // Paint children with their computed bounds
        for (i, child) in self.children.iter_mut().enumerate() {
            if i < self.child_layout_ids.len() {
                let child_bounds = layout_engine.bounds(self.child_layout_ids[i]);
                child.paint(child_bounds, scene, interaction, layout_engine);
            }
        }
    }
}
