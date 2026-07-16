use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BanquoConfig {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub fonts: FontConfig,
    #[serde(default)]
    pub os: OsConfig,
    #[serde(default)]
    pub window: WindowAppearanceConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
}

/// Optional color overrides layered on top of the active theme's spec.
///
/// Every field is a hex string (`"#RRGGBB"` or `"#RRGGBBAA"`). Set any subset;
/// unset fields keep the theme's builtin value. Combined with a custom `theme`
/// name this is how a user defines a whole theme in TOML — no recompile.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ColorsConfig {
    /// Substrate fill behind the grid.
    pub background: Option<String>,
    /// Remap default (light-grayscale) terminal text to this color.
    pub foreground: Option<String>,
    /// Cursor block color.
    pub cursor: Option<String>,
    /// Color of the glyph painted under the cursor block.
    pub cursor_text: Option<String>,
}

/// Which shells Banquo can launch and which one is the default.
///
/// A *shell* is just a child process the PTY runs (`cmd.exe`, `pwsh.exe`,
/// `bash`, `wsl.exe`…). When this is empty Banquo falls back to the OS default
/// program (`new_default_prog`) — exactly the pre-sprint-11 behavior.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ShellConfig {
    /// Name of the profile to launch by default. `None` → OS default shell.
    pub default: Option<String>,
    /// Available named shell profiles.
    #[serde(default)]
    pub profiles: Vec<ShellProfile>,
}

/// A single named, launchable shell definition.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ShellProfile {
    /// The name used to select this profile (e.g. `"pwsh"`).
    pub name: String,
    /// The program to launch (e.g. `"pwsh.exe"`, `"wsl.exe"`).
    pub command: String,
    /// Arguments passed to the program.
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory to start in (inherits Banquo's cwd when `None`).
    pub cwd: Option<String>,
    /// Extra environment variables to set for this shell.
    pub env: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowAppearanceConfig {
    pub edge_style: Option<String>,
    pub corner_style: Option<String>,
    pub radius: Option<f32>,
    pub inset: Option<f32>,
    pub opacity: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct UiConfig {
    pub tab_bar_mode: Option<String>,
    pub top_margin: Option<f32>,
    pub background_mode: Option<String>,
}

impl Default for WindowAppearanceConfig {
    fn default() -> Self {
        Self {
            edge_style: Some("flat".to_string()),
            corner_style: Some("square".to_string()),
            radius: Some(8.0),
            inset: Some(0.0),
            opacity: Some(1.0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FontConfig {
    pub monospace_path: Option<String>,
    pub symbols_path: Option<String>,
    pub offset_x: Option<f32>,
    pub offset_y: Option<f32>,
    pub size: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct OsConfig {
    pub windows: Option<WindowsConfig>,
    pub macos: Option<MacosConfig>,
    pub linux: Option<LinuxConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WindowsConfig {
    pub blur: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct MacosConfig {
    pub vibrancy: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LinuxConfig {}

/// How bad a config finding is. `Error` = Banquo cannot honor the config
/// (`banquo check` exits non-zero); `Warning` = it runs, but something is off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// One finding from [`validate_str`].
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }
    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }
}

/// The top-level tables/keys `BanquoConfig` actually reads. Anything else in
/// a config file is silently ignored by serde — the validator surfaces it.
const KNOWN_TOP_LEVEL_KEYS: [&str; 7] = ["theme", "fonts", "os", "window", "ui", "shell", "colors"];

/// Validate config file *content*. Pure over the string except for font-path
/// existence probes. This is the engine behind `banquo check`.
pub fn validate_str(content: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    // 1. It must be TOML at all.
    let value: toml::Value = match toml::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            out.push(Diagnostic::error(format!("TOML parse error: {e}")));
            return out;
        }
    };

    // 2. Unknown top-level keys (legacy fields like [grid] land here).
    if let toml::Value::Table(table) = &value {
        for key in table.keys() {
            if !KNOWN_TOP_LEVEL_KEYS.contains(&key.as_str()) {
                out.push(Diagnostic::warning(format!(
                    "unknown top-level key `{key}` (ignored by Banquo)"
                )));
            }
        }
    }

    // 3. It must deserialize into the config shape (type errors land here).
    let config: BanquoConfig = match value.try_into() {
        Ok(c) => c,
        Err(e) => {
            out.push(Diagnostic::error(format!("invalid config shape: {e}")));
            return out;
        }
    };

    // 4. The default shell must reference a defined profile.
    if let Some(default) = config.shell.default.as_deref() {
        if !config.shell.profiles.iter().any(|p| p.name == default) {
            out.push(Diagnostic::error(format!(
                "shell.default = \"{default}\" does not match any [[shell.profiles]] name"
            )));
        }
    }

    // 5. Configured font files should exist.
    for (label, path) in [
        ("fonts.monospace_path", &config.fonts.monospace_path),
        ("fonts.symbols_path", &config.fonts.symbols_path),
    ] {
        if let Some(p) = path {
            if !std::path::Path::new(p).exists() {
                out.push(Diagnostic::warning(format!(
                    "{label} points to a missing file: {p} (Banquo will fall back)"
                )));
            }
        }
    }

    // 6. Unknown theme names are legal (custom themes) but worth flagging.
    if let Some(theme) = config.theme.as_deref() {
        if crate::theme::builtin_spec(theme).is_none() {
            out.push(Diagnostic::warning(format!(
                "theme \"{theme}\" is not a builtin; its base spec falls back to zircon \
                 (customize it via [colors])"
            )));
        }
    }

    // 7. Ranges and color formats.
    if let Some(opacity) = config.window.opacity {
        if !(0.0..=1.0).contains(&opacity) {
            out.push(Diagnostic::warning(format!(
                "window.opacity = {opacity} is outside [0.0, 1.0] and will be clamped"
            )));
        }
    }
    for (label, val) in [
        ("colors.background", &config.colors.background),
        ("colors.foreground", &config.colors.foreground),
        ("colors.cursor", &config.colors.cursor),
        ("colors.cursor_text", &config.colors.cursor_text),
    ] {
        if let Some(s) = val {
            if crate::theme::parse_hex_color(s).is_none() {
                out.push(Diagnostic::warning(format!(
                    "{label} = \"{s}\" is not a valid #RRGGBB or #RRGGBBAA color (ignored)"
                )));
            }
        }
    }

    out
}

/// Validate the active config file (the `banquo check` entry point).
/// A missing file is valid: it means defaults, like a fresh install.
pub fn validate() -> (Option<PathBuf>, Vec<Diagnostic>) {
    match BanquoConfig::config_path() {
        Some(path) if path.exists() => match fs::read_to_string(&path) {
            Ok(content) => (Some(path), validate_str(&content)),
            Err(e) => (
                Some(path),
                vec![Diagnostic::error(format!("failed to read config: {e}"))],
            ),
        },
        other => (other, Vec::new()),
    }
}

/// Recursively merge `overlay` into `base`: tables merge key-by-key with the
/// overlay winning on conflicts; scalars and arrays replace wholesale. This is
/// the preset-application primitive — a preset overrides exactly the keys it
/// declares and nothing else.
fn merge_toml(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(b), toml::Value::Table(o)) => {
            for (k, v) in o {
                match b.get_mut(&k) {
                    Some(slot) => merge_toml(slot, v),
                    None => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (slot, v) => *slot = v,
    }
}

impl BanquoConfig {
    /// Apply a preset (TOML text) over this config, returning the merged
    /// config. The preset's keys win; everything it doesn't mention — the
    /// user's `[shell]`, font paths, sizes — survives untouched. On any
    /// parse/serialize error the original config is left unchanged.
    pub fn apply_preset(&self, preset_toml: &str) -> anyhow::Result<BanquoConfig> {
        let overlay: toml::Value = toml::from_str(preset_toml)?;
        let base_str = toml::to_string(self)?;
        let mut base: toml::Value = toml::from_str(&base_str)?;
        merge_toml(&mut base, overlay);
        Ok(base.try_into()?)
    }
}

impl BanquoConfig {
    /// Loads the configuration leniently: any failure logs to stderr and
    /// falls back to defaults. This is the GUI path — the window must open
    /// even with a broken config. `banquo check` reports what's wrong.
    pub fn load() -> Self {
        match Self::load_strict() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("banquo: {e:#}. Falling back to defaults.");
                Self::default()
            }
        }
    }

    /// Loads the configuration strictly: read/parse failures are real errors
    /// carrying the TOML parser's message. A *missing* file is not an error —
    /// it means "defaults", exactly like a fresh install.
    pub fn load_strict() -> anyhow::Result<Self> {
        match Self::config_path() {
            Some(path) => Self::load_strict_from(&path),
            None => Ok(Self::default()),
        }
    }

    /// [`Self::load_strict`] against an explicit path (unit-testable).
    pub fn load_strict_from(path: &std::path::Path) -> anyhow::Result<Self> {
        use anyhow::Context;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        let config = toml::from_str(&content)
            .with_context(|| format!("failed to parse config at {}", path.display()))?;
        Ok(config)
    }

    /// Saves the configuration back to the active config path.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = toml::to_string(self).map_err(std::io::Error::other)?;
            fs::write(&path, content)?;
        }
        Ok(())
    }

    /// The active config file path: the `BANQUO_CONFIG` env var when set
    /// (point it into a dotfiles clone to keep your config in git), otherwise
    /// `~/.config/banquo/banquo.toml` on Unix/macOS or
    /// `%APPDATA%\banquo\banquo.toml` on Windows.
    pub fn config_path() -> Option<PathBuf> {
        Self::config_path_from(std::env::var_os("BANQUO_CONFIG"))
    }

    /// Pure form of [`Self::config_path`]: resolve from an explicit env value.
    pub fn config_path_from(env_val: Option<std::ffi::OsString>) -> Option<PathBuf> {
        if let Some(v) = env_val {
            if !v.is_empty() {
                return Some(PathBuf::from(v));
            }
        }
        dirs::config_dir().map(|mut p| {
            p.push("banquo");
            p.push("banquo.toml");
            p
        })
    }

    /// Spawns a background thread that watches the config file for changes.
    /// When changed, it reloads the config and sends it down the channel.
    /// Honors the `BANQUO_CONFIG` override like every other path user.
    pub fn watch(tx: std::sync::mpsc::Sender<BanquoConfig>) {
        let config_path = Self::config_path();
        if let Some(path) = config_path {
            std::thread::spawn(move || {
                use notify::{RecursiveMode, Watcher};
                let (notify_tx, notify_rx) = std::sync::mpsc::channel();

                let mut watcher = match notify::recommended_watcher(notify_tx) {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("banquo: Failed to initialize config watcher: {:?}", e);
                        return;
                    }
                };

                // Watch the parent directory because some editors use atomic saves (write temp, rename)
                // which can break single-file watches.
                if let Some(parent) = path.parent() {
                    if parent.exists() {
                        let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
                    }
                } else if path.exists() {
                    let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
                }

                for res in notify_rx {
                    match res {
                        Ok(event) => {
                            if event.paths.iter().any(|p| p == &path) {
                                // Brief sleep to let file IO settle
                                std::thread::sleep(std::time::Duration::from_millis(50));
                                let config = Self::load();
                                let _ = tx.send(config);
                            }
                        }
                        Err(e) => eprintln!("banquo: config watch error: {:?}", e),
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_config_deserializes() {
        let toml_src = r#"
[shell]
default = "pwsh"

[[shell.profiles]]
name = "pwsh"
command = "pwsh.exe"
args = ["-NoLogo"]

[[shell.profiles]]
name = "wsl"
command = "wsl.exe"
"#;
        let cfg: BanquoConfig = toml::from_str(toml_src).expect("valid TOML");
        assert_eq!(cfg.shell.default.as_deref(), Some("pwsh"));
        assert_eq!(cfg.shell.profiles.len(), 2);
        assert_eq!(cfg.shell.profiles[0].name, "pwsh");
        assert_eq!(cfg.shell.profiles[0].command, "pwsh.exe");
        assert_eq!(cfg.shell.profiles[0].args, vec!["-NoLogo".to_string()]);
    }

    #[test]
    fn test_shell_config_defaults_when_absent() {
        // A config with no [shell] table must still parse, with an empty default.
        let cfg: BanquoConfig = toml::from_str("theme = \"blanco\"").expect("valid TOML");
        assert!(cfg.shell.default.is_none());
        assert!(cfg.shell.profiles.is_empty());
    }

    #[test]
    fn test_shell_profile_args_default_empty() {
        let toml_src = r#"
[[shell.profiles]]
name = "cmd"
command = "cmd.exe"
"#;
        let cfg: BanquoConfig = toml::from_str(toml_src).expect("valid TOML");
        let p = &cfg.shell.profiles[0];
        assert!(p.args.is_empty());
        assert!(p.cwd.is_none());
        assert!(p.env.is_none());
    }

    // --- T-1906: preset application = deep merge, never replace ---

    fn user_config() -> BanquoConfig {
        toml::from_str(
            r#"
theme = "zircon"

[fonts]
size = 22.0
monospace_path = "C:/fonts/MyMono.ttf"

[window]
edge_style = "flat"

[shell]
default = "pwsh"

[[shell.profiles]]
name = "pwsh"
command = "pwsh.exe"
args = ["-NoLogo"]
"#,
        )
        .expect("valid user config")
    }

    // --- T-1908: validation diagnostics ---

    fn errors(diags: &[Diagnostic]) -> usize {
        diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    fn warnings(diags: &[Diagnostic]) -> usize {
        diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    #[test]
    fn test_validate_parse_error() {
        let diags = validate_str("= bad");
        assert_eq!(errors(&diags), 1);
        assert!(diags[0].message.contains("TOML parse error"));
    }

    #[test]
    fn test_validate_shell_default_unresolved() {
        let diags = validate_str("[shell]\ndefault = \"ghost\"");
        assert_eq!(errors(&diags), 1);
        assert!(diags.iter().any(|d| d.message.contains("ghost")));
    }

    #[test]
    fn test_validate_unknown_top_level_key() {
        let src = "[grid]\nmode = \"fixed\"";
        let diags = validate_str(src);
        assert_eq!(
            errors(&diags),
            0,
            "legacy [grid] is a warning, not an error"
        );
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("`grid`")));
        // And the config still parses (serde tolerance).
        let cfg: Result<BanquoConfig, _> = toml::from_str(src);
        assert!(cfg.is_ok());
    }

    #[test]
    fn test_validate_missing_font_file() {
        let diags = validate_str("[fonts]\nmonospace_path = \"Z:/nope.ttf\"");
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("Z:/nope.ttf")));
    }

    #[test]
    fn test_validate_unknown_theme_warns() {
        let diags = validate_str("theme = \"mytheme\"");
        assert_eq!(errors(&diags), 0);
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("mytheme")));
    }

    #[test]
    fn test_validate_opacity_range() {
        let diags = validate_str("[window]\nopacity = 1.5");
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("opacity")));
    }

    #[test]
    fn test_validate_bad_hex_warns() {
        let diags = validate_str("[colors]\ncursor = \"banana\"");
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("banana")));
    }

    #[test]
    fn test_validate_clean_config_empty() {
        // Every shipped preset validates without errors or warnings.
        for name in crate::presets::builtin_names() {
            let content = crate::presets::builtin(name).unwrap();
            let diags = validate_str(content);
            assert_eq!(errors(&diags), 0, "{name}: {diags:?}");
            assert_eq!(warnings(&diags), 0, "{name}: {diags:?}");
        }
    }

    #[test]
    fn test_legacy_fields_still_parse() {
        // Removed fields (grid table, fonts.ui_path/serif_path) must not
        // break parsing of old config files.
        let src = r#"
theme = "zircon"

[grid]
mode = "fixed"

[fonts]
ui_path = "C:/old/ui.ttf"
serif_path = "C:/old/serif.ttf"
"#;
        let cfg: BanquoConfig = toml::from_str(src).expect("legacy config parses");
        assert_eq!(cfg.theme.as_deref(), Some("zircon"));
    }

    // --- T-1907: config path resolution + strict loading ---

    #[test]
    fn test_config_path_from_env_override() {
        let p = BanquoConfig::config_path_from(Some(std::ffi::OsString::from("C:/tmp/x.toml")));
        assert_eq!(p, Some(PathBuf::from("C:/tmp/x.toml")));
    }

    #[test]
    fn test_config_path_from_default() {
        let p = BanquoConfig::config_path_from(None).expect("platform config dir exists");
        let s = p.to_string_lossy().replace('\\', "/");
        assert!(s.ends_with("banquo/banquo.toml"), "unexpected path: {s}");
        // Empty env value falls back to the default too.
        let p2 = BanquoConfig::config_path_from(Some(std::ffi::OsString::new()));
        assert_eq!(p2, Some(p));
    }

    #[test]
    fn test_load_strict_bad_toml_err() {
        let dir = std::env::temp_dir().join(format!("banquo_cfg_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.toml");
        std::fs::write(&path, "= bad").unwrap();
        let err = BanquoConfig::load_strict_from(&path).unwrap_err();
        std::fs::remove_dir_all(&dir).ok();
        let msg = format!("{err:#}");
        assert!(msg.contains("failed to parse config"), "got: {msg}");
    }

    #[test]
    fn test_load_strict_missing_file_ok_default() {
        let path = std::env::temp_dir().join("banquo_definitely_missing_dir/banquo.toml");
        let cfg = BanquoConfig::load_strict_from(&path).expect("missing file is not an error");
        assert!(cfg.theme.is_none());
    }

    #[test]
    fn test_apply_preset_preserves_shell() {
        let user = user_config();
        let merged = user
            .apply_preset(crate::presets::builtin("blanco").unwrap())
            .expect("merge succeeds");
        assert_eq!(merged.shell.default.as_deref(), Some("pwsh"));
        assert_eq!(merged.shell.profiles.len(), 1);
        assert_eq!(merged.shell.profiles[0].args, vec!["-NoLogo".to_string()]);
        // And the preset did take effect.
        assert_eq!(merged.theme.as_deref(), Some("blanco"));
    }

    #[test]
    fn test_apply_preset_overrides_window() {
        let user = user_config();
        let merged = user
            .apply_preset("theme = \"blanco\"\n[window]\nedge_style = \"beveled\"")
            .unwrap();
        assert_eq!(merged.window.edge_style.as_deref(), Some("beveled"));
    }

    #[test]
    fn test_apply_preset_keeps_user_font_size() {
        let user = user_config();
        let merged = user
            .apply_preset(crate::presets::builtin("concrete").unwrap())
            .unwrap();
        assert_eq!(merged.fonts.size, Some(22.0));
        assert_eq!(
            merged.fonts.monospace_path.as_deref(),
            Some("C:/fonts/MyMono.ttf")
        );
    }

    #[test]
    fn test_apply_preset_invalid_toml_errors() {
        let user = user_config();
        assert!(user.apply_preset("= not toml").is_err());
        // Original untouched (apply_preset borrows immutably by design).
        assert_eq!(user.theme.as_deref(), Some("zircon"));
    }

    #[test]
    fn test_merge_toml_recurses_tables() {
        let user = user_config();
        // Preset sets only window.radius; edge_style must survive the merge.
        let merged = user.apply_preset("[window]\nradius = 12.0").unwrap();
        assert_eq!(merged.window.radius, Some(12.0));
        assert_eq!(merged.window.edge_style.as_deref(), Some("flat"));
    }

    #[test]
    fn test_preset_apply_pipeline() {
        // Component-B integration: the "preset switch never destroys personal
        // config" contract, over a real builtin preset.
        let user = user_config();
        let preset = crate::presets::builtin("blanco").unwrap();
        let merged = user.apply_preset(preset).unwrap();
        assert_eq!(merged.theme.as_deref(), Some("blanco"));
        assert_eq!(merged.window.corner_style.as_deref(), Some("g3"));
        assert_eq!(merged.fonts.size, Some(22.0));
        assert_eq!(merged.shell.default.as_deref(), Some("pwsh"));
    }
}
