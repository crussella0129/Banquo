use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowAppearanceConfig {
    pub edge_style: Option<String>,
    pub corner_style: Option<String>,
    pub radius: Option<f32>,
    pub inset: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct UiConfig {
    pub tab_bar_mode: Option<String>,
}

impl Default for WindowAppearanceConfig {
    fn default() -> Self {
        Self {
            edge_style: Some("flat".to_string()),
            corner_style: Some("square".to_string()),
            radius: Some(8.0),
            inset: Some(0.0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FontConfig {
    pub monospace_path: Option<String>,
    pub ui_path: Option<String>,
    pub serif_path: Option<String>,
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
pub struct LinuxConfig {
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
                        eprintln!("banquo: Failed to parse TOML at {:?}. Falling back to defaults.", path);
                    }
                } else {
                    eprintln!("banquo: Failed to read config file at {:?}. Falling back to defaults.", path);
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
                use notify::{Watcher, RecursiveMode};
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
                        },
                        Err(e) => eprintln!("banquo: config watch error: {:?}", e),
                    }
                }
            });
        }
    }
}
