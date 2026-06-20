# Agent Tasks (Persistent Backlog)

- [ ] T-1203 (sprint 12): ADR-012 + README launch-independence note — touches: decisions.md, README.md

- [ ] (sprint 13): Glass effects configs — the real material engine. (1) **WGPU/WGSL shaders** for the glass materials (Zircon, Volcanic-Glass, Primordial) via eframe `CallbackTrait` (design M6 "Fire"); direct `wgpu` dep added here, matched to eframe's resolved version. (2) **Glassy morphing cursor:** replace the block/line cursor — glyphs the cursor glides over morph/distort slightly (refraction-like), driven by the existing motion-easing cursor interpolation. (3) **STRETCH, gated:** curved bevel edge effect on the window/cells — "don't attempt unless there's a concrete plan grounded in a known prior implementation" (per user); research must find a proven technique (e.g. SDF-based rounded bevel, established egui/shader precedent) or explicitly defer it. Keep `#![forbid(unsafe_code)]` (wgpu is safe-wrapped).
