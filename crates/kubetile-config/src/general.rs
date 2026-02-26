use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct GeneralConfig {
    #[serde(alias = "tick-rate-ms")]
    pub tick_rate_ms: u64,
    #[serde(alias = "default-namespace")]
    pub default_namespace: String,
    #[serde(alias = "default-view")]
    pub default_view: String,
    pub editor: String,
    pub shell: String,
    #[serde(alias = "log-tail-lines")]
    pub log_tail_lines: u32,
    #[serde(alias = "confirm-delete")]
    pub confirm_delete: bool,
    #[serde(alias = "show-managed-fields")]
    pub show_managed_fields: bool,
    #[serde(alias = "query-open-new-tab")]
    pub query_open_new_tab: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 250,
            default_namespace: "default".into(),
            default_view: "pods".into(),
            editor: "$EDITOR".into(),
            shell: "$SHELL".into(),
            log_tail_lines: 1000,
            confirm_delete: true,
            show_managed_fields: false,
            query_open_new_tab: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TerminalConfig {
    #[serde(alias = "scrollback-lines")]
    pub scrollback_lines: u32,
    #[serde(alias = "cursor-style")]
    pub cursor_style: String,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self { scrollback_lines: 10000, cursor_style: "block".into() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FeatureFlags {
    #[serde(alias = "hot-reload")]
    pub hot_reload: bool,
    #[serde(alias = "command-palette")]
    pub command_palette: bool,
    #[serde(alias = "port-forward")]
    pub port_forward: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self { hot_reload: true, command_palette: true, port_forward: true }
    }
}
