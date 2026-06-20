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
- [x] Edit $env:APPDATA\banquo\banquo.toml to add ackground_mode = "reveal".
- [x] Delete ssets/fonts/Geist-Regular.ttf and ssets/fonts/geist/.
- [x] Refactor src/fonts.rs to remove proportional fonts and map UI fonts to BANQUO_MONO.
- [x] Refine 	exture_gen.rs dot sizes and scarcity.
- [x] Add generate_primordial_texture to 	exture_gen.rs.
- [x] Update pp.rs state and rendering for primordial.
- [x] Create configs/primordial.toml.
- [x] Update recipe card artifact.

## T-1099 (sprint 10) — Build sanity-gate remediation
- **Description:** The sprint-10 test report only ran `cargo check`, which missed the protocol's `cargo clippy --all-targets -- -D warnings` gate. Fixed 4 issues: (1) test `test_transparency_invariants` referenced the removed `FLAT_FIELD` constant (E0425 — tests didn't compile); replaced with the still-valid zero-alpha `CLEAR_COLOR` invariant. (2) `font_for_char` manual range check → `('\u{2500}'..='\u{259F}').contains(&ch)`. (3) collapsed an `else { if }` block in the tab-close path. (4) removed redundant `.trim()` before `.split_whitespace()` in the command palette. Also stripped trailing whitespace (rustfmt internal error) and gitignored stray `banquo_stderr.txt`. Now green: clippy clean under `-D warnings`, 25 + 4 tests pass.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/app.rs, .gitignore
- **Commit:** `dbdc3f7`

## T-1101 (sprint 11) — Shell config schema
- **Description:** Added `ShellConfig { default, profiles }` and `ShellProfile { name, command, args, cwd, env }` to config.rs and wired `shell` into `BanquoConfig` with `#[serde(default)]`. `env` is a `BTreeMap` for idiomatic TOML tables + deterministic ordering. 3 unit tests (deserialize, defaults-when-absent, args-default-empty).
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/config.rs
- **Commit:** `ff7566f`

## T-1102 (sprint 11) — ResolvedShell + to_command
- **Description:** Added pure `ResolvedShell { prog, args, cwd, env }` + `to_command() -> CommandBuilder` in pty.rs. Named `to_command` (not the plan's `into_command`) to satisfy clippy `wrong_self_convention` — it takes `&self` because one spec spawns many tabs; EARS behavior (argv[0]==prog, args, cwd/env) unchanged. Temporary `#[allow(dead_code)]` until T-1103/T-1105 consume it. 2 unit tests assert against `get_argv()`/`get_cwd()`.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/core/pty.rs
- **Commit:** `2fdd850`

## T-1103 (sprint 11) — resolve_shell resolver
- **Description:** New `src/core/shell.rs` with pure `resolve_shell(config, name) -> Option<ResolvedShell>` (+ private `profile_to_resolved`). `Some(name)` selects that profile; `None` selects `config.shell.default`; unknown/unset → `None` (caller uses `new_default_prog`/no-op). Registered `pub mod shell` in core/mod.rs. Temporary `#[allow(dead_code)]` until T-1106 wires it. 4 unit tests (named, default, fallback-None, args/cwd/env mapping).
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/core/shell.rs, src/core/mod.rs
- **Commit:** `790f81f`

## T-1104 (sprint 11) — os::detect_shells PATH probe
- **Description:** Added pure `detect_in(paths) -> Vec<ShellProfile>` + ambient-`PATH` wrapper `detect_shells()` in os/mod.rs. Per-OS candidate table (Windows: pwsh/powershell/cmd/bash/wsl; Unix: bash/zsh/sh) probed via `Path::join(exe).exists()` — no spawning. Guaranteed non-empty (fallback cmd.exe / /bin/sh). WSL detected as a single profile (per-distro enumeration deferred — avoids the UTF-16LE hazard). Temporary `#[allow(dead_code)]` until T-1106. 2 cross-platform unit tests.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/os/mod.rs
- **Commit:** `d0d637c`

## T-1105 (sprint 11) — open_pty/spawn accept a shell
- **Description:** `open_pty(cols, rows, shell: Option<&ResolvedShell>)` and `session::spawn(.., shell: Option<ResolvedShell>)` now build the `CommandBuilder` from the resolved shell, falling back to `new_default_prog()` on `None`. All three existing spawn sites (main.rs:66, app.rs `+`-button, app.rs Ctrl+Shift+T) pass `None` here — behavior unchanged; T-1106 swaps in real resolution. Removed the temporary `#[allow(dead_code)]` on `ResolvedShell`/`to_command` (now consumed). 36+4 tests pass.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/core/pty.rs, src/core/session.rs, src/main.rs, src/app.rs
- **Commit:** `fe66557`

## T-1106 (sprint 11) — wire startup + new-tab to configured default
- **Description:** Added `default_shell: Option<ResolvedShell>` to `BanquoApp`, resolved once in `new()` via `resolve_shell(&config, None)` and refreshed on config hot-reload. main.rs now loads config *before* the startup spawn and launches the configured default; both new-tab sites (`+` button, Ctrl+Shift+T) spawn `self.default_shell.clone()`. Removed the `#[allow(dead_code)]` on `resolve_shell`. No config → OS default (unchanged). 36+4 tests pass.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/app.rs, src/main.rs, src/core/shell.rs
- **Commit:** `5f444d2`

## T-1107 (sprint 11) — command-palette `shell <name>` verb
- **Description:** Added a `shell <name>` arm to the palette dispatch: opens a new tab running the named shell. Resolution prefers a configured profile, then falls back to `os::detect_shells()` lookup by name — so `shell pwsh`/`shell wsl` work with **zero configuration**. Unknown name → safe no-op (palette closes, no panic). Made `shell::profile_to_resolved` pub for the fallback conversion; removed the remaining `#[allow(dead_code)]` on `detect_shells`/`detect_in` (now consumed). 36+4 tests pass.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/app.rs, src/core/shell.rs, src/os/mod.rs
- **Commit:** `8861b15`

## T-1108 (sprint 11) — windows_subsystem release attribute
- **Description:** Added `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` to main.rs so release `banquo.exe` launches with no console window; debug keeps the console for `eprintln!`. Verified: debug gate clean + 36/4 tests pass, `cargo build --release` succeeds. The "no console flash" confirmation is the manual E2E checkpoint at Loop (no headless test).
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** src/main.rs
- **Commit:** `5044ac2`

## T-1109 (sprint 11) — install.ps1
- **Description:** Added `install.ps1`: builds `cargo build --release` (aborts the whole install if the build fails or the exe is missing), copies `banquo.exe` to `%LOCALAPPDATA%\Banquo`, and creates a Start-menu shortcut via `WScript.Shell` (no unsafe code). Optional `-Desktop` and `-AddToPath` switches. Verified it parses via the PowerShell AST parser (PARSE OK). Ends the cargo-run-from-repo dance.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** install.ps1
- **Commit:** `a7e0f0c`

## T-1110 (sprint 11) — README rewrite + ADR-011
- **Description:** Rewrote README "Run it" → "Install it (standalone)" + "Choose your shell" + "Develop it": documents install.ps1, the palette `shell <name>` verb (zero-config detection), and a `[shell]` config example. Added ADR-011 to decisions.md recording the shell-profile model + windows-subsystem decision and the two explicit deferrals (default-terminal handoff; ADR-009 elevated-launch/shield). Gate green; `test_decisions_has_four_adrs` still passes.
- **Completed:** 2026-06-18T00:00:00Z
- **Files modified:** README.md, decisions.md
- **Commit:** `60a8733`

## T-1111 (sprint 11) — CI remediation (pre-existing Linux failures)
- **Description:** Loop-phase CI verify exposed that `main` had been red on ubuntu across multiple prior pushes — unrelated to sprint 11. Two pre-existing root causes: (1) `eframe { default-features = false }` dropped winit's Linux x11/wayland backends → `compile_error!("...not supported by winit")`; re-enabled `x11`+`wayland` features + matching apt deps. (2) `os::apply_window_effects` left `config`/`frame` unused when the `#[cfg(windows)]` call compiled out → `-D warnings` failure (only reachable once the build was fixed). Verified the full Linux build/clippy/test green via WSL (`CARGO_TARGET_DIR=/tmp/banquo-linux-target`) before pushing. **CI now green on ubuntu + windows** (run 27810160906, HEAD 10c47d4).
- **Completed:** 2026-06-19T00:00:00Z
- **Files modified:** Cargo.toml, Cargo.lock, .github/workflows/ci.yml, src/os/mod.rs
- **Commit:** `4f94cf8` + `10c47d4`

## T-1201 (sprint 12) — ensure_detached + win_detach guard
- **Description:** Added `os::ensure_detached()` (unconditional def + call; release-windows body re-spawns Banquo via the safe `CommandExt::creation_flags(CREATE_BREAKAWAY_FROM_JOB | DETACHED_PROCESS)`, exits original on Ok, runs in place on Err) plus `#[cfg(all(windows, any(not(debug_assertions), test)))] mod win_detach` with `should_detach`, `build_relaunch_command` (the real spawn-command builder, unit-tested), and release-only `run()`. Zero unsafe (ADR-002 intact). cfg design reconciled per critic C-002/C-008 — verified clean across all four targets: win debug/release/test clippy + Linux(WSL) clippy/test. 4 win_detach unit tests pass.
- **Completed:** 2026-06-20T00:00:00Z
- **Files modified:** src/os/mod.rs
- **Commit:** `9ca3a53`

## T-1202 (sprint 12) — call ensure_detached in main
- **Description:** Call `os::ensure_detached()` at the top of `main()`'s GUI path — immediately after the `Compose` subcommand early-return, before config load / PTY spawn / window. CLI path keeps its console (never detaches); debug/non-Windows is a called no-op. Verified clean on Windows debug+release clippy + 44/4 tests, and Linux clippy via WSL.
- **Completed:** 2026-06-20T00:00:00Z
- **Files modified:** src/main.rs
- **Commit:** `942cba6`
