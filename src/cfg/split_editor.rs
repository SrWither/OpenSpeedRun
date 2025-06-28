use chrono::Duration;
use eframe::egui;
use egui::{Context, TextureHandle};
use image::GenericImageView;
use openspeedrun::core::split::{Run, Split};
use rfd::FileDialog;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct SplitEditor {
    pub run_path: PathBuf,
    pub run: Run,
    icon_selection_index: Option<usize>,
    icon_cache: HashMap<String, TextureHandle>,
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
        }
    }

    fn duration_to_string(dur_opt: Option<Duration>) -> String {
        dur_opt.map_or_else(
            || "".to_string(),
            |dur| {
                let total_millis = dur.num_milliseconds();
                if total_millis < 0 {
                    return "-".to_string();
                }
                let minutes = total_millis / 60000;
                let seconds = (total_millis % 60000) / 1000;
                let millis = total_millis % 1000;
                format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
            },
        )
    }

    fn string_to_duration(s: &str) -> Option<Duration> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let minutes: i64 = parts[0].parse().ok()?;
        let sec_parts: Vec<&str> = parts[1].split('.').collect();
        if sec_parts.len() != 2 {
            return None;
        }
        let seconds: i64 = sec_parts[0].parse().ok()?;
        let millis: i64 = sec_parts[1].parse().ok()?;
        Some(
            Duration::minutes(minutes)
                + Duration::seconds(seconds)
                + Duration::milliseconds(millis),
        )
    }

    fn load_textures(&mut self, ctx: &Context) {
        let icon_paths: Vec<_> = self.run.splits.iter()
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
        ui.label("Edit Run");

        ui.horizontal(|ui| {
            ui.label("Name run:");
            ui.text_edit_singleline(&mut self.run.title);
        });

        ui.horizontal(|ui| {
            ui.label("Category:");
            ui.text_edit_singleline(&mut self.run.category);
        });

        ui.separator();

        self.load_textures(ctx);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut to_remove = None;

            for i in 0..self.run.splits.len() {
                let split = &mut self.run.splits[i];
                let texture = split.icon_path.as_ref()
                    .and_then(|path| self.icon_cache.get(path));

                let mut split_changed = false;
                let mut new_icon_path = None;

                ui.group(|ui| {
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
                                        new_icon_path = Some(format!("icons/{}", filename.to_string_lossy()));
                                    }
                                }
                            }
                        }

                        if ui.button("Select existing icon").clicked() {
                            self.icon_selection_index = Some(i);
                        }

                        if ui.button("Delete").clicked() {
                            to_remove = Some(i);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("PB Time (mm:ss.SSS):");
                        let mut pb_str = Self::duration_to_string(split.pb_time);
                        if ui.text_edit_singleline(&mut pb_str).changed() {
                            split.pb_time = Self::string_to_duration(&pb_str);
                            split_changed = true;
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
        });

        if ui.button("Add split").clicked() {
            self.run.splits.push(Split {
                name: "New split".to_string(),
                pb_time: None,
                last_time: None,
                icon_path: None,
            });
        }

        ui.separator();

        if ui.button("Save splits").clicked() {
            if let Err(e) = self.run.save_to_file(self.run_path.to_str().unwrap()) {
                eprintln!("Error saving splits: {}", e);
            }
        }
    }
}