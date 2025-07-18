use std::path::PathBuf;

use openspeedrun::Run;

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
        use egui::{Color32, Grid, RichText, ScrollArea};

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
            if ui.button("🗑 Clear History").clicked() {
                self.confirm_clear = true;
            }
        });

        ui.separator();

        match self.active_tab {
            Tab::Attempts => {
                if self.run.attempt_history.is_empty() {
                    ui.label("No attempts recorded yet.");
                    return;
                }
                ScrollArea::vertical().show(ui, |ui| {
                    Grid::new("attempt_history_grid")
                        .striped(true)
                        .min_col_width(100.0)
                        .show(ui, |ui| {
                            ui.label("Attempt #");
                            ui.label("Date");
                            ui.label("Total Time");
                            ui.label("Ingame Time");
                            ui.label("Ended");
                            ui.label("Is PB");
                            ui.end_row();

                            for attempt in &self.run.attempt_history {
                                ui.label(attempt.run_index.to_string());
                                let date_str = attempt
                                    .date
                                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                    .unwrap_or_else(|| "-".to_string());
                                ui.label(date_str);

                                let total_time_str = attempt
                                    .total_time
                                    .map(format_duration)
                                    .unwrap_or_else(|| "-".to_string());
                                ui.label(total_time_str);

                                let ingame_time_str = attempt
                                    .ingame_time
                                    .map(format_duration)
                                    .unwrap_or_else(|| "-".to_string());
                                ui.label(ingame_time_str);

                                ui.label(if attempt.ended { "Yes" } else { "No" });

                                let is_pb = self
                                    .run
                                    .pb_history
                                    .iter()
                                    .any(|pb| pb.run_index == attempt.run_index);
                                let pb_text = if is_pb {
                                    RichText::new("Yes").color(Color32::LIGHT_GREEN)
                                } else {
                                    RichText::new("No")
                                };
                                ui.label(pb_text);

                                ui.end_row();
                            }
                        });
                });
            }

            Tab::PbHistory => {
                if self.run.pb_history.is_empty() {
                    ui.label("No PB history available.");
                    return;
                }
                ScrollArea::vertical().show(ui, |ui| {
                    Grid::new("pb_history_grid")
                        .striped(true)
                        .min_col_width(100.0)
                        .show(ui, |ui| {
                            ui.label("PB Attempt #");
                            ui.label("Date");
                            ui.label("Total Time");
                            ui.label("Ingame Time");
                            ui.label("Ended");
                            ui.end_row();

                            for pb in &self.run.pb_history {
                                ui.label(pb.run_index.to_string());

                                let date_str = pb
                                    .date
                                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                                    .unwrap_or_else(|| "-".to_string());
                                ui.label(date_str);

                                ui.label(
                                    pb.total_time
                                        .map(format_duration)
                                        .unwrap_or_else(|| "-".to_string()),
                                );
                                ui.label(
                                    pb.ingame_time
                                        .map(format_duration)
                                        .unwrap_or_else(|| "-".to_string()),
                                );

                                ui.label(if pb.ended { "Yes" } else { "No" });

                                ui.end_row();
                            }
                        });
                });
            }

            Tab::SplitHistory => {
                if self.run.splits.is_empty() {
                    ui.label("No splits available.");
                    return;
                }
                ScrollArea::vertical().show(ui, |ui| {
                    for (i, split) in self.run.splits.iter().enumerate() {
                        ui.group(|ui| {
                            ui.label(format!("Split #{}: {}", i + 1, split.name));

                            Grid::new(format!("split_{}_gold", i))
                                .striped(true)
                                .min_col_width(100.0)
                                .show(ui, |ui| {
                                    ui.label("Gold History (Run #)");
                                    ui.label("Time");
                                    ui.end_row();

                                    for entry in &split.gold_history {
                                        ui.label(entry.run_index.to_string());
                                        ui.label(
                                            entry
                                                .time
                                                .map(format_duration)
                                                .unwrap_or_else(|| "-".to_string()),
                                        );
                                        ui.end_row();
                                    }
                                });

                            ui.separator();

                            Grid::new(format!("split_{}_pb", i))
                                .striped(true)
                                .min_col_width(100.0)
                                .show(ui, |ui| {
                                    ui.label("PB History (Run #)");
                                    ui.label("Time");
                                    ui.end_row();

                                    for entry in &split.pb_history {
                                        ui.label(entry.run_index.to_string());
                                        ui.label(
                                            entry
                                                .time
                                                .map(format_duration)
                                                .unwrap_or_else(|| "-".to_string()),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                        ui.separator();
                    }
                });
            }
        }

        if self.confirm_clear {
            egui::Window::new("Confirm Clear History")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Are you sure you want to clear all attempt and PB history?");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_clear = false;
                        }
                        if ui
                            .button(RichText::new("Yes, clear").color(Color32::RED))
                            .clicked()
                        {
                            self.run.attempt_history.clear();
                            self.run.pb_history.clear();
                            self.run.attempts = 0;
                            for split in &mut self.run.splits {
                                split.pb_history.clear();
                                split.gold_history.clear();
                            }

                            let _ = self.run.save_to_file(self.run_path.to_str().unwrap());
                            self.confirm_clear = false;
                        }
                    });
                });
        }
    }
}
