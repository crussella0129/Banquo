# Agent Tasks (Persistent Backlog)

## Sprint 0 (Milestone 1 — window + one Iosevka line)
(all Sprint 0 tasks complete)

## Sprint 1 (Milestone 2 — "It echoes")
- [ ] T-100: GitHub Actions CI (`.github/workflows/ci.yml` — test + clippy + fmt on ubuntu + windows)
- [ ] T-101: Add alacritty_terminal, portable-pty, arc-swap deps (`Cargo.toml`)
- [ ] T-102: Define Snapshot truth surface (`src/core/snapshot.rs`, `src/core/mod.rs`)
- [ ] T-103: Color translation resolve_color (`src/core/term.rs`)
- [ ] T-104: BanquoTerm wrapper — advance, resize, build_snapshot (`src/core/term.rs`)
- [ ] T-105: PTY spawn — open_pty (`src/core/pty.rs`)
- [ ] T-106: Session reader thread + ArcSwap publish (`src/core/session.rs`)
- [ ] T-107: CellMetrics grid_size pure math (`src/metrics.rs`)
- [ ] T-108: Grid render — Face paints snapshot cells (`src/app.rs`)
- [ ] T-109: Keystroke encoding — encode_key → PTY bytes (`src/app.rs`)
- [ ] T-110: Resize wiring — size change → SessionHandle::resize (`src/app.rs`)
- [ ] T-111: Main.rs wiring — spawn Session, pass to Face (`src/main.rs`)
- [ ] T-112: ADR-010 + README M2 update (`decisions.md`, `README.md`)

## Future Direction (noted 2026-06-15; for any dev running sprint loops — see ADR-007/008/009)

### Window chrome & controls — ADR-008 (per-OS component, not core)
- [ ] Custom window-chrome COMPONENT: drag-to-move + resize handles + close
      affordance, because frameless = no native chrome. On Windows the window
      currently can't be mouse-dragged/closed (Alt+F4 works); this fixes it.
- [ ] Make the component OVERRIDABLE/supersedable by DE/compositor-native window
      controls (don't fight the WM on Wayland/macOS).
- [ ] Close control = small stylized "×" / window-closing icon, top-right;
      appears/disappears TOGETHER WITH the tabs.

### Tabs — ADR-007 (REVISES design §VII "no tabs")
- [ ] Terminal tabs that AUTO-COLLAPSE: tab strip + top-right close icon hidden
      by default, reveal when the cursor goes to the top edge, hide on leave.
- [ ] Tabs ONLY — no splits/panes/multiplexing (keep §VII's spirit). Each tab =
      independent PTY+core; Face switches which snapshot it renders.

### Platform layering — ADR-009 (extends §VIII)
- [ ] Build the Unix-compatible BASE first, then OS-specific compatibility
      components on top (mirrors `trait Substrate`).
- [ ] Windows: wire to PowerShell specifically; support launching ELEVATED
      (admin); show a SHIELD icon (top-left) when elevated. Base (Unix) has no
      admin-mode concept (sudo in-shell) → shield is Windows-only, not in core.

### Typography near-term (refines ADR-006)
- [ ] Keep Geist Light (OFL, ships fine even in a public crate). Add ONE OFL
      serif display face. Target = Geist Light + one serif, no font pile.

## Backlog — Milestone 3 (Typography) — see ADR-006
- [ ] Font registry: load faces from (a) curated embedded OFL set + (b) user
      font paths in the TOML config; honest fallback when a path is bad.
- [ ] Curated embedded MONO roster (OFL/Apache only): Iosevka (have), JetBrains
      Mono, IBM Plex Mono, Commit Mono, Monaspace, Geist Mono. Pick the flagship
      default (candidates: Commit Mono / Monaspace for "Klim-adjacent" feel).
- [ ] User-supplied PREMIUM support (no redistribution): Berkeley Mono (TX-02),
      Söhne Mono (Klim, app licence), MD IO (Mass-Driver), MonoLisa, etc.
- [ ] Per-role config: mono (grid) vs display (UI) families + weights.
