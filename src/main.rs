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
mod core;
mod fonts;
mod metrics;

/// The window's initial logical size. Milestone 2 is a real terminal — larger
/// default than M1's hero card.
const INITIAL_SIZE: [f32; 2] = [1024.0, 640.0];

fn main() -> eframe::Result {
    // Spawn the terminal session (PTY + reader thread + snapshot publisher)
    // before the window opens. Initial size = 80×24 (corrected on first resize
    // when the Face knows the actual panel dimensions).
    let session = core::session::spawn(80, 24).expect("failed to spawn terminal session");

    // For Windows, we use a custom frameless window.
    // For Unix/macOS, we default to native decorations so we don't fight the WM/compositor.
    let native_decorations = !cfg!(target_os = "windows");

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
        Box::new(move |cc| Ok(Box::new(app::BanquoApp::new(cc, session, native_decorations)))),
    )
}
