use eframe::egui::{ColorImage, Context, TextureHandle};
use image::GenericImageView;

use crate::app::state::AppState;

impl AppState {
    pub fn get_or_load_texture(&mut self, ctx: &Context, path: &str) -> Option<TextureHandle> {
        let full_path = self.split_base_path.join(path);
        let cache_key = full_path.to_string_lossy().to_string();

        if let Some(tex) = self.textures.get(&cache_key) {
            return Some(tex.clone());
        }

        if let Ok(img) = image::open(&full_path) {
            let size = img.dimensions();
            let rgba = img.to_rgba8().into_raw();
            let color_image =
                ColorImage::from_rgba_unmultiplied([size.0 as usize, size.1 as usize], &rgba);
            let texture = ctx.load_texture(cache_key.clone(), color_image, Default::default());
            self.textures.insert(cache_key, texture.clone());
            Some(texture)
        } else {
            None
        }
    }
}
