use eframe::egui;
use crate::app::state::AppState;
use crate::app::AppWrapper;

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
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.undo_pb();
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
        self.draw_footer(ctx);
        self.draw_splits_panel(ctx);
        self.draw_help_window(ctx);
    }
}

impl eframe::App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut app = self.app_state.lock().unwrap();
        app.handle_input(ctx);
        app.draw_ui(ctx);
        ctx.request_repaint();
    }
}