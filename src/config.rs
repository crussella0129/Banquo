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
}
