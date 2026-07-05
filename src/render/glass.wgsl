// Banquo glass materials — the custom WGSL shader (Milestone 6 "Fire").
//
// One shader, branched on `material_id`, drives all glass materials as a
// background/substrate pass painted *behind* the egui-rendered glyphs:
//   0 = Volcanic Glass  (iridescent red/purple field + breathing active-row radiance)
//   1 = Primordial      (dark purple plasma + breathing radiance)
//   2 = Zircon fallback (frosted tint/scrim when no compositor blur)
//
// IMPORTANT: this struct's field order MUST mirror the Rust `GlassUniforms`
// (`src/render/mod.rs`) exactly — naga validates WGSL syntax, not host layout.
struct GlassUniforms {
    resolution: vec2<f32>,
    time: f32,
    material_id: u32,
    cursor_px: vec2<f32>,
    active_row_y: f32,
    active_row_h: f32,
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: GlassUniforms;

// Fullscreen triangle — no vertex buffer; egui scissors it to the callback rect.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    // v1 scaffold: a solid, premultiplied debug tint so we can confirm the pass
    // is wired and scissored correctly. Material shaders land in T-1304+.
    // (premultiplied alpha: rgb already multiplied by a = 0.5)
    return vec4<f32>(0.15, 0.0, 0.15, 0.5);
}
