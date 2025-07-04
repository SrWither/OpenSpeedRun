use eframe::egui;
use openspeedrun::config::layout::LayoutConfig;
use std::path::PathBuf;

use crate::send_message;

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
        ui.heading("🎨 Edit Theme");
        ui.add_space(12.0);
        if ui.button("💾 Save Changes").clicked() {
            if let Err(e) = self.layout.save(self.current_theme_path.to_str().unwrap()) {
                eprintln!("Error saving theme: {}", e);
            }
            send_message("reloadtheme");
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

                    ui.label("Window size:");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.layout.options.window_size.0).speed(1.0));
                        ui.label("x");
                        ui.add(egui::DragValue::new(&mut self.layout.options.window_size.1).speed(1.0));
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
