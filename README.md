# Banquo

A terminal with a conscience about its own correctness.

Banquo is a **100% Rust** GPU-accelerated terminal emulator built on a clean separation between **truth** (PTY, parser, grid) and **appearance** (fonts, materials, shaders). The truth half is pure, deterministic, and testable headlessly. The appearance half is a layered material engine that ships six presets and a live-reloading TOML configurator.

> *Banquo gets kings, though he be none* -- a terminal is the parent of every process it spawns and the author of none of their work.

---

## Quick Start

### Install (Windows)

```powershell
git clone https://github.com/crussella0129/Banquo.git
cd Banquo
.\install.ps1                 # build, install, create Start-menu shortcut
.\install.ps1 -Desktop -AddToPath   # optional: Desktop shortcut + PATH
```

Then launch **Banquo** from the Start menu.

### Build from Source (any platform)

```sh
cargo build --release
./target/release/banquo config init    # bootstrap a config from the zircon preset
./target/release/banquo
```

Banquo also runs with **zero config** — the config file is optional. See [docs/installation.md](docs/installation.md) for the full guide.

---

## Features

**Multi-tab terminal.** Each tab is an independent PTY session. Click `+` to add tabs, or use the command palette to open a tab on a specific shell.

**Six built-in themes, all data.** Zircon (glass), Blanco (canvas), Concrete (stone), Concrete Dark (slab), Primordial (abyss), Volcanic Glass (plasma). Switch live via the command palette (`Ctrl+Shift+P`, `theme blanco`) or `banquo preset apply blanco`.

**Custom themes in pure TOML.** The `[colors]` section overrides any theme's background, foreground, cursor, and cursor-text colors — no recompile:

```toml
theme = "midnight"        # any name you like

[colors]
background = "#0b1021e0"  # #RRGGBB or #RRGGBBAA
foreground = "#7fdbca"
cursor     = "#ffcb6b"
```

**Portable presets.** A preset is an appearance bundle (theme + window chrome + UI) with no font paths, no shell profiles, no personal data. The six builtins are embedded in the binary; drop extra `.toml` files into the `presets/` directory next to your config and they appear in `banquo preset list`. Applying a preset **merges** — it only overrides the keys it declares, so your shell profiles and fonts always survive.

**A real CLI.** `banquo check` validates your config with honest exit codes and named diagnostics; `banquo config init` bootstraps a launchable config from any preset; `banquo config path`/`show` make the config easy to find and export; `banquo preset list`/`apply` manage presets.

**Dotfiles/GitHub-friendly.** Set `BANQUO_CONFIG=~/dotfiles/banquo.toml` and Banquo reads, watches, and saves that file — your terminal config lives in your repo, along with the `presets/` directory beside it. `banquo config show` prints the effective config for sharing.

**Hot-reloading config.** Edit `banquo.toml` and changes take effect immediately. No restart needed. Fonts, themes, colors, window chrome, shell defaults, and font size all reload live.

**Configurable font size.** Set `[fonts] size = 22.0` for comfortable 4K usage. The entire grid geometry scales from this single value.

**Background opacity control.** Dial `[window] opacity = 0.7` to control how much OS blur bleeds through.

**OS compositor integration.** Request blur from Windows Acrylic/Mica with `[os.windows] blur = true`.

**Custom fonts.** Point `monospace_path` to any `.ttf` or `.otf` on your system. A separate `symbols_path` handles box-drawing characters.

**Shell switching.** Configure named shell profiles or use `Ctrl+Shift+P` then `shell pwsh` to open a tab on any shell on your PATH, with zero config. The palette shows suggestions as you type and tells you when it doesn't understand — nothing is silently ignored.

**Frameless window.** Custom chrome with configurable edge styles (flat, beveled, 3D), corner styles (square, G1, G2, G3 squircle), and radius.

---

## Keep Your Config in Git

Banquo's entire configuration is one TOML file plus an optional `presets/` directory — made to live in a dotfiles repo:

```sh
# point Banquo at your dotfiles clone (set it in your shell profile)
export BANQUO_CONFIG=~/dotfiles/banquo/banquo.toml   # $env:BANQUO_CONFIG on Windows

banquo config path      # confirm where Banquo reads/writes
banquo config show      # print the effective config (pipe it anywhere)
banquo check            # validate before you commit
```

Presets you drop in `~/dotfiles/banquo/presets/` travel with the config and show up in `banquo preset list` as `(user)` entries.

---

## Documentation

| Document | Description |
|----------|-------------|
| [Configuration Reference](docs/configuration.md) | Every field in `banquo.toml`, with types, defaults, and examples |
| [Themes](docs/themes.md) | Theme gallery, background modes, and opacity control |
| [Keybindings](docs/keybindings.md) | Keyboard shortcuts and command palette commands |
| [Installation](docs/installation.md) | Full install guide for Windows, macOS, and Linux |
| [Architecture](docs/architecture.md) | Codebase map, data flow, threading model, and how to add themes |
| [Troubleshooting](docs/troubleshooting.md) | Common issues and fixes |
| [Design Document](BANQUO_DESIGN.md) | The full design argument (the "why" behind every choice) |

---

## The Six Guarantees

Banquo dies for these. A feature request that violates one is answered *no*.

1. **No `unsafe` in Banquo's own crates.** `#![forbid(unsafe_code)]`, not `deny`.
2. **The core never blocks the frame.** A 2 GB `cat` may fall behind, never freeze the window.
3. **Monospace alignment is sacred.** Every cell is exactly one (or two) cells wide; paint obeys the grid.
4. **Font size is a setting, not a function of window size.** Resizing reflows (changes rows/cols via SIGWINCH); it never scales glyphs.
5. **A theme can never kill your shell.** Truth and appearance are separate lifetimes; appearance is disposable.
6. **It tells the truth about what it can't do.** Materials degrade visibly and honestly; Banquo never fakes a capability it lacks.

---

## Develop

```sh
cargo run                               # dev loop (keeps console for diagnostics)
cargo test                              # unit tests (config, shell, fonts, shaders)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Do not use `cargo run` as your daily terminal. It is the dev loop. The installed release binary is a standalone GUI process that survives shell closure. See [Troubleshooting](docs/troubleshooting.md) for why.

Honesty note (guarantee #6): the WGSL glass pipeline in `src/render/` is **experimental and not yet wired into the frame** — no theme currently drives GPU shader effects. It stays in-tree for the Milestone 6 work; the features above list only what ships.

---

## What Banquo Refuses to Be

Banquo handles **multi-tab** but refuses splits/pane multiplexing (compose with a real WM or tmux). There is no telemetry, no auto-update, and **no network code at all**. Its entire surface to the outside world is one PTY per tab and one config file.

---

## License

Banquo's own code: MIT OR Apache-2.0. The vendored Iosevka font (`assets/fonts/`) is under the SIL Open Font License 1.1. See `assets/fonts/Iosevka-LICENSE.md`.
