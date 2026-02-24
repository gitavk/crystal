use k8s_openapi::api::core::v1::Pod;
use kube::Api;

#[derive(Debug, Clone)]
pub struct QueryConfig {
    pub pod: String,
    pub namespace: String,
    pub database: String,
    pub user: String,
    pub password: String,
    pub port: String,
}

pub async fn read_postgres_env(client: &kube::Client, pod: &str, namespace: &str) -> QueryConfig {
    let mut config = QueryConfig {
        pod: pod.to_string(),
        namespace: namespace.to_string(),
        database: String::new(),
        user: String::new(),
        password: String::new(),
        port: "5432".to_string(),
    };

    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let Ok(pod_obj) = pods.get(pod).await else {
        return config;
    };
    let Some(spec) = pod_obj.spec else {
        return config;
    };

    if let Some(container) = spec.containers.into_iter().next() {
        if let Some(env_vars) = container.env {
            for env in env_vars {
                match env.name.as_str() {
                    "POSTGRES_DB" => {
                        if let Some(val) = env.value {
                            config.database = val;
                        }
                    }
                    "POSTGRES_USER" => {
                        if let Some(val) = env.value {
                            config.user = val;
                        }
                    }
                    "POSTGRES_PASSWORD" => {
                        if let Some(val) = env.value {
                            config.password = val;
                        }
                    }
                    "PGPORT" => {
                        if let Some(val) = env.value {
                            config.port = val;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    config
}
