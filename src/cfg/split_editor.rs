use chrono::Duration;
use eframe::egui;
use egui::{Context, RichText, Sense, TextureHandle};
use image::GenericImageView;
use openspeedrun::core::split::{
    COMPARISON_BEST_SEGMENTS, COMPARISON_PERSONAL_BEST, Run, RunVariable, Split, TimingMethod,
};
use openspeedrun::formats::{lss, native};
use rfd::FileDialog;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::send_message;
use crate::speedrun_com_picker::SpeedrunComPicker;
use crate::style;

pub struct SplitEditor {
    pub run_path: PathBuf,
    pub run: Run,
    icon_selection_index: Option<usize>,
    icon_cache: HashMap<String, TextureHandle>,
    dragging_split_index: Option<usize>,
    import_export_status: Option<String>,
    speedrun_com_picker: SpeedrunComPicker,
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

/// h/m/s/ms `DragValue` row + Reset button for editing an `Option<Duration>`.
/// Returns whether `value` changed.
fn edit_duration(ui: &mut egui::Ui, id_salt: &str, value: &mut Option<Duration>) -> bool {
    let (mut hours, mut minutes, mut seconds, mut millis) = (0u32, 0u32, 0u32, 0u32);
    if let Some(dur) = value {
        let total_millis = dur.num_milliseconds();
        if total_millis >= 0 {
            let total_millis = total_millis as u32;
            hours = total_millis / 3_600_000;
            let rem = total_millis % 3_600_000;
            minutes = rem / 60_000;
            let rem = rem % 60_000;
            seconds = rem / 1_000;
            millis = rem % 1_000;
        }
    }

    let mut changed = false;
    let mut reset = false;

    ui.push_id(id_salt, |ui| {
        changed |= ui
            .add(egui::DragValue::new(&mut hours).range(0..=99))
            .changed();
        ui.label("h");

        changed |= ui
            .add(egui::DragValue::new(&mut minutes).range(0..=59))
            .changed();
        ui.label("m");

        changed |= ui
            .add(egui::DragValue::new(&mut seconds).range(0..=59))
            .changed();
        ui.label("s");

        changed |= ui
            .add(egui::DragValue::new(&mut millis).range(0..=999))
            .changed();
        ui.label("ms");

        if ui.button("Reset").clicked() {
            reset = true;
        }
    });

    if reset {
        *value = None;
        true
    } else if changed {
        let total_millis = (hours as i64) * 3_600_000
            + (minutes as i64) * 60_000
            + (seconds as i64) * 1_000
            + (millis as i64);

        *value = if total_millis == 0 {
            None
        } else {
            Some(Duration::milliseconds(total_millis))
        };
        true
    } else {
        false
    }
}

impl SplitEditor {
    pub fn new(run_path: PathBuf) -> Self {
        let run = Run::load_from_file(run_path.to_str().unwrap())
            .unwrap_or_else(|_| Run::new("New Run", "Category", &["Split 1", "Split 2"]));
        Self {
            run_path,
            run,
            icon_selection_index: None,
            icon_cache: HashMap::new(),
            dragging_split_index: None,
            import_export_status: None,
            speedrun_com_picker: SpeedrunComPicker::default(),
        }
    }

    fn load_textures(&mut self, ctx: &Context) {
        let icon_paths: Vec<_> = self
            .run
            .splits
            .iter()
            .filter_map(|split| split.icon_path.as_ref())
            .collect();

        for icon_path in icon_paths {
            if !self.icon_cache.contains_key(icon_path) {
                let full_path = self.run_path.parent().unwrap().join(icon_path);
                if full_path.exists()
                    && let Ok(image) = image::open(&full_path)
                {
                    let size = image.dimensions();
                    let rgba = image.to_rgba8();
                    let tex = ctx.load_texture(
                        icon_path.to_string(),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [size.0 as usize, size.1 as usize],
                            &rgba,
                        ),
                        Default::default(),
                    );
                    self.icon_cache.insert(icon_path.to_string(), tex);
                }
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let total_width = ui.available_width();
            let edit_width = (total_width - style::SPACE_MD) * 0.42;
            let actions_width = total_width - style::SPACE_MD - edit_width;

            ui.allocate_ui_with_layout(
                egui::vec2(edit_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                ui.set_width(edit_width);
                style::section_card(ui, "Edit Run", egui_phosphor::regular::FLAG_CHECKERED, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name run:");
                        ui.text_edit_singleline(&mut self.run.title);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Category:");
                        ui.text_edit_singleline(&mut self.run.category);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Start offset (seconds):");
                        let mut offset_secs = self.run.start_offset.unwrap_or(0);
                        let mut changed = false;
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut offset_secs)
                                    .range(0..=600)
                                    .speed(1)
                                    .prefix("⏱ "),
                            )
                            .changed();
                        if changed {
                            self.run.start_offset = Some(offset_secs);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Splits per page:");
                        let mut value = self.run.splits_per_page.unwrap_or(5);
                        if ui
                            .add(egui::DragValue::new(&mut value).range(1..=50))
                            .changed()
                        {
                            self.run.splits_per_page = Some(value);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Auto-update PB:");
                        ui.checkbox(&mut self.run.auto_update_pb, "");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Timing method:");
                        egui::ComboBox::from_id_salt("timing_method")
                            .selected_text(match self.run.timing_method {
                                TimingMethod::RealTime => "Real Time",
                                TimingMethod::GameTime => "Game Time",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.run.timing_method, TimingMethod::RealTime, "Real Time");
                                ui.selectable_value(&mut self.run.timing_method, TimingMethod::GameTime, "Game Time");
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Compare against:");
                        let comparison_names = self.run.comparison_names();
                        egui::ComboBox::from_id_salt("selected_comparison")
                            .selected_text(self.run.selected_comparison.clone())
                            .show_ui(ui, |ui| {
                                for name in &comparison_names {
                                    ui.selectable_value(
                                        &mut self.run.selected_comparison,
                                        name.clone(),
                                        name,
                                    );
                                }
                            });
                    });

                    ui.add_space(style::SPACE_SM);
                    ui.collapsing("Category metadata", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Platform:");
                            let mut platform = self.run.metadata.platform.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut platform).changed() {
                                self.run.metadata.platform =
                                    if platform.is_empty() { None } else { Some(platform) };
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Region:");
                            let mut region = self.run.metadata.region.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut region).changed() {
                                self.run.metadata.region =
                                    if region.is_empty() { None } else { Some(region) };
                            }
                        });

                        ui.label("Variables:");
                        let mut variable_to_remove = None;
                        for (i, variable) in self.run.metadata.variables.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut variable.name);
                                ui.label("=");
                                ui.text_edit_singleline(&mut variable.value);
                                if ui
                                    .button(RichText::new(egui_phosphor::regular::TRASH))
                                    .clicked()
                                {
                                    variable_to_remove = Some(i);
                                }
                            });
                        }
                        if let Some(i) = variable_to_remove {
                            self.run.metadata.variables.remove(i);
                        }
                        if ui.button("Add variable").clicked() {
                            self.run.metadata.variables.push(RunVariable::default());
                        }
                    });
                });
            });

            ui.allocate_ui_with_layout(
                egui::vec2(actions_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                ui.set_width(actions_width);
                style::section_card(ui, "Actions", egui_phosphor::regular::SLIDERS, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(format!("{} Add split", egui_phosphor::regular::PLUS))
                            .clicked()
                        {
                            self.run.splits.push(Split {
                                name: "New split".to_string(),
                                ..Split::default()
                            });
                        }

                        let save_button = egui::Button::new(format!(
                            "{} Save all",
                            egui_phosphor::regular::FLOPPY_DISK
                        ))
                        .fill(style::ACCENT_BG)
                        .stroke(egui::Stroke::new(1.0_f32, style::ACCENT));
                        if ui.add(save_button).clicked() {
                            if let Err(e) = self.run.save_to_file(self.run_path.to_str().unwrap()) {
                                eprintln!("Error saving all: {}", e);
                            }
                            send_message("reloadrun");
                        }
                    });

                    ui.add_space(style::SPACE_SM);

                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(format!("{} Import .lss", egui_phosphor::regular::UPLOAD_SIMPLE))
                            .clicked()
                            && let Some(path) = FileDialog::new().add_filter("LiveSplit", &["lss"]).pick_file() {
                                let icons_dir = self.run_path.parent().unwrap().join("icons");
                                self.import_export_status = Some(match lss::import(&path, &icons_dir) {
                                    Ok(result) => {
                                        self.run = result.run;
                                        // `import` names extracted icons purely
                                        // by segment index ("imported_0.png",
                                        // etc.), so a second import reuses the
                                        // same icon_path strings as the first
                                        // even though the on-disk bytes just
                                        // changed underneath them. The cache
                                        // is keyed by that string, so without
                                        // clearing it here it'd keep showing
                                        // the previous import's textures.
                                        self.icon_cache.clear();
                                        let version = result.source_version.as_deref().unwrap_or("unknown");
                                        format!(
                                            "Imported from LiveSplit v{version}. Review it, then \"Save all\" to keep it."
                                        )
                                    }
                                    Err(e) => format!("Import failed: {e}"),
                                });
                            }

                        if ui
                            .button(format!("{} Export .lss", egui_phosphor::regular::DOWNLOAD_SIMPLE))
                            .clicked()
                        {
                            let default_name = format!("{}.lss", self.run.title);
                            if let Some(path) = FileDialog::new()
                                .set_file_name(&default_name)
                                .add_filter("LiveSplit", &["lss"])
                                .save_file()
                            {
                                let icons_base_dir = self.run_path.parent().unwrap();
                                self.import_export_status = Some(match lss::export(&self.run, &path, icons_base_dir) {
                                    Ok(()) => format!("Exported to {}", path.display()),
                                    Err(e) => format!("Export failed: {e}"),
                                });
                            }
                        }

                        if ui
                            .button(format!("{} Export folder", egui_phosphor::regular::FOLDER_OPEN))
                            .clicked()
                            && let Some(dest) = FileDialog::new().pick_folder() {
                                let run_dir = self.run_path.parent().unwrap();
                                self.import_export_status = Some(match native::export_folder(run_dir, &dest) {
                                    Ok(()) => format!("Exported folder to {}", dest.display()),
                                    Err(e) => format!("Folder export failed: {e}"),
                                });
                            }

                        if ui
                            .button(format!(
                                "{} Import folder",
                                egui_phosphor::regular::FOLDER_SIMPLE_PLUS
                            ))
                            .clicked()
                            && let Some(src) = FileDialog::new().pick_folder() {
                                let splits_base = crate::config_base_dir().join("splits");
                                let name = src
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("imported")
                                    .to_string();
                                self.import_export_status =
                                    Some(match native::import_folder(&src, &splits_base, &name) {
                                        Ok(dest) => format!(
                                            "Imported to {}. Reopen the Selector tab to see it.",
                                            dest.display()
                                        ),
                                        Err(e) => format!("Folder import failed: {e}"),
                                    });
                            }

                        if ui
                            .button(format!(
                                "{} Fill from speedrun.com",
                                egui_phosphor::regular::GLOBE
                            ))
                            .clicked()
                        {
                            self.speedrun_com_picker.open = true;
                        }
                    });

                    if let Some(status) = &self.import_export_status {
                        ui.add_space(style::SPACE_SM);
                        ui.label(RichText::new(status.as_str()).color(style::TEXT_MUTED));
                    }
                });
            });
        });

        if let Some(picked) = self.speedrun_com_picker.ui(ctx) {
            self.run.title = picked.title;
            self.run.category = picked.category;
            self.run.metadata.speedrun_com_game_id = Some(picked.speedrun_com_game_id);
            self.run.metadata.speedrun_com_category_id = Some(picked.speedrun_com_category_id);
            for (name, value) in picked.variables {
                if let Some(existing) = self
                    .run
                    .metadata
                    .variables
                    .iter_mut()
                    .find(|v| v.name == name)
                {
                    existing.value = value;
                } else {
                    self.run
                        .metadata
                        .variables
                        .push(RunVariable { name, value });
                }
            }

            let message = if let Some(splits) = picked.splits {
                let split_count = splits.len();
                self.run.splits = splits;
                format!(
                    "Filled from speedrun.com, with {split_count} real splits from therun.gg. \"Save all\" to keep it."
                )
            } else {
                "Filled from speedrun.com. \"Save all\" to keep it.".to_string()
            };
            self.import_export_status = Some(message);
        }

        ui.separator();

        self.load_textures(ctx);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut to_remove = None;
            let mut swap_request: Option<(usize, usize)> = None;
            let splits_len = self.run.splits.len();

            for i in 0..splits_len {
                let id = ui.make_persistent_id(format!("split_drag_{}", i));
                let mut rect = ui.available_rect_before_wrap();

                let visual_height = 60.0;
                if rect.height() < visual_height * 0.75 {
                    rect.max.y = rect.min.y + visual_height;
                }

                let response = ui.interact(rect, id, egui::Sense::click_and_drag());

                if response.drag_started() {
                    self.dragging_split_index = Some(i);
                }

                if response.hovered()
                    && ctx.input(|i| i.pointer.any_released())
                    && let Some(from_index) = self.dragging_split_index
                    && from_index != i
                {
                    swap_request = Some((from_index, i));
                }

                let split = &mut self.run.splits[i];
                let texture = split
                    .icon_path
                    .as_ref()
                    .and_then(|path| self.icon_cache.get(path));

                let mut split_changed = false;
                let mut new_icon_path = None;

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new(egui_phosphor::regular::ARROW_UP))
                            .clicked()
                            && i > 0
                        {
                            swap_request = Some((i, i - 1));
                        }

                        if ui
                            .button(RichText::new(egui_phosphor::regular::ARROW_DOWN))
                            .clicked()
                            && i < splits_len - 1
                        {
                            swap_request = Some((i, i + 1));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.style_mut().interaction.selectable_labels = false;
                            ui.add_space(8.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(egui_phosphor::regular::DOTS_SIX_VERTICAL)
                                        .italics()
                                        .size(14.0)
                                        .color(style::TEXT_MUTED),
                                )
                                .sense(Sense::empty()),
                            );
                        });
                    });

                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label(format!("Split {}", i + 1));
                        if ui.text_edit_singleline(&mut split.name).changed() {
                            split_changed = true;
                        }

                        if let Some(texture) = texture {
                            ui.add(egui::Image::new(texture).max_width(20.0));
                        }

                        if ui.button("Change Icon").clicked()
                            && let Some(path) = FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "gif"])
                                .pick_file()
                        {
                            let base_folder = self.run_path.parent().unwrap();
                            let icons_dir = base_folder.join("icons");
                            fs::create_dir_all(&icons_dir).ok();

                            if let Some(filename) = path.file_name() {
                                let dest = icons_dir.join(filename);
                                if fs::copy(&path, &dest).is_ok() {
                                    new_icon_path =
                                        Some(format!("icons/{}", filename.to_string_lossy()));
                                }
                            }
                        }

                        if ui.button("Select existing icon").clicked() {
                            self.icon_selection_index = Some(i);
                        }

                        if ui
                            .button(RichText::new(egui_phosphor::regular::TRASH))
                            .clicked()
                        {
                            to_remove = Some(i);
                        }
                    });

                    ui.add_space(10.0);

                    // Personal Best / Best Segments (Real Time)
                    ui.horizontal(|ui| {
                        ui.label("Personal Best");
                        let pb = split
                            .comparisons
                            .entry(COMPARISON_PERSONAL_BEST.to_string())
                            .or_default();
                        if edit_duration(ui, &format!("pb_real_{i}"), &mut pb.real_time) {
                            split_changed = true;
                        }
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Best Segments");
                        let best = split
                            .comparisons
                            .entry(COMPARISON_BEST_SEGMENTS.to_string())
                            .or_default();
                        if edit_duration(ui, &format!("best_real_{i}"), &mut best.real_time) {
                            split_changed = true;
                        }
                    });

                    egui::CollapsingHeader::new("Game Time (advanced)")
                        .id_salt(("game_time_advanced", i))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Personal Best");
                                let pb = split
                                    .comparisons
                                    .entry(COMPARISON_PERSONAL_BEST.to_string())
                                    .or_default();
                                if edit_duration(ui, &format!("pb_game_{i}"), &mut pb.game_time) {
                                    split_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Best Segments");
                                let best = split
                                    .comparisons
                                    .entry(COMPARISON_BEST_SEGMENTS.to_string())
                                    .or_default();
                                if edit_duration(ui, &format!("best_game_{i}"), &mut best.game_time)
                                {
                                    split_changed = true;
                                }
                            });
                        });

                    ui.separator();

                    if self.icon_selection_index == Some(i) {
                        let base_folder = self.run_path.parent().unwrap();
                        let icons_dir = base_folder.join("icons");

                        if let Ok(entries) = fs::read_dir(&icons_dir) {
                            ui.group(|ui| {
                                ui.label("Select an existing icon:");
                                for entry in entries.flatten() {
                                    if let Ok(name) = entry.file_name().into_string() {
                                        let rel_path = format!("icons/{}", name);
                                        if ui.button(&rel_path).clicked() {
                                            new_icon_path = Some(rel_path.clone());
                                            self.icon_selection_index = None;
                                        }
                                    }
                                }
                            });
                        } else {
                            ui.label("There are no icons available.");
                        }
                    }
                });

                if let Some(path) = new_icon_path {
                    split.icon_path = Some(path.clone());
                    self.icon_cache.remove(&path);
                }
            }

            if let Some(index) = to_remove {
                self.run.splits.remove(index);
            }

            if let Some((from, to)) = swap_request {
                self.run.splits.swap(from, to);
            }
        });

        if let Some(index) = self.dragging_split_index
            && let Some(split) = self.run.splits.get(index)
            && let Some(cursor_pos) = ctx.input(|i| i.pointer.hover_pos())
        {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("dragging_split_preview"),
            ));

            let size = egui::vec2(240.0, 56.0);
            let offset = egui::vec2(-size.x - 12.0, -size.y / 2.0);
            let rect = egui::Rect::from_min_size(cursor_pos + offset, size);

            let bg_color = egui::Color32::from_rgba_unmultiplied(27, 28, 33, 230);
            painter.rect_filled(rect, egui::CornerRadius::same(10), bg_color);
            painter.rect_stroke(
                rect,
                egui::CornerRadius::same(10),
                egui::Stroke::new(1.0_f32, style::ACCENT),
                egui::StrokeKind::Outside,
            );

            let mut x = rect.left() + 10.0;
            let y = rect.top() + 8.0;

            if let Some(icon_path) = &split.icon_path
                && let Some(tex) = self.icon_cache.get(icon_path)
            {
                let icon_size = 40.0;
                let icon_rect =
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(icon_size, icon_size));
                painter.add(egui::Shape::image(
                    tex.id(),
                    icon_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                ));
                x += icon_size + 10.0;
            }

            let title_font = egui::TextStyle::Heading.resolve(&ctx.global_style());
            let subtitle_font = egui::TextStyle::Body.resolve(&ctx.global_style());

            painter.text(
                egui::pos2(x, y + 4.0),
                egui::Align2::LEFT_TOP,
                &split.name,
                title_font.clone(),
                egui::Color32::WHITE,
            );

            let secondary_text = if let Some(pb) =
                split.comparison_time(COMPARISON_PERSONAL_BEST, TimingMethod::RealTime)
            {
                format!("PB: {}", format_duration(pb))
            } else if let Some(best) =
                split.comparison_time(COMPARISON_BEST_SEGMENTS, TimingMethod::RealTime)
            {
                format!("Best: {}", format_duration(best))
            } else {
                "No time set".to_string()
            };

            painter.text(
                egui::pos2(x, y + 28.0),
                egui::Align2::LEFT_TOP,
                secondary_text,
                subtitle_font,
                egui::Color32::GRAY,
            );
        }

        if ctx.input(|i| i.pointer.any_released()) {
            self.dragging_split_index = None;
        }
    }
}
