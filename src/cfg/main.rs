mod split_editor;
mod theme_editor;

use eframe::egui;
use openspeedrun::{
    LayoutConfig, Run,
    config::load::{AppConfig, config_base_dir},
};
use std::fs;

use split_editor::SplitEditor;
use theme_editor::ThemeEditor;

pub struct ConfigApp {
    app_config: AppConfig,
    available_splits: Vec<String>,
    available_themes: Vec<String>,
    selected_split: Option<String>,
    selected_theme: Option<String>,
    tab: usize,
    theme_editor: Option<ThemeEditor>,
    split_editor: Option<SplitEditor>,
    new_name_input: String,
    show_name_input: bool,
    is_creating_theme: bool,
    show_delete_confirm: bool,
    item_to_delete: Option<(String, bool)>,
}

impl ConfigApp {
    pub fn new() -> Self {
        let base = config_base_dir();
        fs::create_dir_all(base.join("splits")).ok();
        fs::create_dir_all(base.join("themes")).ok();

        let app_config = AppConfig::load();

        let available_splits = fs::read_dir(base.join("splits"))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let available_themes = fs::read_dir(base.join("themes"))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .map(|s| s.strip_suffix(".json").unwrap_or(&s).to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let selected_split = Some(
            app_config
                .last_split_path
                .strip_prefix("splits/")
                .unwrap_or("")
                .to_string(),
        );

        let selected_theme = Some(
            app_config
                .theme
                .strip_prefix("themes/")
                .unwrap_or("")
                .strip_suffix(".json")
                .unwrap_or("")
                .to_string(),
        );

        println!("selected theme: {:?}", selected_theme);

        let theme_editor = selected_theme
            .as_ref()
            .map(|t| ThemeEditor::new(base.join("themes").join(format!("{}.json", t))));

        let split_editor = selected_split
            .as_ref()
            .map(|s| SplitEditor::new(base.join("splits").join(s).join("split.json")));

        Self {
            app_config,
            available_splits,
            available_themes,
            selected_split,
            selected_theme,
            tab: 0,
            theme_editor,
            split_editor,
            new_name_input: String::new(),
            show_name_input: false,
            is_creating_theme: false,
            show_delete_confirm: false,
            item_to_delete: None,
        }
    }

    fn show_name_input_popup(&mut self, is_theme: bool) {
        self.new_name_input.clear();
        self.show_name_input = true;
        self.is_creating_theme = is_theme;
    }

    fn create_item_with_name(&mut self) {
        let name = self.new_name_input.trim().to_string();
        if name.is_empty() {
            return;
        }

        if self.is_creating_theme {
            self.create_theme(&name);
        } else {
            self.create_split(&name);
        }

        self.show_name_input = false;
    }

    fn create_theme(&mut self, name: &str) {
        let base = config_base_dir().join("themes");
        let path = base.join(format!("{}.json", name));

        if path.exists() {
            return;
        }

        let theme = LayoutConfig::default();
        if theme.save(path.to_str().unwrap()).is_ok() {
            self.available_themes.push(name.to_string());
            self.selected_theme = Some(name.to_string());
            self.app_config.theme = format!("themes/{}.json", name);
            self.theme_editor = Some(ThemeEditor::new(path));
        }
    }

    fn create_split(&mut self, name: &str) {
        let base = config_base_dir().join("splits");
        let new_folder = base.join(name);

        if new_folder.exists() {
            return;
        }

        if std::fs::create_dir_all(&new_folder).is_ok() {
            let split_path = new_folder.join("split.json");
            let run = Run::new("New Run", "Category", &["Split 1", "Split 2"]);
            if run.save_to_file(split_path.to_str().unwrap()).is_ok() {
                self.available_splits.push(name.to_string());
                self.selected_split = Some(name.to_string());
                self.app_config.last_split_path = format!("splits/{}", name);
                self.split_editor = Some(SplitEditor::new(split_path));
            }
        }
    }

    fn show_delete_confirmation(&mut self, name: String, is_theme: bool) {
        self.item_to_delete = Some((name, is_theme));
        self.show_delete_confirm = true;
    }

    fn delete_item(&mut self) {
        if let Some((name, is_theme)) = self.item_to_delete.take() {
            if is_theme {
                self.delete_theme(&name);
            } else {
                self.delete_split(&name);
            }
        }
        self.show_delete_confirm = false;
    }

    fn delete_theme(&mut self, name: &str) {
        let base = config_base_dir().join("themes");
        let path = base.join(format!("{}.json", name));
        if path.exists() && std::fs::remove_file(&path).is_ok() {
            self.available_themes.retain(|t| t != name);
            if self.selected_theme.as_ref() == Some(&name.to_string()) {
                self.selected_theme = None;
                self.app_config.theme = "".to_string();
                self.theme_editor = None;
            }
        }
    }

    fn delete_split(&mut self, name: &str) {
        let base = config_base_dir().join("splits");
        let path = base.join(name);
        if path.exists() && std::fs::remove_dir_all(&path).is_ok() {
            self.available_splits.retain(|s| s != name);
            if self.selected_split.as_ref() == Some(&name.to_string()) {
                self.selected_split = None;
                self.app_config.last_split_path = "".to_string();
                self.split_editor = None;
            }
        }
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Name input popup
        if self.show_name_input {
            egui::Window::new(if self.is_creating_theme {
                "New Theme"
            } else {
                "New Split"
            })
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_name_input);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_name_input = false;
                    }
                    if ui.button("Create").clicked() {
                        self.create_item_with_name();
                    }
                });
            });
        }

        // Delete confirmation popup
        if self.show_delete_confirm {
            if let Some((name, is_theme)) = &self.item_to_delete {
                let name = name.clone();
                let is_theme = *is_theme;
                egui::Window::new("Confirmar eliminación")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "Are you sure you want to delete {} '{}'?",
                            if is_theme { "the theme" } else { "the split" },
                            name
                        ));

                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_delete_confirm = false;
                            }
                            if ui.button("Delete").clicked() {
                                self.delete_item();
                            }
                        });
                    });
            }
        }

        // Main UI
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(self.tab == 0, "Selector").clicked() {
                    self.tab = 0;
                }
                if ui.selectable_label(self.tab == 1, "Themes").clicked() {
                    self.tab = 1;
                }
                if ui.selectable_label(self.tab == 2, "Splits").clicked() {
                    self.tab = 2;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            0 => self.ui_selector(ui),
            1 => {
                if let Some(editor) = &mut self.theme_editor {
                    editor.ui(ui);
                } else {
                    ui.label("Select a theme to edit.");
                }
            }
            2 => {
                if let Some(editor) = &mut self.split_editor {
                    editor.ui(ctx, ui);
                } else {
                    ui.label("Select a split to edit.");
                }
            }
            _ => {}
        });
    }
}

impl ConfigApp {
    fn ui_selector(&mut self, ui: &mut egui::Ui) {
        ui.label("Current Theme:");
        for theme in &self.available_themes {
            if ui
                .selectable_label(Some(theme) == self.selected_theme.as_ref(), theme)
                .clicked()
            {
                self.selected_theme = Some(theme.clone());
                self.app_config.theme = format!("themes/{}.json", theme);
                let base = config_base_dir();
                self.theme_editor = Some(ThemeEditor::new(
                    base.join("themes").join(format!("{}.json", theme)),
                ));
            }
        }

        ui.separator();

        ui.label("Current Split:");
        for split in &self.available_splits {
            if ui
                .selectable_label(Some(split) == self.selected_split.as_ref(), split)
                .clicked()
            {
                self.selected_split = Some(split.clone());
                self.app_config.last_split_path = format!("splits/{}", split);
                let base = config_base_dir();
                self.split_editor = Some(SplitEditor::new(
                    base.join("splits").join(split).join("split.json"),
                ));
            }
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("New theme").clicked() {
                self.show_name_input_popup(true);
            }
            if ui.button("Delete theme").clicked() {
                if let Some(theme) = &self.selected_theme {
                    self.show_delete_confirmation(theme.clone(), true);
                }
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("New split").clicked() {
                self.show_name_input_popup(false);
            }
            if ui.button("Delete split").clicked() {
                if let Some(split) = &self.selected_split {
                    self.show_delete_confirmation(split.clone(), false);
                }
            }
        });

        ui.separator();

        if ui.button("Save Changes").clicked() {
            self.app_config.save();
            if let Some(editor) = &self.theme_editor {
                let _ = editor.layout.save(self.app_config.theme.as_str());
            }
            if let Some(editor) = &self.split_editor {
                let _ = editor
                    .run
                    .save_to_file(&format!("{}/split.json", self.app_config.last_split_path));
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "OpenSpeedRun Config",
        options,
        Box::new(|_cc| Ok(Box::new(ConfigApp::new()))),
    )
}
