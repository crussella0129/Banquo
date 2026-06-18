#![forbid(unsafe_code)]
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
pub mod texture_gen;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "banquo")]
#[command(about = "Banquo — a most beautiful terminal.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Banquo Compose — The TOML Configurator tool
    Compose {
        /// Check if the `banquo.toml` configuration is valid
        #[arg(long)]
        check: bool,
    },
}

/// The window's initial logical size. Milestone 2 is a real terminal — larger
/// default than M1's hero card.
const INITIAL_SIZE: [f32; 2] = [1024.0, 640.0];

fn main() -> Result<(), eframe::Error> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Compose { check } => {
                if check {
                    println!("Banquo Compose: Checking config...");
                    let _config = config::BanquoConfig::load();
                    println!("Config loaded successfully. All parameters are valid.");
                } else {
                    println!("Banquo Compose: Missing --check flag. Try `banquo compose --help`.");
                }
            }
        }
        return Ok(());
    }

    // Spawn the terminal session (PTY + reader thread + snapshot publisher)
    // before the window opens. Initial size = 80×24 (corrected on first resize
    // when the Face knows the actual panel dimensions).
    let session = core::session::spawn(80, 24).expect("failed to spawn terminal session");

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

    let config = config::BanquoConfig::load();

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
