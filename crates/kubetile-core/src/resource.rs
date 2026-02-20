use std::time::Duration;

use jiff::Timestamp;

#[derive(Debug, Clone)]
pub struct DetailSection {
    pub title: String,
    pub fields: Vec<(String, String)>,
}

pub trait ResourceSummary: Send + Sync {
    fn name(&self) -> &str;
    fn namespace(&self) -> Option<&str>;
    fn status_display(&self) -> String;
    fn age(&self) -> Duration;
    fn columns(&self) -> Vec<(&str, String)>;
    fn row(&self) -> Vec<String>;
    fn detail_sections(&self) -> Vec<DetailSection>;
}

pub fn calculate_age(creation: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>) -> Duration {
    creation
        .and_then(|ts| {
            let diff = Timestamp::now().since(ts.0).ok()?;
            Some(Duration::from_secs(diff.get_seconds().unsigned_abs()))
        })
        .unwrap_or_default()
}

pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_ranges() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h");
        assert_eq!(format_duration(Duration::from_secs(172800)), "2d");
    }
}
