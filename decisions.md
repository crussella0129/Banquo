# Architectural Decisions

Architecture Decision Records, newest first. Each records a choice that constrains
future work; later sprints' "ignored-ADR" review screens against this log.

---

## 2026-06-15 — ADR-001: Crate stack (eframe owns wgpu/winit) — *Accepted* (sprint 0)

**Context.** Banquo is a 100% Rust GUI terminal (design §I). The Face is built on
`egui` + `wgpu` (§IV); the window/event layer is `winit`. `eframe` bundles a
specific, mutually-compatible `wgpu` and `winit`; declaring those crates directly
at mismatched versions yields two incompatible copies in the dependency tree.

**Decision.** Depend on `eframe = "0.34"` with `default-features = false` and
features `["wgpu", "default_fonts"]`, plus `egui = "0.34"` directly (same minor,
for the types Banquo names). Let `eframe` transitively own `wgpu` and `winit`. A
direct `wgpu` dependency is added only at Milestone 6 (custom WGSL), matched to
eframe's resolved version via `cargo tree`.

**Consequences.** Single renderer backend (wgpu) from the first window; minimal
version-conflict surface; the wgpu version is upgraded by bumping eframe, not
independently. Revisit if a future milestone needs a wgpu feature ahead of
eframe's pin.

---

## 2026-06-15 — ADR-002: `#![forbid(unsafe_code)]`, not `deny` — *Accepted* (sprint 0)

**Context.** Guarantee #1 (design §II): no `unsafe` in Banquo's own crates. `deny`
can be locally overridden by an `#[allow(unsafe_code)]`; `forbid` cannot.

**Decision.** Every Banquo crate root carries `#![forbid(unsafe_code)]`.
Dependencies may use `unsafe` internally — that is their ownership graph, not
ours.

**Consequences.** Banquo's own code is fully accountable and cannot be quietly
opted out by a future tired version of us. If a milestone genuinely needs
`unsafe` (it should not), removing `forbid` is a deliberate, reviewable ADR
change — exactly the friction intended.

---

## 2026-06-15 — ADR-003: Truth/appearance seam as the organizing boundary — *Accepted* (sprint 0)

**Context.** The design's spine (§I, §IV) is a clean seam between *truth* (PTY →
parser → grid → cursor → scrollback; pure, deterministic, GUI-unaware) and
*appearance* (`view = render(snapshot, material)`; a pure function of the truth).
This is what makes "mathematically beautiful Rust" *checkable*.

**Decision.** Express the seam structurally from day one. At Milestone 1 (no
truth-half yet) it lives as module boundaries — `app` + `fonts` are appearance.
At Milestone 2, when the truth-half gains real content (PTY + grid + `Snapshot`),
the seam is promoted to a Cargo workspace split: `banquo-core` (truth) +
`banquo-face` (appearance). Nothing in appearance ever writes truth.

**Consequences.** The core can be unit-tested headlessly; the Face can be
re-themed/crash-restarted without the session noticing (guarantees #2, #5). The
`[workspace]` table is reserved in `Cargo.toml` for the M2 promotion.

---

## 2026-06-15 — ADR-004: Build the truth-half on `alacritty_terminal` — *Accepted (deferred use — Milestone 2)* (sprint 0)

**Context.** The VT parser/grid is a conformance target (one correct behavior for
`CSI 2 J`), not a place to be original (design §III). Hand-rolling it risks
subtle divergence; the originality belongs in the material engine and
architecture.

**Decision.** Use `alacritty_terminal` (safe Rust, the proven core inside Zed) as
the truth-half engine, wrapped behind Banquo's own `Grid`/`Cell`/`Snapshot` types
so no downstream code imports an `alacritty_*` type. The seam keeps the engine
replaceable. Recorded now; first used at Milestone 2.

**Consequences.** Faster, correct grid behavior; the adapter boundary is extra
code but buys engine-swappability. If we ever forge our own parser, only the
adapter changes.
