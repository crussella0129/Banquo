#![forbid(unsafe_code)]
//! Banquo — a most beautiful terminal.
//!
//! Milestone 1: a frameless, transparency-capable `eframe`+`wgpu` window that
//! paints one centered line of monospace text on a flat field. No PTY, no grid,
//! no materials yet — this proves the toolchain (compile → window → font
//! pipeline → wgpu backend) end-to-end.
//!
//! ## The truth/appearance seam (design §I, §IV)
//!
//! Banquo is organized around a clean boundary between *truth* (PTY bytes →
//! parser → grid → cursor → scrollback; pure, deterministic, GUI-unaware) and
//! *appearance* (`view = render(snapshot, material)`; a pure function of the
//! truth). At Milestone 1 there is no truth-half yet, so this binary is all
//! appearance: [`app`] is the Face, [`fonts`] is the font side of appearance.
//! These module boundaries are deliberate — they cleave into a `banquo-core` /
//! `banquo-face` workspace at Milestone 2 when the truth-half gains real content.

mod app;
mod fonts;

/// The window's initial logical size. Milestone 1 holds font size constant and
/// adapts geometry to text (design guarantee #4); here we pick a pleasant default
/// for the two-line hero card.
const INITIAL_SIZE: [f32; 2] = [760.0, 280.0];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        // The wgpu renderer is the supported custom-render path Banquo commits to
        // (design §VI Milestone 6). We select it now so the whole project rides a
        // single backend from the first window.
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Banquo")
            // Guarantee: the window is transparency-capable from Milestone 1 — the
            // wgpu framebuffer can be non-opaque. M1's substrate is still a flat
            // tinted field (see `app`); true Zircon glass is Milestone 5.
            .with_transparent(true)
            // Frameless: Banquo wears no native chrome. "The tool configures
            // itself" (design §VII) — the window is just the looking-glass.
            .with_decorations(false)
            .with_inner_size(INITIAL_SIZE)
            .with_min_inner_size([420.0, 180.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Banquo",
        options,
        Box::new(|cc| Ok(Box::new(app::BanquoApp::new(cc)))),
    )
}
