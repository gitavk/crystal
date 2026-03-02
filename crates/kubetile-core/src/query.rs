use k8s_openapi::api::core::v1::Pod;
use kube::api::AttachParams;
use kube::Api;
use std::collections::BTreeMap;
use std::collections::HashMap;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone)]
pub struct QueryConfig {
    pub pod: String,
    pub namespace: String,
    pub container: Option<String>,
    pub database: String,
    pub user: String,
    pub password: String,
    pub port: String,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub async fn read_postgres_env(client: &kube::Client, pod: &str, namespace: &str) -> QueryConfig {
    let mut config = QueryConfig {
        pod: pod.to_string(),
        namespace: namespace.to_string(),
        container: None,
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

    let Some(container) = select_postgres_container(&spec.containers) else {
        return config;
    };
    config.container = Some(container.name.clone());

    let mut resolved_env = HashMap::new();
    let config_maps = Api::<k8s_openapi::api::core::v1::ConfigMap>::namespaced(client.clone(), namespace);
    let secrets = Api::<k8s_openapi::api::core::v1::Secret>::namespaced(client.clone(), namespace);
    let mut config_map_cache: HashMap<String, BTreeMap<String, String>> = HashMap::new();
    let mut secret_cache: HashMap<String, BTreeMap<String, String>> = HashMap::new();

    if let Some(env_froms) = &container.env_from {
        for env_from in env_froms {
            let prefix = env_from.prefix.clone().unwrap_or_default();

            if let Some(cm_ref) = &env_from.config_map_ref {
                let data = load_config_map_data(&config_maps, &mut config_map_cache, &cm_ref.name).await;
                for (k, v) in data {
                    resolved_env.insert(format!("{prefix}{k}"), v);
                }
            }

            if let Some(secret_ref) = &env_from.secret_ref {
                let data = load_secret_data(&secrets, &mut secret_cache, &secret_ref.name).await;
                for (k, v) in data {
                    resolved_env.insert(format!("{prefix}{k}"), v);
                }
            }
        }
    }

    if let Some(env_vars) = &container.env {
        for env in env_vars {
            if let Some(val) = &env.value {
                resolved_env.insert(env.name.clone(), val.clone());
                continue;
            }

            if let Some(value_from) = &env.value_from {
                if let Some(cm_key_ref) = &value_from.config_map_key_ref {
                    let data = load_config_map_data(&config_maps, &mut config_map_cache, &cm_key_ref.name).await;
                    if let Some(value) = data.get(&cm_key_ref.key) {
                        resolved_env.insert(env.name.clone(), value.clone());
                        continue;
                    }
                }

                if let Some(secret_key_ref) = &value_from.secret_key_ref {
                    let data = load_secret_data(&secrets, &mut secret_cache, &secret_key_ref.name).await;
                    if let Some(value) = data.get(&secret_key_ref.key) {
                        resolved_env.insert(env.name.clone(), value.clone());
                    }
                }
            }
        }
    }

    if let Some(val) = resolved_env.get("POSTGRES_DB") {
        config.database = val.clone();
    }
    if let Some(val) = resolved_env.get("POSTGRES_USER") {
        config.user = val.clone();
    }
    if let Some(val) = resolved_env.get("POSTGRES_PASSWORD") {
        config.password = val.clone();
    }
    if let Some(val) = resolved_env.get("PGPORT").or_else(|| resolved_env.get("POSTGRES_PORT")) {
        config.port = val.clone();
    }

    config
}

async fn load_config_map_data(
    api: &Api<k8s_openapi::api::core::v1::ConfigMap>,
    cache: &mut HashMap<String, BTreeMap<String, String>>,
    name: &str,
) -> BTreeMap<String, String> {
    if let Some(data) = cache.get(name) {
        return data.clone();
    }
    let data = api.get(name).await.ok().and_then(|cm| cm.data).unwrap_or_default();
    cache.insert(name.to_string(), data.clone());
    data
}

async fn load_secret_data(
    api: &Api<k8s_openapi::api::core::v1::Secret>,
    cache: &mut HashMap<String, BTreeMap<String, String>>,
    name: &str,
) -> BTreeMap<String, String> {
    if let Some(data) = cache.get(name) {
        return data.clone();
    }

    let mut merged = BTreeMap::new();
    if let Ok(secret) = api.get(name).await {
        if let Some(string_data) = secret.string_data {
            merged.extend(string_data);
        }
        if let Some(data) = secret.data {
            for (k, v) in data {
                if let Ok(s) = String::from_utf8(v.0) {
                    merged.insert(k, s);
                }
            }
        }
    }

    cache.insert(name.to_string(), merged.clone());
    merged
}

pub async fn execute_query(client: &kube::Client, config: &QueryConfig, sql: &str) -> anyhow::Result<QueryResult> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), &config.namespace);

    let command = vec![
        "env".to_string(),
        format!("PGPASSWORD={}", config.password),
        "psql".to_string(),
        "-U".to_string(),
        config.user.clone(),
        "-d".to_string(),
        config.database.clone(),
        "-p".to_string(),
        config.port.clone(),
        "--csv".to_string(),
        "-c".to_string(),
        sql.to_string(),
    ];

    let mut attach = AttachParams::default();
    if let Some(container) = &config.container {
        attach = attach.container(container.clone());
    }

    let mut attached = pods.exec(&config.pod, command, &attach).await?;

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    let mut stdout = attached.stdout().ok_or_else(|| anyhow::anyhow!("stdout not available"))?;
    let mut stderr = attached.stderr().ok_or_else(|| anyhow::anyhow!("stderr not available"))?;

    tokio::try_join!(stdout.read_to_end(&mut stdout_buf), stderr.read_to_end(&mut stderr_buf),)?;

    attached.join().await?;

    let stderr_str = String::from_utf8_lossy(&stderr_buf);
    let stderr_trimmed = stderr_str.trim();
    if !stderr_trimmed.is_empty() {
        return Err(anyhow::anyhow!("{}", stderr_trimmed));
    }

    parse_csv_output(&String::from_utf8_lossy(&stdout_buf))
}

fn select_postgres_container(
    containers: &[k8s_openapi::api::core::v1::Container],
) -> Option<&k8s_openapi::api::core::v1::Container> {
    if containers.is_empty() {
        return None;
    }

    // Prefer the database container over sidecars (for example postgres-exporter).
    if let Some(c) = containers
        .iter()
        .find(|c| c.ports.as_ref().map(|ports| ports.iter().any(|p| p.container_port == 5432)).unwrap_or(false))
    {
        return Some(c);
    }

    if let Some(c) = containers.iter().find(|c| {
        let name = c.name.to_ascii_lowercase();
        let image = c.image.as_deref().unwrap_or_default().to_ascii_lowercase();
        (name.contains("postgres") || image.contains("postgres"))
            && !name.contains("exporter")
            && !image.contains("exporter")
    }) {
        return Some(c);
    }

    containers.first()
}

fn parse_csv_output(output: &str) -> anyhow::Result<QueryResult> {
    let mut reader = csv::Reader::from_reader(output.as_bytes());

    let headers = reader.headers()?.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    let rows = reader
        .records()
        .map(|r| r.map(|rec| rec.iter().map(|s| s.to_string()).collect()))
        .collect::<Result<Vec<Vec<String>>, _>>()?;

    Ok(QueryResult { headers, rows })
}
