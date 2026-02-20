pub mod general;
pub mod keybindings;
pub mod theme;
pub mod views;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use general::{FeatureFlags, GeneralConfig, TerminalConfig};
pub use keybindings::{check_collisions, validate_keybindings, KeybindingsConfig};
pub use theme::ThemeConfig;
pub use views::{ResourceViewConfig, ViewsConfig};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub terminal: TerminalConfig,
    #[serde(default)]
    pub features: FeatureFlags,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub views: ViewsConfig,
}

pub const DEFAULT_CONFIG: &str = include_str!("defaults.toml");

impl Default for AppConfig {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).expect("embedded defaults must parse")
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let mut config = Self::default();

        if let Some(path) = Self::user_config_path() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
                        Ok(user) => config.merge(user),
                        Err(e) => eprintln!("Warning: invalid config at {}: {e}", path.display()),
                    },
                    Err(e) => eprintln!("Warning: could not read {}: {e}", path.display()),
                }
            }
        }

        config
    }

    pub fn load_from(path: &Path) -> anyhow::Result<Self> {
        let mut config = Self::default();
        let contents = std::fs::read_to_string(path)?;
        let user: AppConfig = toml::from_str(&contents)?;
        config.merge(user);
        Ok(config)
    }

    pub fn default_path() -> PathBuf {
        dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("kubetile").join("config.toml")
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn init_default() -> anyhow::Result<PathBuf> {
        let path = Self::default_path();
        if path.exists() {
            anyhow::bail!("Config already exists at {}", path.display());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, DEFAULT_CONFIG)?;
        Ok(path)
    }

    fn user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("kubetile").join("config.toml"))
    }

    fn merge(&mut self, user: AppConfig) {
        self.general = user.general;
        self.terminal = user.terminal;
        self.features = user.features;
        self.theme = user.theme;
        self.views = user.views;

        // Keybindings: merge per-key (user overrides, defaults preserved)
        for (k, v) in user.keybindings.navigation {
            self.keybindings.navigation.insert(k, v);
        }
        for (k, v) in user.keybindings.browse {
            self.keybindings.browse.insert(k, v);
        }
        for (k, v) in user.keybindings.tui {
            self.keybindings.tui.insert(k, v);
        }
        for (k, v) in user.keybindings.global {
            self.keybindings.global.insert(k, v);
        }
        for (k, v) in user.keybindings.mutate {
            self.keybindings.mutate.insert(k, v);
        }
    }

    pub fn tick_rate_ms(&self) -> u64 {
        self.general.tick_rate_ms
    }
}

pub type Config = AppConfig;

#[cfg(test)]
mod tests;
