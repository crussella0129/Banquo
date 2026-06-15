# BANQUO
### A terminal with a conscience about its own correctness

*This is a design document written as an argument, not a checklist. Every choice has a reason; disagree with the reasons and the choices fall. Two things are fixed by you and I won't touch them: it is **100% Rust**, and it ships four presets named **Blanco, Zircon, Concrete, Volcanic Glass**. Everything else here is my taste, defended.*

---

## I. What Banquo actually is

Most terminals are *renderers that happen to run a shell*. The shell is upstream; the terminal is a dumb pane that paints whatever bytes arrive. Banquo inverts the emphasis. Banquo is **a verifiable state machine that happens to have a beautiful face**. The face matters — but it is downstream of a core whose correctness you can reason about, because that reasoning is the whole point of choosing Rust in the first place.

So the organizing principle is not "look good." It's **a clean seam between *truth* and *appearance***:

- **The truth half** — PTY bytes, parser, grid, cursor, scrollback — is pure, synchronous, deterministic, and has *no idea Banquo has a GUI*. You could run it headless in a test harness and assert every cell. No colors, no fonts, no wgpu. Just: given these bytes, the grid is exactly this.
- **The appearance half** — fonts, materials, shaders, animation — is a pure function *of* that truth. `view = render(snapshot_of_grid, material)`. It reads the truth; it never writes it.

Why this is the right spine and not just tidiness: it's the only architecture where your "mathematically beautiful Rust" claim is *checkable*. Beauty you can't test is decoration. A core that is a referentially-transparent function from byte-stream to grid-state is beautiful in the way a proof is beautiful — and you can write the proof as tests. That is the version of this project worth your nights after the 15-month-old is asleep.

---

## II. The six guarantees (the things Banquo refuses to get wrong)

A design has taste when it knows what it will *die* for. Banquo dies for these:

1. **No `unsafe` in Banquo's own crates.** `#![forbid(unsafe_code)]` — `forbid`, not `deny`, so it can't be locally overridden by a future tired version of you. Dependencies may use unsafe internally; that's their ownership graph, not yours. The boundary is honest: *your* code is fully accountable.
2. **The core never blocks the frame.** Rendering at 120fps and a `cat`-ing of a 2GB file are independent events. A flood of output may fall *behind*, but it may never *freeze the window*. This is an architectural promise, enforced by the threading model in §IV, not a hope.
3. **Monospace alignment is sacred.** Every cell is exactly one cell wide (or exactly two, for wide glyphs). No font, ligature, or material effect is ever allowed to shift a glyph off its grid coordinate. The grid is law; the paint obeys it.
4. **Font size is a setting, not a function of window size.** Resizing the window *reflows* — it changes how many rows and columns fit and tells the shell via the PTY (`SIGWINCH`); it **never** scales the glyphs. Text is the same legible size in a tiny split and fullscreen. Physical apparent size is held constant across displays via `pixels_per_point` (DPI scaling), so a 14pt font looks like 14pt on a 1080p laptop and a 4K external alike. The window's geometry adapts to the text; the text never shrinks to fit the geometry. *This is the guarantee that the deleted "Golden Curve" would have violated — geometry derived from window size is exactly how a terminal gets this wrong.*
4. **A theme can never kill your shell.** Reloading `volcanic.toml` while a 4-hour SSH session runs must not drop a single byte of that session. Truth and appearance are separate lifetimes; appearance is disposable.
5. **It tells the truth about what it can't do.** On a compositor with no blur, Zircon does not fake it badly — it degrades to honest transparency and *says so* if asked. Banquo never pretends. This same honesty governs the whole platform story (§IX): the core is universal, the materials degrade gracefully and visibly, and Banquo never ships a material that silently looks broken on a platform that can't support it.

If a feature request violates one of these, the answer is no. That's what taste is.

---

## III. The grid engine — my actual opinion

You build on **`alacritty_terminal`** as the truth-half core, and you do it without guilt.

Here's the design-taste argument, separate from the time argument I made before. The temptation is to think hand-writing the VT parser is the "pure" choice — that a real craftsman forges his own nails. But look at what that core *is*: it's a faithful implementation of a 1970s DEC state machine plus forty years of xterm barnacles. There is exactly one correct behavior for `CSI 2 J`, and it is not yours to reinvent — it's a spec to *conform to*. Hand-rolling it doesn't express taste; it just risks subtly diverging from the one right answer, and you'd spend your creativity debugging why `tmux` draws garbage instead of building the thing only Banquo will have.

**Craft is choosing the right altitude to be original at.** Banquo's altitude is the material engine and the architecture, not the SGR table. `alacritty_terminal` is safe Rust, it's the proven core inside Zed, and it leaves the grid as a clean readable snapshot — exactly the "truth" surface §I wants. Wrap it so the rest of Banquo never imports an `alacritty_*` type directly: define your own `Grid`/`Cell`/`Snapshot` types and adapt at the seam. Then if you ever *do* want to forge your own nails, you swap the core and nothing downstream notices. That's the mature version of purity: a boundary so clean the engine behind it is replaceable.

---

## IV. Architecture — three actors, one direction of flow

```
   ┌─────────────┐   bytes    ┌──────────────┐  snapshot   ┌─────────────┐
   │  PTY READER │ ─────────► │   THE CORE   │ ──────────► │  THE FACE   │
   │  (OS thread)│            │ (truth half) │  (lock-free │ (UI thread, │
   │             │ ◄───────── │              │   handoff)  │  egui+wgpu) │
   └─────────────┘   resize   └──────────────┘ ◄────────── └─────────────┘
        ▲                                         keystrokes      │
        │                                                         │
        └──────────────── keystrokes routed to PTY ───────────────┘
```

Three actors, and the data flow is a **loop with a single writer at each stage** — the discipline that makes it reasonable:

- **PTY Reader** (dedicated OS thread): the only thing that touches the shell's output fd. Reads raw bytes, hands them to the core. Blocking I/O lives here and *only* here, so guarantee #2 holds by construction.
- **The Core** (owns the grid): consumes bytes, advances the state machine, owns the single source of truth. It publishes immutable **snapshots** — think of each frame as a git commit of the grid. The UI never holds a lock on the live grid; it reads the latest published snapshot. (Mechanism: double-buffer / `arc-swap`. The reader writes the next buffer, atomically swaps the pointer, the UI always sees a complete consistent frame — never a half-updated grid mid-flicker.)
- **The Face** (UI thread, `egui` + `wgpu`): reads the newest snapshot, paints it through the active material. Captures keystrokes and ships them back to the PTY. It is a *pure function of snapshot + material + time*. Given the same three, it paints the same pixels — which means you can screenshot-test it.

This is the architecture that earns guarantee #2 and #5 for free instead of bolting them on. The shell session lives entirely in the Reader+Core; the Face is a disposable view that can be rebuilt, re-themed, or crash-and-restarted without the session noticing.

**Resize, concretely (guarantee #4 in practice).** On a window resize the Face computes `cols = floor((width − 2·padding) / cell_w)` and `rows = floor((height − 2·padding) / cell_h)` using the *current, fixed* font metrics, then sends that grid size down to the core, which resizes the `Term` and fires `SIGWINCH` so the shell reflows. Font size never enters this calculation — it's an input, not an output. Because `floor` almost never divides evenly, there's a leftover strip of a few pixels on the right/bottom edge; Banquo absorbs it into the padding (centering the grid, or biasing the slack to one edge — a one-line aesthetic choice) rather than stretching cells to fill it. Stretching cells to consume the remainder is the other classic way to break monospace alignment, so the grid stays rigid and the window wears the slack.

---

## V. The material engine — where the originality actually lives

This is Banquo's reason to exist, so it gets the real design thought. Your four presets aren't four hardcoded modes; they're four *points sampled from one expressive space*. Get the space right and the presets fall out — plus everything between them that users will invent.

### The compositing model: every glyph is painted in passes

A flat terminal does one pass: foreground glyph on background color. Banquo's renderer thinks in an ordered stack of **layers**, and a material is just a recipe for which layers are on and how they combine:

```
  ┌─ LAYER 4 · OVERLAY ····· grain, scanlines, vignette, active-row radiance
  ├─ LAYER 3 · GLYPH ······· the text itself — possibly multi-pass (emboss), possibly shaded
  ├─ LAYER 2 · TEXTURE ····· sampled image / procedural surface behind text
  ├─ LAYER 1 · TINT ········ flat base color or gradient
  └─ LAYER 0 · SUBSTRATE ··· what's *behind the window* — opaque, transparent, or compositor-blurred
```

Every preset is one configuration of this stack. That's the entire trick, and it's why the engine is general rather than four `if` branches.

### The materials, designed against the model

**Blanco — *The Canvas.*** Substrate opaque white. The risk with pure `#FFFFFF` is a dead, retina-searing plane, so the overlay layer carries a **sub-pixel structural grid at ~3% opacity** — not decoration, *tooth*. It gives the eye something to register the surface against, the way good paper is never optically flat. Glyphs are near-black but not `#000` — a hair of warmth so the contrast doesn't ring. Blanco is the material for someone who wants the terminal to feel like a drafting table. *It is the hardest one to make feel alive, which is exactly why it should exist — it proves the engine can do restraint.*

**Zircon — *The Glass.*** Substrate = transparent, and the blur is **requested from the compositor, never computed in-app**. This is a real design decision, not a limitation I'm working around: a terminal has no business re-blurring the whole desktop behind it every frame — that's the window manager's job, it's already doing it for everything else, and doing it yourself means fighting the render pass with hacks. So Banquo marks itself transparent and lets Hyprland / KWin / whoever do what they're good at. The glyph layer renders at ~0.9 alpha so desktop light bleeds faintly through the letterforms. **The honest catch I'm designing around up front:** text antialiasing over a transparent substrate has nothing stable to blend against and goes muddy. So Zircon paints a near-invisible **contrast scrim** — a barely-there darkening directly under glyph runs only, not the whole pane — giving the AA something to bite without killing the glass effect. That scrim is the difference between "looks incredible in a screenshot" and "readable during a 9-hour workday."

**Concrete — *The Stone.*** Substrate = a seamless high-res greyscale texture. The signature move is the glyph layer running **two passes**: first a +1 *physical*-pixel highlight in a lighter grey, then the primary dark fill on top. The eye reads the offset highlight as a light source above, so the text looks **pressed into** the stone — embossed, recessed, physical. The reason this needs the §IV/physical-pixel discipline: on a fractional-scaled HiDPI display, "+1 logical pixel" is 1.5 real pixels and the emboss smears into a blur. It must be +1 *device* pixel, which means the renderer has to know its `pixels_per_point` and round to the physical grid. Concrete is where the geometry rigor stops being abstract.

**Volcanic Glass — *The Plasma.*** Substrate = true `#000000` (OLED pixels physically off — on the right monitor the window's dead space *is* the bezel). The glyph layer routes through a **custom WGSL shader** giving an iridescent red/purple aura, low and slow. And the overlay layer holds the one piece of genuine motion design: **active-row radiance** — the cursor's current line gets a soft radial gradient that *breathes*, driven by the snapshot's cursor position from the core. It's a heartbeat. It tells you where you are without a hard highlight bar. This is the preset that shows off the shader pipeline, and it should be a little *too much* — a maximalist counterweight that proves the same engine doing Blanco's restraint can also do excess.

### The space between

Because these are stack-configs in TOML, a user can cross them — Concrete's emboss on a near-black substrate, Zircon's transparency with Volcanic's shader, Blanco's grid over a faint texture. **You ship four corners; the engine is the whole room.** That generality is the gift, and it's only possible because you refused to hardcode the themes.

---

## VI. The build, in the order that respects dependencies

I'm not numbering these as rigid phases — I'm ordering them so each step stands on solid ground and *each one ends at something you can actually run*. Learning-by-doing means every milestone should produce a thing that does something, not a layer of scaffolding you can't see.

**Milestone 1 — A window that is unmistakably yours.** `cargo new`, `forbid(unsafe_code)`, `eframe`+`wgpu`, frameless and transparent. Paint nothing but a single hardcoded line of text in Iosevka on a flat field. *Runs at:* a window opens. Trivial, but it proves the whole toolchain and the font pipeline in one afternoon, and you've typed every line.

**Milestone 2 — It echoes.** Wrap `alacritty_terminal` behind your own `Snapshot` type. Spawn the shell on a PTY thread, wire the snapshot handoff (§IV), route keystrokes back. Paint the snapshot as plain monochrome cells. *Runs at:* you can type `ls`, `cd`, and see output. **This is the day it becomes a terminal.** Gate it hard: it doesn't advance until `vim` and `htop` render and respond correctly — those two exercise alt-screen, cursor addressing, and full SGR, so if they work the truth-half is sound.

**Milestone 3 — Typography you'd brag about.** Real font loading (Iosevka / Geist Mono / IBM Plex Mono), correct metrics, the `CellMetrics` layer carrying `pixels_per_point`. Wide-glyph and CJK width correct. Cursor shapes, selection, scrollback view. *Runs at:* it's a genuinely nice plain terminal you could daily-drive. Ship nothing else until this is true — a beautiful material on a janky grid is lipstick.

**Milestone 4 — The layer compositor.** Build the §V layer stack as the rendering model, then light up the two cheap corners first: **Blanco** (substrate + grid overlay) and **Concrete** (texture substrate + dual-pass emboss). These need no shader, so they validate the compositing model on the `Painter` before you touch wgpu. *Runs at:* two real materials, switchable from TOML.

**Milestone 5 — Glass, and the capability model.** **Zircon.** This is where the `trait Substrate` and runtime capability detection (§VIII) get built, because Zircon is the first material that needs them. Transparent substrate, compositor blur request, the contrast scrim — plus the three-tier degradation (full glass → honest transparency → frosted tint) wired to detected capabilities. Do it as a deliberate spike: it has three independent risks (the blur handshake with the WM, text-AA readability, and clean fallback when blur is absent) and you want them surfaced early, not at the finish line. *Runs at:* Banquo is transparent and readable over your wallpaper on a blur-capable compositor, and *gracefully* translucent on one without.

**Milestone 6 — Fire.** **Volcanic Glass.** Drop into wgpu's `CallbackTrait` (the *supported* custom-render path — the official `custom3d` pattern, never the render-pass `transmute` hack that floats around blog posts). WGSL for the iridescent glyph aura; the breathing active-row radiance wired to the cursor snapshot. *Runs at:* the showpiece preset, and your shader pipeline is proven.

**Milestone 7 — The finish that makes it feel intentional.** Command palette as a floating `egui` layer over the grid. Config hot-reload (`notify` watching the `.toml`) that rebuilds the material **without dropping the PTY** — guarantee #5 made visible. Cursor-motion easing and smooth-scroll interpolation. *Runs at:* it feels designed, not assembled.

---

## VII. The things I'd refuse to add (taste is also subtraction)

- **No tabs, no splits, no multiplexing.** That's `tmux`/`zellij`'s job and they're better at it than Banquo will be. A terminal that tries to be a window manager dilutes the one thing it's for. Banquo is a perfect single pane. Compose it with a real WM — which, on LogOS, *you control*.
- **No config GUI.** The config is a TOML file you edit in the terminal Banquo is rendering. The tool configures itself. That recursion is the aesthetic.
- **No telemetry, no auto-update, no network code at all.** Banquo never opens a socket. A terminal that phones home is a contradiction in trust. Its entire surface area to the outside world is one PTY and one config file.
- **No ligature support in v1.** Controversial, I know. But programming ligatures fight guarantee #3 (one glyph, one cell) and the correct handling is genuinely hard. Ship rock-solid monospace first; earn ligatures later as an opt-in that's clearly bounded.

---

## VIII. Portability — universal core, honest materials

The question "is Banquo universally compatible?" has two answers, and conflating them is how cross-platform apps end up mediocre everywhere. Banquo separates them cleanly, because the truth/appearance seam (§I) was built for exactly this.

**The core is universal, full stop.** The truth-half — PTY, parser, grid, cursor, scrollback — is pure Rust computation over a cross-platform PTY abstraction. `portable-pty` (the same one WezTerm ships) presents one trait over Unix `openpty`/`forkpty`, macOS, and Windows **ConPTY**; `wgpu` runs natively on **Vulkan, Metal, D3D12, and OpenGL**; `winit`/`eframe` cover Windows, macOS, Linux (X11 *and* Wayland), and the BSDs. So on **Linux, macOS, Windows 10 (1809+), FreeBSD/GhostBSD, and OpenBSD**, Banquo *runs and is a correct, fast, fully-functional terminal*. That is genuine universality and it costs nothing extra — it's what choosing this stack buys you.

**The materials are where the OS stops being an abstraction.** A substrate (Layer 0 in §V) is the one layer that reaches *outside* the window into the desktop — transparency, compositor blur, true-black bleed-into-bezel. That's not portable in principle, because the thing it depends on (the compositor) is a different program with different capabilities on every platform. Pretending otherwise would force the lowest common denominator and make all four presets blander everywhere. Banquo refuses that trade. Instead:

### The capability model

Banquo queries what the running environment can actually do and **each material declares which capabilities it wants**. A capability is a runtime fact, not a compile-time target:

```
  TRANSPARENCY     — can the window have a non-opaque framebuffer?
  COMPOSITOR_BLUR  — will the WM blur what's behind a transparent window?
  TRUE_BLACK_OLED  — (informational) is this likely an OLED panel?
  CUSTOM_SHADER    — is a usable wgpu backend present?  (always yes in practice)
```

| Capability | Linux/Wayland | Linux/X11 | macOS | Windows 11 | Windows 10 | FreeBSD/OpenBSD |
|---|---|---|---|---|---|---|
| Transparency | ✓ (compositor) | ✓ (with compositor) | ✓ native | ✓ native | ✓ native | ✓ if compositor present |
| Compositor blur | ✓ Hyprland/KWin/etc. | ✗ (rare) | ✓ native vibrancy | ✓ Acrylic/Mica | ~ partial | ✗ typically |
| Custom WGSL shader | ✓ Vulkan | ✓ Vulkan/GL | ✓ Metal | ✓ D3D12/Vulkan | ✓ D3D12 | ✓ Vulkan/GL |

The ✗ marks are not bugs to fix. They are facts to respect.

### How each material behaves across the gradient

- **Blanco** — needs nothing but `CUSTOM_SHADER` (effectively always present). **Pixel-identical on every platform.** This is the universal floor: if everything else degraded, Blanco alone would still make Banquo worth using everywhere.
- **Concrete** — texture substrate + dual-pass emboss. Pure in-window rendering, touches no compositor. **Also universal.** The only platform variable is HiDPI rounding (the +1 *device*-pixel emboss), which the `CellMetrics` layer already handles per-display.
- **Zircon** — *wants* `TRANSPARENCY` + `COMPOSITOR_BLUR`. Full glass on Wayland-with-blur, macOS, Windows 11. Where blur is absent (X11, BSD, Win10): **degrades to clean honest transparency** — still translucent, just unblurred — and reports that it's doing so. Where transparency itself is absent: falls back to a flat frosted-tint substrate that *evokes* glass without lying about being it. Three tiers, each deliberately designed, none broken.
- **Volcanic Glass** — wants `CUSTOM_SHADER` (universal) + benefits from `TRUE_BLACK_OLED`. The shader runs everywhere wgpu does, so the iridescent glyph aura and breathing active-row radiance are **universal**. The only thing OLED-dependent is the bezel-blend illusion, which is a panel property, not a software capability — on an LCD you simply get very dark grey instead of pixels-off black. The effect is intact; only the physics of the monitor differ.

**The net:** two of your four presets (Blanco, Concrete) are pixel-universal. Volcanic Glass is functionally universal with one panel-dependent flourish. Only Zircon is genuinely tiered — and it was *always* going to be, on any honest terminal, because frosted glass is a compositor effect and compositors are not uniform. Banquo's answer is to make every tier a designed state rather than an accident.

### One platform-specific build note

The substrate layer is the *only* place `#[cfg(target_os = ...)]` is permitted to appear in the codebase. Capability detection and the per-OS transparency/blur handshake live behind a single `trait Substrate` with platform implementations; everything above Layer 0 is platform-agnostic and never sees a `cfg`. This keeps guarantee #1 (no unsafe) and the truth/appearance seam intact: portability complexity is quarantined to one trait, one layer, the lowest one. If a sixth platform appears, you implement one trait and nothing else moves.

### What this does *not* support, honestly

- **Web/WASM:** no. A browser tab cannot spawn a PTY and a host shell; that's not a Banquo limitation, it's what a terminal *is*. Ruled out by definition, not effort.
- **Windows before 10 1809:** no ConPTY, so no. Hard floor, stated plainly.
- **Headless/SSH-only boxes:** Banquo is a GUI. For a remote box you run your shell over SSH *inside* Banquo on your local machine — the universal core renders it identically regardless of where the shell physically lives.

---

## IX. Why "Banquo," really

Macbeth is shown Banquo's line in a mirror — heirs stretching to the crack of doom, *"what, will the line stretch out to the crack of doom?"* — kings Banquo sires but never becomes. A terminal is precisely that mirror. The window is the glass; every command you run spawns a child process, a lineage of PIDs descending from a shell that is itself a child of the terminal. Banquo *gets kings, though he be none*: it is the parent of everything that runs inside it, and the author of none of their work.

That's not a name chosen for theatrical weight. It's the actual computational relationship — a terminal is a fork() made visible — and the fact that it's also a great line from the best play about ambition and consequence is the kind of coincidence you build a whole aesthetic on. The brutalist materials, the refusal to do more than one thing, the obsession with a provably-correct core: it's all a terminal that takes seriously that it is the origin point, the looking-glass, and wants to be *worthy* of the lineage it spawns.

---

*Fixed by you: 100% Rust, and the four presets. Everything else above is one defensible set of choices — argue with any of it. The best version of Banquo is the one where you've fought me on at least three of these and won.*

*Portability verdict, in one line: the core is universal across Linux, macOS, Windows 10+, and the BSDs; two presets are pixel-identical everywhere, one is functionally universal, and only Zircon is tiered — by the nature of frosted glass, not by compromise. The seam that made the architecture beautiful is the same seam that made it portable. That's not a coincidence; it's the whole argument.*
