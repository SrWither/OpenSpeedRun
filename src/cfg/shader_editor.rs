use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::dialog::PendingDialog;
use crate::style;
use crate::syntax;
use eframe::egui;
use eframe::glow;
use egui::RichText;
use openspeedrun::config::layout::LayoutConfig;
use openspeedrun::config::shaders::{
    ShaderBackground, UNIFORM_DOCS, load_shader_channels, save_shader_channels,
};

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

/// Result of the last GL compile/link check run against the edited source.
enum CheckStatus {
    /// Not checked yet, or no GL context is available to check with.
    Unchecked,
    Ok,
    Error(String),
}

pub struct ShaderEditor {
    pub path: PathBuf,
    code_frag: String,
    code_vert: String,
    error: Option<String>,
    saved_message: Option<String>,
    check_status: CheckStatus,
    dirty: bool,

    readonly: bool,
    gl: Option<Arc<glow::Context>>,

    show_new_popup: bool,
    new_shader_name: String,
    show_uniform_help: bool,

    show_channels_popup: bool,
    /// Slot index + in-flight native file dialog for a "Load Image" click
    /// inside the channels popup.
    pending_channel_pick: Option<(usize, PendingDialog)>,
    /// This shader's own channel configuration — a property of the shader
    /// file (persisted to its `.channels.json` sidecar), not of whichever
    /// theme happens to select it.
    channels: Vec<Option<String>>,
}

impl ShaderEditor {
    pub fn new(path: PathBuf, gl: Option<Arc<glow::Context>>) -> Self {
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

        let channels = load_shader_channels(&path);

        let mut editor = Self {
            path,
            code_frag,
            code_vert,
            error: None,
            saved_message: None,
            check_status: CheckStatus::Unchecked,
            dirty: false,
            readonly,
            gl,
            show_new_popup: false,
            new_shader_name: String::new(),
            show_uniform_help: false,
            show_channels_popup: false,
            pending_channel_pick: None,
            channels,
        };

        if !editor.readonly {
            editor.check();
        }

        editor
    }

    /// Tries to compile and link the current source against the shader
    /// editor's own GL context, without touching the running app.
    fn check(&mut self) {
        let Some(gl) = &self.gl else {
            self.check_status = CheckStatus::Unchecked;
            return;
        };

        self.check_status = match ShaderBackground::validate(gl, &self.code_frag, &self.code_vert) {
            Ok(()) => CheckStatus::Ok,
            Err(e) => CheckStatus::Error(e),
        };
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, layout: &mut LayoutConfig, theme_path: &Path) {
        if self.pending_channel_pick.is_some() {
            // Without this, egui (event-driven) might not repaint again
            // until unrelated input arrives, leaving a finished dialog's
            // result unpicked-up for a while.
            ui.ctx().request_repaint();
        }

        if let Some((slot, dialog)) = &self.pending_channel_pick
            && let Some(path) = dialog.poll()
        {
            if let Some(path) = path {
                match copy_image_to_shaders_folder(&path) {
                    Ok(new_path) => match new_path.file_name().and_then(|n| n.to_str()) {
                        Some(file_name) => {
                            if let Some(entry) = self.channels.get_mut(*slot) {
                                *entry = Some(file_name.to_string());
                            }
                            if let Err(e) = save_shader_channels(&self.path, &self.channels) {
                                self.error = Some(format!("Error saving channels: {e}"));
                            } else {
                                crate::send_message("reloadshader");
                            }
                        }
                        None => eprintln!("Error obtaining file name from path: {:?}", new_path),
                    },
                    Err(_) => eprintln!("Error copying image to shaders folder"),
                }
            }
            self.pending_channel_pick = None;
        }

        // syntax highlighting theme and layouter
        let syntax_set = syntax::load_syntax_set();
        let theme = syntax::get_theme("base16-eighties.dark");

        let mut layouter_frag =
            move |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                let highlighted = syntax::highlight_glsl_lines(text.as_str(), syntax_set, theme);
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = wrap_width;

                for (style, segment) in highlighted {
                    let color = egui::Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    job.append(
                        segment,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(14.0),
                            color,
                            ..Default::default()
                        },
                    );
                }

                ui.fonts_mut(|f| f.layout_job(job))
            };

        let mut layouter_vert =
            move |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                let highlighted = syntax::highlight_glsl_lines(text.as_str(), syntax_set, theme);
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = wrap_width;

                for (style, segment) in highlighted {
                    let color = egui::Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    job.append(
                        segment,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(14.0),
                            color,
                            ..Default::default()
                        },
                    );
                }

                ui.fonts_mut(|f| f.layout_job(job))
            };

        ui.horizontal(|ui| {
            if ui
                .button(format!("{} New Shader", egui_phosphor::regular::PLUS))
                .clicked()
            {
                self.new_shader_name.clear();
                self.show_new_popup = true;
            }
            let save_button =
                egui::Button::new(format!("{} Save Both", egui_phosphor::regular::FLOPPY_DISK));
            if style::accent_button(ui, save_button).clicked() {
                let path_vert = self.path.with_extension(format!(
                    "{}{}",
                    self.path.extension().unwrap_or_default().to_string_lossy(),
                    ".vert"
                ));

                let res_frag = fs::write(&self.path, &self.code_frag);
                let res_vert = fs::write(&path_vert, &self.code_vert);
                match (res_frag, res_vert) {
                    (Ok(_), Ok(_)) => {
                        // The "Active shader" selector above only updates
                        // `layout` in memory — persist it here too, so
                        // picking a different shader and hitting Save Both
                        // doesn't silently leave the old selection on disk.
                        match layout.save(theme_path.to_string_lossy().as_ref()) {
                            Ok(()) => {
                                self.error = None;
                                self.saved_message = Some("Saved".to_string());
                            }
                            Err(e) => {
                                self.error = Some(format!("Error saving theme: {e}"));
                                self.saved_message = None;
                            }
                        }
                        self.dirty = false;
                        // The running app only re-reads `shader_path` off
                        // disk on "reloadtheme" — "reloadshader" alone just
                        // recompiles whichever shader it already has in
                        // memory, so it must come after, not instead of.
                        crate::send_message("reloadtheme");
                        crate::send_message("reloadshader");
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        self.error = Some(format!("Error saving shaders: {e}"));
                        self.saved_message = None;
                    }
                }
            }
            if ui
                .add_enabled(
                    self.gl.is_some(),
                    egui::Button::new(format!(
                        "{} Check Shader",
                        egui_phosphor::regular::CHECK_CIRCLE
                    )),
                )
                .on_disabled_hover_text("No GL context available to check with")
                .clicked()
            {
                self.check();
            }
            ui.toggle_value(
                &mut self.show_uniform_help,
                format!("{} Uniforms", egui_phosphor::regular::BOOK_OPEN),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Active shader:");
            let available_shaders = list_available_shaders();
            let mut current_shader = layout.colors.shader_path.clone();

            egui::ComboBox::from_id_salt("shader_select")
                .selected_text(&current_shader)
                .show_ui(ui, |ui| {
                    for shader in available_shaders {
                        if ui
                            .selectable_label(current_shader == shader, &shader)
                            .clicked()
                        {
                            current_shader = shader.clone();
                            layout.colors.shader_path = shader;
                        }
                    }
                });

            ui.separator();

            if ui.button("Manage Channels...").clicked() {
                self.show_channels_popup = true;
            }
            ui.label(format!("{} channel(s) configured", self.channels.len()));
        });

        ui.separator();

        ui.horizontal(|ui| {
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

            if self.dirty {
                ui.label(RichText::new("● Unsaved changes").color(style::WARNING));
            }

            match &self.check_status {
                CheckStatus::Unchecked => {
                    if self.gl.is_none() {
                        ui.label(
                            RichText::new("Cannot check: no GL context").color(style::TEXT_MUTED),
                        );
                    }
                }
                CheckStatus::Ok => {
                    ui.label(
                        RichText::new(format!(
                            "{} Shader compiles",
                            egui_phosphor::regular::CHECK_CIRCLE
                        ))
                        .color(style::SUCCESS),
                    );
                }
                CheckStatus::Error(_) => {
                    ui.label(
                        RichText::new(format!(
                            "{} Shader has errors",
                            egui_phosphor::regular::X_CIRCLE
                        ))
                        .color(style::ERROR)
                        .strong(),
                    );
                }
            }
        });

        if self.show_uniform_help {
            style::section_card(
                ui,
                "Available uniforms",
                egui_phosphor::regular::BOOK_OPEN,
                |ui| {
                    ui.label(
                        RichText::new(
                            "Any one of the listed names works; pick whichever convention you prefer.",
                        )
                        .small()
                        .weak(),
                    );
                    ui.add_space(4.0);

                    egui::Grid::new("uniform_docs_grid")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Names").strong());
                            ui.label(RichText::new("Type").strong());
                            ui.label(RichText::new("Description").strong());
                            ui.end_row();

                            for doc in UNIFORM_DOCS {
                                ui.label(RichText::new(doc.names.join(" / ")).monospace());
                                ui.label(RichText::new(doc.glsl_type).monospace());
                                ui.label(doc.description);
                                ui.end_row();
                            }
                        });
                },
            );
            ui.add_space(6.0);
        }

        if let CheckStatus::Error(err) = &self.check_status {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(40, 18, 18))
                .stroke(egui::Stroke::new(1.0_f32, style::ERROR))
                .corner_radius(8)
                .inner_margin(style::SPACE_MD)
                .show(ui, |ui| {
                    ui.label(RichText::new(err).color(style::ERROR).monospace());
                });
            ui.add_space(6.0);
        }

        let editor_height = (ui.available_height() - 40.0).max(200.0);
        let column_width = (ui.available_width() - style::SPACE_MD) / 2.0;

        ui.horizontal(|ui| {
            ui.allocate_ui(egui::vec2(column_width, editor_height + 30.0), |ui| {
                ui.vertical(|ui| {
                    ui.label("Fragment Shader (.glsl)");
                    let changed = ui
                        .add_enabled_ui(!self.readonly, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("frag_editor_scroll")
                                .max_height(editor_height)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.code_frag)
                                            .font(egui::TextStyle::Monospace)
                                            .code_editor()
                                            .desired_rows(20)
                                            .desired_width(column_width)
                                            .layouter(&mut layouter_frag),
                                    )
                                    .changed()
                                })
                                .inner
                        })
                        .inner;
                    if changed {
                        self.dirty = true;
                        self.saved_message = None;
                        self.check();
                    }
                });
            });

            ui.allocate_ui(egui::vec2(column_width, editor_height + 30.0), |ui| {
                ui.vertical(|ui| {
                    ui.label("Vertex Shader (.glsl.vert)");
                    let changed = ui
                        .add_enabled_ui(!self.readonly, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("vert_editor_scroll")
                                .max_height(editor_height)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.code_vert)
                                            .font(egui::TextStyle::Monospace)
                                            .code_editor()
                                            .desired_rows(20)
                                            .desired_width(column_width)
                                            .layouter(&mut layouter_vert),
                                    )
                                    .changed()
                                })
                                .inner
                        })
                        .inner;
                    if changed {
                        self.dirty = true;
                        self.saved_message = None;
                        self.check();
                    }
                });
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
                                        self.readonly = false;
                                        self.dirty = false;
                                        self.channels = Vec::new();
                                        self.check();
                                        layout.colors.shader_path = filename_frag.clone();
                                        if let Err(e) =
                                            layout.save(theme_path.to_string_lossy().as_ref())
                                        {
                                            self.error = Some(format!("Error saving theme: {e}"));
                                        }
                                        crate::send_message("reloadtheme");
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

        if self.show_channels_popup {
            egui::Window::new("Shader Channels")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    let channel_images = list_shader_folder_images();
                    let mut remove_idx: Option<usize> = None;
                    let mut changed = false;

                    for i in 0..self.channels.len() {
                        ui.horizontal(|ui| {
                            ui.label(format!("iChannel{i}"));

                            if ui.button("Load Image").clicked() {
                                self.pending_channel_pick = Some((
                                    i,
                                    PendingDialog::spawn(|| {
                                        rfd::FileDialog::new()
                                            .add_filter(
                                                "Images",
                                                &["png", "jpg", "jpeg", "gif", "webp"],
                                            )
                                            .pick_file()
                                    }),
                                ));
                            }

                            let before = self.channels[i].clone();
                            let selected_text = before.as_deref().unwrap_or("None").to_string();

                            egui::ComboBox::from_id_salt(format!("shader_channel_popup_{i}"))
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.channels[i], None, "None");
                                    for img in &channel_images {
                                        ui.selectable_value(
                                            &mut self.channels[i],
                                            Some(img.clone()),
                                            img,
                                        );
                                    }
                                });

                            if self.channels[i] != before {
                                changed = true;
                            }

                            if ui
                                .button(egui_phosphor::regular::TRASH)
                                .on_hover_text("Remove channel")
                                .clicked()
                            {
                                remove_idx = Some(i);
                            }
                        });
                    }

                    if let Some(i) = remove_idx {
                        self.channels.remove(i);
                        changed = true;
                    }

                    ui.add_space(6.0);

                    if ui.button("Add Channel").clicked() {
                        self.channels.push(None);
                        changed = true;
                    }

                    if changed {
                        if let Err(e) = save_shader_channels(&self.path, &self.channels) {
                            self.error = Some(format!("Error saving channels: {e}"));
                        } else {
                            crate::send_message("reloadshader");
                        }
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_channels_popup = false;
                        }
                    });
                });
        }

        if let Some(err) = &self.error {
            ui.colored_label(style::ERROR, err);
        } else if let Some(saved) = &self.saved_message {
            style::status_label(ui, saved, false);
        }
    }
}

fn list_available_shaders() -> Vec<String> {
    let shader_dir = crate::config_base_dir().join("shaders");
    if let Ok(entries) = fs::read_dir(shader_dir) {
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

/// Images already living in the `shaders/` folder, i.e. channel textures
/// that travel alongside whichever shader put them there.
fn list_shader_folder_images() -> Vec<String> {
    let shader_dir = crate::config_base_dir().join("shaders");
    let mut files: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(shader_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type()
                && file_type.is_file()
                && let Some(ext) = entry.path().extension()
                && matches!(
                    ext.to_str().unwrap_or("").to_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "gif" | "webp"
                )
                && let Some(file_name) = entry.file_name().to_str()
            {
                files.push(file_name.to_string());
            }
        }
    }

    files
}

/// Copies a picked channel image into the `shaders/` folder, so it travels
/// alongside the shader file rather than living in the shared backgrounds
/// folder.
fn copy_image_to_shaders_folder(image_path: &PathBuf) -> Result<PathBuf, String> {
    let shader_dir = crate::config_base_dir().join("shaders");
    std::fs::create_dir_all(&shader_dir)
        .map_err(|e| format!("Error creating shaders directory: {}", e))?;

    let new_path = shader_dir.join(image_path.file_name().unwrap());
    std::fs::copy(image_path, &new_path).map_err(|e| format!("Error copying image: {}", e))?;

    Ok(new_path)
}
