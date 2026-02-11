use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tick_rate_ms: Option<u64>,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default)]
    pub global: HashMap<String, String>,
    #[serde(default)]
    pub pane: HashMap<String, String>,
}

const DEFAULT_CONFIG: &str = include_str!("defaults.toml");

impl Config {
    pub fn load() -> Self {
        let mut config: Config = toml::from_str(DEFAULT_CONFIG).expect("embedded defaults must parse");

        if let Some(path) = Self::user_config_path() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(contents) => match toml::from_str::<Config>(&contents) {
                        Ok(user) => config.merge(user),
                        Err(e) => eprintln!("Warning: invalid config at {}: {e}", path.display()),
                    },
                    Err(e) => eprintln!("Warning: could not read {}: {e}", path.display()),
                }
            }
        }

        config
    }

    fn user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("crystal").join("config.toml"))
    }

    fn merge(&mut self, user: Config) {
        if user.tick_rate_ms.is_some() {
            self.tick_rate_ms = user.tick_rate_ms;
        }
        for (k, v) in user.keybindings.global {
            self.keybindings.global.insert(k, v);
        }
        for (k, v) in user.keybindings.pane {
            self.keybindings.pane.insert(k, v);
        }
    }

    pub fn tick_rate_ms(&self) -> u64 {
        self.tick_rate_ms.unwrap_or(250)
    }
}

#[cfg(test)]
mod tests;
