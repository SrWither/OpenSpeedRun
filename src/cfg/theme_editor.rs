use eframe::egui;
#[cfg(windows)]
use openspeedrun::config::keys::KeyWrapper;
use openspeedrun::{config::layout::LayoutConfig, config_base_dir};
use std::path::PathBuf;

use crate::send_message;

pub struct ThemeEditor {
    pub current_theme_path: PathBuf,
    pub layout: LayoutConfig,
    #[cfg(windows)]
    pub waiting_for_key: Option<String>,
}

impl ThemeEditor {
    pub fn new(theme_path: PathBuf) -> Self {
        let layout = LayoutConfig::load_or_default(theme_path.to_str().unwrap_or_default());
        let shader_dir = config_base_dir().join("shaders");
        std::fs::create_dir_all(&shader_dir).ok();

        Self {
            current_theme_path: theme_path,
            layout,
            #[cfg(windows)]
            waiting_for_key: None,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("🎨 Edit Theme");
        ui.add_space(12.0);
        if ui.button("💾 Save Changes").clicked() {
            if let Err(e) = self.layout.save(self.current_theme_path.to_str().unwrap()) {
                eprintln!("Error saving theme: {}", e);
            }
            send_message("reloadtheme");
            if self.layout.options.enable_shader {
                send_message("reloadshader");
            }
        }

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Font Sizes:");
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.title, 10.0..=64.0)
                            .text("Title"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.category, 10.0..=64.0)
                            .text("Category"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.timer, 10.0..=64.0)
                            .text("Timer"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.split, 10.0..=64.0)
                            .text("Split Name"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.split_timer, 10.0..=64.0)
                            .text("Split Timer"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.split_gold, 10.0..=64.0)
                            .text("Split Gold"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.split_pb, 10.0..=64.0)
                            .text("Split PB"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.font_sizes.info, 10.0..=64.0)
                            .text("Info"),
                    );
                })
            });

            ui.add_space(6.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Colors:");

                    color_edit(ui, "Background", &mut self.layout.colors.background);
                    color_edit(ui, "Title", &mut self.layout.colors.title);
                    color_edit(ui, "Category", &mut self.layout.colors.category);
                    color_edit(ui, "Timer", &mut self.layout.colors.timer);
                    color_edit(ui, "Split", &mut self.layout.colors.split);
                    color_edit(ui, "Split Selected", &mut self.layout.colors.split_selected);
                    color_edit(ui, "Split Timer", &mut self.layout.colors.split_timer);
                    color_edit(ui, "Gold +", &mut self.layout.colors.gold_positive);
                    color_edit(ui, "Gold -", &mut self.layout.colors.gold_negative);
                    color_edit(ui, "PB +", &mut self.layout.colors.pb_positive);
                    color_edit(ui, "PB -", &mut self.layout.colors.pb_negative);
                    color_edit(ui, "Info", &mut self.layout.colors.info);
                });
            });

            ui.add_space(6.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Options:");
                    ui.checkbox(&mut self.layout.options.show_title, "Show title");
                    ui.checkbox(&mut self.layout.options.show_category, "Show category");
                    ui.checkbox(&mut self.layout.options.show_splits, "Show splits");
                    ui.checkbox(&mut self.layout.options.show_info, "Show info");
                    ui.checkbox(&mut self.layout.options.show_body, "Show body");
                    ui.checkbox(&mut self.layout.options.show_footer, "Show footer");
                    ui.checkbox(&mut self.layout.options.titlebar, "Titlebar");
                    ui.checkbox(&mut self.layout.options.enable_shader, "Enable shader");

                    ui.label("Window size:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.layout.options.window_size.0).speed(1.0),
                        );
                        ui.label("x");
                        ui.add(
                            egui::DragValue::new(&mut self.layout.options.window_size.1).speed(1.0),
                        );
                    });
                    ui.label("Shader file:");
                    let available_shaders = list_available_shaders();

                    let mut current_shader = self.layout.colors.shader_path.clone();

                    egui::ComboBox::from_id_salt("shader_select")
                        .selected_text(&current_shader)
                        .show_ui(ui, |ui| {
                            for shader in available_shaders {
                                if ui
                                    .selectable_label(current_shader == shader, &shader)
                                    .clicked()
                                {
                                    current_shader = shader.clone();
                                    self.layout.colors.shader_path = shader;
                                }
                            }
                        });
                });
            });

            ui.add_space(6.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Spacings:");
                    ui.add(
                        egui::Slider::new(&mut self.layout.spacings.split_top, 0.0..=64.0)
                            .text("Split Top"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.layout.spacings.split_bottom, 0.0..=64.0)
                            .text("Split Bottom"),
                    );
                });
            });
        });

        #[cfg(windows)]
        {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Hotkeys: (Windows only)");

                    let hotkeys = &mut self.layout.hotkeys;
                    let waiting = &mut self.waiting_for_key;

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                hotkey_button(ui, "Split", &hotkeys.split, "split", waiting);
                                hotkey_button(ui, "Start", &hotkeys.start, "start", waiting);
                                hotkey_button(ui, "Pause", &hotkeys.pause, "pause", waiting);
                                hotkey_button(ui, "Reset", &hotkeys.reset, "reset", waiting);
                                hotkey_button(ui, "Save PB", &hotkeys.save_pb, "save_pb", waiting);
                                hotkey_button(
                                    ui,
                                    "Undo Split",
                                    &hotkeys.undo_split,
                                    "undo_split",
                                    waiting,
                                );
                                hotkey_button(ui, "Undo PB", &hotkeys.undo_pb, "undo_pb", waiting);
                            });

                            ui.vertical(|ui| {
                                hotkey_button(
                                    ui,
                                    "Next Page",
                                    &hotkeys.next_page,
                                    "next_page",
                                    waiting,
                                );
                                hotkey_button(
                                    ui,
                                    "Prev Page",
                                    &hotkeys.prev_page,
                                    "prev_page",
                                    waiting,
                                );
                                hotkey_button(
                                    ui,
                                    "Toggle Help",
                                    &hotkeys.toggle_help,
                                    "toggle_help",
                                    waiting,
                                );
                                hotkey_button(
                                    ui,
                                    "Reload All",
                                    &hotkeys.reload_all,
                                    "reload_all",
                                    waiting,
                                );
                                hotkey_button(
                                    ui,
                                    "Reload Run",
                                    &hotkeys.reload_run,
                                    "reload_run",
                                    waiting,
                                );
                                hotkey_button(
                                    ui,
                                    "Reload Theme",
                                    &hotkeys.reload_theme,
                                    "reload_theme",
                                    waiting,
                                );
                            });
                        })
                    })
                });
            });
        }

        #[cfg(windows)]
        if let Some(action) = self.waiting_for_key.clone() {
            use egui::Event;

            for event in ui.ctx().input(|i| i.raw.events.clone()) {
                if let Event::Key {
                    key, pressed: true, ..
                } = event
                {
                    let key_str = format!("{:?}", key);
                    let key_wrapper = KeyWrapper(key_str);

                    match action.as_str() {
                        "split" => self.layout.hotkeys.split = key_wrapper,
                        "start" => self.layout.hotkeys.start = key_wrapper,
                        "pause" => self.layout.hotkeys.pause = key_wrapper,
                        "reset" => self.layout.hotkeys.reset = key_wrapper,
                        "save_pb" => self.layout.hotkeys.save_pb = key_wrapper,
                        "undo_split" => self.layout.hotkeys.undo_split = key_wrapper,
                        "undo_pb" => self.layout.hotkeys.undo_pb = key_wrapper,
                        "next_page" => self.layout.hotkeys.next_page = key_wrapper,
                        "prev_page" => self.layout.hotkeys.prev_page = key_wrapper,
                        "toggle_help" => self.layout.hotkeys.toggle_help = key_wrapper,
                        "reload_all" => self.layout.hotkeys.reload_all = key_wrapper,
                        "reload_run" => self.layout.hotkeys.reload_run = key_wrapper,
                        "reload_theme" => self.layout.hotkeys.reload_theme = key_wrapper,
                        _ => {}
                    }

                    self.waiting_for_key = None;
                }
            }

            egui::Window::new("Esperando tecla...")
                .collapsible(false)
                .resizable(false)
                .fixed_size((200.0, 60.0))
                .show(ui.ctx(), |ui| {
                    ui.label("Presiona una tecla para asignar.");
                });
        }
    }
}

fn color_edit(ui: &mut egui::Ui, label: &str, hex_color: &mut String) {
    let mut color = egui::Color32::from_hex(hex_color).unwrap_or(egui::Color32::WHITE);
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(label);
        changed |= ui.color_edit_button_srgba(&mut color).changed();
        ui.label(hex_color.as_str());
    });

    if changed {
        *hex_color = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
    }
}

#[cfg(windows)]
fn hotkey_button(
    ui: &mut egui::Ui,
    label: &str,
    key: &KeyWrapper,
    action: &str,
    waiting: &mut Option<String>,
) {
    ui.horizontal(|ui| {
        ui.label(label);

        let button_label = if waiting.as_ref() == Some(&action.to_string()) {
            "Presiona una tecla..."
        } else {
            &key.0
        };

        if ui.button(button_label).clicked() {
            *waiting = Some(action.to_string());
        }
    });
}

fn list_available_shaders() -> Vec<String> {
    let shader_dir = config_base_dir().join("shaders");
    if let Ok(entries) = std::fs::read_dir(shader_dir) {
        entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().map(|e| e == "glsl").unwrap_or(false) {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    }
}
