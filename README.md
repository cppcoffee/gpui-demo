# GPUI Demo

A minimal GPU-accelerated UI framework demo that mirrors the core architecture patterns of [Zed](https://github.com/zed-industries/zed)'s GPUI, built from scratch with Rust and Metal.

## What It Does

Opens a macOS window with a centered interactive button rendered entirely via Metal GPU instanced rendering, with an FPS counter overlay. The button responds to hover and click states, demonstrating the full UI lifecycle: layout -> prepaint -> paint -> render.

## Architecture

This project re-implements Zed's GPUI architecture patterns in a minimal, self-contained way:

### Rendering Pipeline

```
Element Tree  ->  Scene (display list)  ->  Metal Renderer  ->  Screen
```

1. **Element tree** is built each frame (Div, Button, Text)
2. **Scene** collects rendering primitives (Quad) with draw order
3. **MetalRenderer** draws quads via instanced rendering (`draw_primitives_instanced`)
4. **CVDisplayLink** drives the render loop at display refresh rate

### Three-Phase Element Lifecycle

Mirrors Zed's `request_layout` -> `prepaint` -> `paint` pattern:

| Phase | Purpose |
|-------|---------|
| `request_layout` | Register style with layout engine, return `LayoutId` |
| `prepaint` | Receive computed bounds, register hitboxes for interaction |
| `paint` | Push rendering primitives (Quad) into the Scene |

### Module Map

| Module | Zed Equivalent | Description |
|--------|---------------|-------------|
| `scene` | `gpui::Scene` | Retained-mode display list of Quad primitives |
| `element` | `gpui::Element` | Core trait with 3-phase lifecycle + interaction state |
| `div` | `gpui::elements::div` | Container element with children, hover/active styles |
| `button` | Zed's Button component | Interactive button with Filled/Outlined/Subtle variants |
| `text` | Platform text system | Bitmap glyph renderer (5x7 pixel font, each lit pixel = 1 Quad) |
| `style` | `gpui::Style` / `StyleRefinement` | Style system with Tailwind-inspired builder API |
| `layout` | Taffy (flexbox) | Simplified flexbox layout engine (Row/Column, padding/margin, alignment) |
| `renderer` | `gpui_macos::MetalRenderer` | Metal backend: instanced quad rendering with SDF rounded rects |
| `shaders` | `shaders.metal` | Metal shaders: HSLA->RGBA, rounded rect SDF, anti-aliased borders |
| `metal_view` | `gpui_macos::MetalView` | NSView subclass with CAMetalLayer, event handling, render loop |
| `display_link` | CVDisplayLink | FFI bindings for VSync-driven rendering |
| `color` | `gpui::Hsla` | HSLA color type matching the shader's `hsla_to_rgba` |
| `geometry` | `gpui::geometry` | Point, Size, Bounds, Edges, Corners |

### Metal Shaders

Adapted from Zed's `shaders.metal`:

- **Instanced rendering**: one instance per Quad, 6 vertices (2 triangles) per instance
- **SDF rounded rectangles**: `quad_sdf` for anti-aliased corners and borders
- **HSLA to RGBA**: color conversion at fragment shader level (same algorithm as Zed)
- **Premultiplied alpha blending**: matches Zed's blend mode

## Build & Run

Requires macOS with Metal support.

```bash
cargo run
```

## Dependencies

- `metal` / `objc2-metal` — Metal GPU API
- `objc2-app-kit` — NSApplication, NSWindow, NSView
- `objc2-quartz-core` — CAMetalLayer
- `objc2-foundation` — Foundation types
- `core-graphics` — Core Graphics
- `dispatch` — GCD for main thread dispatch
- `block2` — Objective-C block interop
