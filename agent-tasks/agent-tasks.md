# Agent Tasks (Persistent Backlog)

## Sprint 0 (Milestone 1 — window + one Iosevka line)
(all Sprint 0 tasks complete)

## Sprint 1 (Milestone 2 — "It echoes")
<!-- T-100 completed; see completed-tasks.md. -->
<!-- All Sprint 1 tasks (T-100 through T-112) completed; see completed-tasks.md. -->

## Future Direction (noted 2026-06-15; for any dev running sprint loops — see ADR-007/008/009)

### Config & Ricing (TOML)
- [ ] Add support for TOML-based configurations to define themes, typefaces, textures, blur, and other effects.
- [ ] Make tab appearance configurable (rounded corners, beveled edges, colors) in the TOML file to let them 'pop'.
- [ ] Allow users to load external themes and configuration files to achieve "ricing" akin to what's possible on DEs like Hyprland.
- [ ] Implement "blur" to the window/tab transparency effects (which can be toggled on or off via the TOML config).

### Sprint 2: Window chrome & controls — ADR-008 (per-OS component, not core)
<!-- T-200 through T-205 completed; see completed-tasks.md. -->

### Sprint 3: Tabs & Auto-Collapse — ADR-007
<!-- T-300 through T-306 completed; see completed-tasks.md. -->

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
