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
  main.rs           Entry point. CLI parsing, window creation, PTY spawn.
  app.rs            The Face. All rendering, input handling, tab management,
                    command palette, theme application.
  config.rs         BanquoConfig and all sub-structs. TOML deserialization,
                    file watching, hot-reload channel.
  fonts.rs          Font loading pipeline. Loads user fonts or falls back
                    to built-in monospace. Produces FontDefinitions for egui.
  metrics.rs        CellMetrics. Grid geometry calculations: cell size,
                    grid dimensions from window size, centering offsets.
  texture_gen.rs    Procedural texture generators for Blanco, Concrete,
                    Concrete Dark, and Primordial themes.

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
    mod.rs          WGSL shader pipeline. GlassUniforms struct, GPU resource
                    setup, CallbackTrait for custom wgpu render passes.
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

The root config struct. All fields use `Option<T>` with `#[serde(default)]` so any subset of the config can be specified. Missing fields fall back to sensible defaults in the consuming code, not in the struct itself.

### `GlassUniforms` (`render/mod.rs`)

A 48-byte struct (frozen std140 layout) passed to the WGSL shader each frame. Contains resolution, time, material ID, cursor position, and material parameters.

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
- `config.rs`: TOML deserialization, shell config parsing, default handling
- `core/shell.rs`: Profile resolution, argument mapping, determinism
- `os/mod.rs`: Detach sentinel logic, relaunch command construction
- `render/mod.rs`: Uniform struct layout (size + offsets), WGSL parse validation

---

## Adding a New Theme

1. Choose a `theme` name (lowercase, no spaces).
2. If the theme uses a procedural texture, add a `generate_<name>_texture()` function in `texture_gen.rs`.
3. In `app.rs`:
   - Add a `texture_<name>: Option<TextureHandle>` field to `BanquoApp`.
   - Initialize it to `None` in `BanquoApp::new`.
   - Add a branch to `ensure_textures()` to generate and cache it.
   - Add a branch to the `(bg_fill, bg_texture)` match in `ui()`.
4. Create a preset file at `configs/<name>.toml` with recommended window/font settings.
5. The command palette `theme <name>` command will automatically pick it up.
