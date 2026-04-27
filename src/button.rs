// Mirrors Zed's Button component.

use crate::color::Hsla;
use crate::element::{Element, ElementId, InteractionState};
use crate::geometry::{Bounds, Edges, Size};
use crate::layout::{LayoutEngine, LayoutId};
use crate::scene::{Quad, Scene};
use crate::style::{AlignItems, JustifyContent, StyleRefinement, Styled};
use crate::text::Text;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum ButtonStyle {
    Filled,
    Outlined,
    Subtle,
}

pub struct Button {
    style: StyleRefinement,
    button_style: ButtonStyle,
    element_id: ElementId,
    label: Text,
    label_layout_id: Option<LayoutId>,
    layout_id: Option<LayoutId>,
}

impl Button {
    const DEFAULT_BG: Hsla = Hsla {
        h: 0.58,
        s: 0.75,
        l: 0.50,
        a: 1.0,
    };
    const HOVER_BG: Hsla = Hsla {
        h: 0.58,
        s: 0.75,
        l: 0.58,
        a: 1.0,
    };
    const ACTIVE_BG: Hsla = Hsla {
        h: 0.58,
        s: 0.75,
        l: 0.42,
        a: 1.0,
    };

    pub fn new(id: ElementId, label: impl Into<String>) -> Self {
        let style = StyleRefinement {
            min_size: Some(Size {
                width: 0.0,
                height: 34.0,
            }),
            padding: Some(Edges {
                top: 0.0,
                right: 18.0,
                bottom: 0.0,
                left: 18.0,
            }),
            corner_radii: Some(crate::geometry::Corners::uniform(7.0)),
            align_items: Some(AlignItems::Center),
            justify_content: Some(JustifyContent::Center),
            ..Default::default()
        };

        Self {
            style,
            button_style: ButtonStyle::Filled,
            element_id: id,
            label: Text::new(label).font_size(14.0).text_color(Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.98,
                a: 1.0,
            }),
            label_layout_id: None,
            layout_id: None,
        }
    }

    pub fn button_style(mut self, style: ButtonStyle) -> Self {
        self.button_style = style;
        self
    }

    fn resolved_bg(&self, interaction: &InteractionState) -> Hsla {
        let base_bg = match self.button_style {
            ButtonStyle::Filled => Self::DEFAULT_BG,
            ButtonStyle::Outlined => Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.0,
            },
            ButtonStyle::Subtle => Hsla {
                h: 0.58,
                s: 0.15,
                l: 0.50,
                a: 0.15,
            },
        };

        if interaction.active_id == Some(self.element_id) {
            match self.button_style {
                ButtonStyle::Filled => Self::ACTIVE_BG,
                ButtonStyle::Outlined => Hsla {
                    h: 0.58,
                    s: 0.75,
                    l: 0.42,
                    a: 0.3,
                },
                ButtonStyle::Subtle => Hsla {
                    h: 0.58,
                    s: 0.15,
                    l: 0.42,
                    a: 0.25,
                },
            }
        } else if interaction.hovered_id == Some(self.element_id) {
            match self.button_style {
                ButtonStyle::Filled => Self::HOVER_BG,
                ButtonStyle::Outlined => Hsla {
                    h: 0.58,
                    s: 0.75,
                    l: 0.58,
                    a: 0.3,
                },
                ButtonStyle::Subtle => Hsla {
                    h: 0.58,
                    s: 0.15,
                    l: 0.58,
                    a: 0.25,
                },
            }
        } else {
            base_bg
        }
    }
}

impl Styled for Button {
    fn style_refinement(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Element for Button {
    fn request_layout(&mut self, layout_engine: &mut LayoutEngine) -> LayoutId {
        let label_id = self.label.request_layout(layout_engine);
        self.label_layout_id = Some(label_id);

        let style = self.style.resolve();
        let id = layout_engine.add_node(style, vec![label_id]);
        self.layout_id = Some(id);
        id
    }

    fn prepaint(
        &mut self,
        bounds: Bounds,
        layout_engine: &LayoutEngine,
        interaction: &mut InteractionState,
    ) {
        interaction.register_hitbox(self.element_id, bounds);

        if let Some(label_id) = self.label_layout_id {
            let label_bounds = layout_engine.bounds(label_id);
            self.label
                .prepaint(label_bounds, layout_engine, interaction);
        }
    }

    fn paint(
        &mut self,
        bounds: Bounds,
        scene: &mut Scene,
        interaction: &InteractionState,
        layout_engine: &LayoutEngine,
    ) {
        let bg = self.resolved_bg(interaction);
        let style = self.style.resolve();

        scene.push_quad(Quad {
            order: 0,
            bounds,
            background: bg,
            border_color: match self.button_style {
                ButtonStyle::Outlined => Self::DEFAULT_BG,
                _ => Hsla::transparent(),
            },
            corner_radii: style.corner_radii,
            border_widths: match self.button_style {
                ButtonStyle::Outlined => Edges::uniform(1.0),
                _ => Edges::default(),
            },
        });

        if let Some(label_id) = self.label_layout_id {
            let label_bounds = layout_engine.bounds(label_id);
            self.label
                .paint(label_bounds, scene, interaction, layout_engine);
        }
    }
}
