# Agent Tasks (Persistent Backlog)

## Sprint 0 (Milestone 1 — window + one Iosevka line)
(all Sprint 0 tasks complete)

## Backlog — Milestone 3 (Typography) — see ADR-006
- [ ] Font registry: load faces from (a) curated embedded OFL set + (b) user
      font paths in the TOML config; honest fallback when a path is bad.
- [ ] Curated embedded MONO roster (OFL/Apache only): Iosevka (have), JetBrains
      Mono, IBM Plex Mono, Commit Mono, Monaspace, Geist Mono. Pick the flagship
      default (candidates: Commit Mono / Monaspace for "Klim-adjacent" feel).
- [ ] User-supplied PREMIUM support (no redistribution): Berkeley Mono (TX-02),
      Söhne Mono (Klim, app licence), MD IO (Mass-Driver), MonoLisa, etc.
- [ ] Per-role config: mono (grid) vs display (UI) families + weights.
