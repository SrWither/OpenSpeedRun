use eframe::egui;
use openspeedrun::config::layout::LayoutConfig;
use std::path::PathBuf;

pub struct ThemeEditor {
    pub current_theme_path: PathBuf,
    pub layout: LayoutConfig,
}

impl ThemeEditor {
    pub fn new(theme_path: PathBuf) -> Self {
        let layout = LayoutConfig::load_or_default(theme_path.to_str().unwrap_or_default());
        Self {
            current_theme_path: theme_path,
            layout,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Edit theme");

        ui.add(egui::Slider::new(&mut self.layout.font_size, 10.0..=64.0).text("Font Size"));

        ui.label("Background Color:");
        if color_edit(ui, &mut self.layout.background_color) {}

        ui.label("Text Color:");
        if color_edit(ui, &mut self.layout.text_color) {}

        ui.checkbox(&mut self.layout.show_title, "Show title");
        ui.checkbox(&mut self.layout.show_category, "Show category");
        ui.checkbox(&mut self.layout.show_splits, "Show splits");
        ui.checkbox(&mut self.layout.titlebar, "Titlebar");
        ui.label("Default Window Size:");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.layout.window_size.0).speed(1.0));
            ui.label("x");
            ui.add(egui::DragValue::new(&mut self.layout.window_size.1).speed(1.0));
        });

        if ui.button("Save Changes").clicked() {
            if let Err(e) = self.layout.save(self.current_theme_path.to_str().unwrap()) {
                eprintln!("Error saving theme: {}", e);
            }
        }
    }
}

fn color_edit(ui: &mut egui::Ui, hex_color: &mut String) -> bool {
    let mut color = egui::Color32::from_hex(hex_color).unwrap_or(egui::Color32::WHITE);
    let mut changed = false;

    ui.horizontal(|ui| {
        changed |= ui.color_edit_button_srgba(&mut color).changed();
        ui.label(hex_color.as_str());
    });

    if changed {
        *hex_color = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
    }

    changed
}
