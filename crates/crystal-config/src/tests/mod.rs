use super::*;

#[test]
fn default_config_has_expected_tick_rate() {
    let config = AppConfig::default();
    assert_eq!(config.tick_rate_ms(), 250);
}

#[test]
fn default_config_has_all_general_fields() {
    let config = AppConfig::default();
    assert_eq!(config.general.tick_rate_ms, 250);
    assert_eq!(config.general.default_namespace, "default");
    assert_eq!(config.general.default_view, "pods");
    assert_eq!(config.general.log_tail_lines, 1000);
    assert!(config.general.confirm_delete);
    assert!(!config.general.show_managed_fields);
}

#[test]
fn default_config_has_terminal_fields() {
    let config = AppConfig::default();
    assert_eq!(config.terminal.scrollback_lines, 10000);
    assert_eq!(config.terminal.cursor_style, "block");
}

#[test]
fn feature_flags_default_to_true() {
    let config = AppConfig::default();
    assert!(config.features.hot_reload);
    assert!(config.features.command_palette);
    assert!(config.features.port_forward);
}

#[test]
fn parse_general_from_toml() {
    let raw = r#"
[general]
tick_rate_ms = 100
default_view = "deployments"
"#;
    let config: AppConfig = toml::from_str(raw).unwrap();
    assert_eq!(config.general.tick_rate_ms, 100);
    assert_eq!(config.general.default_view, "deployments");
    // other fields get defaults
    assert_eq!(config.general.default_namespace, "default");
}

#[test]
fn partial_toml_only_general_merges_with_defaults() {
    let mut base = AppConfig::default();
    let user_toml = r#"
[general]
tick_rate_ms = 500
"#;
    let user: AppConfig = toml::from_str(user_toml).unwrap();
    base.merge(user);

    assert_eq!(base.general.tick_rate_ms, 500);
    // keybindings preserved from base
    assert!(!base.keybindings.global.is_empty());
    assert_eq!(base.keybindings.global.get("quit").unwrap(), "q");
}

#[test]
fn partial_toml_only_features_merges_with_defaults() {
    let mut base = AppConfig::default();
    let user_toml = r#"
[features]
port_forward = false
"#;
    let user: AppConfig = toml::from_str(user_toml).unwrap();
    base.merge(user);

    assert!(!base.features.port_forward);
    assert!(base.features.hot_reload);
    assert!(base.features.command_palette);
}

#[test]
fn embedded_defaults_parse() {
    let config: AppConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
    assert_eq!(config.keybindings.global.get("quit").unwrap(), "q");
    assert_eq!(config.keybindings.global.get("help").unwrap(), "?");
    assert_eq!(config.keybindings.global.get("app_logs").unwrap(), "alt+l");
    assert_eq!(config.keybindings.global.get("context_selector").unwrap(), "ctrl+o");
    assert_eq!(config.keybindings.global.get("close_tab").unwrap(), "alt+c");
    assert_eq!(config.keybindings.global.get("split_vertical").unwrap(), "alt+v");
    assert_eq!(config.keybindings.pane.get("select").unwrap(), "enter");
    assert_eq!(config.keybindings.pane.get("back").unwrap(), "esc");
}

#[test]
fn merge_overrides_keybindings() {
    let mut base: AppConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
    let user_toml = r#"
[keybindings.global]
quit = "ctrl+q"
"#;
    let user: AppConfig = toml::from_str(user_toml).unwrap();
    base.merge(user);

    assert_eq!(base.keybindings.global.get("quit").unwrap(), "ctrl+q");
    assert_eq!(base.keybindings.global.get("help").unwrap(), "?");
}

#[test]
fn empty_user_config_keeps_defaults() {
    let mut base: AppConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
    let user: AppConfig = toml::from_str("").unwrap();
    base.merge(user);
    assert!(!base.keybindings.global.is_empty());
    assert!(!base.keybindings.pane.is_empty());
}

#[test]
fn load_returns_defaults_without_user_config() {
    let config = AppConfig::load();
    assert!(!config.keybindings.global.is_empty());
    assert_eq!(config.keybindings.global.get("quit").unwrap(), "q");
}

#[test]
fn tick_rate_ms_helper_reads_from_general() {
    let mut config = AppConfig::default();
    config.general.tick_rate_ms = 500;
    assert_eq!(config.tick_rate_ms(), 500);
}

#[test]
fn config_type_alias_works() {
    let config = Config::default();
    assert_eq!(config.tick_rate_ms(), 250);
}

#[test]
fn save_and_load_roundtrip() {
    let dir = std::env::temp_dir().join("crystal_config_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");

    let config = AppConfig::default();
    config.save(&path).unwrap();

    let loaded = AppConfig::load_from(&path).unwrap();
    assert_eq!(loaded.general.tick_rate_ms, config.general.tick_rate_ms);
    assert_eq!(loaded.features.hot_reload, config.features.hot_reload);
    assert_eq!(loaded.keybindings.global.get("quit").unwrap(), "q");

    let _ = std::fs::remove_dir_all(&dir);
}
