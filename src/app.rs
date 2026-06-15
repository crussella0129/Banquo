//! The Face (design §IV): the UI half that paints appearance.
//!
//! At Milestone 1 there is no truth-half snapshot to read yet, so the Face paints
//! a restrained two-line card: the hero tagline in **Geist** (display) above one
//! **Iosevka** monospace line (the grid's voice). Two type roles, no clutter.
//! The shape is already the design's: a pure function of (fonts, time) → pixels.
//!
//! Everything that decides *what* gets painted is factored into pure helpers and
//! data, so the Face's choices are asserted headlessly (see the tests) — only the
//! actual rasterization needs a window.

use eframe::{App, CreationContext};
use egui::{pos2, Align2, Color32, FontFamily, FontId};

use crate::fonts::{build_fonts, FontSource, BANQUO_MONO, EMBEDDED_IOSEVKA};

/// The hero line — Banquo's promise (and the project's subtitle).
const HERO: &str = "A Most Beautiful Terminal.";
/// Hero size in logical points.
const HERO_SIZE: f32 = 42.0;
/// The Geist weight the hero is set in (light reads elegant at display size).
const HERO_FAMILY: &str = "geist-light";

/// The one monospace line — the terminal's own voice. Iosevka, fixed size: this
/// is the face the grid will use, so it must be monospace (guarantee #3).
const MONO_LINE: &str = "user@banquo:~$  cargo run   # the grid speaks Iosevka";
/// Mono line size in logical points. Font size is a *setting*, not a function of
/// window size (guarantee #4) — hence a constant.
const MONO_SIZE: f32 = 16.0;

/// The framebuffer clear color: fully transparent. The window is genuinely
/// transparency-capable (the user's M1 instruction); the visible substrate is the
/// flat field painted on top, not the clear. Keeping clear at zero-alpha while the
/// field is near-opaque is what makes M1 transparency-capable without committing
/// to Zircon's full glass (design §V, Milestone 5).
pub const CLEAR_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// The flat field (Layer 0/1 of §V, collapsed for M1): a warm near-black tint at
/// ~0.92 alpha. Near-opaque so glyph antialiasing has a stable backing (dodging
/// the over-transparency muddiness the design flags for Zircon, §V), yet
/// translucent enough that the desktop reads faintly through it.
const FLAT_FIELD: Color32 = Color32::from_rgba_premultiplied(16, 14, 19, 235);

/// Warm off-white glyphs — a hair off pure white so the contrast doesn't ring
/// (the §V Blanco reasoning, applied to text here).
const GLYPH: Color32 = Color32::from_rgb(235, 232, 226);

/// Dimmer ink for the secondary monospace line.
const GLYPH_DIM: Color32 = Color32::from_rgb(150, 150, 158);

/// The hero font: Geist at display size. Pure, so "the hero is Geist, not the
/// default font" is a test rather than an eyeball check.
fn hero_font() -> FontId {
    FontId::new(HERO_SIZE, FontFamily::Name(HERO_FAMILY.into()))
}

/// The monospace line's font: Iosevka (`banquo-mono`) at the fixed size.
fn mono_font() -> FontId {
    FontId::new(MONO_SIZE, FontFamily::Name(BANQUO_MONO.into()))
}

/// Banquo's application state for Milestone 1.
pub struct BanquoApp {
    /// Which face actually backs [`BANQUO_MONO`] — reportable per guarantee #6.
    font_source: FontSource,
}

impl BanquoApp {
    /// Construct the app and install the fonts **up front**, before the first
    /// frame.
    ///
    /// Fonts must be set here (via `cc.egui_ctx`), not on the first frame:
    /// `Context::set_fonts` only takes effect at the *next* frame's font-atlas
    /// rebuild, so installing it during the first `ui` pass would paint a family
    /// before it is bound and panic ("not bound to any fonts"). Installing in
    /// `new` means every family is resolvable by the time anything paints — and it
    /// happens exactly once by construction.
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let (defs, font_source) = build_fonts(EMBEDDED_IOSEVKA);
        cc.egui_ctx.set_fonts(defs);
        let app = Self { font_source };
        // Honest report of which face actually backs the monospace family
        // (guarantee #6) — local stderr, once, no telemetry.
        eprintln!("banquo: monospace face = {:?}", app.font_source());
        app
    }

    /// Which face backs the monospace family (Iosevka when embedded, the built-in
    /// monospace on fallback). Exposed for honest reporting (guarantee #6).
    pub fn font_source(&self) -> FontSource {
        self.font_source
    }
}

impl App for BanquoApp {
    /// Paint pass. eframe hands us a margin-less, background-less `Ui` spanning
    /// the root viewport — this *is* the central area. We fill the flat field and
    /// paint the specimen as a centered vertical stack.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        let painter = ui.painter();
        // Flat field (the M1 substrate): a single filled rect over the whole area.
        painter.rect_filled(rect, 0.0, FLAT_FIELD);

        // Two centered lines: hero just above the midline, the mono line just
        // below it — balanced, uncluttered.
        let cx = rect.center().x;
        let cy = rect.center().y;
        painter.text(
            pos2(cx, cy - 16.0),
            Align2::CENTER_BOTTOM,
            HERO,
            hero_font(),
            GLYPH,
        );
        painter.text(
            pos2(cx, cy + 18.0),
            Align2::CENTER_TOP,
            MONO_LINE,
            mono_font(),
            GLYPH_DIM,
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
    fn test_hero_is_the_tagline() {
        assert_eq!(HERO, "A Most Beautiful Terminal.");
    }

    #[test]
    fn test_hero_uses_geist() {
        assert_eq!(
            hero_font().family,
            FontFamily::Name(HERO_FAMILY.into()),
            "the hero must be set in a Geist display family"
        );
        assert!(
            HERO_FAMILY.starts_with("geist-"),
            "the hero family must be a Geist weight"
        );
    }

    #[test]
    fn test_mono_line_uses_iosevka() {
        assert_eq!(
            mono_font().family,
            FontFamily::Name(BANQUO_MONO.into()),
            "the terminal line must be painted in the monospace family (guarantee #3)"
        );
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
