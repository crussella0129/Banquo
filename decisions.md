# Architectural Decisions

Architecture Decision Records, newest first. Each records a choice that constrains
future work; later sprints' "ignored-ADR" review screens against this log.

---

## 2026-06-15 — ADR-009: Platform strategy — Unix-compatible base first, OS-specific components on top — *Accepted* (sprint 0; built later) — *extends design §VIII*

**Context.** The design's §VIII already quarantines portability behind a single
`trait Substrate` (the only place `#[cfg(target_os)]` is allowed). The user wants
this generalized beyond the substrate: **build the Unix-compatible base, then layer
OS-specific compatibility components.** Concretely surfaced on this Windows machine:
the frameless window is "too minimal" and PowerShell/elevation needs differ from
Unix.

**Decision.** Establish a base layer that targets the Unix model (Linux/macOS/BSD:
one PTY, `sudo` inside the shell, no "admin mode" concept), then add **per-OS
components** that compose on top, behind traits like the existing `Substrate` — for
window chrome (ADR-008), shell wiring, and privilege/elevation. Windows-specific:
- Wire to **PowerShell** specifically (in addition to ConPTY generic spawn) so the
  shell can be launched **elevated (admin)** when requested.
- When running elevated, show a **shield indicator** (top-left). The base has *no*
  shield — Unix has no admin-mode concept, so the indicator is a Windows-only
  component, not in the universal core.

**Consequences.** The universal core stays platform-agnostic; OS quirks live in
named, optional components. New platform = implement the relevant component traits,
nothing above moves. Keeps guarantee #1 (no unsafe) and the truth/appearance seam
intact.

---

## 2026-06-15 — ADR-008: Window chrome as an overridable component — *Accepted* (sprint 0; built later)

**Context.** Banquo is frameless (`with_decorations(false)`, design §VII / M1). That
means **no native title bar** — no mouse drag-to-move, no resize handles, no close
button. Confirmed painful on Windows now: the window can't be mouse-dragged or
closed (Alt+F4 still works). Doing custom drag/resize well is non-trivial.

**Decision.** Build a **window-chrome component** that provides drag-to-move, resize
affordances, and a **close control** — but make it **overridable / supersedable by
the DE/compositor's native window management** where that exists (so we don't fight
the WM on Linux/Wayland). The close affordance is a small stylized "×" / window-
closing icon in the **top-right**, and it **appears and disappears together with
the tabs** (ADR-007).

**Consequences.** Frameless aesthetic is preserved while the window becomes usable
on bare Windows; on a capable compositor the native controls can take over. The
component is part of the per-OS layering (ADR-009), not the universal core.

---

## 2026-06-15 — ADR-007: Collapsing terminal tabs — *Accepted, REVISES design §VII* (sprint 0; built later)

**Context.** Design §VII says **"No tabs, no splits, no multiplexing"** — compose
with a real WM/`tmux`. The user (the author of that constraint) is **deliberately
overriding the tabs part**: they want tabs, but unobtrusive ones.

**Decision.** Add **terminal tabs** that **auto-collapse**: the tab strip (and the
top-right close icon, ADR-008) is hidden by default and **reveals when the cursor
moves to the top edge of the window**, hiding again when it leaves. To keep §VII's
spirit (don't become `tmux`): **tabs only — no splits, no panes, no multiplexing
logic.** Each tab is an independent PTY+core; the Face just switches which snapshot
it renders.

**Consequences.** A real revision of the design doc — §VII's "no tabs" no longer
holds; "no splits/multiplexing" still does. The truth/appearance seam makes this
clean: N independent cores, one Face selecting among their snapshots. Revisit §VII
prose in `BANQUO_DESIGN.md` when convenient.

---

## 2026-06-15 — ADR-006: Font strategy — curated OFL defaults + user-supplied premium via config — *Accepted* (sprint 0; built at Milestone 3)

**Context.** The user wants a *sophisticated, design-language-grade* monospace
selection (Klim-tier), open to licensed premium faces. But Banquo embeds fonts in
the binary (`include_bytes!`) and the repo is on GitHub — and the standout premium
monospaces (**Berkeley Mono**, Klim's **Söhne Mono**, Mass-Driver's **MD IO**,
**MonoLisa**, Operator Mono, PragmataPro) are **licensed and may not be
redistributed/embedded** in a public repo. We also keep two type *roles*: mono
(grid, must be monospace — guarantee #3) and display (UI/hero, currently Geist).

**Decision.** A two-source font registry:
1. **Curated embedded defaults — OFL/Apache only**, so out-of-box quality is high
   with zero license risk. Mono candidates: Iosevka (current), JetBrains Mono,
   IBM Plex Mono (named in §VI M3), Commit Mono, Monaspace, Geist Mono. Display:
   Geist (in).
2. **User-supplied premium — load by path from the TOML config**, never vendored.
   This lets a user point Banquo at *their own licensed* Berkeley Mono / Söhne
   Mono / MD IO on their machine. Matches the design's config-driven, no-network,
   honest ethos (§VII) — Banquo ships great free faces and *respects* licensing
   rather than pirating it. (If we ever bundle a premium face, it needs an app
   font licence, e.g. Klim's perpetual App Font Licence.)

**Consequences.** Default install is legally clean and looks good; power users get
foundry-grade faces without Banquo redistributing them. The font system must
support runtime loading from arbitrary paths (validated, with honest fallback —
guarantee #6) in addition to the embedded set. egui weight-axis limitation
(ADR-noted in `fonts.rs`) means weights remain discrete static faces. Built at
Milestone 3 ("Typography you'd brag about").

**Licensing clarification (publishing as a crate).** Whether a published crate is
"immutable and unencrypted" is **irrelevant** to font licensing — and unencrypted
actually makes it *worse*, because the `.ttf` is trivially extractable by anyone
who downloads the crate. Publishing to crates.io is **public redistribution** of
whatever font bytes are embedded. So:
- **OFL/Apache faces (Geist, Iosevka, JetBrains Mono, …): fine to bundle** in a
  public crate — OFL explicitly permits redistribution. **Geist Light stays**, and
  we add **one OFL serif** display face (user wants Geist Light + one serif, not a
  pile of fonts).
- **Klim / Berkeley / MD IO / other licensed faces: NOT okay** to ship in a public
  crate, regardless of format. A foundry "app font licence" lets you embed a font
  in *your distributed application*, but an open crate exposing the raw `.ttf`
  effectively grants every downstream user the font for any use — beyond that
  licence. Keep them **out of the published crate**; load from the user's machine
  by config path. (In Banquo's *current private* repo, bundling your own licensed
  copy for personal use is fine — the constraint bites only on public release.)

---

## 2026-06-15 — ADR-005: Milestone-1 substrate = flat tinted field + zero-alpha clear — *Accepted* (sprint 0)

**Context.** The user's instruction for M1 was a "frameless transparent window";
the design doc's §VI M1 says one line "on a flat field," and reserves true glass
(Zircon, transparent substrate + compositor blur + contrast scrim) for Milestone
5. A naive full alpha-0 clear leaves no flat field and exposes the text-AA-over-
nothing muddiness the design explicitly flags for Zircon (§V).

**Decision.** Separate the two: the framebuffer `clear_color` is fully transparent
(`[0,0,0,0]`) so the window is genuinely transparency-capable, while the visible
substrate is a near-opaque (~0.92 alpha) flat tinted field painted on top. This is
"transparency-capable, not yet Zircon." The eframe `App` is implemented against
the 0.34 `logic`/`ui` split (not the deprecated `update`); the install-once font
latch lives in `logic`, painting in `ui`.

**Consequences.** Honors both the user's transparency ask and the design's M1
"flat field" + M5 glass sequencing; gives glyph AA a stable backing. When Zircon
lands (M5) it replaces this flat field with the real `trait Substrate` + capability
model — this ADR is the explicit placeholder it supersedes.

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
