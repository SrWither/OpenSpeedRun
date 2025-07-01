use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::config::load::{AppConfig, config_base_dir};
use crate::core::split::{Run, Split};
use crate::core::timer::Timer;
use crate::{config::layout::LayoutConfig, core::timer::TimerState};
use chrono::Duration;
use eframe::egui;
use egui::{Color32, ColorImage, RichText, TextureHandle};
use image::GenericImageView;

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
        if !(self.current_split == self.run.splits.len()) {
            return;
        }

        self.run.splits = self.splits_display.clone();

        if self.run.gold_split {
            for split in self.run.splits.iter_mut() {
                if let Some(last) = split.last_time {
                    match split.pb_time {
                        Some(pb) if last < pb => {
                            split.pb_time = Some(last);
                        }
                        None => {
                            split.pb_time = Some(last);
                        }
                        _ => {}
                    }
                }
                split.last_time = None;
            }
        } else {
            let mut is_better = true;

            for split in &self.run.splits {
                match (split.last_time, split.pb_time) {
                    (Some(last), Some(pb)) if last > pb => {
                        is_better = false;
                        break;
                    }
                    (None, _) => {
                        is_better = false;
                        break;
                    }
                    _ => {}
                }
            }

            if is_better {
                for split in self.run.splits.iter_mut() {
                    split.pb_time = split.last_time;
                    split.last_time = None;
                }
            }
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
    }

    fn get_or_load_texture(&mut self, ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
        let full_path = self.split_base_path.join(path);

        let cache_key = full_path.to_string_lossy().to_string();

        if let Some(tex) = self.textures.get(&cache_key) {
            return Some(tex.clone());
        }

        if let Ok(img) = image::open(&full_path) {
            let size = img.dimensions();
            let rgba = img.to_rgba8().into_raw();
            let color_image =
                ColorImage::from_rgba_unmultiplied([size.0 as usize, size.1 as usize], &rgba);
            let texture = ctx.load_texture(cache_key.clone(), color_image, Default::default());
            self.textures.insert(cache_key, texture.clone());
            Some(texture)
        } else {
            None
        }
    }

    fn save_pb(&mut self) -> std::io::Result<()> {
        if self.current_split == self.run.splits.len() {
            let path = self.split_base_path.join("split.json");
            return self.run.save_to_file(path.to_str().unwrap());
        }
        Ok(())
    }

    fn undo_pb(&mut self) {
        self.run.splits = self.splits_backup.clone();
        self.save_pb().unwrap_or_else(|e| {
            eprintln!("Error saving PB after undo: {}", e);
        });
        self.reset_splits();
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.split();
        }

        let LayoutConfig {
            background_color,
            text_color,
            font_size,
            show_title,
            show_category,
            show_splits,
            show_total_time: _,
        } = self.layout.clone();

        let bg_color = Color32::from_hex(&background_color).unwrap_or(Color32::BLACK);
        let text_color_parsed = Color32::from_hex(&text_color).unwrap_or(Color32::WHITE);

        // === HEADER ===
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::default().fill(bg_color))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    if show_title {
                        ui.label(
                            RichText::new(&self.run.title)
                                .color(text_color_parsed)
                                .size(font_size + 4.0),
                        );
                    }
                    if show_category {
                        ui.label(
                            RichText::new(&self.run.category)
                                .color(text_color_parsed)
                                .size(font_size),
                        );
                    }

                    let elapsed = self.timer.current_time();
                    let sign = if elapsed < Duration::zero() { "-" } else { "" };
                    let elapsed_abs = elapsed.abs();
                    let time_str = format!(
                        "{}{:02}:{:02}.{:03}",
                        sign,
                        elapsed_abs.num_minutes(),
                        elapsed_abs.num_seconds() % 60,
                        elapsed_abs.num_milliseconds() % 1000
                    );

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(time_str)
                            .size(font_size * 2.0)
                            .color(Color32::from_rgb(250, 200, 100))
                            .strong(),
                    );
                    ui.add_space(10.0);
                });
            });

        // === FOOTER ===
        egui::TopBottomPanel::bottom("footer")
            .resizable(false)
            .min_height(60.0)
            .frame(egui::Frame {
                fill: bg_color,
                stroke: egui::Stroke::NONE,
                ..Default::default()
            })
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let top = rect.top();
                let left = rect.left();
                let right = rect.right();
                let stroke = egui::Stroke::new(1.0, Color32::from_gray(100));
                ui.painter()
                    .line_segment([egui::pos2(left, top), egui::pos2(right, top)], stroke);

                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    if show_splits {
                        let total_splits = self.run.splits.len();
                        let total_pages =
                            (total_splits + self.splits_per_page - 1) / self.splits_per_page;

                        ui.horizontal(|ui| {
                            if ui.button("⬅").clicked() && self.current_page > 0 {
                                self.current_page -= 1;
                            }

                            ui.label(format!("Page {}/{}", self.current_page + 1, total_pages));

                            if ui.button("➡").clicked() && self.current_page + 1 < total_pages {
                                self.current_page += 1;
                            }
                        });
                    }

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if ui.button("Start").clicked() {
                            let offset = self.run.start_offset.unwrap_or(0);
                            self.timer.start_with_offset(offset);
                        }
                        if ui.button("Pause").clicked() {
                            self.timer.pause();
                        }
                        if ui.button("Reset").clicked() {
                            self.timer.reset();
                            self.reset_splits();
                        }
                        if ui.button("Split").clicked() {
                            self.split();
                        }
                        if ui.button("Save PB").clicked() {
                            if let Err(e) = self.save_pb() {
                                eprintln!("Error saving PB: {}", e);
                            }
                        }
                        if ui.button("Undo PB").clicked() {
                            self.undo_pb();
                        }
                    });
                });
            });

        // === CENTRAL PANEL CON SCROLL ===
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(bg_color))
            .show(ctx, |ui| {
                if show_splits {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let total_splits = self.run.splits.len();
                        let page_start = self.current_page * self.splits_per_page;
                        let page_end = (page_start + self.splits_per_page).min(total_splits);
                        let splits = self.splits_display.clone();
                        let current_split = self.current_split;

                        for (i, split) in splits.iter().enumerate().take(page_end).skip(page_start) {
                            let is_current = i == current_split;
                            let is_first = i == page_start;

                            if is_first {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 1.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().line_segment(
                                    [rect.left_top(), rect.right_top()],
                                    egui::Stroke::new(1.0, Color32::from_gray(100)),
                                );
                            }

                            ui.add_space(6.0);

                            ui.horizontal(|ui| {
                                ui.set_min_height(32.0);
                                ui.set_min_width(ui.available_width());
                                ui.add_space(10.0);

                                let texture = split
                                    .icon_path
                                    .as_ref()
                                    .and_then(|path| self.get_or_load_texture(ctx, path));

                                if let Some(tex) = texture {
                                    ui.add(egui::Image::new(&tex).max_width(20.0));
                                }

                                let name_text = if is_current {
                                    RichText::new(format!("> {}", split.name))
                                        .color(Color32::YELLOW)
                                        .strong()
                                        .size(font_size - 6.0)
                                } else {
                                    RichText::new(&split.name)
                                        .color(text_color_parsed)
                                        .size(font_size - 8.0)
                                };

                                ui.label(name_text);

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if let Some(last) = &split.last_time {
                                            let time_text = format!(
                                                "{:02}:{:02}.{:03}",
                                                last.num_minutes(),
                                                last.num_seconds() % 60,
                                                last.num_milliseconds() % 1000
                                            );

                                            if let Some(pb) = &split.pb_time {
                                                if pb.num_milliseconds() > 0 {
                                                    let diff = *last - *pb;
                                                    let sign = if diff < Duration::zero() {
                                                        "-"
                                                    } else {
                                                        "+"
                                                    };
                                                    let diff_abs = diff.num_milliseconds().abs();
                                                    let diff_secs = diff_abs / 1000;
                                                    let diff_millis = diff_abs % 1000;

                                                    let diff_text = format!(
                                                        "{}{:02}.{:03}",
                                                        sign, diff_secs, diff_millis
                                                    );

                                                    let diff_color = if diff < Duration::zero() {
                                                        Color32::GREEN
                                                    } else {
                                                        Color32::RED
                                                    };

                                                    ui.label(
                                                        RichText::new(diff_text)
                                                            .size(font_size - 10.0)
                                                            .color(diff_color),
                                                    );
                                                }
                                            }

                                            ui.label(
                                                RichText::new(time_text)
                                                    .size(font_size - 2.0)
                                                    .color(Color32::from_rgb(200, 230, 200)),
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new("--:--.---")
                                                    .size(font_size - 2.0)
                                                    .color(Color32::GRAY),
                                            );
                                        }
                                    },
                                );
                            });

                            ui.add_space(6.0);

                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 1.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().line_segment(
                                [rect.left_bottom(), rect.right_bottom()],
                                egui::Stroke::new(1.0, Color32::from_gray(100)),
                            );
                        }
                    });
                }
            });

        ctx.request_repaint();
    }
}

pub struct AppWrapper {
    pub app_state: Arc<Mutex<AppState>>,
}

impl eframe::App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut app = self.app_state.lock().unwrap();
        app.update(ctx, frame);
    }
}
