# Agent Tasks (Persistent Backlog)

## Sprint 0 (Milestone 1 — window + one Iosevka line)
(all Sprint 0 tasks complete)

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
