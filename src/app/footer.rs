use crate::core::split::{COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST};
use crate::{app::state::AppState, config::layout::LayoutConfig};
use chrono::Duration;
use eframe::egui::{self, Color32, RichText};

impl AppState {
    pub fn draw_footer(&mut self, ui: &mut egui::Ui) {
        let LayoutConfig {
            font_sizes,
            colors,
            spacings: _,
            options,
            #[cfg(windows)]
                hotkeys: _,
        } = self.layout.clone();

        let bg_color = if options.enable_shader || options.enable_background_image {
            Color32::TRANSPARENT
        } else {
            Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK)
        };

        let info_color = Color32::from_hex(&colors.info).unwrap_or(Color32::WHITE);

        let gold_positive_color = Color32::from_hex(&colors.gold_positive).unwrap_or(Color32::GOLD);
        let gold_negative_color = Color32::from_hex(&colors.gold_negative).unwrap_or(Color32::RED);
        let pb_positive_color = Color32::from_hex(&colors.pb_positive).unwrap_or(Color32::GREEN);
        let pb_negative_color = Color32::from_hex(&colors.pb_negative).unwrap_or(Color32::RED);

        let method = self.run.timing_method;
        let selected_comparison = self.run.selected_comparison.clone();

        let sum_of_bests = self
            .run
            .splits
            .iter()
            .filter_map(|s| s.comparison_time(COMPARISON_BEST_SEGMENTS, method))
            .fold(Duration::zero(), |acc, d| acc + d);

        let current_time = if self.current_split > 0 {
            self.splits_display
                .get(self.current_split - 1)
                .and_then(|s| s.last_time_for(method))
                .unwrap_or(Duration::zero())
        } else {
            Duration::zero()
        };

        let remaining_best: Duration = self
            .run
            .splits
            .iter()
            .skip(self.current_split)
            .filter_map(|s| s.comparison_time(COMPARISON_BEST_SEGMENTS, method))
            .sum();

        let best_possible_time = current_time + remaining_best;

        let pb_time: Option<Duration> = self
            .run
            .splits
            .iter()
            .map(|s| s.comparison_time(COMPARISON_PERSONAL_BEST, method))
            .collect::<Option<Vec<_>>>()
            .map(|times| times.into_iter().sum());

        let previous_split_relative = if self.current_split == 1 {
            self.splits_display
                .first()
                .and_then(|s| s.last_time_for(method))
                .unwrap_or(Duration::zero())
        } else if self.current_split > 1 {
            let current = self
                .splits_display
                .get(self.current_split - 1)
                .and_then(|s| s.last_time_for(method))
                .unwrap_or(Duration::zero());

            let previous = self
                .splits_display
                .get(self.current_split - 2)
                .and_then(|s| s.last_time_for(method))
                .unwrap_or(Duration::zero());

            current - previous
        } else {
            Duration::zero()
        };

        let previous_segment_comparison = self
            .splits_display
            .get(self.current_split.saturating_sub(1))
            .and_then(|s| s.comparison_time(&selected_comparison, method))
            .unwrap_or(Duration::zero());

        let delta_vs_selected = previous_split_relative - previous_segment_comparison;
        // "Best Segments" gets the gold color scheme, everything else (PB,
        // Average, Median, custom) gets the PB one.
        let is_gold_style = selected_comparison == COMPARISON_BEST_SEGMENTS;

        let format_dur = |dur: Duration| -> String {
            self.format_duration(dur, 0) // no signos
        };
        let format_diff = |dur: Duration| -> String {
            self.format_duration(dur, 2) // signo + y -
        };

        let mut comparison_clicked = false;

        egui::Panel::bottom("footer")
            .resizable(false)
            .min_size(64.0)
            .frame(egui::Frame {
                fill: bg_color,
                stroke: egui::Stroke::NONE,
                ..Default::default()
            })
            .show_inside(ui, |ui| {
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
                                    "{} Attempts: {}",
                                    egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
                                    self.run.attempts
                                ))
                                .color(info_color)
                                .size(font_sizes.info),
                            );
                        });

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

                        let (delta_icon, delta_color) = if is_gold_style {
                            (
                                egui_phosphor::regular::STAR,
                                if delta_vs_selected < Duration::zero() {
                                    gold_positive_color
                                } else {
                                    gold_negative_color
                                },
                            )
                        } else {
                            (
                                egui_phosphor::regular::ARROW_LINE_UP,
                                if delta_vs_selected < Duration::zero() {
                                    pb_positive_color
                                } else {
                                    pb_negative_color
                                },
                            )
                        };

                        if self.current_split > 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                let response = ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(format!(
                                                "{} Prev {} Segment: {}",
                                                delta_icon,
                                                selected_comparison,
                                                format_diff(delta_vs_selected)
                                            ))
                                            .color(delta_color)
                                            .size(font_sizes.info),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text("Click to switch comparison (or press C)");

                                if response.clicked() {
                                    comparison_clicked = true;
                                }
                            });
                        }

                        ui.add_space(4.0);
                    });
                }
            });

        if comparison_clicked {
            self.cycle_comparison();
        }
    }
}
