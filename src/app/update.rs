use crate::app::AppWrapper;
use crate::app::resize::draw_resize_borders;
use crate::app::state::AppState;
use crate::config::load::config_base_dir;
use crate::config::shaders::ShaderBackground;
#[cfg(unix)]
use crate::core::server::UICommand;
#[cfg(windows)]
use crate::core::winserver::UICommand;
use chrono::{Datelike, Timelike};
use eframe::egui;
use egui::Color32;

impl AppState {
    pub fn handle_input(&mut self, ctx: &egui::Context) {
        let total_splits = self.run.splits.len();
        let total_pages = (total_splits + self.splits_per_page - 1) / self.splits_per_page;

        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            let offset = self.run.start_offset.unwrap_or(0);
            self.timer.start_with_offset(offset);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::P)) {
            self.timer.pause();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.timer.reset();
            self.reset_splits();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.split();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            if let Err(e) = self.save_pb() {
                eprintln!("Error saving PB: {}", e);
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            self.undo_pb();
        }

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.undo_split();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if self.current_page > 0 {
                self.current_page -= 1;
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            if self.current_page + 1 < total_pages {
                self.current_page += 1;
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.show_help = !self.show_help;
        }
    }

    pub fn draw_ui(&mut self, ctx: &egui::Context) {
        self.draw_header(ctx);
        if self.layout.options.show_footer {
            self.draw_footer(ctx);
        }
        if self.layout.options.show_body {
            self.draw_splits_panel(ctx);
        }
        self.draw_help_window(ctx);
    }
}

impl eframe::App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                UICommand::ReloadShader => {
                    let (gl_opt, shader_name) = {
                        let state = self.app_state.lock().unwrap();
                        (state.gl.clone(), state.layout.colors.shader_path.clone())
                    };

                    if let Some(gl) = gl_opt {
                        let shader_path = config_base_dir()
                            .join("shaders")
                            .join(&shader_name)
                            .to_string_lossy()
                            .to_string();
                        let vertex_shader_path = config_base_dir()
                            .join("shaders")
                            .join(format!("{}.vert", shader_name))
                            .to_string_lossy()
                            .to_string();

                        if let Some(shader) =
                            ShaderBackground::new(gl.clone(), shader_path, vertex_shader_path)
                        {
                            let mut state = self.app_state.lock().unwrap();
                            state.shader = Some(shader);
                        } else {
                            eprintln!("Error: no se pudo recargar el shader '{}'", shader_name);
                        }
                    } else {
                        eprintln!("No OpenGL context available to reload shader");
                    }
                }
            }
        }

        let mut app = self.app_state.lock().unwrap();

        if !app.fonts_loaded {
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            ctx.set_fonts(fonts);
            app.fonts_loaded = true;
        }

        let elapsed = app.start_time.elapsed().as_secs_f32();

        let delta_time = elapsed - app.last_elapsed;
        app.last_elapsed = elapsed;

        if app.layout.options.enable_shader && !app.transparent_set {
            let mut style = (*ctx.style()).clone();
            style.visuals.window_fill = Color32::TRANSPARENT;
            style.visuals.extreme_bg_color = Color32::TRANSPARENT;
            style.visuals.panel_fill = Color32::TRANSPARENT;
            style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.active.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.hovered.bg_fill = Color32::TRANSPARENT;
            style.visuals.window_stroke = egui::Stroke::NONE;
            style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
            ctx.set_style(style);
            app.transparent_set = true;
        }

        if app.layout.options.enable_shader {
            if let Some(shader) = &mut app.shader {
                let screen = ctx.screen_rect();
                let native_pixels_per_point = ctx.native_pixels_per_point().unwrap_or(1.0);
                let (w, h) = (
                    screen.width() * native_pixels_per_point,
                    screen.height() * native_pixels_per_point,
                );

                let now = chrono::Local::now();
                let date = (
                    now.year(),
                    now.month() as i32,
                    now.day() as i32,
                    (now.hour() * 3600 + now.minute() * 60 + now.second()) as f32,
                );

                shader.render(elapsed, w, h, date, delta_time);
            }
        }

        app.handle_input(ctx);
        app.draw_ui(ctx);
        if !app.layout.options.titlebar {
            draw_resize_borders(ctx);
        }
        ctx.request_repaint();
    }
}
