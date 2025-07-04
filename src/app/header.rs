use crate::{app::state::AppState, config::layout::LayoutConfig};
use eframe::egui::{self, Color32, RichText};

impl AppState {
    pub fn draw_header(&self, ctx: &egui::Context) {
        let LayoutConfig {
            font_sizes,
            colors,
            spacings: _,
            options,
        } = self.layout.clone();

        let bg_color = Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK);
        let title_color = Color32::from_hex(&colors.title).unwrap_or(Color32::WHITE);
        let category_color = Color32::from_hex(&colors.category).unwrap_or(Color32::WHITE);
        let timer_color = Color32::from_hex(&colors.timer).unwrap_or(Color32::RED);

        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::default().fill(bg_color))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    if options.show_title {
                        ui.label(
                            RichText::new(&self.run.title)
                                .color(title_color)
                                .size(font_sizes.title),
                        );
                    }
                    if options.show_category {
                        ui.label(
                            RichText::new(&self.run.category)
                                .color(category_color)
                                .size(font_sizes.category),
                        );
                    }

                    let elapsed = self.timer.current_time();
                    let time_str = self.format_duration(elapsed, 1);

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(time_str)
                            .size(font_sizes.timer)
                            .color(timer_color)
                            .strong(),
                    );
                    ui.add_space(10.0);
                });
            });
    }
}
