# Completed Tasks Log (Append-Only)

## T-001 (sprint 0)
- **Description:** Cargo.toml manifest pinning the eframe/wgpu stack (eframe 0.34, wgpu feature, edition 2021); workspace split reserved for Milestone 2.
- **Completed:** 2026-06-15T04:50:00Z
- **Files modified:** Cargo.toml
- **Commit:** `2054ed4`

## T-002 (sprint 0)
- **Description:** Entry point src/main.rs — #![forbid(unsafe_code)], eframe::run_native with transparent + undecorated wgpu NativeOptions; declares the app/fonts module seam.
- **Completed:** 2026-06-15T04:53:00Z
- **Files modified:** src/main.rs
- **Commit:** `b84036a`

## T-003 (sprint 0)
- **Description:** Vendored Iosevka-Regular.ttf (v34.6.3, SIL OFL 1.1) into assets/fonts/ with its license. Real Iosevka obtained — fallback path not needed.
- **Completed:** 2026-06-15T05:00:00Z
- **Files modified:** assets/fonts/Iosevka-Regular.ttf, assets/fonts/Iosevka-LICENSE.md
- **Commit:** `3444705`

## T-004 (sprint 0)
- **Description:** Font pipeline src/fonts.rs — pure build_font_definitions(Option<&[u8]>) -> (FontDefinitions, FontSource) registering banquo-mono (Iosevka) and aliasing on fallback; 2 unit tests. Added egui direct dep + [workspace] root to Cargo.toml.
- **Completed:** 2026-06-15T05:20:00Z
- **Files modified:** src/fonts.rs, Cargo.toml
- **Commit:** `9c29bb9`

## T-005 (sprint 0)
- **Description:** The Face src/app.rs — BanquoApp impl eframe::App (0.34 logic/ui API): install-once font latch via pure should_install_fonts, paints centered Iosevka line on a flat tinted field, zero-alpha clear_color for transparency; 2 unit tests. cargo fmt normalized main.rs.
- **Completed:** 2026-06-15T05:22:00Z
- **Files modified:** src/app.rs, .gitignore, Cargo.lock
- **Commit:** `cf3d1c4`

## T-006 (sprint 0)
- **Description:** Four founding ADRs in decisions.md — crate stack (eframe owns wgpu/winit), forbid(unsafe_code) over deny, truth/appearance seam as module boundary, alacritty_terminal core (deferred to M2).
- **Completed:** 2026-06-15T05:30:00Z
- **Files modified:** decisions.md
- **Commit:** `6793cbb`

## T-007 (sprint 0)
- **Description:** README.md — what Banquo is, milestone roadmap table (M1 done), cargo run/test instructions, the six guarantees, the subtraction list, licensing.
- **Completed:** 2026-06-15T05:35:00Z
- **Files modified:** README.md, agent-tasks/
- **Commit:** `782ef09` (this entry's hash back-filled in a follow-up housekeeping commit)

## T-100 (sprint 1)
- **Description:** GitHub Actions CI workflow — cargo test + clippy + fmt on ubuntu-latest + windows-latest, triggered on push/PR to main.
- **Completed:** 2026-06-16T04:51:00Z
- **Files modified:** .github/workflows/ci.yml, agent-tasks/agent-tasks.md
- **Commit:** `9b41d11`

## Sprint 3 (Milestone 2 - Tabs & Auto-Collapse)
- `[x]` T-300: Update `BanquoApp` struct: replace `session: SessionHandle` with `sessions: Vec<SessionHandle>`, `active_tab: usize`, `last_mouse_pos: Option<Pos2>`, and `last_mouse_move_time: Instant`.
- `[x]` T-301: Update `BanquoApp::new` to initialize the vector with the first session. Update render loop to use `self.sessions[self.active_tab]`.
- `[x]` T-302: Implement `BanquoApp` methods/logic to spawn a new tab (Ctrl+Shift+T) and close the active tab (Ctrl+Shift+W).
- `[x]` T-303: In `BanquoApp::update`, remove the code that offsets `content_rect` downwards. The terminal grid must occupy the full window.
- `[x]` T-304: Implement idle detection: Update `last_mouse_pos` and `last_mouse_move_time` when pointer moves. Evaluate `show_tabs = pointer.y <= 40.0 && time_since_move < 3s`.
- `[x]` T-305: When `show_tabs` is true, render the tab bar overlay using `egui::Area` (or a window without frames) anchored to the top. Draw the custom window chrome (drag rect, close button) inside this overlay.
- `[x]` T-306: Render the active tabs inside the overlay, allowing the user to click to switch `active_tab`, and a `+` button to spawn a new tab.
- `[x]` T-200: Implement OS-detection config flag in `main.rs` to set `ViewportBuilder::with_decorations` appropriately (true for Unix/macOS, false for Windows).
- `[x]` T-201: Pass `decorations: bool` flag into `BanquoApp`'s state.
- `[x]` T-202: If `!decorations`, render an invisible drag-to-move rect at the top of `BanquoApp::update` sending `ViewportCommand::StartDrag`.
- `[x]` T-203: If `!decorations`, render a `×` close button at the top-right of the window sending `ViewportCommand::Close`.
- `[x]` T-204: If `!decorations`, render invisible resize borders at the window edges sending `ViewportCommand::StartResize(ResizeDirection)`.
- `[x]` T-205: Offset the terminal grid layout down by the height of the custom titlebar if `!decorations` so text isn't covered by the drag area.
- **Completed:** 2026-06-16T05:01:00Z
- **Files modified:** src/main.rs, src/app.rs, agent-tasks/agent-tasks.md
- **Commit:** `2a7e1f2`

## T-101 through T-111 (sprint 1)
- **Description:** Milestone 2 "It echoes" core implementation — PTY spawning, session reader thread, lock-free ArcSwap snapshot publishing, termVT adapter, metrics math, keystroke encoding, and grid rendering in the Face.
- **Completed:** 2026-06-16T05:01:00Z
- **Files modified:** Cargo.toml, src/core/*.rs, src/metrics.rs, src/app.rs, src/main.rs, agent-tasks/agent-tasks.md
- **Commit:** `1e3b0f0`

## T-112 (sprint 1)
- **Description:** Wrote ADR-010 documenting the three-actor lock-free handoff snapshot/threading model. Updated README milestone roadmap to mark Milestone 2 as complete and revised run instructions.
- **Completed:** 2026-06-16T05:02:00Z
- **Files modified:** decisions.md, README.md, agent-tasks/agent-tasks.md
- **Commit:** `pending`
- [x] Update Milestones in README
- [x] Update Section VII (Tabs) in README
- [x] Add Auto-Snap Proportional Grid documentation
- [x] Add Banquo Compose documentation
- Update config.rs to add WindowAppearanceConfig
- Update main.rs to disable native_decorations globally
- Implement G1/G2/G3 corner tessellation logic
- Implement edge style drawing (flat, rounded, beveled glass chamfer, 3d)
- Update app.rs rendering loop to use the shape instead of flat clear screen

## Sprint 4 (Themes & Graphics Foundations)
- Implement `get_squircle_path` for g1/g2/g3.
- Fix anti-aliasing halo and tab bar over-clamping.
- Implement UI persistent tab bar mode ("Finesstra" layout).
- **Completed:** 2026-06-16

## Sprint 5 (Themes)
- Implemented Blanco, Zircon, Concrete, and Volcanic Glass presets.
- Built 8K procedural texture generator.
- Built text contrast mapping and multi-pass rendering.
- Completed: 2026-06-17

## Sprint 6 (Tab Bar Polish)
- Added individual tab closure `×` button and tab cleanup logic.
- Hooked `Event::Title` to hot-reload session titles via OSC sequences.
- Fixed 32px tab bar vertical alignment math.
- Completed: 2026-06-17

## Sprint 7 (Milestone 7: The Finish)
- Implemented `notify` crate background thread for filesystem config watching.
- Implemented `BanquoApp::update` receiver to instantly hot-reload UI state from config changes without tearing down the shell session.
- Built Command Palette `egui` overlay toggled via `Ctrl+Shift+P` allowing direct theme setting (`theme blanco`).
- Implemented exponential mathematical easing for organic cursor interpolation.
- Fixed passthrough bugs preventing `egui` typing from sending keystrokes to the active terminal.
- Completed: 2026-06-17
