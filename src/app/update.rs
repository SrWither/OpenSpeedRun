use crate::app::AppWrapper;
use crate::app::resize::draw_resize_borders;
use crate::app::state::AppState;
use crate::config::load::config_base_dir;
use crate::config::shaders::ShaderBackground;
use crate::core::timer::TimerState;
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
            self.start_timers();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::P)) {
            self.pause_timers();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.timer.reset();
            self.reset_splits();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.split();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            if let Err(e) = self.save_comparisons() {
                eprintln!("Error saving comparisons: {}", e);
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            self.undo_pb();
        }

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.undo_split();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::L)) {
            self.toggle_igt_pause();
        }

        if ctx.input(|i| i.key_pressed(egui::Key::C)) {
            self.cycle_comparison();
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

    pub fn draw_ui(&mut self, ui: &mut egui::Ui) {
        self.draw_header(ui);
        if self.layout.options.show_footer {
            self.draw_footer(ui);
        }
        if self.layout.options.show_body {
            self.draw_splits_panel(ui);
        }
        self.draw_help_window(ui.ctx());
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
                    let screen_rect = ctx.content_rect();
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

    fn apply_transparency_if_needed(&self, ctx: &egui::Context, app: &mut AppState) {
        if (app.layout.options.enable_background_image || app.layout.options.enable_shader)
            && !app.transparent_set
        {
            let mut style = (*ctx.global_style()).clone();
            style.visuals.window_fill = Color32::TRANSPARENT;
            style.visuals.extreme_bg_color = Color32::TRANSPARENT;
            style.visuals.panel_fill = Color32::TRANSPARENT;
            style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.active.bg_fill = Color32::TRANSPARENT;
            style.visuals.widgets.hovered.bg_fill = Color32::TRANSPARENT;
            style.visuals.window_stroke = egui::Stroke::NONE;
            style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
            ctx.set_global_style(style);

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
        timer_state: i32,
        attempt_count: i32,
        is_gold_split: i32,
        is_new_pb: i32,
        igt_time: f32,
        igt_paused: i32,
        live_delta: f32,
        best_possible_time: f32,
        pb_time: f32,
    ) {
        if app.layout.options.enable_shader {
            if let Some(shader) = &mut app.shader {
                let screen = ctx.content_rect();
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
                    timer_state,
                    attempt_count,
                    is_gold_split,
                    is_new_pb,
                    igt_time,
                    igt_paused,
                    live_delta,
                    best_possible_time,
                    pb_time,
                );
            }
        }
    }

    fn draw_ui_and_misc(&self, ui: &mut egui::Ui, app: &mut AppState) {
        app.handle_input(ui.ctx());
        app.draw_ui(ui);

        if !app.layout.options.titlebar {
            draw_resize_borders(ui.ctx());
        }
    }
}

impl eframe::App for AppWrapper {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_commands();
        let mut app = self.app_state.lock().unwrap();

        let ctx = ui.ctx().clone();

        let _ = self.prepare_background(&ctx, &mut app);
        self.load_fonts_if_needed(&ctx, &mut app);

        let elapsed = app.start_time.elapsed().as_secs_f32();
        let delta_time = elapsed - app.last_elapsed;
        app.last_elapsed = elapsed;

        let current_split = app.current_split as i32;
        let total_splits = app.run.splits.len() as i32;
        let elapsed_time = app.timer.current_time().as_seconds_f32();

        let last_split_time = if app.current_split > 0 && (app.current_split as usize) < app.run.splits.len() {
            app.splits_display[app.current_split - 1]
                .last_time
                .map(|t| t.as_seconds_f32())
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let elapsed_split_time = elapsed_time - last_split_time;

        let timer_state = match app.timer.state {
            TimerState::NotStarted => 0,
            TimerState::Running => 1,
            TimerState::Paused => 2,
            TimerState::Ended => 3,
        };
        let attempt_count = app.run.attempts as i32;
        let is_gold_split = app.last_segment_is_gold as i32;
        let is_new_pb = app.last_run_is_pb as i32;
        let igt_time = app.igt_timer.current_time().as_seconds_f32();
        let igt_paused = app.igt_timer.is_paused() as i32;
        let live_delta = app.live_delta(elapsed_split_time);
        let best_possible_time = app.best_possible_time();
        let pb_time = app.pb_time();

        self.apply_transparency_if_needed(&ctx, &mut app);
        self.render_shader_if_enabled(
            &ctx,
            &mut app,
            elapsed,
            delta_time,
            current_split,
            total_splits,
            elapsed_time,
            elapsed_split_time,
            timer_state,
            attempt_count,
            is_gold_split,
            is_new_pb,
            igt_time,
            igt_paused,
            live_delta,
            best_possible_time,
            pb_time,
        );
        self.draw_ui_and_misc(ui, &mut app);

        ctx.request_repaint();
    }
}
