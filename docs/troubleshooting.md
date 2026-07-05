# Troubleshooting

Common issues and how to fix them.

---

## Banquo closes when I close the terminal I launched it from

You launched the debug build via `cargo run`. The debug build is a child process of your shell; closing the shell kills it.

**Fix:** Use the installed release binary. Run `.\install.ps1` and launch from the Start menu or type `banquo` (if you used `-AddToPath`). The release binary detaches from the launching terminal's job object and runs as an independent GUI process.

---

## Config changes have no effect

The config file must be in the correct location:

| Platform | Path |
|----------|------|
| Windows  | `%APPDATA%\banquo\banquo.toml` |
| macOS    | `~/.config/banquo/banquo.toml` |
| Linux    | `~/.config/banquo/banquo.toml` |

Note: this is `%APPDATA%` (Roaming), **not** `%LOCALAPPDATA%`. The binary lives in `%LOCALAPPDATA%\Banquo\`, but the config lives in `%APPDATA%\banquo\`.

**Common mistakes:**
- Placing the config in the Banquo source directory (`C:\Users\you\Banquo\banquo.toml`) instead of the AppData path.
- Placing it in `%LOCALAPPDATA%\banquo\` instead of `%APPDATA%\banquo\`.
- TOML syntax errors. Run `banquo compose --check` to validate.

---

## Font not loading / falling back to default

Check stderr output (visible in `cargo run` or in `banquo_stderr.txt`):

```
banquo: Failed to load font from C:/path/to/font.ttf; falling back.
```

**Common causes:**
- The path uses backslashes. TOML requires forward slashes or escaped backslashes: use `"C:/Users/you/font.ttf"` or `"C:\\Users\\you\\font.ttf"`.
- The file does not exist at that path.
- The file is not a valid `.ttf` or `.otf`.

---

## Text is too small on a 4K display

The default font size is 16 logical pixels. On high-DPI displays this can feel small.

**Fix:** Add `size` to your `[fonts]` section:

```toml
[fonts]
size = 22.0
```

Values between 20.0 and 24.0 work well for 4K.

---

## Window is too transparent / not transparent enough

Transparency is controlled by two independent settings:

1. **OS blur** (`[os.windows] blur = true`): Enables the compositor blur effect behind the window. Without this, transparent areas show your desktop without blur.

2. **Opacity** (`[window] opacity = 0.8`): Multiplies the theme's background alpha. Lower values = more see-through. Default is `1.0`.

**Too transparent:** Increase `opacity` toward `1.0`.

**Not transparent enough:** Decrease `opacity` toward `0.0` and ensure `blur = true`.

**No transparency at all:** Make sure you are not using the `blanco` theme (which has an opaque white substrate). Switch to `zircon` or `volcanic_glass`.

---

## OS blur is not working (Windows)

Requirements:
- Windows 10 version 1903 or later
- Desktop composition must be enabled (it is by default)
- Transparency effects must be enabled in Windows Settings > Personalization > Colors

If blur is enabled in the config but not visible, Banquo is requesting it from the compositor but the compositor is not providing it. This is a graceful degradation; the terminal remains usable with flat transparency.

---

## Command palette `theme` command does not load full preset

When you type `theme concrete` in the command palette, Banquo tries to load `configs/concrete.toml` from the source directory relative to the current working directory. If you launched Banquo from a different directory (e.g. via the Start menu shortcut), the `configs/` directory is not in scope.

**Fix:** The theme name is still applied (so the background color/texture changes), but the full preset (edge style, corner style, etc.) only loads if Banquo can find the `configs/` directory. For full control, edit your `banquo.toml` directly.

---

## Shell not found in command palette

When you type `shell nushell` and nothing happens, the shell was not detected.

**Check:**
1. Is `nu.exe` (or whatever the shell binary is) on your system `PATH`?
2. Banquo auto-detects: `pwsh`, `powershell`, `cmd`, `bash`, `wsl` (Windows) or `bash`, `zsh`, `sh` (Unix). Other shells need a `[[shell.profiles]]` entry in your config.

**Fix:** Add a profile:

```toml
[[shell.profiles]]
name = "nushell"
command = "nu.exe"
```

Then `shell nushell` in the palette will work.

---

## Build fails with wgpu errors

Ensure you have a recent Rust toolchain:

```sh
rustup update stable
```

If you see `multiview_mask` or similar type errors, your `wgpu` dependency may be out of date. Run:

```sh
cargo update
cargo build
```

---

## Tabs are not visible

The tab bar auto-hides by default. Move your mouse to the top of the window; it appears when the cursor enters the top 40px.

To keep it always visible:

```toml
[ui]
tab_bar_mode = "persistent"
```
