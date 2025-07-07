use chrono::Duration;
use eframe::egui;
use egui::{Context, RichText, Sense, TextureHandle};
use image::GenericImageView;
use openspeedrun::core::split::{Run, Split};
use rfd::FileDialog;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::send_message;

pub struct SplitEditor {
    pub run_path: PathBuf,
    pub run: Run,
    icon_selection_index: Option<usize>,
    icon_cache: HashMap<String, TextureHandle>,
    dragging_split_index: Option<usize>,
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
                if full_path.exists() {
                    if let Ok(image) = image::open(&full_path) {
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
    }

    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        ctx.set_fonts(fonts);

        ui.label("Edit Run");

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
            ui.label("Gold split:");
            ui.checkbox(&mut self.run.gold_split, "");
        });

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Add split").clicked() {
                self.run.splits.push(Split {
                    name: "New split".to_string(),
                    pb_time: None,
                    last_time: None,
                    icon_path: None,
                    gold_time: None,
                });
            }

            if ui.button("Save all").clicked() {
                if let Err(e) = self.run.save_to_file(self.run_path.to_str().unwrap()) {
                    eprintln!("Error saving all: {}", e);
                }
                send_message("reloadrun");
            }
        });
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

                if response.hovered() && ctx.input(|i| i.pointer.any_released()) {
                    if let Some(from_index) = self.dragging_split_index {
                        if from_index != i {
                            swap_request = Some((from_index, i));
                        }
                    }
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
                        {
                            if i > 0 {
                                swap_request = Some((i, i - 1));
                            }
                        }

                        if ui
                            .button(RichText::new(egui_phosphor::regular::ARROW_DOWN))
                            .clicked()
                        {
                            if i < splits_len - 1 {
                                swap_request = Some((i, i + 1));
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.style_mut().interaction.selectable_labels = false;
                            ui.add_space(8.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(egui_phosphor::regular::DOTS_SIX_VERTICAL)
                                        .italics()
                                        .size(14.0)
                                        .color(egui::Color32::GRAY),
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

                        if ui.button("Change Icon").clicked() {
                            if let Some(path) = FileDialog::new()
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

                    // PB Time
                    ui.horizontal(|ui| {
                        ui.label("PB Time");

                        let (mut hours, mut minutes, mut seconds, mut millis) =
                            (0u32, 0u32, 0u32, 0u32);
                        if let Some(dur) = split.pb_time {
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

                        if changed {
                            let total_millis = (hours as i64) * 3_600_000
                                + (minutes as i64) * 60_000
                                + (seconds as i64) * 1_000
                                + (millis as i64);

                            split.pb_time = if total_millis == 0 {
                                None
                            } else {
                                Some(Duration::milliseconds(total_millis))
                            };

                            split_changed = true;
                        }

                        if ui.button("Reset PB").clicked() {
                            split.pb_time = None;
                            split_changed = true;
                        }
                    });

                    ui.add_space(10.0);

                    // Gold Time
                    ui.horizontal(|ui| {
                        ui.label("Gold Time");

                        let (mut hours, mut minutes, mut seconds, mut millis) =
                            (0u32, 0u32, 0u32, 0u32);
                        if let Some(dur) = split.gold_time {
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

                        if changed {
                            let total_millis = (hours as i64) * 3_600_000
                                + (minutes as i64) * 60_000
                                + (seconds as i64) * 1_000
                                + (millis as i64);

                            split.gold_time = if total_millis == 0 {
                                None
                            } else {
                                Some(Duration::milliseconds(total_millis))
                            };
                        }

                        if ui.button("Reset Gold").clicked() {
                            split.gold_time = None;
                        }
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

        if let Some(index) = self.dragging_split_index {
            if let Some(split) = self.run.splits.get(index) {
                if let Some(cursor_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Tooltip,
                        egui::Id::new("dragging_split_preview"),
                    ));

                    let size = egui::vec2(240.0, 56.0);
                    let offset = egui::vec2(-size.x - 12.0, -size.y / 2.0);
                    let rect = egui::Rect::from_min_size(cursor_pos + offset, size);

                    let bg_color = egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220);
                    painter.rect_filled(rect, egui::CornerRadius::same(10), bg_color);

                    let mut x = rect.left() + 10.0;
                    let y = rect.top() + 8.0;

                    if let Some(icon_path) = &split.icon_path {
                        if let Some(tex) = self.icon_cache.get(icon_path) {
                            let icon_size = 40.0;
                            let icon_rect = egui::Rect::from_min_size(
                                egui::pos2(x, y),
                                egui::vec2(icon_size, icon_size),
                            );
                            painter.add(egui::Shape::image(
                                tex.id(),
                                icon_rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            ));
                            x += icon_size + 10.0;
                        }
                    }

                    let title_font = egui::TextStyle::Heading.resolve(&ctx.style());
                    let subtitle_font = egui::TextStyle::Body.resolve(&ctx.style());

                    painter.text(
                        egui::pos2(x, y + 4.0),
                        egui::Align2::LEFT_TOP,
                        &split.name,
                        title_font.clone(),
                        egui::Color32::WHITE,
                    );

                    let secondary_text = if let Some(pb) = split.pb_time {
                        format!("PB: {}", format_duration(pb))
                    } else if let Some(gold) = split.gold_time {
                        format!("Gold: {}", format_duration(gold))
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
            }
        }

        if ctx.input(|i| i.pointer.any_released()) {
            self.dragging_split_index = None;
        }
    }
}
