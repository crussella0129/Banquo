//! The font side of *appearance* (design §I).
//!
//! This module is a pure, GUI-context-free function from "do we have an embedded
//! face?" to an [`egui::FontDefinitions`] plus an observable [`FontSource`]. It
//! holds no `egui::Context`, touches no wgpu, and never blocks — which is exactly
//! what lets it be unit-tested headlessly (the tests at the bottom run with no
//! window). The [`crate::app`] Face installs the result up front, in its
//! constructor, before the first frame.

use std::sync::Arc;
use std::fs;
use egui::{FontData, FontDefinitions, FontFamily};

/// The family name Banquo registers for its monospace face. Callers paint with
/// `FontFamily::Name(BANQUO_MONO.into())`; it always resolves (to Geist Mono when
/// embedded, to the user's path, or to the built-in monospace alias on fallback).
pub const BANQUO_MONO: &str = "banquo-mono";

/// The family name for the OFL serif face (EB Garamond).
pub const BANQUO_SERIF: &str = "banquo-serif";

/// The embedded Geist Sans regular face (SIL OFL 1.1), vendored at
/// `assets/fonts/Geist-Regular.ttf`.
pub const EMBEDDED_GEIST_MONO: Option<&[u8]> =
    Some(include_bytes!("../assets/fonts/Geist-Regular.ttf"));

/// The embedded EB Garamond face (SIL OFL 1.1), vendored at
/// `assets/fonts/EBGaramond-Regular.ttf`.
pub const EMBEDDED_SERIF: Option<&[u8]> =
    Some(include_bytes!("../assets/fonts/EBGaramond-Regular.ttf"));

/// The display (UI / hero) faces: proportional **Geist** (SIL OFL 1.1) as a
/// discrete weight ladder, vendored under `assets/fonts/geist/`.
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
/// Face can report, not a hidden state. `Embedded` = the vendored default was
/// registered; `UserPath` = the user's config file loaded successfully; 
/// `Fallback` = no face was supplied and egui's built-in monospace stands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontSource {
    Embedded,
    UserPath(String),
    Fallback,
}

/// Helper to load font data either from a user config path or a fallback embed.
fn load_font(
    defs: &mut FontDefinitions,
    family_name: &str,
    user_path: &Option<String>,
    embedded: Option<&[u8]>,
    is_monospace: bool,
) -> FontSource {
    let mut loaded_bytes = None;
    let mut source = FontSource::Fallback;

    // 1. Try user path
    if let Some(path) = user_path {
        if let Ok(bytes) = fs::read(path) {
            loaded_bytes = Some(bytes);
            source = FontSource::UserPath(path.clone());
        } else {
            eprintln!("banquo: Failed to load font from {}; falling back.", path);
        }
    }

    // 2. Try embedded
    if loaded_bytes.is_none() {
        if let Some(bytes) = embedded {
            loaded_bytes = Some(bytes.to_vec());
            source = FontSource::Embedded;
        }
    }

    // 3. Register
    if let Some(bytes) = loaded_bytes {
        defs.font_data.insert(
            family_name.to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        defs.families.insert(
            FontFamily::Name(family_name.into()),
            vec![family_name.to_owned()],
        );
        if is_monospace {
            // Promote to lead the default Monospace family
            defs.families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, family_name.to_owned());
        }
    } else if is_monospace {
        // Fallback alias
        let monospace = defs
            .families
            .get(&FontFamily::Monospace)
            .cloned()
            .unwrap_or_default();
        defs.families
            .insert(FontFamily::Name(family_name.into()), monospace);
    }

    source
}

/// Register the [`GEIST_FACES`] display ladder into `defs`. Each weight becomes
/// its own `FontFamily::Name("geist-…")`, addressable by the Face for display text.
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

/// Build the full font set Banquo paints with: the monospace face, the serif face,
/// and the Geist display ladder. This is what the Face installs.
pub fn build_fonts(config: &crate::config::BanquoConfig) -> (FontDefinitions, FontSource) {
    let mut defs = FontDefinitions::default();

    // Mono
    let mono_source = load_font(
        &mut defs,
        BANQUO_MONO,
        &config.fonts.monospace_path,
        EMBEDDED_GEIST_MONO,
        true,
    );

    // Serif
    // load_font(
    //     &mut defs,
    //     BANQUO_SERIF,
    //     &config.fonts.serif_path,
    //     EMBEDDED_SERIF,
    //     false,
    // );

    // Display
    register_geist_faces(&mut defs);

    (defs, mono_source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BanquoConfig;

    #[test]
    fn test_build_fonts_with_embedded() {
        let config = BanquoConfig::default();
        let (defs, source) = build_fonts(&config);

        assert_eq!(source, FontSource::Embedded);

        let family = defs.families.get(&FontFamily::Name(BANQUO_MONO.into()));
        assert!(family.is_some(), "banquo-mono family must be registered");
        assert!(
            !family.unwrap().is_empty(),
            "banquo-mono family must have at least one registered font"
        );
    }

    #[test]
    fn test_banquo_mono_binds_in_a_real_context() {
        let ctx = egui::Context::default();
        let config = BanquoConfig::default();
        let (defs, _) = build_fonts(&config);
        ctx.set_fonts(defs);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let family = egui::FontFamily::Name(BANQUO_MONO.into());
            let bound = ui.ctx().fonts(|fonts| fonts.families().contains(&family));
            assert!(bound, "banquo-mono must be bound");
        });
    }

    #[test]
    fn test_build_fonts_registers_geist_ladder() {
        let config = BanquoConfig::default();
        let (defs, _) = build_fonts(&config);
        for (family, _) in GEIST_FACES {
            assert!(
                defs.families
                    .contains_key(&FontFamily::Name((*family).into())),
                "display family `{family}` must be registered"
            );
        }
    }
}
