// A tiny measured text element.
//
// Zed delegates text shaping and measurement to the platform text system, then
// submits shaped glyphs during paint. This demo keeps the same lifecycle while
// using the existing quad renderer: each lit pixel in a 5x7 bitmap glyph becomes
// a small Quad. It is intentionally simple, but it demonstrates why text needs a
// measured layout node instead of a purely style-driven rectangle.

use crate::color::Hsla;
use crate::element::{Element, InteractionState};
use crate::geometry::{Bounds, Corners, Edges, Point, Size};
use crate::layout::{LayoutEngine, LayoutId};
use crate::scene::{Quad, Scene};
use crate::style::{StyleRefinement, Styled};

const GLYPH_WIDTH: f32 = 5.0;
const GLYPH_HEIGHT: f32 = 7.0;
const GLYPH_GAP: f32 = 1.0;
const SPACE_WIDTH: f32 = 4.0;

pub struct Text {
    content: String,
    style: StyleRefinement,
    layout_id: Option<LayoutId>,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: StyleRefinement::default(),
            layout_id: None,
        }
    }

    fn scale(font_size: f32) -> f32 {
        (font_size / GLYPH_HEIGHT).max(1.0)
    }

    fn measure(content: &str, font_size: f32) -> Size {
        let scale = Self::scale(font_size);
        let mut width = 0.0;
        let mut first = true;

        for ch in content.chars() {
            if !first {
                width += GLYPH_GAP * scale;
            }

            width += if ch == ' ' {
                SPACE_WIDTH * scale
            } else {
                GLYPH_WIDTH * scale
            };
            first = false;
        }

        Size {
            width,
            height: GLYPH_HEIGHT * scale,
        }
    }
}

impl Styled for Text {
    fn style_refinement(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Element for Text {
    fn request_layout(&mut self, layout_engine: &mut LayoutEngine) -> LayoutId {
        let style = self.style.resolve();
        let measured_size = Self::measure(&self.content, style.font_size);
        let id = layout_engine.add_measured_node(style, measured_size);
        self.layout_id = Some(id);
        id
    }

    fn prepaint(
        &mut self,
        _bounds: Bounds,
        _layout_engine: &LayoutEngine,
        _interaction: &mut InteractionState,
    ) {
    }

    fn paint(
        &mut self,
        bounds: Bounds,
        scene: &mut Scene,
        _interaction: &InteractionState,
        _layout_engine: &LayoutEngine,
    ) {
        let style = self.style.resolve();
        let scale = Self::scale(style.font_size);
        let glyph_height = GLYPH_HEIGHT * scale;
        let y_offset = ((bounds.size.height - glyph_height) / 2.0).max(0.0);
        let mut cursor_x = bounds.origin.x;

        for ch in self.content.chars() {
            if ch == ' ' {
                cursor_x += SPACE_WIDTH * scale + GLYPH_GAP * scale;
                continue;
            }

            let glyph = glyph_rows(ch);
            for (row_index, row_bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    let mask = 1 << (4 - col);
                    if row_bits & mask == 0 {
                        continue;
                    }

                    scene.push_quad(Quad {
                        order: 0,
                        bounds: Bounds {
                            origin: Point {
                                x: cursor_x + col as f32 * scale,
                                y: bounds.origin.y + y_offset + row_index as f32 * scale,
                            },
                            size: Size {
                                width: scale,
                                height: scale,
                            },
                        },
                        background: style.text_color,
                        border_color: Hsla::transparent(),
                        corner_radii: Corners::default(),
                        border_widths: Edges::default(),
                    });
                }
            }

            cursor_x += (GLYPH_WIDTH + GLYPH_GAP) * scale;
        }
    }
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '_' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        '!' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
        '?' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100,
        ],
        _ => [
            0b11111, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
    }
}
