#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Pods,
    Deployments,
    Services,
    StatefulSets,
    DaemonSets,
    Jobs,
    CronJobs,
    ConfigMaps,
    Secrets,
    Ingresses,
    Nodes,
    Namespaces,
    PersistentVolumes,
    PersistentVolumeClaims,
    Custom(String),
}

impl ResourceKind {
    pub fn short_name(&self) -> &str {
        match self {
            Self::Pods => "po",
            Self::Deployments => "deploy",
            Self::Services => "svc",
            Self::StatefulSets => "sts",
            Self::DaemonSets => "ds",
            Self::Jobs => "job",
            Self::CronJobs => "cj",
            Self::ConfigMaps => "cm",
            Self::Secrets => "secret",
            Self::Ingresses => "ing",
            Self::Nodes => "no",
            Self::Namespaces => "ns",
            Self::PersistentVolumes => "pv",
            Self::PersistentVolumeClaims => "pvc",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Pods => "Pods",
            Self::Deployments => "Deployments",
            Self::Services => "Services",
            Self::StatefulSets => "StatefulSets",
            Self::DaemonSets => "DaemonSets",
            Self::Jobs => "Jobs",
            Self::CronJobs => "CronJobs",
            Self::ConfigMaps => "ConfigMaps",
            Self::Secrets => "Secrets",
            Self::Ingresses => "Ingresses",
            Self::Nodes => "Nodes",
            Self::Namespaces => "Namespaces",
            Self::PersistentVolumes => "PersistentVolumes",
            Self::PersistentVolumeClaims => "PersistentVolumeClaims",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn all() -> &'static [ResourceKind] {
        &[
            Self::Pods,
            Self::Deployments,
            Self::Services,
            Self::StatefulSets,
            Self::DaemonSets,
            Self::Jobs,
            Self::CronJobs,
            Self::ConfigMaps,
            Self::Secrets,
            Self::Ingresses,
            Self::Nodes,
            Self::Namespaces,
            Self::PersistentVolumes,
            Self::PersistentVolumeClaims,
        ]
    }

    pub fn from_short_name(s: &str) -> Option<Self> {
        Self::all().iter().find(|k| k.short_name() == s).cloned()
    }

    pub fn is_namespaced(&self) -> bool {
        !matches!(self, Self::Nodes | Self::Namespaces | Self::PersistentVolumes)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    ResourceList(ResourceKind),
    Detail(ResourceKind, String), // kind + resource name
    Terminal,
    Logs(String),               // pod name
    Exec(String),               // pod name
    Yaml(ResourceKind, String), // kind + resource name
    Help,
    Empty,
    Plugin(String), // plugin name
    Query(String),  // pod name
}
