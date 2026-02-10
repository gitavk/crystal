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
