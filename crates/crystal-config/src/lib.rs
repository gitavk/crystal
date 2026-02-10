use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tick_rate_ms: Option<u64>,
}

impl Config {
    pub fn tick_rate_ms(&self) -> u64 {
        self.tick_rate_ms.unwrap_or(250)
    }
}

#[cfg(test)]
mod tests;
