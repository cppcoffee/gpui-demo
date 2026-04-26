// Metal shaders adapted from Zed's crates/gpui_macos/src/shaders.metal.
// Key algorithms preserved:
//   - hsla_to_rgba: HSLA → RGBA conversion (same as Zed line 889)
//   - pick_corner_radius: selects the correct corner radius for each vertex (Zed line 847)
//   - quad_sdf_impl: SDF for rounded rectangles (Zed line 866)
//   - Instanced rendering: one instance per Quad, 6 vertices (2 triangles) per instance
//
// Architecture notes:
//   Zed uses a fullscreen quad (6 vertices covering the entire viewport) for each Quad instance.
//   The vertex shader reads per-instance data (bounds, colors, radii) to position the quad
//   and pass data to the fragment shader. The fragment shader uses SDF to render rounded
//   rectangles with anti-aliasing, background fill, and borders.
//
// Note: Metal does not allow nested structs with duplicate field names in vertex output /
// fragment input (stage_in). We use float4 for HSLA colors in the inter-stage struct
// to avoid "duplicated name" errors, and convert in the fragment shader.

pub const SHADER_SOURCE: &str = r#"

#include <metal_stdlib>
using namespace metal;

// --- Structs matching Rust side (#[repr(C)]) ---
// These are used for the instance buffer only.

struct Point { float x, y; };
struct Size  { float width, height; };
struct Bounds { Point origin; Size size; };
struct Hsla  { float h, s, l, a; };
struct Corners { float top_left, top_right, bottom_right, bottom_left; };
struct Edges   { float top, right, bottom, left; };

// Mirrors gpui::Quad — per-instance data for instanced quad rendering.
struct Quad {
    uint32_t order;
    Bounds bounds;
    Hsla background;
    Hsla border_color;
    Corners corner_radii;
    Edges border_widths;
};

// Vertex shader output / fragment shader input
// Uses float4 for colors to avoid Metal's "duplicated name" error with nested structs
struct QuadVertexOut {
    float4 position [[position]];
    float2 local_position;
    Bounds bounds;
    float4 background;      // HSLA packed as (h, s, l, a)
    float4 border_color;    // HSLA packed as (h, s, l, a)
    Corners corner_radii;
    Edges border_widths;
};

// --- HSLA to RGBA conversion ---
// Same algorithm as Zed's shaders.metal hsla_to_rgba (line 889)
float4 hsla_to_rgba(float4 hsla) {
    float h = hsla.x * 6.0;
    float s = hsla.y;
    float l = hsla.z;
    float a = hsla.w;

    float c = (1.0 - abs(2.0 * l - 1.0)) * s;
    float x = c * (1.0 - abs(fmod(h, 2.0) - 1.0));
    float m = l - c / 2.0;

    float r, g, b;
    if      (h >= 0.0 && h < 1.0) { r = c; g = x; b = 0.0; }
    else if (h >= 1.0 && h < 2.0) { r = x; g = c; b = 0.0; }
    else if (h >= 2.0 && h < 3.0) { r = 0.0; g = c; b = x; }
    else if (h >= 3.0 && h < 4.0) { r = 0.0; g = x; b = c; }
    else if (h >= 4.0 && h < 5.0) { r = x; g = 0.0; b = c; }
    else                           { r = c; g = 0.0; b = x; }

    return float4(r + m, g + m, b + m, a);
}

// --- Corner radius selection ---
// Mirrors Zed's pick_corner_radius (line 847)
float pick_corner_radius(Corners radii, float2 position, Bounds bounds) {
    float2 center = float2(bounds.origin.x + bounds.size.width / 2.0,
                           bounds.origin.y + bounds.size.height / 2.0);
    if (position.x < center.x) {
        if (position.y < center.y) return radii.top_left;
        else return radii.bottom_left;
    } else {
        if (position.y < center.y) return radii.top_right;
        else return radii.bottom_right;
    }
}

// --- Rounded rectangle SDF ---
// Mirrors Zed's quad_sdf_impl (line 866)
// Returns signed distance: negative = inside, positive = outside
float quad_sdf(float2 position, Bounds bounds, float corner_radius) {
    float2 half_size = float2(bounds.size.width / 2.0, bounds.size.height / 2.0);
    float2 center = float2(bounds.origin.x + half_size.x, bounds.origin.y + half_size.y);
    float2 d = abs(position - center) - half_size + corner_radius;
    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0)) - corner_radius;
}

// --- Vertex shader ---
// Mirrors Zed's quad_vertex (line 920)
// Uses instanced rendering: vertex_id selects corner (0-5 for 2 triangles),
// instance_id selects which Quad to render.
vertex QuadVertexOut quad_vertex(
    uint vid [[vertex_id]],
    uint instance_id [[instance_id]],
    constant Quad* quads [[buffer(0)]],
    constant float2& viewport_size [[buffer(1)]]
) {
    Quad quad = quads[instance_id];

    // Fullscreen quad vertices (2 triangles covering the quad bounds)
    float2 positions[6] = {
        float2(0.0, 0.0),  // top-left
        float2(1.0, 0.0),  // top-right
        float2(0.0, 1.0),  // bottom-left
        float2(0.0, 1.0),  // bottom-left
        float2(1.0, 0.0),  // top-right
        float2(1.0, 1.0),  // bottom-right
    };

    float2 pos = positions[vid];
    float2 quad_pos = float2(
        quad.bounds.origin.x + pos.x * quad.bounds.size.width,
        quad.bounds.origin.y + pos.y * quad.bounds.size.height
    );

    // Convert to clip space (Metal: y=0 at top, y=1 at bottom in NDC)
    float2 clip_pos;
    clip_pos.x = (quad_pos.x / viewport_size.x) * 2.0 - 1.0;
    clip_pos.y = 1.0 - (quad_pos.y / viewport_size.y) * 2.0;

    QuadVertexOut out;
    out.position = float4(clip_pos, 0.0, 1.0);
    out.local_position = quad_pos;
    out.bounds = quad.bounds;
    out.background = float4(quad.background.h, quad.background.s, quad.background.l, quad.background.a);
    out.border_color = float4(quad.border_color.h, quad.border_color.s, quad.border_color.l, quad.border_color.a);
    out.corner_radii = quad.corner_radii;
    out.border_widths = quad.border_widths;
    return out;
}

// --- Fragment shader ---
// Mirrors Zed's quad_fragment (line 960)
// Uses SDF to render rounded rectangle with background fill and border.
fragment float4 quad_fragment(
    QuadVertexOut in [[stage_in]]
) {
    float corner_radius = pick_corner_radius(in.corner_radii, in.local_position, in.bounds);
    float dist = quad_sdf(in.local_position, in.bounds, corner_radius);

    // Anti-aliasing: smooth transition at the edge (1px)
    float aa = fwidth(dist);
    float alpha = 1.0 - smoothstep(-aa, aa, dist);

    if (alpha <= 0.0) {
        discard_fragment();
    }

    // Background fill
    float4 bg = hsla_to_rgba(in.background);

    // Border: compute inner SDF (shrunk by border width)
    float border_width = 0.0;
    float2 center = float2(
        in.bounds.origin.x + in.bounds.size.width / 2.0,
        in.bounds.origin.y + in.bounds.size.height / 2.0
    );
    if (in.local_position.x < center.x) border_width = in.border_widths.left;
    else border_width = in.border_widths.right;
    if (in.local_position.y < center.y) border_width = max(border_width, in.border_widths.top);
    else border_width = max(border_width, in.border_widths.bottom);

    float4 result = bg;

    if (border_width > 0.0) {
        // Inner SDF: shrink bounds by border width
        Bounds inner_bounds;
        inner_bounds.origin.x = in.bounds.origin.x + in.border_widths.left;
        inner_bounds.origin.y = in.bounds.origin.y + in.border_widths.top;
        inner_bounds.size.width = in.bounds.size.width - in.border_widths.left - in.border_widths.right;
        inner_bounds.size.height = in.bounds.size.height - in.border_widths.top - in.border_widths.bottom;

        float inner_corner_radius = max(0.0, corner_radius - max(
            max(in.border_widths.left, in.border_widths.right),
            max(in.border_widths.top, in.border_widths.bottom)
        ));

        float inner_dist = quad_sdf(in.local_position, inner_bounds, inner_corner_radius);
        float inner_aa = fwidth(inner_dist);
        float inner_alpha = 1.0 - smoothstep(-inner_aa, inner_aa, inner_dist);

        float4 border_color = hsla_to_rgba(in.border_color);

        // Blend: outside border = border_color, inside = background
        result = mix(border_color, bg, inner_alpha);
 }

    // Premultiplied alpha (Zed uses premultiplied alpha blending)
    result.rgb *= result.a;
    result.a *= alpha;

    return result;
}

"#;
