use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct KeybindingsConfig {
    #[serde(default)]
    pub navigation: IndexMap<String, String>,
    #[serde(default)]
    pub browse: IndexMap<String, String>,
    #[serde(default)]
    pub tui: IndexMap<String, String>,
    #[serde(default)]
    pub global: IndexMap<String, String>,
    #[serde(default)]
    pub mutate: IndexMap<String, String>,
    #[serde(default)]
    pub interact: IndexMap<String, String>,
}

impl KeybindingsConfig {
    fn group_entries(&self) -> [(&str, &IndexMap<String, String>); 6] {
        [
            ("global", &self.global),
            ("mutate", &self.mutate),
            ("interact", &self.interact),
            ("browse", &self.browse),
            ("navigation", &self.navigation),
            ("tui", &self.tui),
        ]
    }
}

pub fn validate_keybindings(config: &KeybindingsConfig) -> Vec<(String, String, String)> {
    let mut errors = Vec::new();
    for (group, map) in config.group_entries() {
        for (name, key_str) in map {
            if let Err(e) = validate_key_string(key_str) {
                errors.push((group.to_string(), name.clone(), e));
            }
        }
    }
    errors
}

pub fn check_collisions(config: &KeybindingsConfig) -> Vec<(String, String, String)> {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut collisions = Vec::new();
    for (group, map) in config.group_entries() {
        for key_str in map.values() {
            let normalized = key_str.trim().to_ascii_lowercase();
            if let Some(prev_group) = seen.get(&normalized) {
                collisions.push((key_str.clone(), prev_group.clone(), group.to_string()));
            } else {
                seen.insert(normalized, group.to_string());
            }
        }
    }
    collisions
}

fn validate_key_string(s: &str) -> Result<(), String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty key string".to_string());
    }
    let parts: Vec<&str> = trimmed.split('+').collect();
    if parts.len() == 1 {
        validate_key_part(parts[0])?;
        return Ok(());
    }
    for &modifier in &parts[..parts.len() - 1] {
        match modifier.to_ascii_lowercase().as_str() {
            "alt" | "ctrl" | "shift" => {}
            other => return Err(format!("unknown modifier: {other}")),
        }
    }
    validate_key_part(parts[parts.len() - 1])
}

fn validate_key_part(s: &str) -> Result<(), String> {
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "tab" | "enter" | "esc" | "backspace" | "delete" | "up" | "down" | "left" | "right" | "home" | "end"
        | "pageup" | "pagedown" | "space" => Ok(()),
        _ if s.len() == 1 => Ok(()),
        f if f.starts_with('f') => f[1..].parse::<u8>().map(|_| ()).map_err(|_| format!("invalid function key: {s}")),
        _ => Err(format!("unrecognized key: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_catches_bad_key_string() {
        let mut config = KeybindingsConfig::default();
        config.global.insert("quit".into(), "notakey+combo+bad".into());
        config.global.insert("help".into(), "?".into());

        let errors = validate_keybindings(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, "global");
        assert_eq!(errors[0].1, "quit");
    }

    #[test]
    fn validate_accepts_valid_keys() {
        let mut config = KeybindingsConfig::default();
        config.global.insert("quit".into(), "ctrl+q".into());
        config.navigation.insert("scroll_up".into(), "k".into());
        config.tui.insert("split_vertical".into(), "alt+v".into());
        config.mutate.insert("delete".into(), "ctrl+alt+d".into());
        config.browse.insert("view_yaml".into(), "y".into());

        let errors = validate_keybindings(&config);
        assert!(errors.is_empty());
    }

    #[test]
    fn check_collisions_detects_duplicates() {
        let mut config = KeybindingsConfig::default();
        config.global.insert("quit".into(), "q".into());
        config.navigation.insert("scroll_up".into(), "q".into());

        let collisions = check_collisions(&config);
        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].0, "q");
    }

    #[test]
    fn check_collisions_none_when_unique() {
        let mut config = KeybindingsConfig::default();
        config.global.insert("quit".into(), "ctrl+q".into());
        config.navigation.insert("scroll_up".into(), "k".into());

        let collisions = check_collisions(&config);
        assert!(collisions.is_empty());
    }
}
