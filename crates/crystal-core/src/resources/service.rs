use std::time::Duration;

use k8s_openapi::api::core::v1::Service;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct ServiceSummary {
    pub name: String,
    pub namespace: String,
    pub type_: String,
    pub cluster_ip: String,
    pub external_ip: String,
    pub ports: String,
    pub age: Duration,
}

impl ResourceSummary for ServiceSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        self.type_.clone()
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("TYPE", self.type_.clone()),
            ("CLUSTER-IP", self.cluster_ip.clone()),
            ("EXTERNAL-IP", self.external_ip.clone()),
            ("PORT(S)", self.ports.clone()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.type_.clone(),
            self.cluster_ip.clone(),
            self.external_ip.clone(),
            self.ports.clone(),
            format_duration(self.age),
        ]
    }

    fn detail_sections(&self) -> Vec<DetailSection> {
        vec![
            DetailSection {
                title: "Metadata".into(),
                fields: vec![
                    ("Name".into(), self.name.clone()),
                    ("Namespace".into(), self.namespace.clone()),
                    ("Age".into(), format_duration(self.age)),
                ],
            },
            DetailSection {
                title: "Spec".into(),
                fields: vec![
                    ("Type".into(), self.type_.clone()),
                    ("Cluster IP".into(), self.cluster_ip.clone()),
                    ("External IP".into(), self.external_ip.clone()),
                    ("Ports".into(), self.ports.clone()),
                ],
            },
        ]
    }
}

impl From<&Service> for ServiceSummary {
    fn from(svc: &Service) -> Self {
        let meta = &svc.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let spec = svc.spec.as_ref();
        let type_ = spec.and_then(|s| s.type_.clone()).unwrap_or_else(|| "ClusterIP".into());
        let cluster_ip = spec.and_then(|s| s.cluster_ip.clone()).unwrap_or_else(|| "<none>".into());

        let external_ip = spec
            .and_then(|s| s.external_ips.as_ref())
            .filter(|ips| !ips.is_empty())
            .map(|ips| ips.join(","))
            .unwrap_or_else(|| "<none>".into());

        let ports = spec
            .and_then(|s| s.ports.as_ref())
            .map(|ports| {
                ports
                    .iter()
                    .map(|p| {
                        let port = p.port;
                        let protocol = p.protocol.as_deref().unwrap_or("TCP");
                        match p.node_port {
                            Some(np) => format!("{port}:{np}/{protocol}"),
                            None => format!("{port}/{protocol}"),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "<none>".into());

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, type_, cluster_ip, external_ip, ports, age }
    }
}

impl From<Service> for ServiceSummary {
    fn from(s: Service) -> Self {
        Self::from(&s)
    }
}
