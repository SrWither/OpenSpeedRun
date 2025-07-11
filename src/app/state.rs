use chrono::Duration;
use eframe::glow::Context;
use eframe::{egui, glow};
use egui::TextureHandle;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use crate::config::layout::LayoutConfig;
use crate::config::load::{AppConfig, config_base_dir};
use crate::config::shaders::ShaderBackground;
#[cfg(unix)]
use crate::core::server::UICommand;
use crate::core::split::{Run, Split};
use crate::core::timer::{Timer, TimerState};
#[cfg(windows)]
use crate::core::winserver::UICommand;

pub struct AppState {
    pub timer: Timer,
    pub run: Run,
    pub layout: LayoutConfig,
    pub current_split: usize,
    pub textures: HashMap<String, TextureHandle>,
    pub split_base_path: std::path::PathBuf,
    pub current_page: usize,
    pub splits_per_page: usize,
    pub splits_display: Vec<Split>,
    pub splits_backup: Vec<Split>,
    pub show_help: bool,
    pub start_time: std::time::Instant,
    pub last_elapsed: f32,
    pub shader: Option<ShaderBackground>,
    pub gl: Option<Arc<Context>>,
    pub fonts_loaded: bool,
    pub transparent_set: bool,
    pub background_image: Option<TextureHandle>,
    pub background_image_name: Option<String>,
    pub background_gl_texture: Option<glow::NativeTexture>,
}

impl Default for AppState {
    fn default() -> Self {
        let app_config = AppConfig::load();
        println!("Using config: {:?}", app_config);
        let split_base_path = config_base_dir().join(&app_config.last_split_path);
        let run_path = split_base_path.join("split.json");

        let run = Run::load_from_file(run_path.to_str().unwrap()).unwrap_or_else(|_| {
            Run::new("Untitled", "Any%", &["Split 1", "Split 2", "Final Split"])
        });

        let splits_per_page = run.splits_per_page.unwrap_or(5);
        let layout_path = config_base_dir().join(&app_config.theme);
        let layout = LayoutConfig::load_or_default(layout_path.to_str().unwrap());

        let splits = run.splits.clone();

        Self {
            timer: Timer::new(),
            run,
            layout,
            current_split: 0,
            textures: HashMap::new(),
            split_base_path,
            current_page: 0,
            splits_per_page,
            splits_display: splits.clone(),
            splits_backup: splits,
            show_help: false,
            start_time: std::time::Instant::now(),
            last_elapsed: 0.0,
            shader: None,
            gl: None,
            fonts_loaded: false,
            transparent_set: false,
            background_image: None,
            background_image_name: None,
            background_gl_texture: None,
        }
    }
}

impl AppState {
    pub fn split(&mut self) {
        match self.timer.state {
            TimerState::NotStarted => self.start_run(),
            TimerState::Running => self.record_split(),
            _ => {}
        }
    }

    fn start_run(&mut self) {
        self.splits_backup = self.run.splits.clone();
        let offset = self.run.start_offset.unwrap_or(0);
        self.timer.start_with_offset(offset);
        self.current_split = 0;
    }

    fn record_split(&mut self) {
        let now = self.timer.current_time();
        if now < Duration::zero() {
            return;
        }

        if let Some(split) = self.splits_display.get_mut(self.current_split) {
            split.last_time = Some(now);
        }

        self.current_split += 1;

        if self.current_split >= self.splits_display.len() {
            self.timer.pause();
        }

        self.update_page();
        self.check_auto_update_pb();
    }

    fn update_page(&mut self) {
        let next_page = self.current_split / self.splits_per_page;
        let max_page = (self.splits_display.len().saturating_sub(1)) / self.splits_per_page;
        self.current_page = next_page.min(max_page);
    }

    fn check_auto_update_pb(&mut self) {
        if self.run.gold_split {
            for i in 0..self.splits_display.len() {
                let current = &self.splits_display[i];

                if let Some(current_time) = current.last_time {
                    let prev_time = if i == 0 {
                        Duration::zero()
                    } else {
                        self.splits_display[i - 1]
                            .last_time
                            .unwrap_or(Duration::zero())
                    };

                    let relative = current_time - prev_time;

                    let target = &mut self.run.splits[i];

                    match target.gold_time {
                        Some(gold) if relative < gold => target.gold_time = Some(relative),
                        None => target.gold_time = Some(relative),
                        _ => {}
                    }
                }
            }

            self.save_gold_only().unwrap_or_else(|e| {
                eprintln!("Error saving gold splits: {}", e);
            });
        }

        if self.current_split != self.run.splits.len() {
            return;
        }

        let is_new_pb = self.run.splits.iter().enumerate().all(|(i, split)| {
            let current = &self.splits_display[i];

            if let Some(current_time) = current.last_time {
                let prev_time = if i == 0 {
                    Duration::zero()
                } else {
                    self.splits_display[i - 1]
                        .last_time
                        .unwrap_or(Duration::zero())
                };
                let relative = current_time - prev_time;

                match split.pb_time {
                    Some(pb) => relative <= pb,
                    None => true,
                }
            } else {
                false
            }
        });

        if is_new_pb {
            for i in 0..self.splits_display.len() {
                let current = &self.splits_display[i];
                if let Some(current_time) = current.last_time {
                    let prev_time = if i == 0 {
                        Duration::zero()
                    } else {
                        self.splits_display[i - 1]
                            .last_time
                            .unwrap_or(Duration::zero())
                    };

                    let relative = current_time - prev_time;

                    self.run.splits[i].pb_time = Some(relative);
                }
            }
        }

        for split in self.run.splits.iter_mut() {
            split.last_time = None;
        }

        if self.run.auto_update_pb {
            if let Err(e) = self.save_pb() {
                eprintln!("Error saving PB: {}", e);
            }
        }
    }

    pub fn reset_splits(&mut self) {
        if self.current_split == self.run.splits.len() {
            for (snapshot, real) in self.splits_display.iter_mut().zip(&self.run.splits) {
                snapshot.pb_time = real.pb_time;
                snapshot.last_time = None;
            }
        } else {
            for snapshot in &mut self.splits_display {
                snapshot.last_time = None;
            }
        }

        self.current_split = 0;
        self.timer.reset();
        self.sync_splits();
    }

    pub fn undo_split(&mut self) {
        if self.current_split > 0 {
            self.current_split -= 1;

            if let Some(backup) = self.splits_backup.get(self.current_split) {
                if let Some(display_split) = self.splits_display.get_mut(self.current_split) {
                    display_split.last_time = None;
                    display_split.pb_time = backup.pb_time;
                    display_split.gold_time = backup.gold_time;
                }

                if let Some(run_split) = self.run.splits.get_mut(self.current_split) {
                    run_split.last_time = None;
                    run_split.pb_time = backup.pb_time;
                    run_split.gold_time = backup.gold_time;
                }
            }

            self.update_page();

            let path = self.split_base_path.join("split.json");
            let mut saved_run = self.run.clone();

            for split in saved_run.splits.iter_mut() {
                split.last_time = None;
            }

            if let Err(e) = saved_run.save_to_file(path.to_str().unwrap()) {
                eprintln!("Error saving after undo split: {}", e);
            }
        }
    }

    fn sync_splits(&mut self) {
        let run =
            Run::load_from_file(self.split_base_path.join("split.json").to_str().unwrap()).unwrap();
        self.run = run.clone();
        self.splits_display = run.splits.clone();
    }

    pub fn save_pb(&mut self) -> std::io::Result<()> {
        if self.current_split == self.run.splits.len() {
            let path = self.split_base_path.join("split.json");
            return self.run.save_to_file(path.to_str().unwrap());
        }
        Ok(())
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        let path = self.split_base_path.join("split.json");
        return self.run.save_to_file(path.to_str().unwrap());
    }

    fn save_gold_only(&mut self) -> std::io::Result<()> {
        let path = self.split_base_path.join("split.json");
        let mut saved_run = Run::load_from_file(path.to_str().unwrap())?;

        for (saved_split, current_split) in saved_run.splits.iter_mut().zip(self.run.splits.iter())
        {
            saved_split.gold_time = current_split.gold_time.clone();
        }

        saved_run.save_to_file(path.to_str().unwrap())
    }

    pub fn undo_pb(&mut self) {
        self.run.splits = self.splits_backup.clone();
        self.save().unwrap_or_else(|e| {
            eprintln!("Error saving PB after undo: {}", e);
        });
        self.reset_splits();
    }

    pub fn reload_theme(&mut self) {
        if self.timer.state == TimerState::NotStarted {
            let app_config = AppConfig::load();
            let layout_path = config_base_dir().join(&app_config.theme);
            self.layout = LayoutConfig::load_or_default(layout_path.to_str().unwrap());
        }
    }

    pub fn reload_run(&mut self) {
        if self.timer.state == TimerState::NotStarted {
            let app_config = AppConfig::load();
            let split_base_path = config_base_dir().join(&app_config.last_split_path);
            self.split_base_path = split_base_path.clone();

            let run_path = split_base_path.join("split.json");
            self.run = Run::load_from_file(run_path.to_str().unwrap()).unwrap_or_else(|_| {
                Run::new("Untitled", "Any%", &["Split 1", "Split 2", "Final Split"])
            });

            self.splits_display = self.run.splits.clone();
            self.splits_backup = self.run.splits.clone();
            self.current_split = 0;
            self.current_page = 0;
            self.update_page();
        }
    }

    pub fn reload_all(&mut self) {
        if self.timer.state == TimerState::NotStarted {
            self.reload_theme();
            self.reload_run();
            self.sync_splits();
        }
    }

    pub fn format_duration(&self, dur: Duration, sign_mode: u8) -> String {
        let sign = match sign_mode {
            1 => {
                if dur < Duration::zero() {
                    "-"
                } else {
                    ""
                }
            }
            2 => {
                if dur < Duration::zero() {
                    "-"
                } else {
                    "+"
                }
            }
            _ => "",
        };

        let dur_abs = dur.abs();
        let minutes = dur_abs.num_minutes();
        let seconds = dur_abs.num_seconds() % 60;
        let millis = dur_abs.num_milliseconds() % 1000;

        if dur_abs.num_hours() > 0 {
            format!(
                "{}{}:{:02}:{:02}.{:03}",
                sign,
                dur_abs.num_hours(),
                minutes % 60,
                seconds,
                millis
            )
        } else if minutes > 0 {
            format!("{}{:02}:{:02}.{:03}", sign, minutes, seconds, millis)
        } else {
            format!("{}{:02}.{:03}", sign, seconds, millis)
        }
    }
}

pub struct AppWrapper {
    pub app_state: Arc<Mutex<AppState>>,
    pub command_rx: Receiver<UICommand>,
}

impl AppWrapper {
    pub fn new(
        app_state: Arc<Mutex<AppState>>,
        command_rx: Receiver<UICommand>,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        // Asignar shader
        let gl = cc.gl.as_ref().unwrap().clone();
        let shader_path = config_base_dir()
            .join("shaders")
            .join(&app_state.lock().unwrap().layout.colors.shader_path)
            .to_string_lossy()
            .to_string();
        let vertex_shader_path = config_base_dir()
            .join("shaders")
            .join(format!(
                "{}.vert",
                app_state.lock().unwrap().layout.colors.shader_path
            ))
            .to_string_lossy()
            .to_string();

        {
            let mut state = app_state.lock().unwrap();
            state.gl = Some(gl.clone());
            state.shader = ShaderBackground::new(gl.clone(), shader_path, vertex_shader_path);
        }

        Self {
            app_state,
            command_rx,
        }
    }
}
