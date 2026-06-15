//! The Face (design §IV): the UI half that paints appearance.
//!
//! At Milestone 1 there is no truth-half snapshot to read yet, so the Face paints
//! a single hardcoded line. But the shape is already the design's: a value that,
//! given the same inputs, paints the same pixels — here just
//! `render(line, flat_field)` instead of the eventual `render(snapshot, material)`.

use eframe::{App, CreationContext};
use egui::{Align2, Color32, FontFamily, FontId};

use crate::fonts::{build_font_definitions, FontSource, BANQUO_MONO, EMBEDDED_IOSEVKA};

/// The one line Milestone 1 exists to render — Banquo's thesis (design §IX).
const LINE: &str = "Banquo — gets kings, though he be none.";

/// Glyph size in logical points. Font size is a *setting*, never a function of
/// window size (design guarantee #4) — so it lives as a constant, not derived
/// geometry.
const FONT_SIZE: f32 = 22.0;

/// The flat field (Layer 0/1 of §V, collapsed for M1): a warm near-black tint at
/// ~0.92 alpha. Near-opaque so glyph antialiasing has a stable backing (dodging
/// the over-transparency muddiness the design flags for Zircon, §V), yet
/// translucent enough that the desktop reads faintly through it — proving the
/// window's transparency without yet being Zircon's true glass (Milestone 5).
const FLAT_FIELD: Color32 = Color32::from_rgba_premultiplied(16, 14, 19, 235);

/// Warm off-white glyphs — a hair off pure white so the contrast doesn't ring
/// (the §V Blanco reasoning, applied to text here).
const GLYPH: Color32 = Color32::from_rgb(235, 232, 226);

/// Banquo's application state for Milestone 1.
pub struct BanquoApp {
    /// Font definitions staged in [`BanquoApp::new`], installed on the first
    /// frame and then taken (so we never re-upload them).
    staged_fonts: Option<egui::FontDefinitions>,
    /// Which face actually backs [`BANQUO_MONO`] — reportable per guarantee #6.
    font_source: FontSource,
    /// The install-once latch (see [`should_install_fonts`]).
    fonts_installed: bool,
}

/// Pure decision for the font-install latch: install only while not yet
/// installed. Extracted from [`App::update`] so it is unit-testable without an
/// `egui::Context` or a window — the "exactly once" guarantee is then a property
/// of plain state, not of the renderer.
pub fn should_install_fonts(installed: bool) -> bool {
    !installed
}

impl BanquoApp {
    /// Construct the app, resolving the font definitions up front (pure, no
    /// context needed) and staging them for first-frame install.
    pub fn new(_cc: &CreationContext<'_>) -> Self {
        let (defs, font_source) = build_font_definitions(EMBEDDED_IOSEVKA);
        Self {
            staged_fonts: Some(defs),
            font_source,
            fonts_installed: false,
        }
    }

    /// Which face backs the monospace family (Iosevka when embedded, the built-in
    /// monospace on fallback). Exposed for honest reporting (guarantee #6); also
    /// keeps `font_source` a live read rather than dead state.
    pub fn font_source(&self) -> FontSource {
        self.font_source
    }
}

impl App for BanquoApp {
    /// State-only pass (no painting): install fonts exactly once, on the first
    /// frame. eframe calls `logic` before each `ui`, with the context available
    /// and painting forbidden — the right home for the install-once latch.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if should_install_fonts(self.fonts_installed) {
            if let Some(defs) = self.staged_fonts.take() {
                ctx.set_fonts(defs);
            }
            self.fonts_installed = true;
            // Honest report of which face actually backs the monospace family
            // (guarantee #6) — local stderr, once, no telemetry.
            eprintln!("banquo: monospace face = {:?}", self.font_source());
        }
    }

    /// Paint pass. eframe hands us a margin-less, background-less `Ui` spanning
    /// the root viewport — this *is* the central area, so we fill it with the
    /// flat field and center the one line on it.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        let painter = ui.painter();
        // Flat field (the M1 substrate): a single filled rect over the whole area.
        painter.rect_filled(rect, 0.0, FLAT_FIELD);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            LINE,
            FontId::new(FONT_SIZE, FontFamily::Name(BANQUO_MONO.into())),
            GLYPH,
        );
    }

    /// Zero-alpha clear so the window's framebuffer is genuinely transparent
    /// where nothing is painted; the flat field above provides the visible
    /// substrate. Separating these is what makes M1 transparency-capable without
    /// committing to Zircon's full glass (design §V, Milestone 5).
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_install_fonts_first_frame() {
        assert!(
            should_install_fonts(false),
            "fonts must be installed when not yet installed"
        );
    }

    #[test]
    fn test_should_install_fonts_latched() {
        assert!(
            !should_install_fonts(true),
            "fonts must not be re-installed once the latch is set"
        );
    }
}
