use std::time::Duration;

use k8s_openapi::api::networking::v1::Ingress;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct IngressSummary {
    pub name: String,
    pub namespace: String,
    pub class: String,
    pub hosts: String,
    pub address: String,
    pub ports: String,
    pub age: Duration,
}

impl ResourceSummary for IngressSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        self.class.clone()
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("CLASS", self.class.clone()),
            ("HOSTS", self.hosts.clone()),
            ("ADDRESS", self.address.clone()),
            ("PORTS", self.ports.clone()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.class.clone(),
            self.hosts.clone(),
            self.address.clone(),
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
                    ("Class".into(), self.class.clone()),
                    ("Hosts".into(), self.hosts.clone()),
                    ("Address".into(), self.address.clone()),
                    ("Ports".into(), self.ports.clone()),
                ],
            },
        ]
    }
}

impl From<&Ingress> for IngressSummary {
    fn from(ing: &Ingress) -> Self {
        let meta = &ing.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let spec = ing.spec.as_ref();
        let class = spec.and_then(|s| s.ingress_class_name.clone()).unwrap_or_else(|| "<none>".into());

        let hosts = spec
            .and_then(|s| s.rules.as_ref())
            .map(|rules| rules.iter().filter_map(|r| r.host.as_deref()).collect::<Vec<_>>().join(","))
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| "*".into());

        let address = ing
            .status
            .as_ref()
            .and_then(|s| s.load_balancer.as_ref())
            .and_then(|lb| lb.ingress.as_ref())
            .and_then(|ingresses| ingresses.first().and_then(|i| i.ip.clone().or_else(|| i.hostname.clone())))
            .unwrap_or_default();

        let has_tls = spec.and_then(|s| s.tls.as_ref()).is_some_and(|t| !t.is_empty());
        let ports = if has_tls { "80, 443".into() } else { "80".into() };

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, class, hosts, address, ports, age }
    }
}

impl From<Ingress> for IngressSummary {
    fn from(i: Ingress) -> Self {
        Self::from(&i)
    }
}
