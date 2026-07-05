# Configuration Reference

Banquo is configured through a single TOML file. Changes are picked up instantly via hot-reload; no restart required.

## Config File Location

| Platform | Path |
|----------|------|
| Windows  | `%APPDATA%\banquo\banquo.toml` |
| macOS    | `~/.config/banquo/banquo.toml` |
| Linux    | `~/.config/banquo/banquo.toml` |

On a fresh install, the installer bootstraps `configs/zircon.toml` into this location. You can also run `banquo compose --check` to validate your config file.

---

## Top-Level

```toml
theme = "zircon"
```

| Field   | Type   | Default    | Description |
|---------|--------|------------|-------------|
| `theme` | string | `"zircon"` | Active theme. Built-in options: `zircon`, `blanco`, `concrete`, `concrete-dark`, `primordial`, `volcanic_glass`. |

---

## `[fonts]`

Controls which font files Banquo loads and how text is sized and positioned.

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
| `monospace_path`  | string | *none*  | Absolute path to a `.ttf` or `.otf` file for the terminal grid font. When absent, Banquo uses its built-in monospace fallback. |
| `ui_path`         | string | *none*  | Font for future UI chrome (currently unused; reserved). |
| `serif_path`      | string | *none*  | Reserved for future use. |
| `symbols_path`    | string | *none*  | Font for box-drawing and block characters (U+2500..U+259F). Defaults to the monospace font when absent. A Nerd Font Mono variant works well here. |
| `size`            | float  | `16.0`  | Base font size in logical pixels. Increase for high-DPI (4K) displays; `20.0`-`24.0` is a good range for 4K. The entire grid geometry scales from this value. |
| `offset_x`        | float  | `0.0`   | Horizontal spacing adjustment (logical pixels) added to each cell width. |
| `offset_y`        | float  | `0.0`   | Vertical spacing adjustment (logical pixels) added to each cell height. |

### Font Fallback Behavior

1. If `monospace_path` points to a valid file, that font is loaded.
2. If the file is missing or unreadable, Banquo logs a warning to stderr and falls back to egui's built-in monospace font.
3. Emoji and symbol coverage is preserved: the user font leads the fallback chain, but egui's default fonts remain available for missing glyphs.

---

## `[grid]`

```toml
[grid]
mode = "fixed"
```

| Field  | Type   | Default   | Description |
|--------|--------|-----------|-------------|
| `mode` | string | `"fixed"` | Grid rendering mode. `"fixed"` uses strict monospace cell alignment. `"auto"` enables the Auto-Snap Proportional Rendering Engine, which dynamically positions cells based on each glyph's true advance width. |

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
| `tab_bar_mode`    | string | `"auto"` | Tab bar visibility. `"auto"` shows the tab bar only when the mouse enters the top 40px (auto-collapses after 3 seconds of inactivity). `"persistent"` or `"fixed"` keeps it always visible. |
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
| `default` | string | *none*  | Name of the profile to launch for new tabs. When absent, Banquo launches the OS default shell (e.g. `cmd.exe` on Windows, `/bin/sh` on Unix). |

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

## Full Example

A complete `banquo.toml` using every available option:

```toml
theme = "zircon"

[fonts]
monospace_path = "C:/Users/you/fonts/IosevkaNerdFontMono-Regular.ttf"
symbols_path = "C:/Users/you/fonts/TerminessNerdFontMono-Regular.ttf"
size = 20.0
offset_x = 0.0
offset_y = 1.0

[grid]
mode = "fixed"

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
