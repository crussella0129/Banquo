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
