//! The font side of *appearance* (design §I).
//!
//! This module is a pure, GUI-context-free function from "do we have an embedded
//! face?" to an [`egui::FontDefinitions`] plus an observable [`FontSource`]. It
//! holds no `egui::Context`, touches no wgpu, and never blocks — which is exactly
//! what lets it be unit-tested headlessly (the tests at the bottom run with no
//! window). The [`crate::app`] Face installs the result up front, in its
//! constructor, before the first frame.

use egui::{FontData, FontDefinitions, FontFamily};
use std::fs;
use std::sync::Arc;

/// The family name Banquo registers for its monospace face. Callers paint with
/// `FontFamily::Name(BANQUO_MONO.into())`; it always resolves (to Geist Mono when
/// embedded, to the user's path, or to the built-in monospace alias on fallback).
pub const BANQUO_MONO: &str = "banquo-mono";
pub const BANQUO_SYMBOLS: &str = "banquo-symbols";

/// Which face actually backs [`BANQUO_MONO`] after building the definitions.
///
/// Honesty over silent fallback (design guarantee #6): the choice is a value the
/// Face can report, not a hidden state. `Embedded` = the vendored default was
/// registered; `UserPath` = the user's config file loaded successfully;
/// `Fallback` = no face was supplied and egui's built-in monospace stands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontSource {
    UserPath(String),
    Fallback,
}

/// Helper to load font data either from a user config path or a fallback embed.
fn load_font(
    defs: &mut FontDefinitions,
    family_name: &str,
    user_path: &Option<String>,
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

    // 2. Register
    if let Some(bytes) = loaded_bytes {
        defs.font_data.insert(
            family_name.to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        let mut fallbacks = vec![family_name.to_owned()];

        // Ensure default fallbacks are preserved so that emojis and symbols render
        // properly if the user-selected font lacks them!
        if is_monospace {
            if let Some(default_mono) = defs.families.get(&FontFamily::Monospace) {
                for fb in default_mono {
                    if !fallbacks.contains(fb) {
                        fallbacks.push(fb.clone());
                    }
                }
            }
        }

        defs.families
            .insert(FontFamily::Name(family_name.into()), fallbacks);
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

/// Build the full font set Banquo paints with: the monospace face, and the fallback symbols face.
/// This is what the Face installs.
pub fn build_fonts(config: &crate::config::BanquoConfig) -> (FontDefinitions, FontSource) {
    let mut defs = FontDefinitions::default();

    // Mono
    let mono_source = load_font(&mut defs, BANQUO_MONO, &config.fonts.monospace_path, true);

    // Symbols (Font Fallback)
    // If a symbols_path is provided, we load it. Otherwise we fallback to the
    // default mono font. We don't embed a separate symbols font by default to
    // keep binary size down.
    if let Some(symbols_path) = &config.fonts.symbols_path {
        load_font(
            &mut defs,
            BANQUO_SYMBOLS,
            &Some(symbols_path.clone()),
            false,
        );
    } else {
        // Alias symbols to mono if no path provided
        let mut fallbacks = Vec::new();
        if defs.font_data.contains_key(BANQUO_MONO) {
            fallbacks.push(BANQUO_MONO.to_owned());
        }
        if let Some(default_mono) = defs.families.get(&FontFamily::Monospace) {
            fallbacks.extend(default_mono.iter().cloned());
        }
        defs.families
            .insert(FontFamily::Name(BANQUO_SYMBOLS.into()), fallbacks);
    }

    // Force all UI proportional fonts to use our monospace font instead,
    // ensuring the entire app is strictly monospace.
    let mut mono_fallbacks = Vec::new();
    if defs.font_data.contains_key(BANQUO_MONO) {
        mono_fallbacks.push(BANQUO_MONO.to_owned());
    }
    if let Some(default_mono) = defs.families.get(&FontFamily::Monospace) {
        mono_fallbacks.extend(default_mono.iter().cloned());
    }
    defs.families
        .insert(FontFamily::Proportional, mono_fallbacks);

    (defs, mono_source)
}
