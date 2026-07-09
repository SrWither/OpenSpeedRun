use crate::core::split::{COMPARISON_BEST_SEGMENTS, TimingMethod};
use crate::{app::state::AppState, config::layout::LayoutConfig, core::timer::TimerState};
use chrono::Duration;
use eframe::egui::{self, Color32, RichText};

impl AppState {
    pub fn draw_splits_panel(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let LayoutConfig {
            colors,
            font_sizes,
            spacings,
            options,
            #[cfg(windows)]
                hotkeys: _,
        } = self.layout.clone();

        // set colors
        let bg_color = if options.enable_shader || options.enable_background_image {
            Color32::TRANSPARENT
        } else {
            Color32::from_hex(&colors.background).unwrap_or(Color32::BLACK)
        };
        let split_color = Color32::from_hex(&colors.split).unwrap_or(Color32::WHITE);
        let split_selected_color =
            Color32::from_hex(&colors.split_selected).unwrap_or(Color32::YELLOW);
        let split_timer_color =
            Color32::from_hex(&colors.split_timer).unwrap_or(Color32::from_rgb(0, 0, 255));
        let gold_positive_color = Color32::from_hex(&colors.gold_positive).unwrap_or(Color32::GOLD);
        let gold_negative_color = Color32::from_hex(&colors.gold_negative).unwrap_or(Color32::RED);
        let pb_positive_color = Color32::from_hex(&colors.pb_positive).unwrap_or(Color32::GREEN);
        let pb_negative_color = Color32::from_hex(&colors.pb_negative).unwrap_or(Color32::RED);

        let method = self.run.timing_method;
        let selected_comparison = self.run.selected_comparison.clone();
        let is_gold_style = selected_comparison == COMPARISON_BEST_SEGMENTS;

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(bg_color))
            .show_inside(ui, |ui| {
                if options.show_splits {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let total_splits = self.run.splits.len();
                        let page_start = self.current_page * self.splits_per_page;
                        let page_end = (page_start + self.splits_per_page).min(total_splits);
                        let splits = self.splits_display.clone();
                        let current_split = self.current_split;

                        for (i, split) in splits.iter().enumerate().take(page_end).skip(page_start)
                        {
                            let is_current = i == current_split;
                            let is_first = i == page_start;

                            // Determine if this is the first split on the current page
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

                            ui.add_space(spacings.split_top);

                            let row = ui.horizontal(|ui| {
                                ui.set_min_height(32.0);
                                ui.set_min_width(ui.available_width());
                                ui.add_space(10.0);

                                // Display the split icon if available
                                let texture = split
                                    .icon_path
                                    .as_ref()
                                    .and_then(|path| self.get_or_load_texture(&ctx, path));

                                if let Some(tex) = texture {
                                    ui.add(egui::Image::new(&tex).max_width(20.0));
                                }

                                let name_text = if is_current {
                                    RichText::new(format!("> {}", split.name))
                                        .color(split_selected_color)
                                        .strong()
                                        .size(font_sizes.split + 2.0)
                                } else {
                                    RichText::new(&split.name)
                                        .color(split_color)
                                        .size(font_sizes.split)
                                };

                                ui.label(name_text);

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if let Some(last) = split.last_time_for(method) {
                                            // Show total time
                                            let time_text = self.format_duration(last, 0);

                                            let prev = if i == 0 {
                                                Duration::zero()
                                            } else {
                                                self.splits_display
                                                    .get(i - 1)
                                                    .and_then(|s| s.last_time_for(method))
                                                    .unwrap_or(Duration::zero())
                                            };

                                            let relative = last - prev;

                                            let relative_time_text =
                                                self.format_duration(relative, 3);

                                            if options.show_last_relative_time {
                                                ui.label(
                                                    RichText::new(relative_time_text)
                                                        .size(font_sizes.split)
                                                        .color(split_timer_color),
                                                );
                                            } else {
                                                ui.label(
                                                    RichText::new(time_text)
                                                        .size(font_sizes.split)
                                                        .color(split_timer_color),
                                                );
                                            }

                                            if let Some(comparison) =
                                                split.comparison_time(&selected_comparison, method)
                                                && comparison.num_milliseconds() > 0
                                            {
                                                let diff = relative - comparison;
                                                if !is_gold_style || diff != Duration::zero() {
                                                    let diff_text = self.format_duration(diff, 2);

                                                    let diff_color = if is_gold_style {
                                                        if diff < Duration::zero() {
                                                            gold_positive_color
                                                        } else {
                                                            gold_negative_color
                                                        }
                                                    } else if diff < Duration::zero() {
                                                        pb_positive_color
                                                    } else {
                                                        pb_negative_color
                                                    };

                                                    ui.label(
                                                        RichText::new(diff_text)
                                                            .size(font_sizes.split_gold)
                                                            .color(diff_color),
                                                    );
                                                }
                                            }
                                            // Show relative time if applicable
                                        } else if is_current
                                            && options.show_relative_times
                                            && self.timer.state == TimerState::Running
                                            && self.timer.current_time() >= Duration::zero()
                                        {
                                            let start_of_split = if i == 0 {
                                                Duration::zero()
                                            } else {
                                                self.splits_display
                                                    .get(i - 1)
                                                    .and_then(|s| s.last_time_for(method))
                                                    .unwrap_or(Duration::zero())
                                            };
                                            let current_time = match method {
                                                TimingMethod::RealTime => self.timer.current_time(),
                                                TimingMethod::GameTime => {
                                                    self.igt_timer.current_time()
                                                }
                                            };
                                            let live_relative = current_time - start_of_split;

                                            let formatted = self.format_duration(live_relative, 0);
                                            ui.label(
                                                RichText::new(formatted)
                                                    .size(font_sizes.split_timer)
                                                    .color(split_timer_color),
                                            );

                                            let threshold = Duration::seconds(5);

                                            if let Some(comparison) =
                                                split.comparison_time(&selected_comparison, method)
                                                && comparison.num_milliseconds() > 0
                                            {
                                                let diff = live_relative - comparison;
                                                if diff >= -threshold {
                                                    let diff_text = self.format_duration(diff, 2);
                                                    let diff_color = if is_gold_style {
                                                        if diff < Duration::zero() {
                                                            gold_positive_color
                                                        } else {
                                                            gold_negative_color
                                                        }
                                                    } else if diff < Duration::zero() {
                                                        pb_positive_color
                                                    } else {
                                                        pb_negative_color
                                                    };
                                                    ui.label(
                                                        RichText::new(diff_text)
                                                            .size(font_sizes.split_gold)
                                                            .color(diff_color),
                                                    );
                                                }
                                            }
                                        } else {
                                            ui.label(
                                                RichText::new("--:--.---")
                                                    .size(font_sizes.split_timer)
                                                    .color(split_timer_color),
                                            );
                                        }
                                    },
                                );
                            });

                            if is_current {
                                row.response.scroll_to_me(Some(egui::Align::Center));
                            }

                            ui.add_space(spacings.split_bottom);

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
    }
}
