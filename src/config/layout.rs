#[cfg(windows)]
use crate::config::keys::KeyWrapper;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    pub font_sizes: FontSizes,
    pub colors: Colors,
    pub spacings: Spacings,
    pub options: Options,
    #[cfg(windows)]
    pub hotkeys: Hotkeys,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSizes {
    pub title: f32,
    pub category: f32,
    pub timer: f32,
    pub split: f32,
    pub split_timer: f32,
    pub split_gold: f32,
    pub split_pb: f32,
    pub info: f32,
    pub font: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Colors {
    pub background: String,
    pub title: String,
    pub category: String,
    pub timer: String,
    pub split: String,
    pub split_selected: String,
    pub split_timer: String,
    pub gold_positive: String,
    pub gold_negative: String,
    pub pb_positive: String,
    pub pb_negative: String,
    pub info: String,
    pub shader_path: String,
    pub background_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Spacings {
    pub split_top: f32,
    pub split_bottom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub show_title: bool,
    pub show_category: bool,
    pub show_splits: bool,
    pub show_info: bool,
    pub show_body: bool,
    pub show_footer: bool,
    pub show_relative_times: bool,
    pub show_last_relative_time: bool,
    pub titlebar: bool,
    pub enable_shader: bool,
    pub enable_background_image: bool,
    pub window_size: (u32, u32),
}

#[cfg(windows)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub split: KeyWrapper,
    pub start: KeyWrapper,
    pub pause: KeyWrapper,
    pub reset: KeyWrapper,
    pub save_pb: KeyWrapper,
    pub undo_split: KeyWrapper,
    pub undo_pb: KeyWrapper,
    pub next_page: KeyWrapper,
    pub prev_page: KeyWrapper,
    pub toggle_help: KeyWrapper,
    pub reload_all: KeyWrapper,
    pub reload_run: KeyWrapper,
    pub reload_theme: KeyWrapper,
}

#[cfg(windows)]
impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            split: KeyWrapper::default(),
            start: KeyWrapper::default(),
            pause: KeyWrapper::default(),
            reset: KeyWrapper::default(),
            save_pb: KeyWrapper::default(),
            undo_split: KeyWrapper::default(),
            undo_pb: KeyWrapper::default(),
            next_page: KeyWrapper::default(),
            prev_page: KeyWrapper::default(),
            toggle_help: KeyWrapper::default(),
            reload_all: KeyWrapper::default(),
            reload_run: KeyWrapper::default(),
            reload_theme: KeyWrapper::default(),
        }
    }
}

impl Default for FontSizes {
    fn default() -> Self {
        Self {
            title: 24.0,
            category: 20.0,
            timer: 18.0,
            split: 16.0,
            split_timer: 20.0,
            split_gold: 14.0,
            split_pb: 14.0,
            info: 14.0,
            font: None,
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: "#000000".to_string(),
            title: "#FFFFFF".to_string(),
            category: "#CCCCCC".to_string(),
            timer: "#FF0000".to_string(),
            split: "#00FF00".to_string(),
            split_timer: "#0000FF".to_string(),
            split_selected: "#FFFF00".to_string(),
            gold_positive: "#FFD700".to_string(),
            gold_negative: "#FF4500".to_string(),
            pb_positive: "#32CD32".to_string(),
            pb_negative: "#FF6347".to_string(),
            info: "#808080".to_string(),
            shader_path: "".to_string(),
            background_image: None,
        }
    }
}

impl Default for Spacings {
    fn default() -> Self {
        Self {
            split_top: 6.0,
            split_bottom: 6.0,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            show_title: true,
            show_category: true,
            show_splits: true,
            show_info: true,
            show_body: true,
            show_footer: true,
            show_relative_times: false,
            show_last_relative_time: false,
            titlebar: true,
            enable_shader: false,
            enable_background_image: false,
            window_size: (720, 1280),
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            font_sizes: FontSizes::default(),
            colors: Colors::default(),
            spacings: Spacings::default(),
            options: Options::default(),
            #[cfg(windows)]
            hotkeys: Hotkeys::default(),
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
