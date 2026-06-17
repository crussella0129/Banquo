use serde::Deserialize;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct BanquoConfig {
    #[serde(default)]
    pub fonts: FontConfig,
    #[serde(default)]
    pub grid: GridConfig,
    #[serde(default)]
    pub os: OsConfig,
    #[serde(default)]
    pub window: WindowAppearanceConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowAppearanceConfig {
    pub edge_style: Option<String>,
    pub corner_style: Option<String>,
    pub radius: Option<f32>,
    pub inset: Option<f32>,
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

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FontConfig {
    pub monospace_path: Option<String>,
    pub ui_path: Option<String>,
    pub serif_path: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct GridConfig {
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct OsConfig {
    pub windows: Option<WindowsConfig>,
    pub macos: Option<MacosConfig>,
    pub linux: Option<LinuxConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct WindowsConfig {
    pub blur: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MacosConfig {
    pub vibrancy: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
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

    /// Determines the config file path: `~/.config/banquo/banquo.toml` on Unix/macOS,
    /// and `%APPDATA%\banquo\banquo.toml` on Windows.
    fn get_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("banquo");
            p.push("banquo.toml");
            p
        })
    }
}
