//! Presets — portable appearance bundles, embedded in the binary.
//!
//! A preset is a TOML fragment carrying *appearance only* (theme + window
//! chrome + UI). It never contains font paths, shell profiles, or anything
//! machine-specific, so it is safe to share — commit one to a dotfiles repo,
//! hand it to a friend, drop theirs into your presets directory.
//!
//! Lookup order: the user presets directory (`<config-dir>/banquo/presets/
//! <name>.toml`) first, then the six builtins embedded via `include_str!`
//! (single source of truth with the repo's `configs/` files). Nothing is ever
//! resolved relative to the process CWD — the palette and CLI work identically
//! from an installed binary and a source checkout.

// TODO(T-1909/T-1910): remove once the CLI and palette consume this module.
#![allow(dead_code)]

use crate::theme::normalize_name;
use std::path::PathBuf;

/// The six builtin presets, embedded at compile time. Keys are canonical
/// theme names (see [`crate::theme::BUILTIN_NAMES`]).
const BUILTINS: [(&str, &str); 6] = [
    ("zircon", include_str!("../configs/zircon.toml")),
    ("blanco", include_str!("../configs/blanco.toml")),
    ("concrete", include_str!("../configs/concrete.toml")),
    (
        "concrete-dark",
        include_str!("../configs/concrete-dark.toml"),
    ),
    ("primordial", include_str!("../configs/primordial.toml")),
    (
        "volcanic-glass",
        include_str!("../configs/volcanic-glass.toml"),
    ),
];

/// Canonical names of the builtin presets, in presentation order.
pub fn builtin_names() -> Vec<&'static str> {
    crate::theme::BUILTIN_NAMES.to_vec()
}

/// The embedded TOML for a builtin preset. Accepts any alias
/// (`volcanic_glass`, `Volcanic Glass`, bare `volcanic`, …).
pub fn builtin(name: &str) -> Option<&'static str> {
    let canonical = normalize_name(name);
    BUILTINS
        .iter()
        .find(|(n, _)| *n == canonical)
        .map(|(_, toml)| *toml)
}

/// Where a found preset came from — shown by `banquo preset list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetSource {
    Builtin,
    User(PathBuf),
}

/// A found preset: its TOML content and provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub content: String,
    pub source: PresetSource,
}

/// The user presets directory: `<config-dir>/banquo/presets`.
pub fn user_preset_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("banquo");
        p.push("presets");
        p
    })
}

/// Pure lookup over explicit directories (unit-testable, no ambient state):
/// the first `dir/<name>.toml` wins; builtins are the fallback.
pub fn find_in(user_dirs: &[PathBuf], name: &str) -> Option<Preset> {
    let canonical = normalize_name(name);
    for dir in user_dirs {
        let path = dir.join(format!("{canonical}.toml"));
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(Preset {
                content,
                source: PresetSource::User(path),
            });
        }
    }
    builtin(&canonical).map(|content| Preset {
        content: content.to_string(),
        source: PresetSource::Builtin,
    })
}

/// Find a preset by name: user presets directory first, then builtins.
pub fn find(name: &str) -> Option<Preset> {
    let dirs: Vec<PathBuf> = user_preset_dir().into_iter().collect();
    find_in(&dirs, name)
}

/// Every available preset name: builtins plus any `*.toml` in the user
/// presets directory (user presets flagged `true`). Sorted, deduplicated —
/// a user preset shadowing a builtin name is listed once, as a user preset.
pub fn list() -> Vec<(String, bool)> {
    let dirs: Vec<PathBuf> = user_preset_dir().into_iter().collect();
    list_in(&dirs)
}

/// Pure form of [`list`] over explicit directories.
pub fn list_in(user_dirs: &[PathBuf]) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for dir in user_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        out.push((stem.to_string(), true));
                    }
                }
            }
        }
    }
    for name in builtin_names() {
        if !out.iter().any(|(n, _)| n == name) {
            out.push((name.to_string(), false));
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- T-1904 gates: shipped presets are portable and well-formed ---

    #[test]
    fn test_presets_parse_as_config() {
        for (name, content) in BUILTINS {
            let cfg: Result<crate::config::BanquoConfig, _> = toml::from_str(content);
            assert!(cfg.is_ok(), "builtin preset {name} must parse: {cfg:?}");
        }
    }

    #[test]
    fn test_presets_are_portable() {
        for (name, content) in BUILTINS {
            for marker in ["monospace_path", "symbols_path", "[shell]", ":/", ":\\"] {
                assert!(
                    !content.contains(marker),
                    "builtin preset {name} must not contain {marker:?}"
                );
            }
        }
    }

    #[test]
    fn test_presets_tab_bar_mode_recognized() {
        for (name, content) in BUILTINS {
            let cfg: crate::config::BanquoConfig = toml::from_str(content).unwrap();
            if let Some(mode) = cfg.ui.tab_bar_mode.as_deref() {
                assert!(
                    mode == "auto" || mode == "persistent",
                    "preset {name} tab_bar_mode {mode:?} must be a recognized value"
                );
            }
        }
    }

    #[test]
    fn test_presets_theme_matches_name() {
        for (name, content) in BUILTINS {
            let cfg: crate::config::BanquoConfig = toml::from_str(content).unwrap();
            assert_eq!(
                cfg.theme.as_deref(),
                Some(name),
                "preset {name}'s theme field must equal its canonical name"
            );
        }
    }

    // --- T-1905: lookup ---

    #[test]
    fn test_builtin_names_six() {
        let names = builtin_names();
        assert_eq!(names.len(), 6);
        for (name, _) in BUILTINS {
            assert!(names.contains(&name), "{name} missing from builtin_names");
        }
    }

    #[test]
    fn test_builtin_lookup_with_alias() {
        assert_eq!(builtin("volcanic_glass"), builtin("volcanic-glass"));
        assert_eq!(builtin("Volcanic Glass"), builtin("volcanic-glass"));
        assert!(builtin("volcanic-glass").is_some());
    }

    #[test]
    fn test_find_in_prefers_user_dir() {
        let dir = std::env::temp_dir().join(format!("banquo_presets_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("zircon.toml"), "theme = \"zircon\"\n# user copy").unwrap();

        let found = find_in(std::slice::from_ref(&dir), "zircon");
        std::fs::remove_dir_all(&dir).ok();

        let found = found.expect("user preset found");
        assert!(matches!(found.source, PresetSource::User(_)));
        assert!(found.content.contains("# user copy"));
    }

    #[test]
    fn test_find_in_falls_back_to_builtin() {
        let found = find_in(&[], "blanco").expect("builtin fallback");
        assert_eq!(found.source, PresetSource::Builtin);
        assert_eq!(found.content, builtin("blanco").unwrap());
    }

    #[test]
    fn test_find_unknown_none() {
        assert!(find_in(&[], "nope").is_none());
    }

    #[test]
    fn test_list_in_marks_user_presets() {
        let dir = std::env::temp_dir().join(format!("banquo_preset_list_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mytheme.toml"), "theme = \"mytheme\"").unwrap();

        let listed = list_in(std::slice::from_ref(&dir));
        std::fs::remove_dir_all(&dir).ok();

        assert!(listed.contains(&("mytheme".to_string(), true)));
        assert!(listed.contains(&("zircon".to_string(), false)));
        assert_eq!(listed.len(), 7);
    }
}
