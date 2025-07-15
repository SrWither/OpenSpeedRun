use crate::{app::state::AppState, config::layout::LayoutConfig, core::timer::TimerState};
use chrono::Duration;
use eframe::egui::{self, Color32, RichText};

impl AppState {
    pub fn draw_splits_panel(&mut self, ctx: &egui::Context) {
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

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(bg_color))
            .show(ctx, |ui| {
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

                            ui.horizontal(|ui| {
                                ui.set_min_height(32.0);
                                ui.set_min_width(ui.available_width());
                                ui.add_space(10.0);
                                
                                // Display the split icon if available
                                let texture = split
                                    .icon_path
                                    .as_ref()
                                    .and_then(|path| self.get_or_load_texture(ctx, path));

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
                                        if let Some(last) = &split.last_time {
                                            // Show total time
                                            let time_text = self.format_duration(*last, 0);

                                            ui.label(
                                                RichText::new(time_text)
                                                    .size(font_sizes.split)
                                                    .color(split_timer_color),
                                            );

                                            let prev = if i == 0 {
                                                Duration::zero()
                                            } else {
                                                self.splits_display
                                                    .get(i - 1)
                                                    .and_then(|s| s.last_time)
                                                    .unwrap_or(Duration::zero())
                                            };

                                            let relative = *last - prev;

                                            if self.run.gold_split {
                                                if let Some(gold) = &split.gold_time {
                                                    if gold.num_milliseconds() > 0 {
                                                        let diff = relative - *gold;
                                                        if diff != Duration::zero() {
                                                            let diff_text =
                                                                self.format_duration(diff, 2);

                                                            let diff_color =
                                                                if diff < Duration::zero() {
                                                                    gold_positive_color
                                                                } else {
                                                                    gold_negative_color
                                                                };

                                                            ui.label(
                                                                RichText::new(diff_text)
                                                                    .size(font_sizes.split_gold)
                                                                    .color(diff_color),
                                                            );
                                                        }
                                                    }
                                                }
                                            } else {
                                                if let Some(pb) = &split.pb_time {
                                                    if pb.num_milliseconds() > 0 {
                                                        let diff = relative - *pb;
                                                        let diff_text =
                                                            self.format_duration(diff, 2);

                                                        let diff_color = if diff < Duration::zero()
                                                        {
                                                            pb_positive_color
                                                        } else {
                                                            pb_negative_color
                                                        };

                                                        ui.label(
                                                            RichText::new(diff_text)
                                                                .size(font_sizes.split_pb)
                                                                .color(diff_color),
                                                        );
                                                    }
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
                                                    .and_then(|s| s.last_time)
                                                    .unwrap_or(Duration::zero())
                                            };
                                            let current_time = self.timer.current_time();
                                            let relative = current_time - start_of_split;

                                            let formatted = self.format_duration(relative, 0);
                                            ui.label(
                                                RichText::new(formatted)
                                                    .size(font_sizes.split_timer)
                                                    .color(split_timer_color),
                                            );

                                            let live_relative = relative;

                                            let threshold = Duration::seconds(5);

                                            if self.run.gold_split {
                                                if let Some(gold) = &split.gold_time {
                                                    if gold.num_milliseconds() > 0 {
                                                        let diff = live_relative - *gold;
                                                        if diff >= -threshold {
                                                            let diff_text =
                                                                self.format_duration(diff, 2);
                                                            let diff_color =
                                                                if diff < Duration::zero() {
                                                                    gold_positive_color
                                                                } else {
                                                                    gold_negative_color
                                                                };
                                                            ui.label(
                                                                RichText::new(diff_text)
                                                                    .size(font_sizes.split_gold)
                                                                    .color(diff_color),
                                                            );
                                                        }
                                                    }
                                                }
                                            } else {
                                                if let Some(pb) = &split.pb_time {
                                                    if pb.num_milliseconds() > 0 {
                                                        let diff = live_relative - *pb;
                                                        if diff >= -threshold {
                                                            let diff_text =
                                                                self.format_duration(diff, 2);
                                                            let diff_color =
                                                                if diff < Duration::zero() {
                                                                    pb_positive_color
                                                                } else {
                                                                    pb_negative_color
                                                                };
                                                            ui.label(
                                                                RichText::new(diff_text)
                                                                    .size(font_sizes.split_pb)
                                                                    .color(diff_color),
                                                            );
                                                        }
                                                    }
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
