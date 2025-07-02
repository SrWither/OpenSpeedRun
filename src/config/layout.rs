use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub font_size: f32,
    pub background_color: String,
    pub text_color: String,
    pub show_title: bool,
    pub show_category: bool,
    pub show_splits: bool,
    pub titlebar: bool,
    pub window_size: (u32, u32),
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            font_size: 28f32,
            background_color: "#1e1e2e".to_string(),
            text_color: "#cdd6f4".to_string(),
            show_title: true,
            show_category: true,
            show_splits: true,
            titlebar: true,
            window_size: (720, 1280),
        }
    }
}

impl LayoutConfig {
    pub fn load_or_default(path: &str) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, json)?;
        Ok(())
    }
}
