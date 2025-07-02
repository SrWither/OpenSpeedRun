use eframe::egui;
use crate::app::state::AppState;

impl AppState {
    pub fn draw_help_window(&mut self, ctx: &egui::Context) {
        if self.show_help {
            egui::Window::new("Help / Keyboard Shortcuts")
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("⌨ Keyboard Shortcuts");
                        ui.separator();
                        ui.label("[Space] Start");
                        ui.label("[P] Pause");
                        ui.label("[R] Reset");
                        ui.label("[Enter] Split");
                        ui.label("[Ctrl + S] Save PB");
                        ui.label("[Ctrl + Z] Undo last split");
                        ui.label("[<-] Previous page");
                        ui.label("[->] Next page");
                        ui.label("[H] Toggle this help");

                        ui.add_space(10.0);
                        if ui.button("Close").clicked() {
                            self.show_help = false;
                        }
                    });
                });
        }
    }
}
