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
use egui::{Color32, FontData, FontDefinitions, FontFamily};

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

impl AppWrapper {
    fn handle_commands(&mut self) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                UICommand::ReloadShader => {
                    let (gl_opt, shader_name) = {
                        let state = self.app_state.lock().unwrap();
                        (state.gl.clone(), state.layout.colors.shader_path.clone())
                    };

                    if let Some(gl) = gl_opt {
                        let shader_path = config_base_dir().join("shaders").join(&shader_name);
                        let vertex_path = config_base_dir()
                            .join("shaders")
                            .join(format!("{}.vert", shader_name));

                        if let Some(shader) = ShaderBackground::new(
                            gl.clone(),
                            shader_path.to_string_lossy().to_string(),
                            vertex_path.to_string_lossy().to_string(),
                        ) {
                            self.app_state.lock().unwrap().shader = Some(shader);
                        } else {
                            eprintln!("Error: no se pudo recargar el shader '{}'", shader_name);
                        }
                    } else {
                        eprintln!("No OpenGL context available to reload shader");
                    }
                }
            }
        }
    }

    fn prepare_background(
        &self,
        ctx: &egui::Context,
        app: &mut AppState,
    ) -> Option<egui::TextureHandle> {
        if app.layout.options.enable_shader || app.layout.options.enable_background_image {
            let tex = app.get_or_load_background_image(ctx);

            if app.layout.options.enable_background_image {
                if let Some(tex) = &tex {
                    let screen_rect = ctx.screen_rect();
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    painter.image(
                        tex.id(),
                        screen_rect,
                        egui::Rect::from_min_max([0.0, 0.0].into(), [1.0, 1.0].into()),
                        egui::Color32::WHITE,
                    );
                }
            }

            tex
        } else {
            None
        }
    }

    fn load_fonts_if_needed(&self, ctx: &egui::Context, app: &mut AppState) {
        if !app.fonts_loaded {
            let mut fonts = FontDefinitions::default();

            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

            let font_default = fonts.clone();

            self.load_custom_fonts_into(&mut fonts);

            let font_name = app.layout.font_sizes.font.clone();

            if let Some(font_name) = font_name {
                self.select_font_from_definitions(ctx, app, &font_name, fonts);
            } else {
                ctx.set_fonts(font_default.clone());
                app.loaded_fonts = Some(font_default);
            }

            app.fonts_loaded = true;
        }
    }

    fn apply_transparency_if_needed(&self, ctx: &egui::Context, app: &mut AppState) {
        if (app.layout.options.enable_background_image || app.layout.options.enable_shader)
            && !app.transparent_set
        {
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
    }

    fn render_shader_if_enabled(
        &self,
        ctx: &egui::Context,
        app: &mut AppState,
        elapsed: f32,
        delta_time: f32,
        current_split: i32,
        total_splits: i32,
        elapsed_time: f32,
        elapsed_split_time: f32,
    ) {
        if app.layout.options.enable_shader {
            if let Some(shader) = &mut app.shader {
                let screen = ctx.screen_rect();
                let scale = ctx.native_pixels_per_point().unwrap_or(1.0);
                let (w, h) = (screen.width() * scale, screen.height() * scale);

                let now = chrono::Local::now();
                let date = (
                    now.year(),
                    now.month() as i32,
                    now.day() as i32,
                    (now.hour() * 3600 + now.minute() * 60 + now.second()) as f32,
                );

                shader.render(
                    elapsed,
                    w,
                    h,
                    date,
                    delta_time,
                    app.background_gl_texture.as_ref(),
                    current_split,
                    total_splits,
                    elapsed_time,
                    elapsed_split_time,
                );
            }
        }
    }

    fn draw_ui_and_misc(&self, ctx: &egui::Context, app: &mut AppState) {
        app.handle_input(ctx);
        app.draw_ui(ctx);

        if !app.layout.options.titlebar {
            draw_resize_borders(ctx);
        }
    }

    fn load_custom_fonts_into(&self, fonts: &mut FontDefinitions) {
        let fonts_dir = config_base_dir().join("fonts");

        if !fonts_dir.exists() {
            let _ = std::fs::create_dir_all(&fonts_dir);
            return;
        }

        if let Ok(entries) = std::fs::read_dir(&fonts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf") {
                        if let Ok(font_data) = std::fs::read(&path) {
                            if let Some(font_name) = path.file_stem().and_then(|s| s.to_str()) {
                                let font_id = font_name.to_string();
                                fonts.font_data.insert(
                                    font_id.clone(),
                                    FontData::from_owned(font_data).into(),
                                );
                                fonts
                                    .families
                                    .get_mut(&FontFamily::Proportional)
                                    .unwrap()
                                    .insert(0, font_id.clone());
                                fonts
                                    .families
                                    .get_mut(&FontFamily::Monospace)
                                    .unwrap()
                                    .insert(0, font_id.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    fn select_font_from_definitions(
        &self,
        ctx: &egui::Context,
        app: &mut AppState,
        font_name: &str,
        mut fonts: FontDefinitions,
    ) {
        if fonts.font_data.contains_key(font_name) {
            if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
                prop.retain(|f| f != font_name);
                prop.insert(0, font_name.to_string());
            }

            if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
                mono.retain(|f| f != font_name);
                mono.insert(0, font_name.to_string());
            }

            ctx.set_fonts(fonts.clone());
            app.loaded_fonts = Some(fonts);
            println!("Selected Font: {}", font_name);
        } else {
            eprintln!("Font '{}' is not loaded", font_name);
            ctx.set_fonts(fonts);
        }
    }
}

impl eframe::App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_commands();
        let mut app = self.app_state.lock().unwrap();

        let _ = self.prepare_background(ctx, &mut app);
        self.load_fonts_if_needed(ctx, &mut app);

        let elapsed = app.start_time.elapsed().as_secs_f32();
        let delta_time = elapsed - app.last_elapsed;
        app.last_elapsed = elapsed;

        let current_split = app.current_split as i32;
        let total_splits = app.run.splits.len() as i32;
        let elapsed_time = app.timer.current_time().as_seconds_f32();

        let last_split_time = if app.current_split > 0 {
            app.splits_display[app.current_split - 1]
                .last_time
                .map(|t| t.as_seconds_f32())
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let elapsed_split_time = elapsed_time - last_split_time;

        self.apply_transparency_if_needed(ctx, &mut app);
        self.render_shader_if_enabled(
            ctx,
            &mut app,
            elapsed,
            delta_time,
            current_split,
            total_splits,
            elapsed_time,
            elapsed_split_time,
        );
        self.draw_ui_and_misc(ctx, &mut app);

        ctx.request_repaint();
    }
}
