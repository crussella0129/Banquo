# Keybindings and Command Palette

Banquo's keyboard interface is minimal by design. The terminal itself captures all keystrokes and forwards them to the shell; Banquo reserves only a few global shortcuts for its own UI.

---

## Global Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+P` | Toggle the command palette |

All other key combinations (including `Ctrl+C`, `Ctrl+D`, arrow keys, function keys, etc.) are passed directly to the active shell session.

---

## Command Palette

The command palette is a text input overlay activated by `Ctrl+Shift+P`. Type a command and press `Enter` to execute it. Press `Escape` to dismiss without executing.

A hint line under the input shows the available verbs, and — once you type a verb — the matching preset/shell names. Unknown commands are never silently ignored: the palette stays open and shows what went wrong.

### Available Commands

| Command | Example | Description |
|---------|---------|-------------|
| `theme <name>` | `theme volcanic-glass` | Switch theme. Resolves the name against your user presets directory first, then the embedded builtin presets (never the process working directory), and **merges** the preset — your shell profiles, fonts, and colors survive. A name with no preset is set as a custom theme (styled by `[colors]`). Saved to your config immediately. |
| `preset <name>` | `preset blanco` | Strictly apply a preset bundle. Unknown names show an error in the palette. |
| `shell <name>` | `shell pwsh` | Open a new tab running the named shell. Matches configured `[[shell.profiles]]` first, then shells detected on your `PATH`. Unknown names show visible feedback. |

### Preset / Theme Names

Builtins (always available): `zircon`, `blanco`, `concrete`, `concrete-dark`, `primordial`, `volcanic-glass` — plus any user presets from the `presets/` directory next to your config. Legacy spellings (`volcanic_glass`, bare `volcanic`) are accepted.

### Shell Names (Auto-Detected)

Even without any `[shell]` config, Banquo auto-detects these shells when present on your `PATH`:

**Windows:** `pwsh`, `powershell`, `cmd`, `bash`, `wsl`

**Linux/macOS:** `bash`, `zsh`, `sh`

---

## Tab Management

Tabs are managed through the tab bar, which appears at the top of the window.

| Action | How |
|--------|-----|
| New tab | Click the `+` button in the tab bar |
| Switch tab | Click the tab |
| Close tab | Click the `x` on the tab |

The tab bar auto-collapses when the mouse leaves the top 40px of the window and 3 seconds of inactivity have passed. Set `tab_bar_mode = "persistent"` in your config to keep it always visible.

Closing the last tab closes the window.

---

## Window Management (Frameless Mode)

Banquo runs frameless by default (no native title bar). Window management is handled through:

| Action | How |
|--------|-----|
| Move window | Drag the tab bar area |
| Resize window | Drag the invisible 6px borders on any edge |
| Close window | Click the `x` button in the top-right corner of the tab bar |
