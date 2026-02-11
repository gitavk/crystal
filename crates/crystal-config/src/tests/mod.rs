use super::*;

#[test]
fn default_config() {
    let config = Config::default();
    assert_eq!(config.tick_rate_ms(), 250);
}

#[test]
fn parse_from_toml() {
    let raw = "tick_rate_ms = 100";
    let config: Config = toml::from_str(raw).unwrap();
    assert_eq!(config.tick_rate_ms(), 100);
}

#[test]
fn embedded_defaults_parse() {
    let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    assert_eq!(config.keybindings.global.get("quit").unwrap(), "q");
    assert_eq!(config.keybindings.global.get("help").unwrap(), "?");
    assert_eq!(config.keybindings.global.get("split_vertical").unwrap(), "alt+v");
    assert_eq!(config.keybindings.pane.get("select").unwrap(), "enter");
    assert_eq!(config.keybindings.pane.get("back").unwrap(), "esc");
}

#[test]
fn merge_overrides_keybindings() {
    let mut base: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    let user_toml = r#"
[keybindings.global]
quit = "ctrl+q"
"#;
    let user: Config = toml::from_str(user_toml).unwrap();
    base.merge(user);

    assert_eq!(base.keybindings.global.get("quit").unwrap(), "ctrl+q");
    // other defaults preserved
    assert_eq!(base.keybindings.global.get("help").unwrap(), "?");
}

#[test]
fn merge_preserves_tick_rate_when_not_overridden() {
    let mut base = Config { tick_rate_ms: Some(200), keybindings: KeybindingsConfig::default() };
    let user = Config::default();
    base.merge(user);
    assert_eq!(base.tick_rate_ms(), 200);
}

#[test]
fn empty_user_config_keeps_defaults() {
    let mut base: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    let user: Config = toml::from_str("").unwrap();
    base.merge(user);
    assert!(!base.keybindings.global.is_empty());
    assert!(!base.keybindings.pane.is_empty());
}

#[test]
fn load_returns_defaults_without_user_config() {
    let config = Config::load();
    assert!(!config.keybindings.global.is_empty());
    assert_eq!(config.keybindings.global.get("quit").unwrap(), "q");
}
