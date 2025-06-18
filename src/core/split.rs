use chrono::{Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Split {
    pub name: String,
    pub pb_time: Option<Duration>,
    pub last_time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub title: String,
    pub category: String,
    pub splits: Vec<Split>,
}

impl Run {
    pub fn new(title: &str, category: &str, names: &[&str]) -> Self {
        let splits = names.iter().map(|name| Split {
            name: name.to_string(),
            pb_time: None,
            last_time: None,
        }).collect();

        Self {
            title: title.to_string(),
            category: category.to_string(),
            splits,
        }
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let file = std::fs::read_to_string(path)?;
        let run: Self = serde_json::from_str(&file).expect("Invalid JSON");
        Ok(run)
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, json)?;
        Ok(())
    }
}