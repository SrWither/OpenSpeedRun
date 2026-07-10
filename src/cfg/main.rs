mod history;
mod shader_editor;
mod speedrun_com_picker;
mod split_editor;
mod style;
mod syntax;
mod theme_editor;

use std::fs;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use eframe::egui;
use eframe::glow;
use egui::ViewportBuilder;
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
    split_reload: bool,
    new_name_input: String,
    show_name_input: bool,
    is_creating_theme: bool,
    show_delete_confirm: bool,
    item_to_delete: Option<(String, bool)>,
    gl: Option<Arc<glow::Context>>,
    save_status: Option<(String, bool)>,
}

impl ConfigApp {
    pub fn new(gl: Option<Arc<glow::Context>>) -> Self {
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
            ShaderEditor::new(
                base.join("shaders").join(&layout.colors.shader_path),
                gl.clone(),
            )
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
            split_reload: false,
            new_name_input: String::new(),
            show_name_input: false,
            is_creating_theme: false,
            show_delete_confirm: false,
            item_to_delete: None,
            gl,
            save_status: None,
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
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
        if self.show_delete_confirm
            && let Some((name, is_theme)) = &self.item_to_delete
        {
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

        // Top Tabs
        egui::Panel::top("tabs")
            .frame(egui::Frame {
                fill: style::BG_ELEVATED,
                stroke: egui::Stroke::new(1.0_f32, style::BORDER_SUBTLE),
                inner_margin: egui::Margin::symmetric(style::SPACE_MD as i8, style::SPACE_SM as i8),
                ..Default::default()
            })
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    let tabs: [(&str, &str, bool); 5] = [
                        (egui_phosphor::regular::LIST, "Selector", false),
                        (egui_phosphor::regular::PALETTE, "Themes", false),
                        (egui_phosphor::regular::FLAG_CHECKERED, "Splits", true),
                        (egui_phosphor::regular::CODE, "Shader", false),
                        (
                            egui_phosphor::regular::CLOCK_COUNTER_CLOCKWISE,
                            "History",
                            true,
                        ),
                    ];

                    for (i, (icon, label, reload)) in tabs.into_iter().enumerate() {
                        if ui
                            .selectable_label(self.tab == i, format!("{icon} {label}"))
                            .clicked()
                        {
                            self.tab = i;
                            if reload {
                                self.split_reload = true;
                            }
                        }
                    }
                });
            });

        // Central content. The shader tab manages its own internal scroll
        // areas (one per text editor) so its text editors can be scrolled
        // independently instead of scrolling the whole page around them.
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.tab == 3 {
                if let Some(theme_editor) = &self.theme_editor {
                    let new_path = config_base_dir()
                        .join("shaders")
                        .join(&theme_editor.layout.colors.shader_path);

                    let needs_reload = self
                        .shader_editor
                        .as_ref()
                        .is_none_or(|e| e.path != new_path);

                    if needs_reload {
                        self.shader_editor = Some(ShaderEditor::new(new_path, self.gl.clone()));
                    }
                }

                if let Some(editor) = &mut self.shader_editor {
                    editor.ui(ui);
                } else {
                    ui.label("Select a theme to edit its shader.");
                }
                return;
            }

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
                        if self.split_reload {
                            self.split_editor = Some(SplitEditor::new(
                                config_base_dir()
                                    .join("splits")
                                    .join(self.selected_split.as_ref().unwrap_or(&"".to_string()))
                                    .join("split.json"),
                            ));
                            self.split_reload = false;
                        }

                        if let Some(editor) = &mut self.split_editor {
                            editor.ui(ctx, ui);
                        } else {
                            ui.label("Select a split to edit.");
                        }
                    }
                    4 => {
                        if let Some(split_editor) = &self.split_editor {
                            let needs_reload = self.split_reload
                                || self
                                    .history
                                    .as_ref()
                                    .map(|h| h.run_path != split_editor.run_path)
                                    .unwrap_or(true);

                            if needs_reload {
                                self.history = Some(History::new(split_editor.run_path.clone()));
                                self.split_reload = false;
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
        style::section_card(ui, "Themes", egui_phosphor::regular::PALETTE, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("themes_scroll") // ID único para scroll independiente
                .max_height(160.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for theme in &self.available_themes {
                            let selected = Some(theme) == self.selected_theme.as_ref();
                            let clicked = style::selectable_chip(
                                ui,
                                egui_phosphor::regular::PALETTE,
                                theme,
                                selected,
                            )
                            .clicked();

                            if clicked {
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
                if ui
                    .button(format!("{} New Theme", egui_phosphor::regular::PLUS))
                    .clicked()
                {
                    self.show_name_input_popup(true);
                }
                if ui
                    .button(format!("{} Delete Theme", egui_phosphor::regular::TRASH))
                    .clicked()
                    && let Some(theme) = &self.selected_theme
                {
                    self.show_delete_confirmation(theme.clone(), true);
                }
            });
        });

        ui.add_space(style::SPACE_LG);

        // === SPLITS ===
        style::section_card(ui, "Splits", egui_phosphor::regular::FLAG_CHECKERED, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("splits_scroll") // ID único para scroll independiente
                .max_height(160.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for split in &self.available_splits {
                            let selected = Some(split) == self.selected_split.as_ref();
                            let clicked = style::selectable_chip(
                                ui,
                                egui_phosphor::regular::FLAG_CHECKERED,
                                split,
                                selected,
                            )
                            .clicked();

                            if clicked {
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
                if ui
                    .button(format!("{} New Split", egui_phosphor::regular::PLUS))
                    .clicked()
                {
                    self.show_name_input_popup(false);
                }
                if ui
                    .button(format!("{} Delete Split", egui_phosphor::regular::TRASH))
                    .clicked()
                    && let Some(split) = &self.selected_split
                {
                    self.show_delete_confirmation(split.clone(), false);
                }
            });
        });

        ui.add_space(style::SPACE_LG);

        ui.horizontal(|ui| {
            let save_button = egui::Button::new(format!(
                "{} Save Changes",
                egui_phosphor::regular::FLOPPY_DISK
            ));

            if style::accent_button_sized(ui, save_button, Some(egui::vec2(160.0, 42.0))).clicked()
            {
                self.app_config.save();

                // `app_config.theme`/`last_split_path` are relative (e.g.
                // "themes/mario3.json"), same as everywhere else they're
                // used — they need `config_base_dir()` joined in, unlike
                // `ThemeEditor`/`SplitEditor`'s own Save buttons, which
                // already store a fully-joined path from construction.
                let theme_path = config_base_dir().join(&self.app_config.theme);
                let split_path = config_base_dir()
                    .join(&self.app_config.last_split_path)
                    .join("split.json");

                let theme_result = self
                    .theme_editor
                    .as_ref()
                    .map(|editor| editor.layout.save(theme_path.to_str().unwrap()));
                let split_result = self
                    .split_editor
                    .as_ref()
                    .map(|editor| editor.run.save_to_file(split_path.to_str().unwrap()));

                let error = theme_result
                    .into_iter()
                    .chain(split_result)
                    .find_map(|r| r.err());

                self.save_status = Some(match error {
                    None => ("Saved".to_string(), false),
                    Some(e) => (format!("Error saving: {e}"), true),
                });

                send_message("reloadall");
            }

            if let Some((status, is_error)) = &self.save_status {
                style::status_label(ui, status, *is_error);
            }
        });
    }
}

#[cfg(unix)]
pub fn send_message(msg: &str) {
    println!("Sending message: {}", msg);
    if let Ok(mut stream) = UnixStream::connect(openspeedrun::core::socket_path()) {
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
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: ViewportBuilder::default()
            .with_inner_size(egui::vec2(1360.0, 720.0))
            .with_min_inner_size(egui::vec2(1360.0, 720.0)),
        ..Default::default()
    };

    eframe::run_native(
        "OpenSpeedRun Config",
        options,
        Box::new(|cc| {
            style::apply_style(&cc.egui_ctx);

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(ConfigApp::new(cc.gl.clone())))
        }),
    )
}
