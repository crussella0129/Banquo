//! The font side of *appearance* (design §I).
//!
//! This module is a pure, GUI-context-free function from "do we have an embedded
//! face?" to an [`egui::FontDefinitions`] plus an observable [`FontSource`]. It
//! holds no `egui::Context`, touches no wgpu, and never blocks — which is exactly
//! what lets it be unit-tested headlessly (the tests at the bottom run with no
//! window). The [`crate::app`] Face installs the result up front, in its
//! constructor, before the first frame.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

/// The family name Banquo registers for its monospace face. Callers paint with
/// `FontFamily::Name(BANQUO_MONO.into())`; it always resolves (to Iosevka when
/// embedded, to the built-in monospace alias on the fallback path).
pub const BANQUO_MONO: &str = "banquo-mono";

/// The embedded Iosevka regular face (SIL OFL 1.1), vendored at
/// `assets/fonts/Iosevka-Regular.ttf`. `include_bytes!` bakes it into the binary:
/// the font ships *with* the tool, consistent with the design's "Banquo never
/// opens a socket" stance (§VII) — nothing is fetched at runtime.
pub const EMBEDDED_IOSEVKA: Option<&[u8]> =
    Some(include_bytes!("../assets/fonts/Iosevka-Regular.ttf"));

/// The display (UI / hero) faces: proportional **Geist** (SIL OFL 1.1) as a
/// discrete weight ladder, vendored under `assets/fonts/geist/`.
///
/// Two deliberate facts shape this:
/// - **Geist is proportional, not monospace** — so it is for display text only
///   (the hero line now, the command palette later). The terminal *grid* must
///   stay monospace ([`BANQUO_MONO`] / Iosevka) to honor guarantee #3.
/// - **egui cannot drive a variable font's weight axis** through its public API,
///   so each weight is a separate static face registered under its own family
///   name. That's why we ship the static ladder rather than the variable file.
///
/// A deliberately small ladder — light for display headings, medium as the body
/// weight, semibold for emphasis. Not the whole Thin→Black range: the window
/// should never look like a font specimen sheet.
pub const GEIST_FACES: &[(&str, &[u8])] = &[
    (
        "geist-light",
        include_bytes!("../assets/fonts/geist/Geist-Light.ttf"),
    ),
    (
        "geist-medium",
        include_bytes!("../assets/fonts/geist/Geist-Medium.ttf"),
    ),
    (
        "geist-semibold",
        include_bytes!("../assets/fonts/geist/Geist-SemiBold.ttf"),
    ),
];

/// Which face actually backs [`BANQUO_MONO`] after building the definitions.
///
/// Honesty over silent fallback (design guarantee #6): the choice is a value the
/// Face can report, not a hidden state. `Embedded` = the vendored Iosevka was
/// registered; `Fallback` = no face was supplied and egui's built-in monospace
/// stands in under an alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSource {
    /// The embedded Iosevka face was registered as [`BANQUO_MONO`].
    Embedded,
    /// No embedded face was available; egui's built-in monospace stands in.
    Fallback,
}

/// Build the egui font definitions and report which source backs [`BANQUO_MONO`].
///
/// Pure over `embedded` — no `egui::Context`, no I/O. When `embedded` is
/// `Some(bytes)`, the bytes are registered as `banquo-mono` and also promoted to
/// the front of the `Monospace` family (so default monospace text is Iosevka
/// too). When `None`, the built-in fonts are kept and `banquo-mono` is aliased to
/// the built-in monospace so callers can always request the family safely.
pub fn build_font_definitions(embedded: Option<&[u8]>) -> (FontDefinitions, FontSource) {
    let mut defs = FontDefinitions::default();

    match embedded {
        Some(bytes) => {
            defs.font_data.insert(
                BANQUO_MONO.to_owned(),
                Arc::new(FontData::from_owned(bytes.to_vec())),
            );
            // banquo-mono as its own addressable family.
            defs.families.insert(
                FontFamily::Name(BANQUO_MONO.into()),
                vec![BANQUO_MONO.to_owned()],
            );
            // Promote it to lead the Monospace family so unadorned monospace
            // text renders in Iosevka as well.
            defs.families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, BANQUO_MONO.to_owned());
            (defs, FontSource::Embedded)
        }
        None => {
            // Keep egui's built-in monospace; alias banquo-mono to it so the
            // family is always resolvable regardless of source.
            let monospace = defs
                .families
                .get(&FontFamily::Monospace)
                .cloned()
                .unwrap_or_default();
            defs.families
                .insert(FontFamily::Name(BANQUO_MONO.into()), monospace);
            (defs, FontSource::Fallback)
        }
    }
}

/// Register the [`GEIST_FACES`] display ladder into `defs`. Each weight becomes
/// its own `FontFamily::Name("geist-…")`, addressable by the Face for display
/// text. Does not touch the monospace family — Geist is proportional.
fn register_geist_faces(defs: &mut FontDefinitions) {
    for (family, bytes) in GEIST_FACES {
        defs.font_data
            .insert((*family).to_owned(), Arc::new(FontData::from_static(bytes)));
        defs.families.insert(
            FontFamily::Name((*family).into()),
            vec![(*family).to_owned()],
        );
    }
}

/// Build the full font set Banquo paints with: the monospace face
/// ([`build_font_definitions`]) **plus** the Geist display ladder. This is what
/// the Face installs; `build_font_definitions` remains the pure mono-only core.
pub fn build_fonts(embedded: Option<&[u8]>) -> (FontDefinitions, FontSource) {
    let (mut defs, source) = build_font_definitions(embedded);
    register_geist_faces(&mut defs);
    (defs, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_fonts_with_embedded() {
        let bytes = EMBEDDED_IOSEVKA.expect("Iosevka is vendored at assets/fonts/");
        let (defs, source) = build_font_definitions(Some(bytes));

        assert_eq!(source, FontSource::Embedded);

        let family = defs.families.get(&FontFamily::Name(BANQUO_MONO.into()));
        assert!(family.is_some(), "banquo-mono family must be registered");
        assert!(
            !family.unwrap().is_empty(),
            "banquo-mono family must have at least one registered font"
        );

        // Iosevka leads the Monospace family.
        assert_eq!(
            defs.families
                .get(&FontFamily::Monospace)
                .and_then(|fonts| fonts.first())
                .map(String::as_str),
            Some(BANQUO_MONO),
            "embedded Iosevka should lead the Monospace family"
        );
    }

    #[test]
    fn test_banquo_mono_binds_in_a_real_context() {
        // Regression guard for the "FontFamily::Name(banquo-mono) is not bound to
        // any fonts" panic: build the definitions, install them into a real
        // (headless) egui context, run one frame, and actually lay out text in the
        // banquo-mono family. If the family weren't registered/resolvable, the
        // `painter.text` call would panic and fail this test.
        let ctx = egui::Context::default();
        let (defs, _) = build_font_definitions(EMBEDDED_IOSEVKA);
        ctx.set_fonts(defs);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let family = egui::FontFamily::Name(BANQUO_MONO.into());
            // The exact inverse of the runtime panic: the family must be *bound*.
            let bound = ui.ctx().fonts(|fonts| fonts.families().contains(&family));
            assert!(
                bound,
                "banquo-mono must be a bound font family after set_fonts"
            );
            // And it must actually provide glyphs.
            let has = ui.ctx().fonts_mut(|fonts| {
                fonts.has_glyphs(&egui::FontId::new(14.0, family.clone()), "banquo")
            });
            assert!(has, "banquo-mono must provide glyphs for the painted text");
        });
    }

    #[test]
    fn test_build_fonts_registers_geist_ladder() {
        let (defs, _) = build_fonts(EMBEDDED_IOSEVKA);
        // Every Geist weight is registered as its own addressable family...
        for (family, _) in GEIST_FACES {
            assert!(
                defs.families
                    .contains_key(&FontFamily::Name((*family).into())),
                "display family `{family}` must be registered"
            );
        }
        // ...and the mono family is untouched (Geist is proportional).
        assert!(defs
            .families
            .contains_key(&FontFamily::Name(BANQUO_MONO.into())));
    }

    #[test]
    fn test_build_fonts_fallback() {
        let (defs, source) = build_font_definitions(None);

        assert_eq!(source, FontSource::Fallback);

        // Built-in monospace must still resolve (no panic, non-empty).
        let mono = defs.families.get(&FontFamily::Monospace);
        assert!(
            mono.is_some() && !mono.unwrap().is_empty(),
            "built-in Monospace family must still resolve on the fallback path"
        );

        // banquo-mono is aliased so callers can always request it.
        assert!(
            defs.families
                .contains_key(&FontFamily::Name(BANQUO_MONO.into())),
            "banquo-mono should be aliased to the built-in monospace on fallback"
        );
    }
}
