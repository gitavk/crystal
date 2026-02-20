use std::path::Path;

use anyhow::Result;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use kube::api::ListParams;
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Api, Client, Config};

use crate::context::ClusterContext;
use crate::resources::PodSummary;

#[derive(Clone)]
pub struct KubeClient {
    client: Client,
    current_namespace: String,
    current_context: String,
}

impl KubeClient {
    fn read_kubeconfig_with_fallback() -> Result<Kubeconfig> {
        if let Some(kubeconfig) = Self::load_kubeconfig_from_env()? {
            return Ok(kubeconfig);
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let default_path = std::path::PathBuf::from(home).join(".kube").join("config");
        Kubeconfig::read_from(&default_path).map_err(|e| anyhow::anyhow!(e))
    }

    fn load_kubeconfig_from_env() -> Result<Option<Kubeconfig>> {
        match std::env::var_os("KUBECONFIG") {
            Some(paths) => {
                let mut merged: Option<Kubeconfig> = None;
                for path in std::env::split_paths(&paths).filter(|p| !p.as_os_str().is_empty()) {
                    if !path.exists() {
                        continue;
                    }

                    let config = Kubeconfig::read_from(&path)?;
                    merged = Some(if let Some(previous) = merged { previous.merge(config)? } else { config });
                }

                Ok(merged)
            }
            None => Ok(None),
        }
    }

    pub async fn from_kubeconfig() -> Result<Self> {
        let kubeconfig = Self::read_kubeconfig_with_fallback()?;
        let current_context = kubeconfig.current_context.clone().unwrap_or_else(|| "unknown".into());

        let config = Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default()).await?;
        let default_ns = config.default_namespace.clone();
        let client = Client::try_from(config)?;

        Ok(Self { client, current_namespace: default_ns, current_context })
    }

    pub async fn from_config(path: &Path, context: &str) -> Result<Self> {
        let kubeconfig = Kubeconfig::read_from(path)?;
        let opts = KubeConfigOptions { context: Some(context.to_string()), ..Default::default() };
        let config = Config::from_custom_kubeconfig(kubeconfig, &opts).await?;
        let default_ns = config.default_namespace.clone();
        let client = Client::try_from(config)?;

        Ok(Self { client, current_namespace: default_ns, current_context: context.to_string() })
    }

    pub async fn from_context(context: &str) -> Result<Self> {
        let kubeconfig = Self::read_kubeconfig_with_fallback()?;
        let opts = KubeConfigOptions { context: Some(context.to_string()), ..Default::default() };
        let config = Config::from_custom_kubeconfig(kubeconfig, &opts).await?;
        let default_ns = config.default_namespace.clone();
        let client = Client::try_from(config)?;
        Ok(Self { client, current_namespace: default_ns, current_context: context.to_string() })
    }

    pub fn cluster_context(&self) -> ClusterContext {
        ClusterContext { name: self.current_context.clone(), namespace: self.current_namespace.clone() }
    }

    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let api: Api<Namespace> = Api::all(self.client.clone());
        let list = api.list(&ListParams::default()).await?;
        Ok(list.items.iter().filter_map(|ns| ns.metadata.name.clone()).collect())
    }

    pub fn list_contexts() -> Result<Vec<String>> {
        let kubeconfig = Self::read_kubeconfig_with_fallback()?;
        Ok(kubeconfig.contexts.iter().map(|c| c.name.clone()).collect())
    }

    pub async fn list_pods(&self, namespace: Option<&str>) -> Result<Vec<PodSummary>> {
        let ns = namespace.unwrap_or(&self.current_namespace);
        let api: Api<Pod> = Api::namespaced(self.client.clone(), ns);
        let list = api.list(&ListParams::default()).await?;
        Ok(list.items.iter().map(PodSummary::from).collect())
    }

    pub fn set_namespace(&mut self, ns: &str) {
        self.current_namespace = ns.to_string();
    }

    pub fn context(&self) -> &str {
        &self.current_context
    }

    pub fn namespace(&self) -> &str {
        &self.current_namespace
    }

    pub fn inner_client(&self) -> Client {
        self.client.clone()
    }
}

#[cfg(test)]
mod tests;
