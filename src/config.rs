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
    pub grid: GridConfig,
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
    pub ui_path: Option<String>,
    pub serif_path: Option<String>,
    pub symbols_path: Option<String>,
    pub offset_x: Option<f32>,
    pub offset_y: Option<f32>,
    pub size: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct GridConfig {
    pub mode: Option<String>,
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
    /// Loads the configuration from the OS-specific config directory.
    /// If the file does not exist or is invalid, returns the default config.
    pub fn load() -> Self {
        let config_path = Self::get_config_path();

        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    } else {
                        eprintln!(
                            "banquo: Failed to parse TOML at {:?}. Falling back to defaults.",
                            path
                        );
                    }
                } else {
                    eprintln!(
                        "banquo: Failed to read config file at {:?}. Falling back to defaults.",
                        path
                    );
                }
            }
        }

        Self::default()
    }

    /// Saves the configuration back to the file.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Ok(content) = toml::to_string(self) {
                fs::write(&path, content)?;
            }
        }
        Ok(())
    }

    /// Determines the config file path: `~/.config/banquo/banquo.toml` on Unix/macOS,
    /// and `%APPDATA%\banquo\banquo.toml` on Windows.
    fn get_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("banquo");
            p.push("banquo.toml");
            p
        })
    }

    /// Spawns a background thread that watches the config file for changes.
    /// When changed, it reloads the config and sends it down the channel.
    pub fn watch(tx: std::sync::mpsc::Sender<BanquoConfig>) {
        let config_path = Self::get_config_path();
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
