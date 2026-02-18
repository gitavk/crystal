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
    assert!(!base.keybindings.global.is_empty());
    assert_eq!(base.keybindings.global.get("quit").unwrap(), "ctrl+q");
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
    // global group
    assert_eq!(config.keybindings.global.get("quit").unwrap(), "ctrl+q");
    assert_eq!(config.keybindings.global.get("help").unwrap(), "?");
    assert_eq!(config.keybindings.global.get("app_logs").unwrap(), "ctrl+l");
    assert_eq!(config.keybindings.global.get("context_selector").unwrap(), "ctrl+o");
    // tui group
    assert_eq!(config.keybindings.tui.get("close_tab").unwrap(), "alt+c");
    assert_eq!(config.keybindings.tui.get("split_vertical").unwrap(), "alt+v");
    // navigation group
    assert_eq!(config.keybindings.navigation.get("select").unwrap(), "enter");
    assert_eq!(config.keybindings.navigation.get("back").unwrap(), "esc");
    // browse group
    assert_eq!(config.keybindings.browse.get("view_yaml").unwrap(), "y");
    assert_eq!(config.keybindings.browse.get("save_logs").unwrap(), "ctrl+s");
    assert_eq!(config.keybindings.browse.get("filter").unwrap(), "/");
    // mutate group
    assert_eq!(config.keybindings.mutate.get("delete").unwrap(), "ctrl+alt+d");
    // interact group
    assert_eq!(config.keybindings.interact.get("exec").unwrap(), "e");
    assert_eq!(config.keybindings.interact.get("port_forward").unwrap(), "p");
}

#[test]
fn merge_overrides_keybindings() {
    let mut base: AppConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
    let user_toml = r#"
[keybindings.global]
quit = "ctrl+x"
"#;
    let user: AppConfig = toml::from_str(user_toml).unwrap();
    base.merge(user);

    assert_eq!(base.keybindings.global.get("quit").unwrap(), "ctrl+x");
    assert_eq!(base.keybindings.global.get("help").unwrap(), "?");
}

#[test]
fn empty_user_config_keeps_defaults() {
    let mut base: AppConfig = toml::from_str(DEFAULT_CONFIG).unwrap();
    let user: AppConfig = toml::from_str("").unwrap();
    base.merge(user);
    assert!(!base.keybindings.global.is_empty());
    assert!(!base.keybindings.navigation.is_empty());
    assert!(!base.keybindings.browse.is_empty());
    assert!(!base.keybindings.tui.is_empty());
    assert!(!base.keybindings.mutate.is_empty());
}

#[test]
fn load_returns_defaults_without_user_config() {
    let config = AppConfig::load();
    assert!(!config.keybindings.global.is_empty());
    assert_eq!(config.keybindings.global.get("quit").unwrap(), "ctrl+q");
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
    assert_eq!(loaded.keybindings.global.get("quit").unwrap(), "ctrl+q");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn view_config_roundtrips_through_serde() {
    let config = AppConfig::default();
    let serialized = toml::to_string_pretty(&config).unwrap();
    let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.views.pods.columns, config.views.pods.columns);
    assert_eq!(deserialized.views.deployments.columns, config.views.deployments.columns);
    assert_eq!(deserialized.views.nodes.columns, config.views.nodes.columns);
}

#[test]
fn view_config_unknown_columns_dont_error() {
    let raw = r#"
[views.pods]
columns = ["name", "nonexistent-column", "status"]
"#;
    let config: AppConfig = toml::from_str(raw).unwrap();
    assert_eq!(config.views.pods.columns, vec!["name", "nonexistent-column", "status"]);
}

#[test]
fn filter_columns_empty_config_returns_all() {
    let headers = vec!["NAME".into(), "STATUS".into(), "AGE".into()];
    let rows = vec![vec!["pod1".into(), "Running".into(), "5m".into()]];
    let (h, r) = views::filter_columns(&[], &headers, &rows);
    assert_eq!(h, headers);
    assert_eq!(r, rows);
}

#[test]
fn filter_columns_reorders_to_config_order() {
    let headers = vec!["NAME".into(), "STATUS".into(), "AGE".into(), "NODE".into()];
    let rows = vec![vec!["pod1".into(), "Running".into(), "5m".into(), "node1".into()]];
    let configured: Vec<String> = vec!["age".into(), "name".into(), "status".into()];
    let (h, r) = views::filter_columns(&configured, &headers, &rows);
    assert_eq!(h, vec!["AGE", "NAME", "STATUS"]);
    assert_eq!(r, vec![vec!["5m".to_string(), "pod1".to_string(), "Running".to_string()]]);
}

#[test]
fn filter_columns_unknown_names_silently_ignored() {
    let headers = vec!["NAME".into(), "STATUS".into()];
    let rows = vec![vec!["pod1".into(), "Running".into()]];
    let configured: Vec<String> = vec!["name".into(), "nonexistent".into(), "status".into()];
    let (h, _) = views::filter_columns(&configured, &headers, &rows);
    assert_eq!(h, vec!["NAME", "STATUS"]);
}

#[test]
fn default_config_has_views_from_defaults_toml() {
    let config = AppConfig::default();
    assert_eq!(config.views.pods.columns, vec!["name", "ready", "status", "restarts", "age", "node"]);
    assert_eq!(config.views.services.columns, vec!["name", "type", "cluster-ip", "external-ip", "ports", "age"]);
    assert_eq!(config.views.namespaces.columns, vec!["name", "status", "age"]);
}

#[test]
fn columns_for_returns_correct_resource() {
    let views = ViewsConfig::default();
    assert_eq!(views.columns_for("pods"), &["name", "ready", "status", "restarts", "age", "node"]);
    assert_eq!(views.columns_for("nodes"), &["name", "status", "roles", "age", "version"]);
    assert!(views.columns_for("unknown").is_empty());
}
