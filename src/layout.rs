// Taffy-backed layout engine.
//
// This mirrors the structure of Zed's gpui/src/taffy.rs in a smaller form:
// - LayoutEngine owns a TaffyTree<NodeContext>
// - LayoutId is a transparent wrapper around taffy::NodeId
// - normal nodes and measured leaves are registered separately
// - layout bounds are returned as absolute, window-relative Bounds

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use taffy::{
    TaffyTree,
    geometry::{Rect as TaffyRect, Size as TaffySize},
    style::{
        AvailableSpace as TaffyAvailableSpace, Dimension, LengthPercentage, LengthPercentageAuto,
    },
    tree::NodeId,
};

use crate::geometry::{Bounds, Edges, Point, Size};
use crate::style::{AlignItems, Display, FlexDirection, JustifyContent, Position, Style};

type NodeMeasureFn = Box<dyn FnMut(TaffySize<Option<f32>>, TaffySize<AvailableSpace>) -> Size>;

struct NodeContext {
    measure: NodeMeasureFn,
}

/// Mirrors gpui's TaffyLayoutEngine in a simplified, pixel-only form.
pub struct LayoutEngine {
    taffy: TaffyTree<NodeContext>,
    absolute_layout_bounds: RefCell<HashMap<LayoutId, Bounds>>,
    computed_layouts: HashSet<LayoutId>,
    layout_bounds_scratch_space: Vec<LayoutId>,
}

const EXPECT_MESSAGE: &str = "layout errors should be avoided by construction";

impl LayoutEngine {
    pub fn new() -> Self {
        let mut taffy = TaffyTree::new();
        taffy.enable_rounding();

        Self {
            taffy,
            absolute_layout_bounds: RefCell::new(HashMap::new()),
            computed_layouts: HashSet::new(),
            layout_bounds_scratch_space: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.taffy.clear();
        self.absolute_layout_bounds.borrow_mut().clear();
        self.computed_layouts.clear();
        self.layout_bounds_scratch_space.clear();
    }

    /// Register a node with its style and children. Returns a LayoutId for later queries.
    /// Mirrors gpui's request_layout(), while preserving this demo's existing API.
    pub fn add_node(&mut self, style: Style, children: Vec<LayoutId>) -> LayoutId {
        self.request_layout(style, &children)
    }

    /// Register a leaf node with intrinsic content size.
    /// Mirrors gpui's request_measured_layout(), used by text and other measured content.
    pub fn add_measured_node(&mut self, style: Style, measured_size: Size) -> LayoutId {
        self.request_measured_layout(style, move |known_dimensions, _available_space| Size {
            width: known_dimensions.width.unwrap_or(measured_size.width),
            height: known_dimensions.height.unwrap_or(measured_size.height),
        })
    }

    pub fn request_layout(&mut self, style: Style, children: &[LayoutId]) -> LayoutId {
        let taffy_style = style.to_taffy();

        if children.is_empty() {
            self.taffy
                .new_leaf(taffy_style)
                .expect(EXPECT_MESSAGE)
                .into()
        } else {
            self.taffy
                // This is safe because LayoutId is repr(transparent) to taffy::tree::NodeId.
                .new_with_children(taffy_style, LayoutId::to_taffy_slice(children))
                .expect(EXPECT_MESSAGE)
                .into()
        }
    }

    pub fn request_measured_layout(
        &mut self,
        style: Style,
        measure: impl FnMut(TaffySize<Option<f32>>, TaffySize<AvailableSpace>) -> Size + 'static,
    ) -> LayoutId {
        let taffy_style = style.to_taffy();

        self.taffy
            .new_leaf_with_context(
                taffy_style,
                NodeContext {
                    measure: Box::new(measure),
                },
            )
            .expect(EXPECT_MESSAGE)
            .into()
    }

    /// Compute layout for the entire tree, starting from root.
    pub fn compute(&mut self, root: LayoutId, available_size: Size) {
        self.compute_layout(root, definite_available_size(available_size));
    }

    pub fn compute_layout(&mut self, id: LayoutId, available_space: TaffySize<AvailableSpace>) {
        if !self.computed_layouts.insert(id) {
            let stack = &mut self.layout_bounds_scratch_space;
            stack.push(id);
            while let Some(id) = stack.pop() {
                self.absolute_layout_bounds.borrow_mut().remove(&id);
                stack.extend(
                    self.taffy
                        .children(id.into())
                        .expect(EXPECT_MESSAGE)
                        .into_iter()
                        .map(LayoutId::from),
                );
            }
        }

        let available_space = TaffySize {
            width: available_space.width.into(),
            height: available_space.height.into(),
        };

        self.taffy
            .compute_layout_with_measure(
                id.into(),
                available_space,
                |known_dimensions, available_space, _id, node_context, _style| {
                    let Some(node_context) = node_context else {
                        return TaffySize::ZERO;
                    };

                    let available_space = TaffySize {
                        width: available_space.width.into(),
                        height: available_space.height.into(),
                    };
                    let measured = (node_context.measure)(known_dimensions, available_space);

                    TaffySize {
                        width: measured.width,
                        height: measured.height,
                    }
                },
            )
            .expect(EXPECT_MESSAGE);
    }

    /// Get the computed bounds for a node, relative to the window.
    pub fn bounds(&self, id: LayoutId) -> Bounds {
        self.layout_bounds(id)
    }

    fn layout_bounds(&self, id: LayoutId) -> Bounds {
        if let Some(bounds) = self.absolute_layout_bounds.borrow().get(&id).copied() {
            return bounds;
        }

        let layout = self.taffy.layout(id.into()).expect(EXPECT_MESSAGE);
        let mut bounds = Bounds {
            origin: Point {
                x: layout.location.x,
                y: layout.location.y,
            },
            size: Size {
                width: layout.size.width,
                height: layout.size.height,
            },
        };

        if let Some(parent_id) = self.taffy.parent(id.0) {
            let parent_bounds = self.layout_bounds(parent_id.into());
            bounds.origin.x += parent_bounds.origin.x;
            bounds.origin.y += parent_bounds.origin.y;
        }

        self.absolute_layout_bounds.borrow_mut().insert(id, bounds);
        bounds
    }
}

/// A unique identifier for a layout node, generated when requesting a layout from Taffy.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[repr(transparent)]
pub struct LayoutId(NodeId);

impl LayoutId {
    fn to_taffy_slice(node_ids: &[Self]) -> &[taffy::NodeId] {
        // SAFETY: LayoutId is repr(transparent) to taffy::tree::NodeId.
        unsafe { std::mem::transmute::<&[LayoutId], &[taffy::NodeId]>(node_ids) }
    }
}

impl From<NodeId> for LayoutId {
    fn from(node_id: NodeId) -> Self {
        Self(node_id)
    }
}

impl From<LayoutId> for NodeId {
    fn from(layout_id: LayoutId) -> NodeId {
        layout_id.0
    }
}

/// The space available for an element to be laid out in.
#[derive(Copy, Clone, Default, Debug, PartialEq)]
#[allow(dead_code)]
pub enum AvailableSpace {
    Definite(f32),
    #[default]
    MinContent,
    MaxContent,
}

impl From<AvailableSpace> for TaffyAvailableSpace {
    fn from(space: AvailableSpace) -> Self {
        match space {
            AvailableSpace::Definite(value) => Self::Definite(value),
            AvailableSpace::MinContent => Self::MinContent,
            AvailableSpace::MaxContent => Self::MaxContent,
        }
    }
}

impl From<TaffyAvailableSpace> for AvailableSpace {
    fn from(space: TaffyAvailableSpace) -> Self {
        match space {
            TaffyAvailableSpace::Definite(value) => Self::Definite(value),
            TaffyAvailableSpace::MinContent => Self::MinContent,
            TaffyAvailableSpace::MaxContent => Self::MaxContent,
        }
    }
}

trait ToTaffy<Output> {
    fn to_taffy(&self) -> Output;
}

impl ToTaffy<taffy::style::Style> for Style {
    fn to_taffy(&self) -> taffy::style::Style {
        taffy::style::Style {
            display: self.display.into(),
            position: self.position.into(),
            inset: self.inset.to_taffy(),
            size: self.size.to_taffy(),
            min_size: self.min_size.to_taffy(),
            max_size: self.max_size.to_taffy(),
            margin: self.margin.to_taffy(),
            padding: self.padding.to_taffy(),
            border: self.border_widths.to_taffy(),
            align_items: Some(self.align_items.into()),
            justify_content: Some(self.justify_content.into()),
            flex_direction: self.flex_direction.into(),
            ..Default::default()
        }
    }
}

impl ToTaffy<TaffySize<Dimension>> for Size {
    fn to_taffy(&self) -> TaffySize<Dimension> {
        TaffySize {
            width: dimension(self.width),
            height: dimension(self.height),
        }
    }
}

impl ToTaffy<TaffyRect<LengthPercentageAuto>> for Edges {
    fn to_taffy(&self) -> TaffyRect<LengthPercentageAuto> {
        TaffyRect {
            top: LengthPercentageAuto::length(self.top),
            right: LengthPercentageAuto::length(self.right),
            bottom: LengthPercentageAuto::length(self.bottom),
            left: LengthPercentageAuto::length(self.left),
        }
    }
}

impl ToTaffy<TaffyRect<LengthPercentage>> for Edges {
    fn to_taffy(&self) -> TaffyRect<LengthPercentage> {
        TaffyRect {
            top: LengthPercentage::length(self.top),
            right: LengthPercentage::length(self.right),
            bottom: LengthPercentage::length(self.bottom),
            left: LengthPercentage::length(self.left),
        }
    }
}

impl From<Display> for taffy::style::Display {
    fn from(value: Display) -> Self {
        match value {
            Display::Flex => Self::Flex,
            Display::None => Self::None,
        }
    }
}

impl From<Position> for taffy::style::Position {
    fn from(value: Position) -> Self {
        match value {
            Position::Relative => Self::Relative,
            Position::Absolute => Self::Absolute,
        }
    }
}

impl From<FlexDirection> for taffy::style::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => Self::Row,
            FlexDirection::Column => Self::Column,
        }
    }
}

impl From<AlignItems> for taffy::style::AlignItems {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::Start => Self::Start,
            AlignItems::Center => Self::Center,
            AlignItems::End => Self::End,
            AlignItems::Stretch => Self::Stretch,
        }
    }
}

impl From<JustifyContent> for taffy::style::JustifyContent {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::Start => Self::Start,
            JustifyContent::Center => Self::Center,
            JustifyContent::End => Self::End,
            JustifyContent::SpaceBetween => Self::SpaceBetween,
        }
    }
}

fn definite_available_size(size: Size) -> TaffySize<AvailableSpace> {
    TaffySize {
        width: AvailableSpace::Definite(size.width),
        height: AvailableSpace::Definite(size.height),
    }
}

fn dimension(value: f32) -> Dimension {
    if is_auto_axis(value) {
        Dimension::auto()
    } else {
        Dimension::length(value)
    }
}

fn is_auto_axis(value: f32) -> bool {
    !value.is_finite() || value == f32::MAX
}
