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

/// Resolve the spec the Face paints with: the builtin spec for the config's
/// theme (zircon's for unknown/custom names — the pre-sprint-19 `_ =>`
/// fallback), overlaid with any `[colors]` overrides. Invalid hex values are
/// ignored here (the builtin value stands); `banquo check` reports them.
pub fn resolve_spec(config: &crate::config::BanquoConfig) -> ThemeSpec {
    let name = config.theme.as_deref().unwrap_or("zircon");
    let mut spec = builtin_spec(name)
        .unwrap_or_else(|| builtin_spec("zircon").expect("zircon is always a builtin"));

    let colors = &config.colors;
    if let Some(c) = colors.background.as_deref().and_then(parse_hex_color) {
        spec.background = c;
    }
    if let Some(c) = colors.foreground.as_deref().and_then(parse_hex_color) {
        spec.fg_remap = Some(c);
    }
    if let Some(c) = colors.cursor.as_deref().and_then(parse_hex_color) {
        spec.cursor = c;
    }
    if let Some(c) = colors.cursor_text.as_deref().and_then(parse_hex_color) {
        spec.cursor_text = c;
    }
    spec
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

    // --- T-1902: [colors] overlay resolution ---

    fn cfg(theme: &str, colors_toml: &str) -> crate::config::BanquoConfig {
        let src = format!("theme = \"{theme}\"\n{colors_toml}");
        toml::from_str(&src).expect("valid TOML")
    }

    #[test]
    fn test_resolve_spec_no_colors_section() {
        let c = cfg("zircon", "");
        assert_eq!(resolve_spec(&c), builtin_spec("zircon").unwrap());
    }

    #[test]
    fn test_resolve_spec_background_override() {
        let c = cfg("zircon", "[colors]\nbackground = \"#101010\"");
        // Whole-struct equality: ONLY background may differ from the builtin.
        let expected = ThemeSpec {
            background: Color32::from_rgb(0x10, 0x10, 0x10),
            ..builtin_spec("zircon").unwrap()
        };
        assert_eq!(resolve_spec(&c), expected);
    }

    #[test]
    fn test_resolve_spec_foreground_sets_remap() {
        let c = cfg("zircon", "[colors]\nforeground = \"#ff0000\"");
        assert_eq!(
            resolve_spec(&c).fg_remap,
            Some(Color32::from_rgb(255, 0, 0))
        );
    }

    #[test]
    fn test_resolve_spec_unknown_theme_falls_back_zircon() {
        let c = cfg("mytheme", "[colors]\nbackground = \"#223344\"");
        let spec = resolve_spec(&c);
        assert_eq!(spec.background, Color32::from_rgb(0x22, 0x33, 0x44));
        // Everything not overridden comes from zircon.
        assert_eq!(spec.cursor, builtin_spec("zircon").unwrap().cursor);
        assert_eq!(spec.texture, TextureKind::Flat);
    }

    #[test]
    fn test_resolve_spec_invalid_hex_ignored() {
        let c = cfg("zircon", "[colors]\ncursor = \"banana\"");
        assert_eq!(
            resolve_spec(&c).cursor,
            builtin_spec("zircon").unwrap().cursor
        );
    }

    #[test]
    fn test_theme_pipeline_config_to_spec() {
        // Component-A integration: full TOML → BanquoConfig → resolve_spec.
        let src = r##"
theme = "volcanic_glass"

[colors]
cursor = "#ff00ff"
"##;
        let c: crate::config::BanquoConfig = toml::from_str(src).unwrap();
        let spec = resolve_spec(&c);
        // Alias normalized to volcanic-glass; cursor overridden; rest builtin.
        assert_eq!(spec.cursor, Color32::from_rgb(255, 0, 255));
        assert_eq!(
            spec.fg_remap,
            builtin_spec("volcanic-glass").unwrap().fg_remap
        );
    }

    #[test]
    fn test_colors_config_deserializes() {
        let c = cfg(
            "zircon",
            "[colors]\nbackground = \"#000000\"\nforeground = \"#ffffff\"",
        );
        assert_eq!(c.colors.background.as_deref(), Some("#000000"));
        assert_eq!(c.colors.foreground.as_deref(), Some("#ffffff"));
        assert!(c.colors.cursor.is_none());
        let plain = cfg("zircon", "");
        assert!(plain.colors.background.is_none());
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
