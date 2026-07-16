# Installation

## Requirements

- **Rust toolchain**: A recent stable Rust (1.75+). Install from [rustup.rs](https://rustup.rs/).
- **GPU**: A wgpu-compatible GPU. Most integrated GPUs from the last decade work (Vulkan, DX12, or Metal backend).
- **OS**: Windows 10 1903+, macOS, or Linux with a Wayland/X11 compositor.

---

## Windows (Recommended: Install Script)

The install script builds a release binary, copies it to a stable location, creates a Start-menu shortcut, and bootstraps the default config.

```powershell
git clone https://github.com/crussella0129/Banquo.git
cd Banquo
.\install.ps1
```

### Install Options

```powershell
.\install.ps1                           # Standard install (Start menu shortcut)
.\install.ps1 -Desktop                  # Also create a Desktop shortcut
.\install.ps1 -AddToPath                # Also add banquo to your PATH
.\install.ps1 -Desktop -AddToPath       # All of the above
.\install.ps1 -InstallDir "D:\Apps\Banquo"  # Custom install location
```

### What the Installer Does

1. Builds `cargo build --release` (aborts on failure before touching anything).
2. Copies `banquo.exe` to `%LOCALAPPDATA%\Banquo\` (or your custom `-InstallDir`).
3. Creates a Start-menu shortcut at `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Banquo.lnk`.
4. Optionally creates a Desktop shortcut and adds the install dir to your user `PATH`.
5. If no `%APPDATA%\banquo\banquo.toml` exists, copies `configs\zircon.toml` as the default config — the same portable zircon preset `banquo config init` uses (no font paths or machine-specific data).

### After Install

Launch Banquo from:
- The **Start menu** (search "Banquo")
- The **Desktop shortcut** (if you used `-Desktop`)
- Any shell: type `banquo` (if you used `-AddToPath`; restart shells first)

---

## macOS / Linux

There is no installer script yet. Build and run manually:

```sh
git clone https://github.com/crussella0129/Banquo.git
cd Banquo
cargo build --release
```

Copy the binary and bootstrap your config (the presets are embedded in the binary, so this works from any directory):

```sh
cp target/release/banquo ~/.local/bin/
banquo config init            # creates ~/.config/banquo/banquo.toml from the zircon preset
```

Then launch with `~/.local/bin/banquo` or add `~/.local/bin` to your `PATH`. The config file is optional — Banquo runs with defaults without one. To keep the config in a dotfiles repo instead, set `BANQUO_CONFIG` (see [Configuration](configuration.md)).

---

## Development Build

For development, use `cargo run` instead. This keeps the console attached so `eprintln!` diagnostics are visible:

```sh
cargo run          # Debug build, console stays open
cargo test         # Run unit tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

**Do not use `cargo run` as your daily terminal.** The debug build is a child of your shell; closing that shell kills Banquo. The installed release binary is a standalone GUI process that survives shell closure.

---

## Updating

Pull the latest source and re-run the install script. It overwrites the binary but preserves your existing `banquo.toml`:

```powershell
cd Banquo
git pull
.\install.ps1
```

---

## Uninstalling

1. Delete the install directory: `%LOCALAPPDATA%\Banquo\`
2. Delete the config directory: `%APPDATA%\banquo\`
3. Delete the Start-menu shortcut: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Banquo.lnk`
4. If you used `-AddToPath`, remove the install directory from your user `PATH` environment variable.
