use std::sync::{Arc, Mutex};

use crate::core::split::Run;
use crate::core::timer::Timer;
use crate::{config::layout::LayoutConfig, core::timer::TimerState};
use chrono::Duration;
use eframe::egui;
use egui::{Color32, RichText};

pub struct AppState {
    pub timer: Timer,
    pub run: Run,
    pub layout: LayoutConfig,
    pub current_split: usize,
}

impl Default for AppState {
    fn default() -> Self {
        let run = Run::load_from_file("splits/example.json").unwrap_or_else(|_| {
            Run::new("Untitled", "Any%", &["Split 1", "Split 2", "Final Split"])
        });
        let layout = LayoutConfig::load_or_default("themes/default.json");
        Self {
            timer: Timer::new(),
            run,
            layout,
            current_split: 0,
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
            show_total_time,
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
                        egui::Grid::new("splits_grid").striped(true).show(ui, |ui| {
                            ui.label(
                                RichText::new("Split")
                                    .color(Color32::from_hex(&text_color).unwrap_or(Color32::WHITE))
                                    .strong()
                                    .size(font_size),
                            );
                            ui.label(
                                RichText::new("Time")
                                    .color(Color32::from_hex(&text_color).unwrap_or(Color32::WHITE))
                                    .strong()
                                    .size(font_size),
                            );
                            ui.end_row();

                            for (i, split) in self.run.splits.iter().enumerate() {
                                let is_current = i == self.current_split;

                                let name_text = if is_current {
                                    RichText::new(format!("> {}", split.name))
                                        .color(Color32::YELLOW)
                                        .strong()
                                        .size(font_size + 2.0)
                                } else {
                                    RichText::new(&split.name)
                                        .color(
                                            Color32::from_hex(&text_color)
                                                .unwrap_or(Color32::WHITE),
                                        )
                                        .strong()
                                        .size(font_size - 1.0)
                                };

                                let time_text = if let Some(dur) = &split.last_time {
                                    RichText::new(format!(
                                        "{:02}:{:02}.{:03}",
                                        dur.num_minutes(),
                                        dur.num_seconds() % 60,
                                        dur.num_milliseconds() % 1000
                                    ))
                                    .color(Color32::from_rgb(200, 230, 200))
                                    .size(font_size - 1.0)
                                } else {
                                    RichText::new("--:--.---")
                                        .color(Color32::from_rgb(120, 120, 120))
                                        .size(font_size - 1.0)
                                };

                                ui.label(name_text);
                                ui.label(time_text);
                                ui.end_row();
                            }
                        });
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
