use std::path::{PathBuf};
use dirs::config_dir;
use serde::{Serialize, Deserialize};
use std::fs;

pub fn config_base_dir() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("openspeedrun")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub last_split_path: String,
    pub theme: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let path = config_base_dir().join("config.json");
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| Self {
                last_split_path: "splits/example".into(),
                theme: "themes/default.json".into(),
            })
    }

    pub fn save(&self) {
        let path = config_base_dir().join("config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string_pretty(self).unwrap();
        let _ = fs::write(path, json);
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_split_path: "splits/example".into(),
            theme: "themes/default.json".into(),
        }
    }
}