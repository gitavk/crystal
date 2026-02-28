use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryHistoryEntry {
    pub sql: String,
    pub ts: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryHistory {
    pub entries: Vec<QueryHistoryEntry>,
    #[serde(skip)]
    path: PathBuf,
}

impl QueryHistory {
    pub fn load(namespace: &str, pod: &str, db: &str) -> Self {
        let path = history_path(namespace, pod, db);
        let entries =
            std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { entries, path }
    }

    pub fn append(&mut self, sql: &str) -> io::Result<()> {
        if self.entries.first().map(|e| e.sql.as_str()) == Some(sql) {
            return Ok(());
        }
        let ts = jiff::Timestamp::now().to_string();
        self.entries.insert(0, QueryHistoryEntry { sql: sql.to_string(), ts });
        self.entries.truncate(200);
        self.save()
    }

    pub fn delete(&mut self, index: usize) -> io::Result<()> {
        if index < self.entries.len() {
            self.entries.remove(index);
            self.save()?;
        }
        Ok(())
    }

    fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.entries).map_err(io::Error::other)?;
        std::fs::write(&self.path, data)
    }
}

fn history_path(namespace: &str, pod: &str, db: &str) -> PathBuf {
    let name = format!("{}__{}__{}.json", sanitize(namespace), sanitize(pod), sanitize(db));
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("kubetile").join("query_history").join(name)
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect()
}
