use crate::{app::state::AppState, config::layout::LayoutConfig};
use chrono::Duration;
use eframe::egui::{self, Color32, RichText};

impl AppState {
    pub fn draw_footer(&self, ctx: &egui::Context) {
        let LayoutConfig {
            font_sizes,
            colors,
            show_title: _,
            show_category: _,
            show_splits: _,
            titlebar: _,
            window_size: _,
        } = self.layout.clone();

        let bg_color = Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK);

        let info_color = Color32::from_hex(&colors.info).unwrap_or(Color32::WHITE);

        let sum_of_bests = self
            .splits_backup
            .iter()
            .filter_map(|s| s.gold_time)
            .fold(Duration::zero(), |acc, d| acc + d);

        let current_time = if self.current_split > 0 {
            self.splits_display
                .get(self.current_split - 1)
                .and_then(|s| s.last_time)
                .unwrap_or(Duration::zero())
        } else {
            Duration::zero()
        };

        let remaining_gold: Duration = self
            .run
            .splits
            .iter()
            .skip(self.current_split)
            .filter_map(|s| s.gold_time)
            .sum();

        let best_possible_time = current_time + remaining_gold;

        let pb_time: Option<Duration> = self
            .run
            .splits
            .iter()
            .map(|s| s.pb_time)
            .collect::<Option<Vec<_>>>()
            .map(|times| times.into_iter().sum());

        egui::TopBottomPanel::bottom("footer")
            .resizable(false)
            .min_height(64.0)
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

                let format_dur = |dur: Duration| -> String {
                    format!(
                        "{:02}:{:02}.{:03}",
                        dur.num_minutes(),
                        dur.num_seconds() % 60,
                        dur.num_milliseconds() % 1000
                    )
                };
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} Sum of Best: {}",
                            egui_phosphor::regular::FLAG_CHECKERED,
                            format_dur(sum_of_bests)
                        ))
                        .color(info_color)
                        .size(font_sizes.info),
                    );

                    ui.label(
                        RichText::new(format!(
                            "{} Best Possible: {}",
                            egui_phosphor::regular::GAUGE,
                            format_dur(best_possible_time)
                        ))
                        .color(info_color)
                        .size(font_sizes.info),
                    );

                    if let Some(pb) = pb_time {
                        ui.label(
                            RichText::new(format!(
                                "{} PB: {}",
                                egui_phosphor::regular::CHART_POLAR,
                                format_dur(pb)
                            ))
                            .color(info_color)
                            .size(font_sizes.info),
                        );
                    }
                });
            });
    }
}
