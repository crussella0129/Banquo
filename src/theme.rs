//! The theme engine — appearance as data (design §IV, guarantee #5).
//!
//! A theme used to be a string matched in half a dozen places inside the Face.
//! This module replaces that with a single value: [`ThemeSpec`], a pure
//! description of everything theme-dependent the Face paints — background
//! fill, procedural texture, default-foreground remap, and cursor colors.
//! The six builtin themes are entries in a data table; a user's `[colors]`
//! config section overlays any of those fields, so custom themes are plain
//! TOML, no recompile.
//!
//! Everything here is pure and GUI-context-free, so it is unit-tested
//! headlessly.

// TODO(T-1903): remove once the Face consumes ThemeSpec (dead only until then).
#![allow(dead_code)]

use egui::Color32;

/// Which procedural background texture a theme uses.
///
/// `Flat` means no texture — the background is a solid [`ThemeSpec::background`]
/// fill (zircon, volcanic-glass). The textured kinds map 1:1 onto the
/// generators in [`crate::texture_gen`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureKind {
    Flat,
    Blanco,
    Concrete,
    ConcreteDark,
    Primordial,
}

/// Everything theme-dependent the Face paints, as one pure value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeSpec {
    /// The substrate fill (used directly when `texture` is `Flat`; textured
    /// themes use it as the fallback fill while the texture uploads).
    pub background: Color32,
    /// Procedural texture selection.
    pub texture: TextureKind,
    /// When `Some`, default (light-grayscale) terminal text is remapped to
    /// this color — how blanco/concrete get dark glyphs and volcanic gets
    /// blood-red ones. `None` leaves the PTY's colors untouched.
    pub fg_remap: Option<Color32>,
    /// The cursor block color.
    pub cursor: Color32,
    /// The color of the glyph painted *under* the cursor block.
    pub cursor_text: Color32,
}

/// The canonical builtin theme names, in presentation order.
pub const BUILTIN_NAMES: [&str; 6] = [
    "zircon",
    "blanco",
    "concrete",
    "concrete-dark",
    "primordial",
    "volcanic-glass",
];

/// Normalize a user-supplied theme name to canonical form: lowercase,
/// `_`/space → `-`, plus legacy aliases (bare `"volcanic"` has been accepted
/// since the theme shipped and must keep working).
pub fn normalize_name(name: &str) -> String {
    let mut n = name.trim().to_lowercase().replace(['_', ' '], "-");
    if n == "volcanic" || n == "volcanic-glass" {
        n = "volcanic-glass".to_string();
    }
    n
}

/// The builtin spec table. Colors are exactly the values the Face hardcoded
/// before sprint 19 — the refactor moved them, it did not change them.
pub fn builtin_spec(name: &str) -> Option<ThemeSpec> {
    let spec = match normalize_name(name).as_str() {
        "zircon" => ThemeSpec {
            background: Color32::from_black_alpha(142),
            texture: TextureKind::Flat,
            fg_remap: None,
            cursor: Color32::from_rgba_premultiplied(235, 232, 226, 180),
            cursor_text: Color32::BLACK,
        },
        "blanco" => ThemeSpec {
            background: Color32::WHITE,
            texture: TextureKind::Blanco,
            fg_remap: Some(Color32::from_rgb(15, 15, 20)),
            cursor: Color32::from_rgba_premultiplied(235, 232, 226, 180),
            cursor_text: Color32::BLACK,
        },
        "concrete" => ThemeSpec {
            background: Color32::from_rgb(180, 180, 180),
            texture: TextureKind::Concrete,
            fg_remap: Some(Color32::from_rgb(15, 15, 20)),
            cursor: Color32::from_rgba_premultiplied(70, 70, 75, 180),
            cursor_text: Color32::WHITE,
        },
        "concrete-dark" => ThemeSpec {
            background: Color32::from_rgb(15, 15, 15),
            texture: TextureKind::ConcreteDark,
            fg_remap: None,
            cursor: Color32::from_rgba_premultiplied(235, 232, 226, 180),
            cursor_text: Color32::BLACK,
        },
        "primordial" => ThemeSpec {
            background: Color32::from_black_alpha(204),
            texture: TextureKind::Primordial,
            fg_remap: Some(Color32::from_rgb(160, 40, 200)),
            cursor: Color32::from_rgba_premultiplied(160, 40, 200, 180),
            cursor_text: Color32::BLACK,
        },
        "volcanic-glass" => ThemeSpec {
            background: Color32::from_rgba_unmultiplied(0, 0, 0, 200),
            texture: TextureKind::Flat,
            fg_remap: Some(Color32::from_rgb(200, 10, 10)),
            cursor: Color32::from_rgba_premultiplied(180, 15, 15, 180),
            cursor_text: Color32::WHITE,
        },
        _ => return None,
    };
    Some(spec)
}

/// Parse `#RRGGBB` or `#RRGGBBAA` (alpha unmultiplied). Anything else → `None`.
pub fn parse_hex_color(s: &str) -> Option<Color32> {
    let hex = s.trim().strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    match hex.len() {
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)?,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name_aliases() {
        assert_eq!(normalize_name("volcanic_glass"), "volcanic-glass");
        assert_eq!(normalize_name("volcanic glass"), "volcanic-glass");
        assert_eq!(normalize_name("Volcanic-Glass"), "volcanic-glass");
        // Bare legacy alias, accepted since the theme shipped.
        assert_eq!(normalize_name("volcanic"), "volcanic-glass");
        assert_eq!(normalize_name("concrete_dark"), "concrete-dark");
        assert_eq!(normalize_name("zircon"), "zircon");
    }

    #[test]
    fn test_builtin_spec_all_six() {
        // Backgrounds must equal the pre-sprint-19 hardcoded values exactly.
        assert_eq!(
            builtin_spec("zircon").unwrap().background,
            Color32::from_black_alpha(142)
        );
        assert_eq!(builtin_spec("blanco").unwrap().background, Color32::WHITE);
        assert_eq!(
            builtin_spec("concrete").unwrap().background,
            Color32::from_rgb(180, 180, 180)
        );
        assert_eq!(
            builtin_spec("concrete-dark").unwrap().background,
            Color32::from_rgb(15, 15, 15)
        );
        assert_eq!(
            builtin_spec("primordial").unwrap().background,
            Color32::from_black_alpha(204)
        );
        assert_eq!(
            builtin_spec("volcanic-glass").unwrap().background,
            Color32::from_rgba_unmultiplied(0, 0, 0, 200)
        );
        // Every canonical name resolves.
        for name in BUILTIN_NAMES {
            assert!(builtin_spec(name).is_some(), "builtin {name} must resolve");
        }
    }

    #[test]
    fn test_builtin_spec_unknown_none() {
        assert!(builtin_spec("nonexistent").is_none());
    }

    #[test]
    fn test_builtin_cursor_and_remap_values() {
        assert_eq!(
            builtin_spec("concrete").unwrap().cursor,
            Color32::from_rgba_premultiplied(70, 70, 75, 180)
        );
        assert_eq!(
            builtin_spec("volcanic-glass").unwrap().cursor,
            Color32::from_rgba_premultiplied(180, 15, 15, 180)
        );
        assert_eq!(
            builtin_spec("primordial").unwrap().cursor,
            Color32::from_rgba_premultiplied(160, 40, 200, 180)
        );
        assert_eq!(
            builtin_spec("zircon").unwrap().cursor,
            Color32::from_rgba_premultiplied(235, 232, 226, 180)
        );
        assert_eq!(
            builtin_spec("blanco").unwrap().fg_remap,
            Some(Color32::from_rgb(15, 15, 20))
        );
        assert_eq!(
            builtin_spec("concrete").unwrap().fg_remap,
            Some(Color32::from_rgb(15, 15, 20))
        );
        assert_eq!(
            builtin_spec("volcanic-glass").unwrap().fg_remap,
            Some(Color32::from_rgb(200, 10, 10))
        );
        assert_eq!(builtin_spec("zircon").unwrap().fg_remap, None);
        // Cursor-text inverse rule: white for volcanic/concrete, black otherwise.
        assert_eq!(
            builtin_spec("volcanic-glass").unwrap().cursor_text,
            Color32::WHITE
        );
        assert_eq!(
            builtin_spec("concrete").unwrap().cursor_text,
            Color32::WHITE
        );
        assert_eq!(builtin_spec("zircon").unwrap().cursor_text, Color32::BLACK);
        assert_eq!(builtin_spec("blanco").unwrap().cursor_text, Color32::BLACK);
    }

    #[test]
    fn test_parse_hex_rrggbb() {
        assert_eq!(
            parse_hex_color("#1a2b3c"),
            Some(Color32::from_rgb(0x1a, 0x2b, 0x3c))
        );
    }

    #[test]
    fn test_parse_hex_rrggbbaa() {
        assert_eq!(
            parse_hex_color("#1a2b3c80"),
            Some(Color32::from_rgba_unmultiplied(0x1a, 0x2b, 0x3c, 0x80))
        );
    }

    #[test]
    fn test_parse_hex_invalid() {
        assert_eq!(parse_hex_color("red"), None);
        assert_eq!(parse_hex_color("#12"), None);
        assert_eq!(parse_hex_color("#GGGGGG"), None);
        assert_eq!(parse_hex_color(""), None);
    }
}
