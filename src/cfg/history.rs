use std::path::PathBuf;

use openspeedrun::Run;
use openspeedrun::core::split::{
    COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST, ComparisonTime, TimingMethod,
};

use crate::send_message;
use crate::style;

#[derive(PartialEq)]
enum Tab {
    Attempts,
    PbHistory,
    SplitHistory,
}

pub struct History {
    pub run_path: PathBuf,
    pub run: Run,
    active_tab: Tab,
    confirm_clear: bool,
}

fn format_duration(duration: chrono::Duration) -> String {
    let total_millis = duration.num_milliseconds();
    let hours = total_millis / 3_600_000;
    let minutes = (total_millis % 3_600_000) / 60_000;
    let seconds = (total_millis % 60_000) / 1_000;
    let millis = total_millis % 1_000;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    } else {
        format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
    }
}

impl History {
    pub fn new(run_path: PathBuf) -> Self {
        let run = Run::load_from_file(run_path.to_str().unwrap())
            .unwrap_or_else(|_| Run::new("New Run", "Category", &["Split 1", "Split 2"]));
        Self {
            run_path,
            run,
            active_tab: Tab::Attempts,
            confirm_clear: false,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        use egui::{Grid, RichText, ScrollArea};

        ui.heading(format!("{} [{}]", self.run.title, self.run.category));
        ui.separator();

        // Selector de pestañas
        ui.horizontal(|ui| {
            for (label, tab) in [
                ("Attempts", Tab::Attempts),
                ("PB History", Tab::PbHistory),
                ("Split History", Tab::SplitHistory),
            ] {
                if ui.selectable_label(self.active_tab == tab, label).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        ui.horizontal(|ui| {
            let clear_button =
                egui::Button::new(format!("{} Clear History", egui_phosphor::regular::TRASH))
                    .fill(egui::Color32::from_rgb(50, 22, 22))
                    .stroke(egui::Stroke::new(1.0_f32, style::ERROR));
            if ui.add(clear_button).clicked() {
                self.confirm_clear = true;
            }
        });

        if self.confirm_clear {
            egui::Window::new("Confirm Clear History")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        "Are you sure you want to clear all attempt and PB history? \
                        This also resets each split's Personal Best and Best Segment times.",
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_clear = false;
                        }
                        if ui
                            .button(RichText::new("Yes, clear").color(style::ERROR))
                            .clicked()
                        {
                            self.run.attempt_history.clear();
                            self.run.pb_history.clear();
                            self.run.attempts = 0;
                            for split in &mut self.run.splits {
                                split.segment_history.clear();
                                split.last_time = None;
                                split.last_time_game = None;
                                split.comparisons.insert(
                                    COMPARISON_PERSONAL_BEST.to_string(),
                                    ComparisonTime::default(),
                                );
                                split.comparisons.insert(
                                    COMPARISON_BEST_SEGMENTS.to_string(),
                                    ComparisonTime::default(),
                                );
                            }

                            self.persist_and_reload();
                            self.confirm_clear = false;
                        }
                    });
                });
        }

        ui.separator();

        match self.active_tab {
            Tab::Attempts => {
                if self.run.attempt_history.is_empty() {
                    ui.label("No attempts recorded yet.");
                    return;
                }
                let mut delete_index: Option<u32> = None;
                style::section_card(ui, "Attempts", egui_phosphor::regular::LIST, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        Grid::new("attempt_history_grid")
                            .striped(true)
                            .min_col_width(100.0)
                            .show(ui, |ui| {
                                ui.label("Attempt #");
                                ui.label("Date");
                                ui.label("Real Time");
                                ui.label("Game Time");
                                ui.label("Ended");
                                ui.label("Is PB");
                                ui.label("");
                                ui.end_row();

                                for attempt in &self.run.attempt_history {
                                    ui.label(attempt.run_index.to_string());
                                    let date_str = attempt
                                        .date
                                        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(date_str);

                                    let real_time_str = attempt
                                        .real_time
                                        .map(format_duration)
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(real_time_str);

                                    let game_time_str = attempt
                                        .game_time
                                        .map(format_duration)
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(game_time_str);

                                    ui.label(if attempt.ended { "Yes" } else { "No" });

                                    let is_pb = self
                                        .run
                                        .pb_history
                                        .iter()
                                        .any(|pb| pb.run_index == attempt.run_index);
                                    let pb_text = if is_pb {
                                        RichText::new("Yes").color(style::SUCCESS)
                                    } else {
                                        RichText::new("No")
                                    };
                                    ui.label(pb_text);

                                    if ui
                                        .small_button(
                                            RichText::new(egui_phosphor::regular::TRASH)
                                                .color(style::ERROR),
                                        )
                                        .on_hover_text(
                                            "Delete this attempt (and its segments) everywhere",
                                        )
                                        .clicked()
                                    {
                                        delete_index = Some(attempt.run_index);
                                    }

                                    ui.end_row();
                                }
                            });
                    });
                });

                if let Some(run_index) = delete_index {
                    self.delete_attempt(run_index);
                }
            }

            Tab::PbHistory => {
                if self.run.pb_history.is_empty() {
                    ui.label("No PB history available.");
                    return;
                }
                let mut delete_index: Option<u32> = None;
                style::section_card(ui, "PB History", egui_phosphor::regular::TROPHY, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        Grid::new("pb_history_grid")
                            .striped(true)
                            .min_col_width(100.0)
                            .show(ui, |ui| {
                                ui.label("PB Attempt #");
                                ui.label("Date");
                                ui.label("Real Time");
                                ui.label("Game Time");
                                ui.label("Ended");
                                ui.label("");
                                ui.end_row();

                                for pb in &self.run.pb_history {
                                    ui.label(pb.run_index.to_string());

                                    let date_str = pb
                                        .date
                                        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(date_str);

                                    ui.label(
                                        pb.real_time
                                            .map(format_duration)
                                            .unwrap_or_else(|| "-".to_string()),
                                    );
                                    ui.label(
                                        pb.game_time
                                            .map(format_duration)
                                            .unwrap_or_else(|| "-".to_string()),
                                    );

                                    ui.label(if pb.ended { "Yes" } else { "No" });

                                    if ui
                                        .small_button(
                                            RichText::new(egui_phosphor::regular::TRASH)
                                                .color(style::ERROR),
                                        )
                                        .on_hover_text(
                                            "Remove this entry from the PB log (keeps the attempt and its segments)",
                                        )
                                        .clicked()
                                    {
                                        delete_index = Some(pb.run_index);
                                    }

                                    ui.end_row();
                                }
                            });
                    });
                });

                if let Some(run_index) = delete_index {
                    self.delete_pb_entry(run_index);
                }
            }

            Tab::SplitHistory => {
                if self.run.splits.is_empty() {
                    ui.label("No splits available.");
                    return;
                }
                let mut delete_target: Option<(usize, u32)> = None;
                ScrollArea::vertical().show(ui, |ui| {
                    for (i, split) in self.run.splits.iter().enumerate() {
                        style::section_card(
                            ui,
                            &format!("Split #{}: {}", i + 1, split.name),
                            egui_phosphor::regular::FLAG_CHECKERED,
                            |ui| {
                                ui.horizontal(|ui| {
                                    let pb = split
                                        .comparison_time("Personal Best", TimingMethod::RealTime)
                                        .map(format_duration)
                                        .unwrap_or_else(|| "-".to_string());
                                    let best = split
                                        .comparison_time("Best Segments", TimingMethod::RealTime)
                                        .map(format_duration)
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(format!("Personal Best: {pb}"));
                                    ui.label(format!("Best Segments: {best}"));
                                });

                                Grid::new(format!("split_{}_history", i))
                                    .striped(true)
                                    .min_col_width(100.0)
                                    .show(ui, |ui| {
                                        ui.label("Run #");
                                        ui.label("Real Time");
                                        ui.label("Game Time");
                                        ui.label("");
                                        ui.end_row();

                                        for entry in &split.segment_history {
                                            ui.label(entry.run_index.to_string());
                                            ui.label(
                                                entry
                                                    .real_time
                                                    .map(format_duration)
                                                    .unwrap_or_else(|| "-".to_string()),
                                            );
                                            ui.label(
                                                entry
                                                    .game_time
                                                    .map(format_duration)
                                                    .unwrap_or_else(|| "-".to_string()),
                                            );

                                            if ui
                                                .small_button(
                                                    RichText::new(egui_phosphor::regular::TRASH)
                                                        .color(style::ERROR),
                                                )
                                                .on_hover_text(
                                                    "Delete this split's time for this run only",
                                                )
                                                .clicked()
                                            {
                                                delete_target = Some((i, entry.run_index));
                                            }

                                            ui.end_row();
                                        }
                                    });
                            },
                        );
                        ui.add_space(style::SPACE_SM);
                    }
                });

                if let Some((split_index, run_index)) = delete_target {
                    self.delete_segment_entry(split_index, run_index);
                }
            }
        }
    }

    fn persist_and_reload(&mut self) {
        let _ = self.run.save_to_file(self.run_path.to_str().unwrap());
        send_message("reloadrun");
    }

    /// Deletes a single attempt everywhere it's referenced (attempt log, PB
    /// log, every split's segment history), then recomputes Best
    /// Segments/Personal Best from what remains.
    fn delete_attempt(&mut self, run_index: u32) {
        self.run
            .attempt_history
            .retain(|a| a.run_index != run_index);
        self.run.pb_history.retain(|p| p.run_index != run_index);
        for split in &mut self.run.splits {
            split.segment_history.retain(|e| e.run_index != run_index);
            split.recompute_best_segment();
        }
        self.run.recompute_personal_best();
        self.persist_and_reload();
    }

    /// Removes one entry from the PB log only — the underlying attempt and
    /// its segment times are untouched.
    fn delete_pb_entry(&mut self, run_index: u32) {
        self.run.pb_history.retain(|p| p.run_index != run_index);
        self.persist_and_reload();
    }

    /// Removes one run's segment time from a single split only, then
    /// recomputes that split's Best Segment and the run's Personal Best.
    fn delete_segment_entry(&mut self, split_index: usize, run_index: u32) {
        if let Some(split) = self.run.splits.get_mut(split_index) {
            split.segment_history.retain(|e| e.run_index != run_index);
            split.recompute_best_segment();
        }
        self.run.recompute_personal_best();
        self.persist_and_reload();
    }
}
