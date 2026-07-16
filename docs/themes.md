# Themes

Banquo ships six built-in themes. Each has a matching **preset** — a portable appearance bundle (theme + window chrome + UI) embedded in the binary. Switch live via the command palette (`Ctrl+Shift+P`, then `theme <name>`), via `banquo preset apply <name>`, or by editing `banquo.toml` (hot-reloaded).

Canonical names are lowercase and hyphenated (`concrete-dark`, `volcanic-glass`); legacy spellings (`volcanic_glass`, `volcanic glass`, bare `volcanic`) are accepted everywhere.

---

## Zircon (Default)

**The Glass.** A transparent substrate that delegates blur to the OS compositor. The terminal floats over your desktop, letting wallpapers and windows bleed through. Text renders at near-full opacity so antialiasing stays crisp against the transparent backdrop.

```toml
theme = "zircon"

[os.windows]
blur = true

[window]
edge_style = "flat"
corner_style = "square"
radius = 0.0
```

Best paired with: OS blur enabled (`blur = true`), any font, minimal chrome.

---

## Blanco

**The Canvas.** An opaque white substrate with a subtle procedural dot texture at low opacity, giving the surface tooth like quality paper. Glyphs are near-black. This is the theme for people who want their terminal to feel like a drafting table.

```toml
theme = "blanco"

[window]
edge_style = "beveled"
corner_style = "g3"
radius = 24.0

[ui]
background_mode = "reveal"
```

Best paired with: `background_mode = "reveal"` (the texture tracks your window position on screen), beveled edges, G3 squircle corners.

---

## Concrete

**The Stone.** A mid-grey procedural texture with scattered dark, rust, and brown flecks. The surface has a raw, industrial feel. Good for users who want a neutral workspace that is neither too dark nor too bright.

```toml
theme = "concrete"

[window]
edge_style = "beveled"
corner_style = "g3"
radius = 24.0

[ui]
background_mode = "reveal"
```

Best paired with: `background_mode = "reveal"`, beveled edges, G3 corners.

---

## Concrete Dark

**The Slab.** A near-black variant of Concrete with the same procedural noise pattern, but using a `rgb(20, 20, 20)` base and muted rust/brown speckles. For users who want a dark terminal with physical texture rather than flat black.

```toml
theme = "concrete-dark"

[window]
edge_style = "beveled"
corner_style = "g3"
radius = 16.0
```

Best paired with: beveled edges, moderate radius.

---

## Primordial

**The Abyss.** An 80%-opacity black substrate with sparse red procedural dots. Dark and moody, with just enough texture to avoid feeling flat. The texture is subtle enough that it reads as a deep, living surface rather than a pattern.

```toml
theme = "primordial"

[window]
edge_style = "beveled"
corner_style = "g3"
radius = 24.0

[ui]
background_mode = "reveal"
```

Best paired with: `background_mode = "reveal"`, G3 corners, OS blur off.

---

## Volcanic Glass

**The Plasma.** A near-true-black substrate (`rgba(0,0,0,200)`) with no procedural texture, and default text remapped to blood red. On OLED monitors, dead pixels in the terminal's background physically turn off, making the window disappear into the bezel. The 3D edge style gives it a chunky CRT bezel feel.

> An experimental WGSL shader pipeline for this theme (glyph aura, active-row radiance) lives in `src/render/` but is **not yet wired into the frame** — no GPU shader effects currently run. Honesty over marketing (guarantee #6).

```toml
theme = "volcanic-glass"

[window]
edge_style = "3d"
corner_style = "square"
radius = 0.0

[ui]
tab_bar_mode = "auto"
```

Best paired with: `edge_style = "3d"`, square corners, an OLED display.

---

## Custom Themes (pure TOML, no recompile)

A theme name that isn't a builtin gives you a **custom theme**: it starts from the zircon base (transparent dark substrate, no texture) and takes its colors from your `[colors]` section:

```toml
theme = "midnight"

[colors]
background  = "#0b1021e0"   # RRGGBBAA — the alpha is your transparency
foreground  = "#7fdbca"     # remap for default text
cursor      = "#ffcb6b"
cursor_text = "#000000"
```

`[colors]` also overrides *builtin* themes — e.g. keep concrete's texture but change its cursor:

```toml
theme = "concrete"

[colors]
cursor = "#c46a4a"
```

Rules of the overlay:

- Every field accepts `#RRGGBB` or `#RRGGBBAA` (alpha unmultiplied).
- Unset fields keep the theme's builtin value.
- `foreground` only remaps *default* (light-grayscale) text; text over custom backgrounds (highlight bars, selections) keeps its colors, so contrast survives.
- Invalid values are ignored at runtime; `banquo check` warns about them.

To share a custom theme, put it in a preset: save the `theme` + `[colors]` (+ `[window]`, `[ui]`) fragment as `<name>.toml` in the `presets/` directory next to your config. It shows up in `banquo preset list` as `(user)` and friends can drop the same file into their presets directory.

---

## Switching Themes

### Via Config File

Edit `banquo.toml` and change the `theme` field. Banquo hot-reloads the file automatically.

### Via Command Palette

Press `Ctrl+Shift+P` and type:

```
theme zircon
```

The palette resolves the name against your user presets directory first, then the embedded builtins — it works identically from an installed binary and a source checkout. Applying a preset **merges**: only the keys the preset declares change; your `[shell]`, `[fonts]`, and `[colors]` survive. A name with no preset is set as a custom theme. The switch is saved to your config file immediately.

### Via CLI

```sh
banquo preset apply blanco
```

Same merge semantics; the running Banquo hot-reloads the saved file.

---

## Background Modes

Two modes control how procedural textures map to the window:

| Mode | Behavior |
|------|----------|
| *(default)* | Texture is stretched to fill the window. Resizing the window stretches the texture proportionally. |
| `"reveal"` | Texture is mapped 1:1 to pixel coordinates relative to the window's position on screen. Moving the window reveals different parts of the texture, like looking through a window at a larger surface. |

Set via `[ui] background_mode = "reveal"` in your config.

---

## Opacity Control

The `[window] opacity` field (0.0 to 1.0) acts as a multiplier on the theme's background alpha. This controls how much of the OS compositor's content bleeds through:

- `opacity = 1.0`: Full theme opacity (default).
- `opacity = 0.7`: 70% of the theme's alpha, letting more desktop blur through.
- `opacity = 0.3`: Very transparent, useful with `blur = true` for a frosted glass look.

This affects both solid-color backgrounds and procedural textures.
