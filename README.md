# Banquo

*A most beautiful terminal — with a conscience about its own correctness.*

Banquo is a **100% Rust** GUI terminal emulator built around a clean seam between
**truth** (PTY bytes → parser → grid → cursor → scrollback; pure, deterministic,
GUI-unaware) and **appearance** (`view = render(snapshot, material)`; a pure
function of that truth). The truth-half is checkable in a headless harness; the
appearance-half is a layered *material engine* that ships four presets —
**Blanco**, **Zircon**, **Concrete**, and **Volcanic Glass**.

The full design argument lives in [`BANQUO_DESIGN.md`](./BANQUO_DESIGN.md).

> *Banquo gets kings, though he be none* — a terminal is the parent of every
> process it spawns and the author of none of their work. (§IX)

## Status

**Milestone 1 — "A window that is unmistakably yours."** ✅ *(current)*

A frameless, transparency-capable `eframe` + `wgpu` window painting one centered
line of monospace (Iosevka) text on a flat tinted field, with
`#![forbid(unsafe_code)]`. This proves the toolchain end-to-end (compile → window
→ font pipeline → wgpu backend) and lays down the truth/appearance module seam.
No PTY, grid, or materials yet.

The milestone roadmap (design §VI):

| # | Milestone | State |
|---|-----------|-------|
| 1 | Window + one Iosevka line | ✅ done |
| 2 | It echoes — `alacritty_terminal` core, PTY, snapshot handoff (becomes a real terminal) | next |
| 3 | Typography you'd brag about — metrics, `pixels_per_point`, wide/CJK, cursor, selection, scrollback | — |
| 4 | The layer compositor — Blanco + Concrete | — |
| 5 | Glass + the capability model — Zircon, 3-tier degradation | — |
| 6 | Fire — Volcanic Glass via custom WGSL (`CallbackTrait`) | — |
| 7 | The finish — command palette, config hot-reload, motion easing | — |

## Run it

```sh
cargo run
```

A borderless window opens showing the line in Iosevka on a flat field; the
desktop reads faintly through the substrate (the window is transparency-capable —
true Zircon glass is Milestone 5). Requires a working wgpu backend (D3D12/Vulkan/
Metal). Startup prints which monospace face loaded (`Embedded` = Iosevka).

```sh
cargo test                              # pure unit tests (font pipeline, install latch)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## The six guarantees (design §II)

Banquo dies for these; a feature request that violates one is answered *no*.

1. **No `unsafe` in Banquo's own crates** — `#![forbid(unsafe_code)]`, not `deny`.
2. **The core never blocks the frame** — a 2 GB `cat` may fall behind, never freeze the window.
3. **Monospace alignment is sacred** — every cell is exactly one (or two) cells wide; paint obeys the grid.
4. **Font size is a setting, not a function of window size** — resizing *reflows* (changes rows/cols via `SIGWINCH`); it never scales glyphs.
5. **A theme can never kill your shell** — truth and appearance are separate lifetimes; appearance is disposable.
6. **It tells the truth about what it can't do** — materials degrade visibly and honestly; Banquo never fakes a capability it lacks.

## What Banquo refuses to be (design §VII)

No tabs/splits/multiplexing (compose with a real WM), no config GUI (edit the
TOML in the terminal Banquo renders), no telemetry / auto-update / **network code
at all**, and no ligatures in v1. Its entire surface to the outside world is one
PTY and one config file.

## License

Banquo's own code: MIT OR Apache-2.0. The vendored Iosevka font
(`assets/fonts/`) is under the SIL Open Font License 1.1 — see
`assets/fonts/Iosevka-LICENSE.md`.
