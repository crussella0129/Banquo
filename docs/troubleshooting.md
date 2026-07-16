# Troubleshooting

Common issues and how to fix them.

---

## Banquo closes when I close the terminal I launched it from

You launched the debug build via `cargo run`. The debug build is a child process of your shell; closing the shell kills it.

**Fix:** Use the installed release binary. Run `.\install.ps1` and launch from the Start menu or type `banquo` (if you used `-AddToPath`). The release binary detaches from the launching terminal's job object and runs as an independent GUI process.

---

## Config changes have no effect

First, ask Banquo where it actually reads from:

```sh
banquo config path
```

That prints the active path — `%APPDATA%\banquo\banquo.toml` by default (`~/.config/banquo/banquo.toml` on Unix), or whatever `BANQUO_CONFIG` points at if you set it.

Note: the default is `%APPDATA%` (Roaming), **not** `%LOCALAPPDATA%`. The binary lives in `%LOCALAPPDATA%\Banquo\`, but the config lives in `%APPDATA%\banquo\`.

**Common mistakes:**
- Editing a file at a different path than `banquo config path` reports (e.g. a copy in the source tree).
- A `BANQUO_CONFIG` set in one shell profile but not another, so different shells see different configs.
- TOML syntax errors. Run `banquo check` — it prints the parser's error and exits non-zero.

---

## Font not loading / falling back to default

Run `banquo check` — a missing font file is reported as a warning with the exact path. At runtime the same failure logs to stderr (visible under `cargo run`):

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

**No transparency at all:** Make sure you are not using the `blanco` theme (which has an opaque white substrate). Switch to `zircon` or `volcanic-glass`.

---

## OS blur is not working (Windows)

Requirements:
- Windows 10 version 1903 or later
- Desktop composition must be enabled (it is by default)
- Transparency effects must be enabled in Windows Settings > Personalization > Colors

If blur is enabled in the config but not visible, Banquo is requesting it from the compositor but the compositor is not providing it. This is a graceful degradation; the terminal remains usable with flat transparency.

---

## Palette says "unknown preset" / "unknown command"

The palette never silently ignores input — the hint line under the input tells you what it understood. `theme`, `preset`, and `shell` are the verbs; type one and the hint line lists matching names.

The six builtin presets are embedded in the binary, so `theme concrete` works identically from an installed binary and a source checkout (nothing is loaded from the working directory). If a *user* preset isn't found, confirm it lives at `<config dir>/presets/<name>.toml` — check `banquo preset list`.

## `banquo check` prints nothing on Windows (installed binary)

The installed release `banquo.exe` is a GUI-subsystem app: launched from a console, its output doesn't attach to that console. Pipe or redirect to see it:

```powershell
banquo check | more
banquo config show > my-config.toml
```

(The exit code is always honest either way. In a source checkout, `cargo run -- check` prints directly because debug builds keep the console.)

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
