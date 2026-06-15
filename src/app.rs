//! The Face (design §IV): the UI half that paints appearance.
//!
//! At Milestone 1 there is no truth-half snapshot to read yet, so the Face paints
//! a single hardcoded line. But the shape is already the design's: a value that,
//! given the same inputs, paints the same pixels — here just
//! `render(line, flat_field)` instead of the eventual `render(snapshot, material)`.
//!
//! Everything that decides *what* gets painted is factored into pure helpers and
//! a small [`FontInstaller`] state machine, so the Face's behavior is asserted
//! headlessly (see the tests) — only the actual rasterization needs a window.

use eframe::{App, CreationContext};
use egui::{Align2, Color32, FontFamily, FontId};

use crate::fonts::{build_font_definitions, FontSource, BANQUO_MONO, EMBEDDED_IOSEVKA};

/// The one line Milestone 1 exists to render — Banquo's thesis (design §IX).
const LINE: &str = "Banquo — gets kings, though he be none.";

/// Glyph size in logical points. Font size is a *setting*, never a function of
/// window size (design guarantee #4) — so it lives as a constant, not derived
/// geometry.
const FONT_SIZE: f32 = 22.0;

/// Alignment of the line within the field: dead center.
pub const TEXT_ALIGN: Align2 = Align2::CENTER_CENTER;

/// The framebuffer clear color: fully transparent. The window is genuinely
/// transparency-capable (the user's M1 instruction); the visible substrate is the
/// flat field painted on top, not the clear. Keeping clear at zero-alpha while the
/// field is near-opaque is exactly what makes M1 transparency-capable without
/// committing to Zircon's full glass (design §V, Milestone 5).
pub const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// The flat field (Layer 0/1 of §V, collapsed for M1): a warm near-black tint at
/// ~0.92 alpha. Near-opaque so glyph antialiasing has a stable backing (dodging
/// the over-transparency muddiness the design flags for Zircon, §V), yet
/// translucent enough that the desktop reads faintly through it.
const FLAT_FIELD: Color32 = Color32::from_rgba_premultiplied(16, 14, 19, 235);

/// Warm off-white glyphs — a hair off pure white so the contrast doesn't ring
/// (the §V Blanco reasoning, applied to text here).
const GLYPH: Color32 = Color32::from_rgb(235, 232, 226);

/// The exact text the Face paints. A pure accessor so a regression that changes
/// the string is caught by a test, not only by a human reading the window.
fn line_text() -> &'static str {
    LINE
}

/// The font the line is painted in: Banquo's monospace family at the fixed size.
/// Pure, so "it's painted in `banquo-mono`, not the default proportional font" is
/// an assertion rather than an eyeball check.
fn glyph_font_id() -> FontId {
    FontId::new(FONT_SIZE, FontFamily::Name(BANQUO_MONO.into()))
}

/// Pure decision for the font-install latch: install only while not yet
/// installed. Kept as a named predicate (the build-plan's success criterion names
/// it) and reused inside [`FontInstaller`].
pub fn should_install_fonts(installed: bool) -> bool {
    !installed
}

/// The install-once latch as a tiny, context-free state machine. Holds the staged
/// font definitions and yields them on the *first* call only; every later call
/// returns `None`. This is the "fonts installed exactly once" guarantee as plain
/// state — testable with no `egui::Context` or window.
struct FontInstaller {
    staged: Option<egui::FontDefinitions>,
    installed: bool,
}

impl FontInstaller {
    fn new(staged: egui::FontDefinitions) -> Self {
        Self {
            staged: Some(staged),
            installed: false,
        }
    }

    /// Return the staged definitions exactly once (latching `installed`), then
    /// `None` forever after.
    fn take_for_install(&mut self) -> Option<egui::FontDefinitions> {
        if !should_install_fonts(self.installed) {
            return None;
        }
        self.installed = true;
        self.staged.take()
    }
}

/// Banquo's application state for Milestone 1.
pub struct BanquoApp {
    /// The install-once font latch.
    installer: FontInstaller,
    /// Which face actually backs [`BANQUO_MONO`] — reportable per guarantee #6.
    font_source: FontSource,
}

impl BanquoApp {
    /// Construct the app, resolving the font definitions up front (pure, no
    /// context needed) and staging them for first-frame install.
    pub fn new(_cc: &CreationContext<'_>) -> Self {
        let (defs, font_source) = build_font_definitions(EMBEDDED_IOSEVKA);
        Self {
            installer: FontInstaller::new(defs),
            font_source,
        }
    }

    /// Which face backs the monospace family (Iosevka when embedded, the built-in
    /// monospace on fallback). Exposed for honest reporting (guarantee #6).
    pub fn font_source(&self) -> FontSource {
        self.font_source
    }
}

impl App for BanquoApp {
    /// State-only pass (no painting): install fonts exactly once, on the first
    /// frame. eframe calls `logic` before each `ui`, with the context available
    /// and painting forbidden — the right home for the install-once latch.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(defs) = self.installer.take_for_install() {
            ctx.set_fonts(defs);
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
            TEXT_ALIGN,
            line_text(),
            glyph_font_id(),
            GLYPH,
        );
    }

    /// Fully transparent clear (see [`CLEAR_COLOR`]).
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        CLEAR_COLOR
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

    #[test]
    fn test_font_installer_yields_exactly_once() {
        let mut installer = FontInstaller::new(egui::FontDefinitions::default());
        assert!(
            installer.take_for_install().is_some(),
            "first frame must yield the staged fonts to install"
        );
        assert!(
            installer.take_for_install().is_none(),
            "subsequent frames must not re-yield (installed exactly once)"
        );
        assert!(
            installer.installed,
            "latch must be set after the first install"
        );
    }

    #[test]
    fn test_line_text_is_the_thesis() {
        assert_eq!(line_text(), "Banquo — gets kings, though he be none.");
    }

    #[test]
    fn test_glyph_uses_banquo_mono() {
        let font = glyph_font_id();
        assert_eq!(
            font,
            FontId::new(FONT_SIZE, FontFamily::Name(BANQUO_MONO.into())),
            "the line must be painted in banquo-mono at the fixed size, not the default font"
        );
        assert_eq!(
            font.family,
            FontFamily::Name(BANQUO_MONO.into()),
            "family must be banquo-mono"
        );
    }

    #[test]
    fn test_text_is_centered() {
        assert_eq!(TEXT_ALIGN, Align2::CENTER_CENTER);
    }

    #[test]
    fn test_transparency_invariants() {
        // The clear is fully transparent (window is transparency-capable)...
        assert_eq!(CLEAR_COLOR, [0.0, 0.0, 0.0, 0.0]);
        // ...while the flat field is near-opaque so AA has a stable backing.
        assert!(
            FLAT_FIELD.a() >= 230,
            "flat field must be near-opaque (alpha {} of 255)",
            FLAT_FIELD.a()
        );
    }
}
