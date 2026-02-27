use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SavedQuery {
    pub name: String,
    pub sql: String,
    pub ts: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SavedQueries {
    pub entries: Vec<SavedQuery>,
    #[serde(skip)]
    path: PathBuf,
}

impl SavedQueries {
    pub fn load() -> Self {
        let path = saved_queries_path();
        let entries =
            std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { entries, path }
    }

    pub fn add(&mut self, name: &str, sql: &str) -> io::Result<()> {
        let ts = jiff::Timestamp::now().to_string();
        self.entries.push(SavedQuery { name: name.to_string(), sql: sql.to_string(), ts });
        self.save()
    }

    pub fn rename(&mut self, index: usize, new_name: &str) -> io::Result<()> {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.name = new_name.to_string();
            self.save()?;
        }
        Ok(())
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

fn saved_queries_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("kubetile").join("saved_queries.json")
}
