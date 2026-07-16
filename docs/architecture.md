# Architecture

This document maps Banquo's codebase for contributors. It covers the module layout, data flow, and key abstractions.

---

## The Truth/Appearance Seam

Banquo is organized around a single architectural invariant: a clean boundary between **truth** and **appearance**.

- **Truth** (the `core` module): PTY bytes in, parser, grid state, cursor, scrollback. Pure, deterministic, GUI-unaware. You could run it headless in a test harness and assert every cell.
- **Appearance** (the `app` module + `render` + `texture_gen` + `fonts`): A pure function of that truth. `view = render(snapshot, material, time)`. It reads the truth; it never writes it.

This is not just tidiness. It is the architecture that makes the six guarantees (see `BANQUO_DESIGN.md` section II) hold by construction rather than by hope.

---

## Module Map

```
src/
  main.rs           Entry point. CLI (check / preset / config subcommands),
                    window creation, PTY spawn.
  app.rs            The Face. All rendering, input handling, tab management,
                    command palette (parse + suggestions + feedback).
  config.rs         BanquoConfig and all sub-structs. TOML deserialization,
                    BANQUO_CONFIG path resolution, strict/lenient loading,
                    validation diagnostics, deep-merge preset application,
                    file watching, hot-reload channel.
  theme.rs          The theme engine. ThemeSpec (background, texture kind,
                    fg remap, cursor colors) as pure data; six builtin specs;
                    name normalization; [colors] overlay resolution.
  presets.rs        Portable appearance bundles. Six builtins embedded via
                    include_str!; user presets from the presets/ directory
                    next to the config file; lookup + listing.
  fonts.rs          Font loading pipeline. Loads user fonts or falls back
                    to built-in monospace. Produces FontDefinitions for egui.
  metrics.rs        CellMetrics. Grid geometry calculations: cell size,
                    grid dimensions from window size, centering offsets.
  texture_gen.rs    Procedural texture generators, dispatched by TextureKind
                    (Blanco, Concrete, ConcreteDark, Primordial).

  core/
    mod.rs          Re-exports for the truth half.
    pty.rs          PTY abstraction. Opens a pseudoterminal (ConPTY on
                    Windows, openpty on Unix), spawns child processes.
    session.rs      SessionHandle. Owns a PTY + reader thread + ArcSwap
                    snapshot publisher. The unit of "one terminal tab".
    shell.rs        Shell resolution. Maps config profiles to spawnable
                    commands. Pure, side-effect-free, fully unit-tested.
    snapshot.rs     Snapshot type. An immutable frame of the terminal grid
                    published by the reader thread and consumed by the Face.
    term.rs         Wraps alacritty_terminal. Feeds bytes to the parser,
                    extracts grid state into Snapshots.

  os/
    mod.rs          Platform-specific logic. Window blur (Windows Acrylic),
                    process detach (job object breakaway), shell detection.
    windows.rs      Windows-specific blur application.

  render/
    mod.rs          WGSL shader pipeline (EXPERIMENTAL — built but not yet
                    wired into the frame; kept for Milestone 6).
    glass.wgsl      The glass material shader (Volcanic Glass aura, active
                    row radiance, material parameters).
```

---

## Data Flow

```
  ┌─────────────┐   bytes    ┌──────────────┐  snapshot   ┌─────────────┐
  │  PTY READER │ ─────────> │   THE CORE   │ ──────────> │  THE FACE   │
  │  (OS thread)│            │ (truth half) │  (lock-free │ (UI thread, │
  │             │ <───────── │              │   handoff)  │  egui+wgpu) │
  └─────────────┘   resize   └──────────────┘ <────────── └─────────────┘
       ^                                         keystrokes      │
       │                                                         │
       └──────────────── keystrokes routed to PTY ───────────────┘
```

**Three actors, one direction of flow:**

1. **PTY Reader** (dedicated OS thread): The only thing that touches the shell's output fd. Reads raw bytes, hands them to the core. Blocking I/O lives here and only here, so guarantee #2 (core never blocks the frame) holds by construction.

2. **The Core** (owns the grid): Consumes bytes, advances the `alacritty_terminal` state machine, owns the single source of truth. Publishes immutable **Snapshots** via `ArcSwap`. The UI never holds a lock on the live grid; it reads the latest published snapshot.

3. **The Face** (UI thread, egui + wgpu): Reads the newest snapshot, paints it through the active material. Captures keystrokes and ships them back to the PTY. Given the same snapshot + material + time, it paints the same pixels.

---

## Key Types

### `SessionHandle` (`core/session.rs`)

The unit of "one terminal tab". Contains:
- A write handle to the PTY (for sending keystrokes)
- An `ArcSwap<Snapshot>` (the lock-free snapshot publisher)
- A title (`Arc<Mutex<String>>`) updated by the reader thread from OSC sequences

### `Snapshot` (`core/snapshot.rs`)

An immutable frame of the terminal grid. Contains:
- `cols`, `rows`: Grid dimensions
- `cells`: A flat `Vec<Cell>` (row-major)
- `cursor`: Position and visibility
- Color information per cell (foreground, background, as `Color` enum)

### `CellMetrics` (`metrics.rs`)

Derived from the configured font size. Contains:
- `cell_w`, `cell_h`: Snapped to physical pixel boundaries
- Methods: `grid_size()`, `centering_offset()`

### `BanquoConfig` (`config.rs`)

The root config struct. All fields use `Option<T>` with `#[serde(default)]` so any subset of the config can be specified. Missing fields fall back to sensible defaults in the consuming code, not in the struct itself. Old configs with removed keys still parse (serde ignores unknowns); `validate_str` surfaces them as warnings.

The path is resolved once through `BanquoConfig::config_path()` — `BANQUO_CONFIG` env override or the platform default — and every reader/writer (load, save, watch, CLI) goes through it.

### `ThemeSpec` (`theme.rs`)

Everything theme-dependent the Face paints, as one pure value: background fill, `TextureKind`, optional default-foreground remap, cursor and cursor-text colors. `resolve_spec(&config)` = builtin spec for the (normalized) theme name, overlaid with `[colors]`. The Face caches the resolved spec and recomputes it only on config change; the texture cache is keyed by `TextureKind` (see `needs_texture_regen`).

### `Preset` (`presets.rs`)

A found preset: TOML content + provenance (`Builtin` or `User(path)`). Application is a deep TOML-table merge (`BanquoConfig::apply_preset`): the preset overrides exactly the keys it declares, recursively; arrays and scalars replace wholesale. This is why switching presets never destroys `[shell]` or `[fonts]`.

### `GlassUniforms` (`render/mod.rs`)

A 48-byte struct (frozen std140 layout) for the experimental WGSL shader pipeline (not yet wired into the frame). Contains resolution, time, material ID, cursor position, and material parameters.

---

## Threading Model

| Thread | Owns | Reads | Writes |
|--------|------|-------|--------|
| PTY Reader | Raw byte buffer | PTY output fd | Core (via `process_bytes`) |
| Core (within Reader thread) | Grid state (`Term`) | Byte buffer | Snapshot (via `ArcSwap::store`) |
| UI (Face) | Render state, textures | Snapshot (via `ArcSwap::load`) | PTY input (keystrokes) |
| Config Watcher | File watcher | Config file on disk | Config channel (`mpsc::Sender`) |

The snapshot handoff is lock-free (`arc_swap`). The config channel is a standard `mpsc`. No mutexes guard the hot path.

---

## Build and Test

```sh
cargo build                    # Debug build (keeps console for diagnostics)
cargo build --release          # Release build (GUI-subsystem, no console)
cargo test                     # Unit tests (config, shell resolution, fonts, etc.)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Test Coverage

Tests are headless and require no GPU or window:
- `config.rs`: TOML deserialization, path resolution (`BANQUO_CONFIG`), strict loading, validation diagnostics, preset deep-merge
- `theme.rs`: builtin specs (colors locked to the pre-refactor values), name normalization, hex parsing, `[colors]` overlay
- `presets.rs`: embedded presets parse and are portable (no personal data), user-dir precedence, listing
- `app.rs`: keystroke encoding, palette command parsing + suggestions
- `texture_gen.rs`: generator dispatch, texture-cache regen decision
- `core/shell.rs`: profile resolution, argument mapping, determinism
- `os/mod.rs`: detach sentinel logic, relaunch command construction
- `tests/cli_e2e.rs`: drives the real binary (`CARGO_BIN_EXE_banquo`) with `BANQUO_CONFIG` in a temp dir — `check` exit codes, `preset list/apply`, `config init/path/show`, the `compose` alias

---

## Adding a New Theme

**Most themes need no code at all.** A theme is data: a name plus a `[colors]` overlay (see [Themes → Custom Themes](themes.md)). To share it, save the TOML fragment as `<name>.toml` in the `presets/` directory next to your config — `banquo preset list` and the palette pick it up automatically.

Adding a new **builtin** (with its own procedural texture) is a code change:

1. Add a variant to `TextureKind` in `theme.rs` and its generator arm in `texture_gen::generate` (write the `generate_<name>_texture()` function alongside the existing four).
2. Add the theme's `ThemeSpec` entry to `builtin_spec()` in `theme.rs` and its canonical name to `BUILTIN_NAMES` (and any aliases to `normalize_name`).
3. Create the preset at `configs/<name>.toml` (appearance only — no font paths, no shell; the `test_presets_are_portable` gate enforces this) and register it in the `BUILTINS` table in `presets.rs`.
4. Done — the Face, palette, CLI, and validator all consume the data tables; no paint-loop changes.
