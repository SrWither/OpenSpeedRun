use crate::{app::state::AppState, config::layout::LayoutConfig};
use chrono::Duration;
use eframe::egui::{self, Color32, RichText};

impl AppState {
    pub fn draw_footer(&self, ctx: &egui::Context) {
        let LayoutConfig {
            font_sizes,
            colors,
            spacings: _,
            options,
        } = self.layout.clone();

        let bg_color = Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK);

        let info_color = Color32::from_hex(&colors.info).unwrap_or(Color32::WHITE);

        let gold_positive_color = Color32::from_hex(&colors.gold_positive).unwrap_or(Color32::GOLD);
        let gold_negative_color = Color32::from_hex(&colors.gold_negative).unwrap_or(Color32::RED);
        let pb_positive_color = Color32::from_hex(&colors.pb_positive).unwrap_or(Color32::GREEN);
        let pb_negative_color = Color32::from_hex(&colors.pb_negative).unwrap_or(Color32::RED);

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

        let previous_split_relative = if self.current_split > 0 {
            let current = self
                .splits_display
                .get(self.current_split - 1)
                .and_then(|s| s.last_time)
                .unwrap_or(Duration::zero());

            let previous = self
                .splits_display
                .get(self.current_split - 2)
                .and_then(|s| s.last_time)
                .unwrap_or(Duration::zero());

            current - previous
        } else {
            Duration::zero()
        };

        let previous_segment_pb = self
            .splits_display
            .get(self.current_split.saturating_sub(1))
            .and_then(|s| s.pb_time)
            .unwrap_or(Duration::zero());

        let previous_segment_gold = self
            .splits_display
            .get(self.current_split.saturating_sub(1))
            .and_then(|s| s.gold_time)
            .unwrap_or(Duration::zero());

        let delta_vs_pb = previous_split_relative - previous_segment_pb;
        let delta_vs_gold = previous_split_relative - previous_segment_gold;

        let format_dur = |dur: Duration| -> String {
            self.format_duration(dur, 0) // no signos
        };
        let format_diff = |dur: Duration| -> String {
            self.format_duration(dur, 2) // signo + y -
        };

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

                if options.show_info {
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!(
                                    "{} Sum of Best: {}",
                                    egui_phosphor::regular::FLAG_CHECKERED,
                                    format_dur(sum_of_bests)
                                ))
                                .color(info_color)
                                .size(font_sizes.info),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!(
                                    "{} Best Possible: {}",
                                    egui_phosphor::regular::GAUGE,
                                    format_dur(best_possible_time)
                                ))
                                .color(info_color)
                                .size(font_sizes.info),
                            );
                        });

                        if let Some(pb) = pb_time {
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} PB: {}",
                                        egui_phosphor::regular::CHART_POLAR,
                                        format_dur(pb)
                                    ))
                                    .color(info_color)
                                    .size(font_sizes.info),
                                );
                            });
                        }

                        let (delta_text, delta_icon, delta_value, delta_color) =
                            if self.run.gold_split {
                                (
                                    "Prev Gold Segment",
                                    egui_phosphor::regular::STAR,
                                    delta_vs_gold,
                                    if delta_vs_gold < Duration::zero() {
                                        gold_positive_color
                                    } else {
                                        gold_negative_color
                                    },
                                )
                            } else {
                                (
                                    "Prev PB Segment",
                                    egui_phosphor::regular::ARROW_LINE_UP,
                                    delta_vs_pb,
                                    if delta_vs_pb < Duration::zero() {
                                        pb_positive_color
                                    } else {
                                        pb_negative_color
                                    },
                                )
                            };

                        if self.current_split > 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}: {}",
                                        delta_icon,
                                        delta_text,
                                        format_diff(delta_value)
                                    ))
                                    .color(delta_color)
                                    .size(font_sizes.info),
                                );
                            });
                        }

                        ui.add_space(4.0);
                    });
                }
            });
    }
}
