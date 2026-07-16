#![forbid(unsafe_code)]
// Release builds are GUI-subsystem apps: launching `banquo.exe` directly opens
// no console window (Banquo is a real standalone terminal, not a child of one).
// Debug builds keep the console so `eprintln!` diagnostics stay visible.
// Hosting ConPTY from a GUI-subsystem process is the norm (WezTerm/Alacritty).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Banquo — a most beautiful terminal.
//!
//! ## The truth/appearance seam (design §I, §IV)
//!
//! Banquo is organized around a clean boundary between *truth* (PTY bytes →
//! parser → grid → cursor → scrollback; pure, deterministic, GUI-unaware) and
//! *appearance* (`view = render(snapshot, material)`; a pure function of the
//! truth). The [`core`] module is the truth half; [`app`] is the Face (the
//! appearance half); [`fonts`] is the font pipeline; [`metrics`] provides the
//! geometry contract between grid and Face.

mod app;
mod config;
mod core;
mod fonts;
mod metrics;
pub mod os;
mod presets;
pub mod render;
pub mod texture_gen;
mod theme;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "banquo")]
#[command(about = "Banquo — a most beautiful terminal.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Validate the active config file and report diagnostics
    Check,
    /// List or apply appearance presets (builtin + user)
    Preset {
        #[command(subcommand)]
        action: PresetAction,
    },
    /// Inspect, bootstrap, or export the config file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Deprecated alias for `check`
    #[command(hide = true)]
    Compose {
        /// Ignored (kept for compatibility with `compose --check`)
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PresetAction {
    /// List available presets; user presets (from the presets/ directory
    /// next to your config file) are marked
    List,
    /// Merge a preset into your config — only the keys the preset declares
    /// change; your shell profiles and fonts survive
    Apply {
        /// Preset name (builtin or user), e.g. "zircon"
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Create a fresh config file from a preset
    Init {
        /// Preset to start from
        #[arg(long, default_value = "zircon")]
        preset: String,
        /// Overwrite an existing config file
        #[arg(long)]
        force: bool,
    },
    /// Print the active config file path (honors BANQUO_CONFIG)
    Path,
    /// Print the effective config as TOML — pipe it into a dotfiles repo
    Show,
}

/// Run a console subcommand, returning the process exit code.
fn run_command(command: Commands) -> i32 {
    match command {
        Commands::Check => run_check(),
        Commands::Compose { .. } => {
            eprintln!("banquo: `compose` is deprecated; use `banquo check`.");
            run_check()
        }
        Commands::Preset { action } => match action {
            PresetAction::List => {
                for (name, is_user) in presets::list() {
                    if is_user {
                        println!("{name} (user)");
                    } else {
                        println!("{name}");
                    }
                }
                0
            }
            PresetAction::Apply { name } => run_preset_apply(&name),
        },
        Commands::Config { action } => match action {
            ConfigAction::Init { preset, force } => run_config_init(&preset, force),
            ConfigAction::Path => match config::BanquoConfig::config_path() {
                Some(path) => {
                    println!("{}", path.display());
                    0
                }
                None => {
                    eprintln!("banquo: no config directory available on this platform");
                    1
                }
            },
            ConfigAction::Show => match config::BanquoConfig::load_strict() {
                Ok(cfg) => match toml::to_string(&cfg) {
                    Ok(s) => {
                        print!("{s}");
                        0
                    }
                    Err(e) => {
                        eprintln!("banquo: failed to serialize config: {e}");
                        1
                    }
                },
                Err(e) => {
                    eprintln!("banquo: {e:#}");
                    1
                }
            },
        },
    }
}

/// `banquo check`: report diagnostics, exit non-zero on any error.
fn run_check() -> i32 {
    let (path, diags) = config::validate();
    match &path {
        Some(p) if p.exists() => println!("checking {}", p.display()),
        Some(p) => {
            println!(
                "no config file at {} — defaults in effect (run `banquo config init`)",
                p.display()
            );
            return 0;
        }
        None => {
            println!("no config directory on this platform — defaults in effect");
            return 0;
        }
    }
    let mut errors = 0;
    let mut warnings = 0;
    for d in &diags {
        match d.severity {
            config::Severity::Error => {
                errors += 1;
                println!("error: {}", d.message);
            }
            config::Severity::Warning => {
                warnings += 1;
                println!("warning: {}", d.message);
            }
        }
    }
    if errors == 0 && warnings == 0 {
        println!("config ok");
    } else {
        println!("{errors} error(s), {warnings} warning(s)");
    }
    if errors > 0 {
        1
    } else {
        0
    }
}

/// `banquo preset apply <name>`: deep-merge the preset into the config file.
fn run_preset_apply(name: &str) -> i32 {
    let Some(preset) = presets::find(name) else {
        eprintln!("banquo: unknown preset \"{name}\" — try `banquo preset list`");
        return 1;
    };
    let current = match config::BanquoConfig::load_strict() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("banquo: {e:#}\nfix the config (or `banquo config init --force`) first");
            return 1;
        }
    };
    let merged = match current.apply_preset(&preset.content) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("banquo: preset \"{name}\" failed to apply: {e:#}");
            return 1;
        }
    };
    if let Err(e) = merged.save() {
        eprintln!("banquo: failed to save config: {e}");
        return 1;
    }
    let path = config::BanquoConfig::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let provenance = match &preset.source {
        presets::PresetSource::Builtin => "builtin".to_string(),
        presets::PresetSource::User(p) => format!("user preset at {}", p.display()),
    };
    println!("applied preset \"{name}\" ({provenance}) to {path}");
    println!("presets only override the keys they declare; your shell/fonts survive");
    0
}

/// `banquo config init [--preset <name>] [--force]`: bootstrap a launchable config.
fn run_config_init(preset_name: &str, force: bool) -> i32 {
    let Some(path) = config::BanquoConfig::config_path() else {
        eprintln!("banquo: no config directory available on this platform");
        return 1;
    };
    if path.exists() && !force {
        eprintln!(
            "banquo: {} already exists — pass --force to overwrite",
            path.display()
        );
        return 1;
    }
    let Some(preset) = presets::find(preset_name) else {
        eprintln!("banquo: unknown preset \"{preset_name}\" — try `banquo preset list`");
        return 1;
    };
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("banquo: failed to create {}: {e}", parent.display());
            return 1;
        }
    }
    if let Err(e) = std::fs::write(&path, &preset.content) {
        eprintln!("banquo: failed to write {}: {e}", path.display());
        return 1;
    }
    println!("created {} from preset \"{preset_name}\"", path.display());
    0
}

/// The window's initial logical size. Milestone 2 is a real terminal — larger
/// default than M1's hero card.
const INITIAL_SIZE: [f32; 2] = [1024.0, 640.0];

fn main() -> Result<(), eframe::Error> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        // Console subcommands run (and exit) before ensure_detached(), so they
        // keep the launching console (ADR-012).
        std::process::exit(run_command(command));
    }

    // GUI path. Before creating any window or PTY, ensure we run outside the
    // launching terminal's job object (Windows release only; no-op otherwise),
    // so closing that terminal can't take Banquo down. On a successful detach
    // this call does not return — the relaunched copy continues from here.
    os::ensure_detached();

    // Load config first so the startup session can honor the configured shell.
    let config = config::BanquoConfig::load();
    let default_shell = core::shell::resolve_shell(&config, None);

    // Spawn the terminal session (PTY + reader thread + snapshot publisher)
    // before the window opens. Initial size = 80×24 (corrected on first resize
    // when the Face knows the actual panel dimensions).
    let session =
        core::session::spawn(80, 24, default_shell).expect("failed to spawn terminal session");

    // For Banquo, we use a custom frameless window globally to allow custom edge/corner drawing.
    let native_decorations = false;

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Banquo")
            .with_transparent(true)
            .with_decorations(native_decorations)
            .with_inner_size(INITIAL_SIZE)
            .with_min_inner_size([420.0, 180.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Banquo",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::BanquoApp::new(
                cc,
                session,
                native_decorations,
                config,
            )))
        }),
    )
}
