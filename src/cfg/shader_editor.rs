use std::fs;
use std::path::PathBuf;

use eframe::egui;
use egui::RichText;

const DEFAULT_SHADER: &str = r#"
#version 100
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;

void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution;
    gl_FragColor = vec4(uv, abs(sin(u_time)), 1.0);
}
"#;

const DEFAULT_VERTEX_SHADER: &str = r#"
#version 100
attribute vec2 a_pos;
void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

pub struct ShaderEditor {
    pub path: PathBuf,
    code_frag: String,
    code_vert: String,
    error: Option<String>,

    readonly: bool,

    show_new_popup: bool,
    new_shader_name: String,
}

impl ShaderEditor {
    pub fn new(path: PathBuf) -> Self {
        let path_vert = path.with_extension(format!(
            "{}{}",
            path.extension().unwrap_or_default().to_string_lossy(),
            ".vert"
        ));

        let frag_exists = path.exists();
        let vert_exists = path_vert.exists();
        let readonly = !(frag_exists && vert_exists);

        let (code_frag, code_vert) = if frag_exists && vert_exists {
            let code_frag = fs::read_to_string(&path).unwrap_or_default();
            let code_vert = fs::read_to_string(&path_vert).unwrap_or_default();
            (code_frag, code_vert)
        } else {
            (String::new(), String::new())
        };

        Self {
            path,
            code_frag,
            code_vert,
            error: None,
            readonly,
            show_new_popup: false,
            new_shader_name: String::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New Shader").clicked() {
                self.new_shader_name.clear();
                self.show_new_popup = true;
            }
            if ui.button("Save Both").clicked() {
                let path_vert = self.path.with_extension(format!(
                    "{}{}",
                    self.path.extension().unwrap_or_default().to_string_lossy(),
                    ".vert"
                ));

                let res_frag = fs::write(&self.path, &self.code_frag);
                let res_vert = fs::write(&path_vert, &self.code_vert);
                match (res_frag, res_vert) {
                    (Ok(_), Ok(_)) => {
                        self.error = None;
                        crate::send_message("reloadshader");
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        self.error = Some(format!("Error saving shaders: {e}"));
                    }
                }
            }
        });

        ui.separator();
        if !self.readonly {
            ui.label(
                RichText::new(format!(
                    "Editing: {}",
                    self.path.file_name().unwrap_or_default().to_string_lossy()
                ))
                .size(18.0)
                .strong(),
            );
        }

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Fragment Shader (.glsl)");
                let edit = ui.add_enabled_ui(!self.readonly, |ui| {
                    ui.add_sized(
                        egui::vec2(ui.available_width() / 2.0, 400.0),
                        egui::TextEdit::multiline(&mut self.code_frag)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(20),
                    )
                });
                if edit.response.changed() {
                    ui.label("Unsaved changes");
                }
            });

            ui.vertical(|ui| {
                ui.label("Vertex Shader (.glsl.vert)");
                let edit = ui.add_enabled_ui(!self.readonly, |ui| {
                    ui.add_sized(
                        egui::vec2(ui.available_width(), 400.0),
                        egui::TextEdit::multiline(&mut self.code_vert)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(20),
                    )
                });
                if edit.response.changed() {
                    ui.label("Unsaved changes");
                }
            });
        });

        if self.show_new_popup {
            egui::Window::new("New Shader")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Shader name:");
                    ui.text_edit_singleline(&mut self.new_shader_name);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_new_popup = false;
                        }
                        if ui.button("Create").clicked() {
                            let name = self.new_shader_name.trim();
                            if !name.is_empty() {
                                let base = crate::config_base_dir().join("shaders");
                                let filename_frag = format!("{name}.glsl");
                                let filename_vert = format!("{name}.glsl.vert");
                                let path_frag = base.join(&filename_frag);
                                let path_vert = base.join(&filename_vert);

                                if !path_frag.exists() && !path_vert.exists() {
                                    if let Err(e) = fs::write(&path_frag, DEFAULT_SHADER) {
                                        self.error =
                                            Some(format!("Error creating fragment shader: {e}"));
                                    } else if let Err(e) =
                                        fs::write(&path_vert, DEFAULT_VERTEX_SHADER)
                                    {
                                        self.error =
                                            Some(format!("Error creating vertex shader: {e}"));
                                    } else {
                                        self.path = path_frag.clone();
                                        self.code_frag = DEFAULT_SHADER.to_string();
                                        self.code_vert = DEFAULT_VERTEX_SHADER.to_string();
                                        self.error = None;
                                        crate::send_message("reloadshader");
                                    }
                                } else {
                                    self.error = Some("Shader already exists".to_string());
                                }

                                self.show_new_popup = false;
                            }
                        }
                    });
                });
        }

        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, err);
        }
    }
}
