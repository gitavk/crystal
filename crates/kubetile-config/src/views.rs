use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceViewConfig {
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ViewsConfig {
    pub pods: ResourceViewConfig,
    pub deployments: ResourceViewConfig,
    pub services: ResourceViewConfig,
    pub statefulsets: ResourceViewConfig,
    pub daemonsets: ResourceViewConfig,
    pub jobs: ResourceViewConfig,
    pub cronjobs: ResourceViewConfig,
    pub configmaps: ResourceViewConfig,
    pub secrets: ResourceViewConfig,
    pub ingresses: ResourceViewConfig,
    pub nodes: ResourceViewConfig,
    pub namespaces: ResourceViewConfig,
}

impl Default for ViewsConfig {
    fn default() -> Self {
        Self {
            pods: ResourceViewConfig {
                columns: vec!["name", "ready", "status", "restarts", "age", "node"]
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            deployments: ResourceViewConfig {
                columns: vec!["name", "ready", "up-to-date", "available", "age"].into_iter().map(Into::into).collect(),
            },
            services: ResourceViewConfig {
                columns: vec!["name", "type", "cluster-ip", "external-ip", "ports", "age"]
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            statefulsets: ResourceViewConfig {
                columns: vec!["name", "ready", "age"].into_iter().map(Into::into).collect(),
            },
            daemonsets: ResourceViewConfig {
                columns: vec!["name", "desired", "current", "ready", "age"].into_iter().map(Into::into).collect(),
            },
            jobs: ResourceViewConfig {
                columns: vec!["name", "completions", "duration", "age"].into_iter().map(Into::into).collect(),
            },
            cronjobs: ResourceViewConfig {
                columns: vec!["name", "schedule", "suspend", "active", "last-schedule", "age"]
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            configmaps: ResourceViewConfig {
                columns: vec!["name", "data", "age"].into_iter().map(Into::into).collect(),
            },
            secrets: ResourceViewConfig {
                columns: vec!["name", "type", "data", "age"].into_iter().map(Into::into).collect(),
            },
            ingresses: ResourceViewConfig {
                columns: vec!["name", "class", "hosts", "address", "ports", "age"]
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            nodes: ResourceViewConfig {
                columns: vec!["name", "status", "roles", "age", "version"].into_iter().map(Into::into).collect(),
            },
            namespaces: ResourceViewConfig {
                columns: vec!["name", "status", "age"].into_iter().map(Into::into).collect(),
            },
        }
    }
}

impl ViewsConfig {
    pub fn columns_for(&self, resource_kind: &str) -> &[String] {
        match resource_kind {
            "pods" => &self.pods.columns,
            "deployments" => &self.deployments.columns,
            "services" => &self.services.columns,
            "statefulsets" => &self.statefulsets.columns,
            "daemonsets" => &self.daemonsets.columns,
            "jobs" => &self.jobs.columns,
            "cronjobs" => &self.cronjobs.columns,
            "configmaps" => &self.configmaps.columns,
            "secrets" => &self.secrets.columns,
            "ingresses" => &self.ingresses.columns,
            "nodes" => &self.nodes.columns,
            "namespaces" => &self.namespaces.columns,
            _ => &[],
        }
    }
}

pub fn filter_columns(
    configured: &[String],
    headers: &[String],
    rows: &[Vec<String>],
) -> (Vec<String>, Vec<Vec<String>>) {
    if configured.is_empty() {
        return (headers.to_vec(), rows.to_vec());
    }

    let header_lower: Vec<String> = headers.iter().map(|h| h.to_lowercase()).collect();

    let indices: Vec<usize> = configured.iter().filter_map(|col| header_lower.iter().position(|h| h == col)).collect();

    if indices.is_empty() {
        return (headers.to_vec(), rows.to_vec());
    }

    let filtered_headers: Vec<String> = indices.iter().map(|&i| headers[i].clone()).collect();
    let filtered_rows: Vec<Vec<String>> =
        rows.iter().map(|row| indices.iter().map(|&i| row.get(i).cloned().unwrap_or_default()).collect()).collect();

    (filtered_headers, filtered_rows)
}
