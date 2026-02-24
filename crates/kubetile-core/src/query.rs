use k8s_openapi::api::core::v1::Pod;
use kube::api::AttachParams;
use kube::Api;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone)]
pub struct QueryConfig {
    pub pod: String,
    pub namespace: String,
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

    let mut attached = pods.exec(&config.pod, command, &AttachParams::default()).await?;

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

fn parse_csv_output(output: &str) -> anyhow::Result<QueryResult> {
    let mut reader = csv::Reader::from_reader(output.as_bytes());

    let headers = reader.headers()?.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    let rows = reader
        .records()
        .map(|r| r.map(|rec| rec.iter().map(|s| s.to_string()).collect()))
        .collect::<Result<Vec<Vec<String>>, _>>()?;

    Ok(QueryResult { headers, rows })
}
