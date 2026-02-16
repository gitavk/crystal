use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub accent: String,
    pub bg: String,
    pub fg: String,
    #[serde(alias = "header-bg")]
    pub header_bg: String,
    #[serde(alias = "header-fg")]
    pub header_fg: String,
    #[serde(alias = "selection-bg")]
    pub selection_bg: String,
    #[serde(alias = "selection-fg")]
    pub selection_fg: String,
    pub border: String,
    #[serde(alias = "border-active")]
    pub border_active: String,
    #[serde(alias = "text-dim")]
    pub text_dim: String,
    #[serde(alias = "overlay-bg")]
    pub overlay_bg: String,

    #[serde(alias = "status-running")]
    pub status_running: String,
    #[serde(alias = "status-pending")]
    pub status_pending: String,
    #[serde(alias = "status-failed")]
    pub status_failed: String,
    #[serde(alias = "status-unknown")]
    pub status_unknown: String,

    #[serde(alias = "yaml-key")]
    pub yaml_key: String,
    #[serde(alias = "yaml-string")]
    pub yaml_string: String,
    #[serde(alias = "yaml-number")]
    pub yaml_number: String,
    #[serde(alias = "yaml-boolean")]
    pub yaml_boolean: String,
    #[serde(alias = "yaml-null")]
    pub yaml_null: String,

    #[serde(alias = "insert-mode-bg")]
    pub insert_mode_bg: String,
    #[serde(alias = "insert-mode-fg")]
    pub insert_mode_fg: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent: "#89b4fa".into(),
            bg: "default".into(),
            fg: "#cdd6f4".into(),
            header_bg: "#1e1e2e".into(),
            header_fg: "#cdd6f4".into(),
            selection_bg: "#45475a".into(),
            selection_fg: "#cdd6f4".into(),
            border: "#585b70".into(),
            border_active: "#89b4fa".into(),
            text_dim: "#6c7086".into(),
            overlay_bg: "#1e1e2e".into(),
            status_running: "#a6e3a1".into(),
            status_pending: "#f9e2af".into(),
            status_failed: "#f38ba8".into(),
            status_unknown: "#585b70".into(),
            yaml_key: "#89b4fa".into(),
            yaml_string: "#a6e3a1".into(),
            yaml_number: "#fab387".into(),
            yaml_boolean: "#cba6f7".into(),
            yaml_null: "#585b70".into(),
            insert_mode_bg: "#a6e3a1".into(),
            insert_mode_fg: "#1e1e2e".into(),
        }
    }
}
