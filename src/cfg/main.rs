mod history;
mod shader_editor;
mod split_editor;
mod syntax;
mod theme_editor;

use std::fs;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

use eframe::egui;
use egui::{Color32, RichText, ViewportBuilder};
use openspeedrun::{
    LayoutConfig, Run,
    config::load::{AppConfig, config_base_dir},
};

use history::History;
use shader_editor::ShaderEditor;
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
    shader_editor: Option<ShaderEditor>,
    history: Option<History>,
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

        let theme_editor = selected_theme
            .as_ref()
            .map(|t| ThemeEditor::new(base.join("themes").join(format!("{}.json", t))));

        let split_editor = selected_split
            .as_ref()
            .map(|s| SplitEditor::new(base.join("splits").join(s).join("split.json")));

        let shader_editor = selected_theme.as_ref().map(|t| {
            let path = config_base_dir().join("themes").join(format!("{t}.json"));
            let layout = LayoutConfig::load_or_default(path.to_str().unwrap());
            ShaderEditor::new(base.join("shaders").join(&layout.colors.shader_path))
        });

        let history = selected_split.as_ref().map(|s| {
            let run_path = base.join("splits").join(s).join("split.json");
            History::new(run_path)
        });

        Self {
            app_config,
            available_splits,
            available_themes,
            selected_split,
            selected_theme,
            tab: 0,
            theme_editor,
            split_editor,
            shader_editor,
            history,
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
                egui::Window::new("Confirm Deletion")
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

        // Top Tabs
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
                if ui.selectable_label(self.tab == 3, "Shader").clicked() {
                    self.tab = 3;
                }
                if ui.selectable_label(self.tab == 4, "History").clicked() {
                    self.tab = 4;
                }
            });
        });

        // Central content with scroll
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| match self.tab {
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
                    3 => {
                        if let Some(theme_editor) = &self.theme_editor {
                            let new_path = config_base_dir()
                                .join("shaders")
                                .join(&theme_editor.layout.colors.shader_path);

                            let needs_reload = self
                                .shader_editor
                                .as_ref()
                                .map_or(true, |e| e.path != new_path);

                            if needs_reload {
                                self.shader_editor = Some(ShaderEditor::new(new_path));
                            }
                        }

                        if let Some(editor) = &mut self.shader_editor {
                            editor.ui(ui);
                        } else {
                            ui.label("Select a theme to edit its shader.");
                        }
                    }
                    4 => {
                        if let Some(split_editor) = &self.split_editor {
                            let needs_reload = self
                                .history
                                .as_ref()
                                .map(|h| h.run_path != split_editor.run_path)
                                .unwrap_or(true);

                            if needs_reload {
                                self.history = Some(History::new(split_editor.run_path.clone()));
                            }
                        }

                        if let Some(history) = &mut self.history {
                            history.ui(ctx, ui);
                        } else {
                            ui.label("Select a split to view its history.");
                        }
                    }

                    _ => {}
                });
        });
    }
}

impl ConfigApp {
    fn ui_selector(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Configuration Selector");
        });

        ui.add_space(12.0);

        // === THEMES ===
        ui.group(|ui| {
            ui.label(RichText::new("📁 Themes").strong().size(16.0));
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_salt("themes_scroll") // ID único para scroll independiente
                .max_height(160.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for theme in &self.available_themes {
                            let selected = Some(theme) == self.selected_theme.as_ref();
                            let button = egui::Button::new(theme.clone())
                                .fill(if selected {
                                    Color32::from_rgb(50, 50, 0)
                                } else {
                                    Color32::from_rgb(30, 30, 30)
                                })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if selected {
                                        Color32::YELLOW
                                    } else {
                                        Color32::DARK_GRAY
                                    },
                                ))
                                .min_size(egui::vec2(100.0, 28.0));

                            if ui.add(button).clicked() {
                                self.selected_theme = Some(theme.clone());
                                self.app_config.theme = format!("themes/{}.json", theme);
                                let base = config_base_dir();
                                self.theme_editor = Some(ThemeEditor::new(
                                    base.join("themes").join(format!("{}.json", theme)),
                                ));
                            }
                        }
                    });
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("➕ New Theme").clicked() {
                    self.show_name_input_popup(true);
                }
                if ui.button("🗑 Delete Theme").clicked() {
                    if let Some(theme) = &self.selected_theme {
                        self.show_delete_confirmation(theme.clone(), true);
                    }
                }
            });
        });

        ui.add_space(16.0);

        // === SPLITS ===
        ui.group(|ui| {
            ui.label(RichText::new("🏁 Splits").strong().size(16.0));
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_salt("splits_scroll") // ID único para scroll independiente
                .max_height(160.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for split in &self.available_splits {
                            let selected = Some(split) == self.selected_split.as_ref();
                            let button = egui::Button::new(split.clone())
                                .fill(if selected {
                                    Color32::from_rgb(0, 40, 60)
                                } else {
                                    Color32::from_rgb(20, 20, 20)
                                })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if selected {
                                        Color32::from_rgb(100, 200, 255)
                                    } else {
                                        Color32::GRAY
                                    },
                                ))
                                .min_size(egui::vec2(100.0, 28.0));

                            if ui.add(button).clicked() {
                                self.selected_split = Some(split.clone());
                                self.app_config.last_split_path = format!("splits/{}", split);
                                let base = config_base_dir();
                                self.split_editor = Some(SplitEditor::new(
                                    base.join("splits").join(split).join("split.json"),
                                ));
                            }
                        }
                    });
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("➕ New Split").clicked() {
                    self.show_name_input_popup(false);
                }
                if ui.button("🗑 Delete Split").clicked() {
                    if let Some(split) = &self.selected_split {
                        self.show_delete_confirmation(split.clone(), false);
                    }
                }
            });
        });

        ui.add_space(16.0);

        ui.horizontal(|ui| {
            if ui
                .add_sized([140.0, 42.0], egui::Button::new("💾 Save Changes"))
                .clicked()
            {
                self.app_config.save();

                if let Some(editor) = &self.theme_editor {
                    let _ = editor.layout.save(self.app_config.theme.as_str());
                }

                if let Some(editor) = &self.split_editor {
                    let _ = editor
                        .run
                        .save_to_file(&format!("{}/split.json", self.app_config.last_split_path));
                }

                send_message("reloadall");
            }
        });
    }
}

#[cfg(unix)]
pub fn send_message(msg: &str) {
    println!("Sending message: {}", msg);
    if let Ok(mut stream) = UnixStream::connect("/tmp/openspeedrun.sock") {
        use std::io::Write;
        let msg = format!("{msg}\n");
        if let Err(e) = stream.write_all(msg.as_bytes()) {
            eprintln!("⚠️ Failed to write: {}", e);
        } else if let Err(e) = stream.flush() {
            eprintln!("⚠️ Failed to flush: {}", e);
        } else {
            println!("✅ Message sent: {}", msg.trim());
        }
    } else {
        eprintln!("⚠️ Failed to connect to socket");
    }
}

#[cfg(windows)]
pub fn send_message(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::{BufWriter, Write};
    use std::thread::sleep;
    use std::time::Duration;

    let pipe_path = r"\\.\pipe\openspeedrun";

    for _ in 0..5 {
        let file = OpenOptions::new().write(true).open(pipe_path);

        match file {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let full_msg = format!("{msg}\n");
                if let Err(e) = writer.write_all(full_msg.as_bytes()) {
                    eprintln!("⚠️ Failed to write to pipe: {}", e);
                }
                if let Err(e) = writer.flush() {
                    eprintln!("⚠️ Failed to flush pipe: {}", e);
                }
                println!("✅ Message sent: {}", msg.trim());
                return;
            }
            Err(_) => {
                sleep(Duration::from_millis(100));
            }
        }
    }

    eprintln!("⚠️ Could not connect to pipe: {}", pipe_path);
}

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();
    options.viewport = ViewportBuilder::default().with_inner_size(egui::vec2(850.0, 650.0));

    eframe::run_native(
        "OpenSpeedRun Config",
        options,
        Box::new(|_cc| Ok(Box::new(ConfigApp::new()))),
    )
}
