// Mirrors gpui_macos::MetalRenderer — the Metal rendering backend.
// Key pattern from Zed's metal_renderer.rs draw_quads():
//   1. Get next drawable from CAMetalLayer
//   2. Create command buffer + render command encoder
//   3. Copy instance data to instance buffer
//   4. Set pipeline, draw_primitives_instanced(Triangle, 0, 6, instance_count)
//   5. End encoding, present drawable, commit

use crate::scene::Scene;
use crate::shaders::SHADER_SOURCE;
use metal::*;

pub struct MetalRenderer {
    device: Device,
    command_queue: CommandQueue,
    pipeline_state: RenderPipelineState,
    instance_buffer: Option<Buffer>,
    instance_buffer_capacity: usize,
}

impl MetalRenderer {
    pub fn new(device: &Device) -> Self {
        let library = device
            .new_library_with_source(SHADER_SOURCE, &CompileOptions::new())
            .expect("Failed to compile Metal shaders");

        let vertex_fn = library
            .get_function("quad_vertex", None)
            .expect("Failed to get vertex function");
        let fragment_fn = library
            .get_function("quad_fragment", None)
            .expect("Failed to get fragment function");

        let pipeline_desc = RenderPipelineDescriptor::new();
        pipeline_desc.set_vertex_function(Some(&vertex_fn));
        pipeline_desc.set_fragment_function(Some(&fragment_fn));

        let color_attachment = pipeline_desc.color_attachments().object_at(0).unwrap();
        color_attachment.set_pixel_format(MTLPixelFormat::BGRA8Unorm);

        // Premultiplied alpha blending — same as Zed
        color_attachment.set_blending_enabled(true);
        color_attachment.set_source_rgb_blend_factor(MTLBlendFactor::One);
        color_attachment.set_destination_rgb_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);
        color_attachment.set_source_alpha_blend_factor(MTLBlendFactor::One);
        color_attachment.set_destination_alpha_blend_factor(MTLBlendFactor::OneMinusSourceAlpha);

        let pipeline_state = device
            .new_render_pipeline_state(&pipeline_desc)
            .expect("Failed to create render pipeline state");

        let command_queue = device.new_command_queue();

        Self {
            device: device.clone(),
            command_queue,
            pipeline_state,
            instance_buffer: None,
            instance_buffer_capacity: 0,
        }
    }

    /// Draw the scene — mirrors Zed's MetalRenderer::draw() + draw_quads()
    pub fn draw(&mut self, scene: &Scene, drawable: &MetalDrawableRef, viewport_size: (f32, f32)) {
        let quad_count = scene.quads.len();
        if quad_count == 0 {
            return;
        }

        // Ensure instance buffer is large enough
        let quad_size = std::mem::size_of::<crate::scene::Quad>();
        let needed_bytes = quad_count * quad_size;
        if needed_bytes > self.instance_buffer_capacity {
            let new_capacity = needed_bytes * 2;
            self.instance_buffer = Some(self.device.new_buffer(
                new_capacity as u64,
                MTLResourceOptions::CPUCacheModeWriteCombined,
            ));
            self.instance_buffer_capacity = new_capacity;
        }

        // Copy quad data to instance buffer
        let buffer = self.instance_buffer.as_ref().unwrap();
        let ptr = buffer.contents() as *mut crate::scene::Quad;
        unsafe {
            std::ptr::copy_nonoverlapping(scene.quads.as_ptr(), ptr, quad_count);
        }

        // Create command buffer
        let command_buffer = self.command_queue.new_command_buffer();

        // Render pass
        let pass_desc = RenderPassDescriptor::new();
        let color_attachment = pass_desc.color_attachments().object_at(0).unwrap();
        color_attachment.set_texture(Some(drawable.texture()));
        color_attachment.set_load_action(MTLLoadAction::Clear);
        color_attachment.set_clear_color(MTLClearColor::new(0.11, 0.11, 0.12, 1.0));
        color_attachment.set_store_action(MTLStoreAction::Store);

        let encoder = command_buffer.new_render_command_encoder(pass_desc);

        let viewport = MTLViewport {
            originX: 0.0,
            originY: 0.0,
            width: viewport_size.0 as f64,
            height: viewport_size.1 as f64,
            znear: 0.0,
            zfar: 1.0,
        };
        encoder.set_viewport(viewport);

        // Draw quads — mirrors Zed's draw_quads()
        encoder.set_render_pipeline_state(&self.pipeline_state);
        encoder.set_vertex_buffer(0, Some(buffer), 0);
        encoder.set_fragment_buffer(0, Some(buffer), 0);

        let vp: [f32; 2] = [viewport_size.0, viewport_size.1];
        encoder.set_vertex_bytes(
            1,
            std::mem::size_of::<[f32; 2]>() as u64,
            vp.as_ptr() as *const _,
        );

        encoder.draw_primitives_instanced(MTLPrimitiveType::Triangle, 0, 6, quad_count as u64);

        encoder.end_encoding();

        command_buffer.present_drawable(drawable);
        command_buffer.commit();
    }
}
