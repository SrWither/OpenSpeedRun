use egui::{FontData, FontDefinitions, FontFamily};

use crate::{config::load::config_base_dir, AppState, AppWrapper};

impl AppWrapper {
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

    pub fn load_fonts_if_needed(&self, ctx: &egui::Context, app: &mut AppState) {
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
}
