# Configuration Reference

Banquo is configured through a single TOML file. Changes are picked up instantly via hot-reload; no restart required. The config file is optional — Banquo runs with sensible defaults without one.

## Config File Location

| Platform | Default path |
|----------|--------------|
| Windows  | `%APPDATA%\banquo\banquo.toml` |
| macOS    | `~/.config/banquo/banquo.toml` |
| Linux    | `~/.config/banquo/banquo.toml` |

### `BANQUO_CONFIG` (dotfiles / git workflow)

Set the `BANQUO_CONFIG` environment variable to an absolute file path and Banquo reads, hot-watches, and saves **that** file instead — everywhere (GUI, palette, and every CLI command). This is how you keep your terminal config in a dotfiles repo:

```sh
export BANQUO_CONFIG=~/dotfiles/banquo/banquo.toml     # bash/zsh
$env:BANQUO_CONFIG = "$HOME\dotfiles\banquo\banquo.toml"  # PowerShell
```

The user presets directory (see below) is always `presets/` next to the active config file, so presets travel with a dotfiles-managed config.

### Getting a config

```sh
banquo config init                     # create the config from the zircon preset
banquo config init --preset blanco    # start from a different preset
banquo config init --force            # overwrite an existing config
banquo config path                     # print where the active config lives
banquo config show                     # print the effective config as TOML
banquo check                           # validate: errors exit non-zero
```

---

## Presets

A **preset** is a portable appearance bundle: theme + window chrome + UI settings, and nothing else. Presets never contain font paths, shell profiles, or machine-specific data, so they are safe to share and commit.

- **Builtin presets** (embedded in the binary, available everywhere): `zircon`, `blanco`, `concrete`, `concrete-dark`, `primordial`, `volcanic-glass`.
- **User presets**: any `<name>.toml` dropped into the `presets/` directory next to your config file. User presets shadow builtins of the same name.

**Applying a preset merges, it never replaces.** A preset overrides exactly the keys it declares; everything else in your config — `[shell]` profiles, `[fonts]` paths and size, custom `[colors]` — survives untouched.

```sh
banquo preset list             # all presets; user presets are marked "(user)"
banquo preset apply blanco     # merge the blanco preset into your config
```

Or live, from inside Banquo: `Ctrl+Shift+P`, then `preset blanco` (or `theme blanco`).

---

## Top-Level

```toml
theme = "zircon"
```

| Field   | Type   | Default    | Description |
|---------|--------|------------|-------------|
| `theme` | string | `"zircon"` | Active theme. Builtins: `zircon`, `blanco`, `concrete`, `concrete-dark`, `primordial`, `volcanic-glass` (legacy spellings like `volcanic_glass` are accepted). Any other name is a **custom theme**: it starts from the zircon base spec and is styled entirely by your `[colors]` section. |

---

## `[colors]`

Optional color overrides layered on top of the active theme. This is the custom-theme mechanism: every field accepts `"#RRGGBB"` or `"#RRGGBBAA"` (alpha unmultiplied). Set any subset; unset fields keep the theme's builtin value.

```toml
theme = "midnight"          # custom name — zircon base + your colors

[colors]
background  = "#0b1021e0"   # substrate fill (alpha controls transparency)
foreground  = "#7fdbca"     # remap for default (light-grayscale) text
cursor      = "#ffcb6b"     # cursor block
cursor_text = "#000000"     # glyph painted under the cursor block
```

| Field         | Type   | Default | Description |
|---------------|--------|---------|-------------|
| `background`  | string | *theme* | Substrate fill behind the grid. |
| `foreground`  | string | *theme* | Remaps default terminal text (light grayscale) to this color. Text over custom backgrounds (e.g. CLI highlight bars) is never remapped, so contrast survives. |
| `cursor`      | string | *theme* | Cursor block color. |
| `cursor_text` | string | *theme* | Color of the character under the cursor. |

Invalid hex strings are ignored at runtime (the theme's value stands); `banquo check` warns about them.

---

## `[fonts]`

Controls which font files Banquo loads and how text is sized and positioned. Fonts are personal — they belong in *your* config, never in presets.

```toml
[fonts]
monospace_path = "C:/Users/you/fonts/FiraCode-Regular.ttf"
symbols_path = "C:/Users/you/fonts/NerdFontMono.ttf"
size = 18.0
offset_x = 0.0
offset_y = 0.0
```

| Field            | Type   | Default | Description |
|------------------|--------|---------|-------------|
| `monospace_path` | string | *none*  | Absolute path to a `.ttf` or `.otf` file for the terminal grid font. When absent, Banquo uses its built-in monospace fallback. |
| `symbols_path`   | string | *none*  | Font for box-drawing and block characters (U+2500..U+259F). Defaults to the monospace font when absent. A Nerd Font Mono variant works well here. |
| `size`           | float  | `16.0`  | Base font size in logical pixels. Increase for high-DPI (4K) displays; `20.0`–`24.0` is a good range for 4K. The entire grid geometry scales from this value. |
| `offset_x`       | float  | `0.0`   | Horizontal spacing adjustment (logical pixels) added to each cell width. |
| `offset_y`       | float  | `0.0`   | Vertical spacing adjustment (logical pixels) added to each cell height. |

### Font Fallback Behavior

1. If `monospace_path` points to a valid file, that font is loaded.
2. If the file is missing or unreadable, Banquo logs a warning to stderr and falls back to egui's built-in monospace font (`banquo check` also warns).
3. Emoji and symbol coverage is preserved: the user font leads the fallback chain, but egui's default fonts remain available for missing glyphs.

---

## `[window]`

Controls the visual chrome of the Banquo window itself.

```toml
[window]
edge_style = "beveled"
corner_style = "g3"
radius = 24.0
inset = 0.0
opacity = 0.8
```

| Field          | Type   | Default    | Description |
|----------------|--------|------------|-------------|
| `edge_style`   | string | `"flat"`   | Border style. Options: `"flat"` (no border), `"rounded"` (thin dark stroke), `"beveled"` (outer dark + inner light), `"3d"` (chunky CRT bezel). |
| `corner_style` | string | `"square"` | Corner rounding algorithm. Options: `"square"` (sharp), `"g1"` (circular arc), `"g2"` (superellipse), `"g3"` (tighter superellipse, Apple-style squircle). |
| `radius`       | float  | `8.0`      | Corner radius in logical pixels. Only applies when `corner_style` is not `"square"`. |
| `inset`        | float  | `0.0`      | Shrinks the entire window content area by this many logical pixels on all sides. Creates a floating effect when combined with transparency. |
| `opacity`      | float  | `1.0`      | Background opacity multiplier (0.0 to 1.0). At `1.0`, the theme's default alpha is used. Lower values let more of the OS compositor's blur bleed through. |

---

## `[ui]`

Controls Banquo's interactive UI elements.

```toml
[ui]
tab_bar_mode = "auto"
top_margin = 32.0
background_mode = "reveal"
```

| Field             | Type   | Default  | Description |
|-------------------|--------|----------|-------------|
| `tab_bar_mode`    | string | `"auto"` | Tab bar visibility. `"auto"` shows the tab bar only when the mouse enters the top 40px (auto-collapses after 3 seconds of inactivity). `"persistent"` keeps it always visible. |
| `top_margin`      | float  | `32.0`   | Vertical offset (logical pixels) before the terminal grid begins. Set to `0.0` to draw text at the very top of the window (the auto-collapsing tab bar overlays it). |
| `background_mode` | string | *none*   | Set to `"reveal"` to use a 1:1 pixel-mapped texture mode (the texture scrolls with the window position on screen). When absent, textures are stretched to fit the window. |

---

## `[os.windows]`

Windows-specific compositor integration.

```toml
[os.windows]
blur = true
```

| Field  | Type | Default | Description |
|--------|------|---------|-------------|
| `blur` | bool | `false` | Enables Windows Acrylic/Mica blur behind the transparent window. Requires Windows 10 1903+ with desktop composition enabled. |

## `[os.macos]`

macOS-specific compositor integration.

```toml
[os.macos]
vibrancy = true
```

| Field      | Type | Default | Description |
|------------|------|---------|-------------|
| `vibrancy` | bool | `false` | Enables macOS vibrancy effect behind the window. (Not yet implemented; reserved.) |

---

## `[shell]`

Configures which shell programs Banquo can launch and which one opens by default.

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
cwd = "/home/you"

[[shell.profiles]]
name = "nushell"
command = "nu.exe"
env = { STARSHIP_SHELL = "nu" }
```

### `[shell]` Table

| Field     | Type   | Default | Description |
|-----------|--------|---------|-------------|
| `default` | string | *none*  | Name of the profile to launch for new tabs. When absent, Banquo launches the OS default shell (e.g. `cmd.exe` on Windows, `/bin/sh` on Unix). Must match a profile name — `banquo check` errors otherwise. |

### `[[shell.profiles]]` Array

Each entry defines a launchable shell.

| Field     | Type              | Default | Description |
|-----------|-------------------|---------|-------------|
| `name`    | string            | *required* | Identifier used by `default` and the command palette `shell <name>`. |
| `command` | string            | *required* | Program to execute (e.g. `"pwsh.exe"`, `"wsl.exe"`, `"bash"`). |
| `args`    | array of strings  | `[]`    | Arguments passed to the program. |
| `cwd`     | string            | *none*  | Working directory. Inherits Banquo's cwd when absent. |
| `env`     | table of strings  | *none*  | Extra environment variables set for this shell. |

### Zero-Config Shell Switching

Even without any `[shell]` section, the command palette (`Ctrl+Shift+P`) lets you type `shell pwsh` or `shell bash` to open a new tab. Banquo auto-detects shells on your `PATH` at runtime.

---

## Validation: `banquo check`

`banquo check` validates the active config and exits non-zero on errors. It reports:

| Finding | Severity |
|---------|----------|
| TOML syntax error (with the parser's message) | error |
| Config shape error (wrong type for a known key) | error |
| `shell.default` doesn't match any profile name | error |
| Unknown top-level key (e.g. the removed legacy `[grid]` table) | warning |
| `fonts.monospace_path` / `symbols_path` file missing | warning |
| Theme name that isn't a builtin (custom themes are legal) | warning |
| `window.opacity` outside `[0.0, 1.0]` | warning |
| `[colors]` value that isn't valid `#RRGGBB`/`#RRGGBBAA` | warning |

Legacy configs still parse: unknown keys are ignored at load time (removed fields like `[grid]`, `fonts.ui_path`, and `fonts.serif_path` simply have no effect), and the validator points them out.

---

## CLI Reference

| Command | Description |
|---------|-------------|
| `banquo` | Launch the terminal. |
| `banquo check` | Validate the active config; non-zero exit on errors. |
| `banquo preset list` | List builtin + user presets (user presets marked). |
| `banquo preset apply <name>` | Deep-merge a preset into your config file. |
| `banquo config init [--preset <name>] [--force]` | Create a config from a preset (default `zircon`). Refuses to overwrite without `--force`. |
| `banquo config path` | Print the active config path (honors `BANQUO_CONFIG`). |
| `banquo config show` | Print the effective config as TOML. |
| `banquo --version` | Print the version. |

`banquo compose --check` remains as a hidden, deprecated alias for `banquo check`.

> **Windows note:** the installed release binary is a GUI-subsystem app; when run from a console its output is only visible if you pipe or redirect it (e.g. `banquo check | more`). In a source checkout, `cargo run -- check` always prints directly.

---

## Full Example

A complete `banquo.toml` using every available option:

```toml
theme = "zircon"

[colors]
cursor = "#ffcb6b"

[fonts]
monospace_path = "C:/Users/you/fonts/IosevkaNerdFontMono-Regular.ttf"
symbols_path = "C:/Users/you/fonts/TerminessNerdFontMono-Regular.ttf"
size = 20.0
offset_x = 0.0
offset_y = 1.0

[os.windows]
blur = true

[window]
edge_style = "beveled"
corner_style = "g3"
radius = 16.0
inset = 0.0
opacity = 0.85

[ui]
top_margin = 32.0
tab_bar_mode = "auto"

[shell]
default = "pwsh"

[[shell.profiles]]
name = "pwsh"
command = "pwsh.exe"
args = ["-NoLogo"]

[[shell.profiles]]
name = "cmd"
command = "cmd.exe"

[[shell.profiles]]
name = "wsl"
command = "wsl.exe"
args = ["-d", "Ubuntu"]
```
