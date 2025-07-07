use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::layout::LayoutConfig;
use crate::core::split::Run;

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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_split_path: "splits/sample".to_string(),
            theme: "themes/default.json".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = config_base_dir().join("config.json");

        if path.exists() {
            if let Ok(config_str) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&config_str) {
                    Self::ensure_default_split_if_missing(&config);
                    Self::ensure_default_theme_if_missing(&config);
                    return config;
                }
            }
        }

        let default_config = AppConfig::default();
        default_config.save();
        Self::ensure_default_split_if_missing(&default_config);
        Self::ensure_default_theme_if_missing(&default_config);
        default_config
    }

    pub fn save(&self) {
        let path = config_base_dir().join("config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let json = serde_json::to_string_pretty(self).unwrap();
        let _ = fs::write(path, json);
    }

    fn ensure_default_split_if_missing(config: &AppConfig) {
        let base_dir = config_base_dir();
        let relative_dir = Path::new(&config.last_split_path);

        let split_path = base_dir.join(relative_dir).join("split.json");

        if !split_path.exists() {
            if let Some(parent) = split_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("❌ Failed to create directory for split: {}", e);
                    return;
                }
            }

            let run = Run::new(
                "Sample Game",
                "Any%",
                &["Intro", "Level 1", "Level 2", "Final Boss"],
            );

            match run.save_to_file(split_path.to_str().unwrap()) {
                Ok(_) => println!("✅ Default split created at: {}", split_path.display()),
                Err(e) => eprintln!("❌ Failed to save default split: {}", e),
            }
        }
    }

    fn ensure_default_theme_if_missing(config: &AppConfig) {
        let theme_path = config_base_dir().join(&config.theme);
        if !theme_path.exists() {
            if let Some(parent) = theme_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let theme_config = LayoutConfig::default();

            match serde_json::to_string_pretty(&theme_config) {
                Ok(json) => {
                    if let Err(e) = fs::write(&theme_path, json) {
                        eprintln!("❌ Failed to write default theme: {}", e);
                    } else {
                        println!("✅ Default theme created at: {}", theme_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to serialize default theme: {}", e);
                }
            }
        }
    }
}
