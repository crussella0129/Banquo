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

A frameless, transparency-capable `eframe` + `wgpu` window with
`#![forbid(unsafe_code)]`, proving the toolchain end-to-end (compile → window →
font pipeline → wgpu backend) and laying down the truth/appearance module seam.
No PTY, grid, or materials yet.

It currently paints a small **typographic specimen** on a flat tinted field —
the hero tagline plus a Geist weight ladder (Thin → Black) and one Iosevka
monospace line — exercising Banquo's two font roles:

- **Mono / grid:** **Iosevka** (`banquo-mono`). The terminal grid must be
  monospace (guarantee #3); this is the face it will use.
- **Display / UI:** **Geist**, a proportional family registered as a discrete
  weight ladder (egui can't drive a variable font's weight axis, so each weight
  is its own static face). For the hero now, the command palette later — never
  the grid.

The milestone roadmap (design §VI):

| # | Milestone | State |
|---|-----------|-------|
| 1 | Window + one Iosevka line | ✅ done |
| 2 | It echoes — `alacritty_terminal` core, PTY, snapshot handoff (becomes a real terminal) | ✅ done |
| 3 | Typography you'd brag about — metrics, cursor, hot-swappable TOML Configurator | ✅ done |
| 4 | The layer compositor — multi-tab support, dynamic Grid Auto-Snap rendering | ✅ done |
| 5 | Glass + the capability model — Zircon, 3-tier degradation | next |
| 6 | Fire — Volcanic Glass via custom WGSL (`CallbackTrait`) | — |
| 7 | The finish — command palette, config hot-reload, motion easing | in progress |

## Install it (standalone)

Banquo is a real terminal — install it once and launch it like any other app, with **no console window** and no `cargo run` from the source tree:

```powershell
.\install.ps1                 # build --release, copy banquo.exe, add a Start-menu shortcut
.\install.ps1 -Desktop -AddToPath   # also: Desktop shortcut + `banquo` on PATH
```

Then launch **Banquo** from the Start menu (or type `banquo` in any shell if you used `-AddToPath`). A borderless window opens on a true PTY (ConPTY on Windows, `openpty` on Unix). Type `ls`, `vim`, or `htop` — the parser + grid core handles full SGR colors, alt-screen, and cursor addressing.

> **Don't use `cargo run` to "use" Banquo.** `cargo run` launches the *debug*
> build as a child of your shell and **blocks it** — close that shell and Banquo
> closes too. That's the dev loop, not the product. The installed/release binary
> (`target\release\banquo.exe`, or the Start-menu shortcut) is a standalone GUI
> process: it has **no console window** and is **independent of any shell** —
> closing the terminal you launched it from does not affect it. If you ever see
> Banquo die when a shell closes, you launched the debug build via `cargo run`.

### Choose your shell

Banquo runs **any shell on your machine** — PowerShell, cmd, bash, zsh, or WSL — not a single hardcoded one. Two ways to pick:

- **Per tab, instantly:** open the command palette (`Ctrl+Shift+P`) and type `shell pwsh` (or `cmd`, `wsl`, `bash`…). It opens a new tab on that shell. This works even with zero configuration — Banquo detects shells on your `PATH`.
- **As the default:** add a `[shell]` section to `banquo.toml` (`%APPDATA%\banquo\banquo.toml`):

  ```toml
  [shell]
  default = "pwsh"

  [[shell.profiles]]
  name = "pwsh"
  command = "pwsh.exe"
  args = ["-NoLogo"]

  [[shell.profiles]]
  name = "ubuntu"
  command = "wsl.exe"
  args = ["-d", "Ubuntu"]
  ```

  With no `[shell]` section, Banquo launches your OS default shell (unchanged).

### Develop it

```sh
cargo run                               # dev loop (keeps a console for diagnostics)
cargo test                              # pure unit tests (config, shell resolution, fonts)
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

## Banquo Compose (Configurator)

Banquo features a dynamic TOML configurator known as **Banquo Compose**. You can hot-swap fonts and alter rendering logic simply by editing your config file (`~/.config/banquo/banquo.toml` on Unix/macOS, or `%APPDATA%\banquo\banquo.toml` on Windows).

```toml
[fonts]
monospace_path = "C:\\Users\\charl\\Banquo\\assets\\fonts\\Geist-Regular.ttf"

[grid]
mode = "auto"
```

## Auto-Snap Proportional Rendering Engine

The terminal grid must inherently align character positions. However, when using `grid.mode = "auto"`, Banquo utilizes a revolutionary "Auto-Snap" Proportional Rendering Engine. It calculates the exact typographic advance width of every character to dynamically position cells, turning your terminal from a rigid grid into a beautifully typeset document—all without breaking the cursor logic or SGR background colors!

## What Banquo refuses to be (design §VII)

While Banquo features an incredibly sleek, unopinionated **Multi-Tab** interface that seamlessly allows multiple independent PTY sessions, it refuses to handle splits/pane multiplexing (compose with a real WM like sway, or run tmux). There is no telemetry, no auto-update, and **no network code at all**. Its entire surface to the outside world is one PTY per tab and one config file.

## License

Banquo's own code: MIT OR Apache-2.0. The vendored Iosevka font
(`assets/fonts/`) is under the SIL Open Font License 1.1 — see
`assets/fonts/Iosevka-LICENSE.md`.
