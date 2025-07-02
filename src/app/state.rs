use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::config::load::{AppConfig, config_base_dir};
use crate::core::split::{Run, Split};
use crate::core::timer::Timer;
use crate::{config::layout::LayoutConfig, core::timer::TimerState};
use chrono::Duration;
use eframe::egui;
use egui::TextureHandle;

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
}

impl Default for AppState {
    fn default() -> Self {
        let app_config = AppConfig::load();
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

    pub fn save_pb(&mut self) -> std::io::Result<()> {
        if self.current_split == self.run.splits.len() {
            let path = self.split_base_path.join("split.json");
            return self.run.save_to_file(path.to_str().unwrap());
        }
        Ok(())
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
        self.save_pb().unwrap_or_else(|e| {
            eprintln!("Error saving PB after undo: {}", e);
        });
        self.reset_splits();
    }

    fn sync_splits(&mut self) {
        let run =
            Run::load_from_file(self.split_base_path.join("split.json").to_str().unwrap()).unwrap();
        self.run = run.clone();
        self.splits_display = run.splits.clone();
    }
}

pub struct AppWrapper {
    pub app_state: Arc<Mutex<AppState>>,
}