# Agent Tasks (Persistent Backlog)

- [ ] T-1302 (sprint 13): src/render/ module — GlassUniforms + GlassResources + init_glass_pipeline + minimal glass.wgsl + GlassCallback — touches: src/render/mod.rs, src/render/glass.wgsl, src/main.rs
- [ ] T-1303 (sprint 13): wire GlassCallback into app.rs paint loop for shader materials (theme_material enum, None-guard, premultiplied blend) — touches: src/app.rs
- [ ] T-1304 (sprint 13): Volcanic Glass WGSL (iridescent aura + breathing active-row radiance) — touches: src/render/glass.wgsl
- [ ] T-1305 (sprint 13): Primordial WGSL branch (purple plasma) — touches: src/render/glass.wgsl
- [ ] T-1306 (sprint 13): Zircon hybrid — compositor_blur_available seam + Windows apply_effects + WGSL frosted fallback + honest tier — touches: src/render/mod.rs, src/os/windows.rs, src/app.rs, src/render/glass.wgsl
- [ ] T-1307 (sprint 13): material params in config.rs (TOML, hot-reloadable) + example configs — touches: src/config.rs, configs/*.toml
- [ ] T-1308 (sprint 13): ADR-013 + README materials/shader note — touches: decisions.md, README.md

## Deferred (follow-on sprints, user-confirmed)
- [ ] Glassy morphing cursor — replace block/line cursor; glyphs the cursor glides over morph/distort (refraction), driven by existing motion-easing. Visual-only, Guarantee-#3-safe (offscreen-glyph + lens displacement). Its own sprint.
- [ ] Curved bevel edge — gated stretch; only with a concrete SDF-rounded-bevel plan (sdRoundedBox + distance-field-gradient normal lit by a fixed light). Its own sprint.
- [ ] True per-glyph aura via offscreen glyph sampling (v1 glass is a background/substrate pass).
- [ ] Windows-Terminal job-object breakaway for launch-via-PATH (needs unsafe Win32; ADR-002 tension) — see ADR-012; Start-menu shortcut is the workaround.
