use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::config::load::{AppConfig, config_base_dir};
use crate::core::split::Run;
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
}

impl Default for AppState {
    fn default() -> Self {
        let app_config = AppConfig::load();
        let split_base_path = config_base_dir().join(&app_config.last_split_path);
        let run_path = split_base_path.join("split.json");

        let run = Run::load_from_file(run_path.to_str().unwrap()).unwrap_or_else(|_| {
            Run::new("Untitled", "Any%", &["Split 1", "Split 2", "Final Split"])
        });

        let layout_path = config_base_dir().join(&app_config.theme);
        let layout = LayoutConfig::load_or_default(layout_path.to_str().unwrap());

        Self {
            timer: Timer::new(),
            run,
            layout,
            current_split: 0,
            textures: HashMap::new(),
            split_base_path,
        }
    }
}

impl AppState {
    pub fn split(&mut self) {
        if self.timer.state == TimerState::NotStarted {
            let offset = self.run.start_offset.unwrap_or(0);
            self.timer.start_with_offset(offset);
            self.current_split = 0;
        } else if self.timer.state == TimerState::Running {
            let now = self.timer.current_time();

            if now < Duration::zero() {
                return;
            }

            if let Some(split) = self.run.splits.get_mut(self.current_split) {
                split.last_time = Some(now);
            }
            self.current_split += 1;
            if self.current_split >= self.run.splits.len() {
                self.timer.pause();
            }
        }
    }

    pub fn reset_splits(&mut self) {
        for split in &mut self.run.splits {
            split.last_time = None;
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
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Teclado
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

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_hex(&background_color).unwrap_or(Color32::BLACK)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Título y categoría
                    if show_title {
                        ui.label(
                            RichText::new(&self.run.title)
                                .color(Color32::from_hex(&text_color).unwrap_or(Color32::WHITE))
                                .size(font_size + 4.0),
                        );
                    }
                    if show_category {
                        ui.label(
                            RichText::new(&self.run.category)
                                .color(Color32::from_hex(&text_color).unwrap_or(Color32::WHITE))
                                .size(font_size.into()),
                        );
                    }

                    // Cronómetro grande
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
                            .size(font_size as f32 * 2.0)
                            .color(Color32::from_rgb(250, 200, 100))
                            .strong(),
                    );

                    // Tabla de splits
                    if show_splits {
                        ui.add_space(10.0);

                        let splits = self.run.splits.clone();
                        let current_split = self.current_split;

                        for (i, split) in splits.iter().enumerate() {
                            let is_current = i == current_split;

                            ui.add(egui::Separator::default().spacing(6.0));

                            let texture = split
                                .icon_path
                                .as_ref()
                                .and_then(|path| self.get_or_load_texture(ctx, path));

                            ui.horizontal(|ui| {
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
                                        .color(
                                            Color32::from_hex(&text_color)
                                                .unwrap_or(Color32::WHITE),
                                        )
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

                                            // Mostrar diferencia si hay PB
                                            if let Some(pb) = &split.pb_time {
                                                let diff = *last - *pb;
                                                let sign = if diff < chrono::Duration::zero() {
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

                                                let diff_color = if diff < chrono::Duration::zero()
                                                {
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

                                            // Mostrar el tiempo del split
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
                        }
                    }

                    ui.add_space(10.0);
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
                    });
                });
            });

        ctx.request_repaint(); // para animación
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
