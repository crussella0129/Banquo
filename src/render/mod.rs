//! The render module — Banquo's custom WGSL shader pipeline (Milestone 6 "Fire").
//!
//! The glass materials route through a single `egui_wgpu::CallbackTrait` pass
//! (the design-mandated `custom3d` path — never the render-pass transmute hack).
//! The pipeline + GPU resources are built once at startup from the wgpu
//! `RenderState` and stashed in egui's per-frame callback resource map; each
//! frame the Face constructs a [`GlassCallback`] carrying that frame's uniforms
//! and hands it to `egui_wgpu::Callback`.
//!
//! Everything here is safe: `bytemuck`'s `Pod`/`Zeroable` derives and wgpu's
//! safe API keep `#![forbid(unsafe_code)]` intact (ADR-002).

use bytemuck::{Pod, Zeroable};

/// Per-frame uniforms handed to the glass shader.
///
/// **Frozen std140 layout** (48 bytes; a `vec2` must never straddle a 16-byte
/// boundary). The WGSL `struct GlassUniforms` in `glass.wgsl` mirrors this order
/// field-for-field — reorder here and you MUST reorder there.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GlassUniforms {
    /// Render-target resolution in physical pixels. (offset 0)
    pub resolution: [f32; 2],
    /// Seconds since startup, for animation. (offset 8)
    pub time: f32,
    /// 0 = Volcanic, 1 = Primordial, 2 = Zircon-fallback. (offset 12)
    pub material_id: u32,
    /// Cursor position in physical pixels. (offset 16)
    pub cursor_px: [f32; 2],
    /// Top of the active (cursor) row, physical px. (offset 24)
    pub active_row_y: f32,
    /// Height of one row, physical px. (offset 28)
    pub active_row_h: f32,
    /// Material params: [aura_intensity, radiance_speed, radiance_scale, glass_amount]. (offset 32)
    pub params: [f32; 4],
}

/// GPU resources for the glass pass, stored in egui's callback resource map.
struct GlassResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

/// Build the glass render pipeline once and stash it in egui's callback
/// resources. Call from `BanquoApp::new` with `cc.wgpu_render_state()`.
pub fn init_glass_pipeline(render_state: &egui_wgpu::RenderState) {
    let device = &render_state.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("banquo.glass.wgsl"),
        source: wgpu::ShaderSource::Wgsl(include_str!("glass.wgsl").into()),
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("banquo.glass.uniforms"),
        size: std::mem::size_of::<GlassUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("banquo.glass.bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("banquo.glass.bg"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("banquo.glass.layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("banquo.glass.pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: render_state.target_format,
                // Premultiplied alpha — matches egui's blending and Banquo's
                // transparent substrate (ADR-005): Volcanic near-black and Zircon
                // transparent both composite correctly.
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    render_state
        .renderer
        .write()
        .callback_resources
        .insert(GlassResources {
            pipeline,
            bind_group,
            uniform_buffer,
        });
}

/// A single frame's glass draw. Carries the per-frame uniform snapshot as owned
/// data (the `CallbackTrait` methods are `&self`); the GPU resources live in the
/// callback resource map.
pub struct GlassCallback {
    pub uniforms: GlassUniforms,
}

impl egui_wgpu::CallbackTrait for GlassCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = resources.get::<GlassResources>() {
            queue.write_buffer(
                &res.uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.uniforms]),
            );
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(res) = resources.get::<GlassResources>() {
            render_pass.set_pipeline(&res.pipeline);
            render_pass.set_bind_group(0, &res.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glass_uniforms_layout() {
        // Frozen std140 layout — a reordering vec2 straddle would break the GPU
        // read silently; assert the exact size + key offsets.
        assert_eq!(std::mem::size_of::<GlassUniforms>(), 48);
        assert_eq!(std::mem::offset_of!(GlassUniforms, cursor_px), 16);
        assert_eq!(std::mem::offset_of!(GlassUniforms, params), 32);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_glass_wgsl_parses() {
        // Validate the WGSL headlessly (no GPU) via wgpu's re-exported naga.
        let src = include_str!("glass.wgsl");
        let result = wgpu::naga::front::wgsl::parse_str(src);
        assert!(result.is_ok(), "glass.wgsl must be valid WGSL: {result:?}");
    }
}
