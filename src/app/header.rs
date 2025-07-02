use eframe::egui::{self, RichText, Color32};
use chrono::Duration;
use crate::{app::state::AppState, config::layout::LayoutConfig};

impl AppState {
    pub fn draw_header(&self, ctx: &egui::Context) {
        let LayoutConfig {
            background_color,
            text_color,
            font_size,
            show_title,
            show_category,
            show_splits: _,
            titlebar: _,
            window_size: _,
        } = self.layout.clone();

        let bg_color = Color32::from_hex(&background_color).unwrap_or(Color32::BLACK);
        let text_color_parsed = Color32::from_hex(&text_color).unwrap_or(Color32::WHITE);

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
    }
}
