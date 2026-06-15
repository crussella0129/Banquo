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
