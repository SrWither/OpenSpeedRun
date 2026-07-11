use std::path::Path;

use eframe::{
    egui::{ColorImage, Context, TextureHandle},
    glow::{self, HasContext, PixelUnpackData},
};
use image::GenericImageView;

use crate::{
    app::state::AppState,
    config::{
        load::config_base_dir,
        shaders::{ChannelTarget, ShaderChannel},
    },
};

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

    pub fn get_or_load_background_image(&mut self, ctx: &Context) -> Option<TextureHandle> {
        let Some(image_name) = &self.layout.colors.background_image else {
            return None;
        };

        if let (Some(current_name), Some(tex)) =
            (&self.background_image_name, &self.background_image)
            && current_name == image_name
        {
            return Some(tex.clone());
        }

        let full_path = config_base_dir().join("backgrounds").join(image_name);
        if !full_path.exists() {
            eprintln!("background image doesn't exist: {:?}", full_path);
            return None;
        }

        let (rgba, size) = Self::load_image_rgba(&full_path)?;

        let texture = Self::create_egui_texture(ctx, "background_image", &rgba, size);
        self.background_image = Some(texture.clone());
        self.background_image_name = Some(image_name.clone());

        if let Some(gl) = &self.gl
            && let Some(native_tex) = Self::create_gl_texture(gl, &rgba, size)
        {
            let replace = match self.background_gl_texture {
                Some(existing) => existing != native_tex,
                None => true,
            };

            if replace {
                self.background_gl_texture = Some(native_tex);
            }
        }

        Some(texture)
    }

    /// Loads (and caches) the GL textures for `shader_channel_paths` (the
    /// current shader's own channel config, not the theme's), in slot
    /// order — `None` for empty/incomplete slots, so the returned `Vec`'s
    /// index lines up with the uniform indices `ShaderUniforms::resolve`
    /// resolved (`iChannel{i}`, etc). Unlike `get_or_load_background_image`,
    /// this never touches the 2D `egui` painter — these channels are
    /// shader-only inputs. Images live in the `shaders/` folder, alongside
    /// the shader file, not `backgrounds/`.
    pub fn get_or_load_shader_channels(
        &mut self,
    ) -> Vec<Option<(glow::NativeTexture, ChannelTarget)>> {
        let Some(gl) = self.gl.clone() else {
            return Vec::new();
        };

        self.shader_channel_paths
            .clone()
            .iter()
            .map(|channel| match channel {
                ShaderChannel::Image(name) => {
                    let name = name.as_ref()?;
                    if let Some(tex) = self.shader_channel_cache.get(name) {
                        return Some((*tex, ChannelTarget::Image));
                    }

                    let full_path = config_base_dir().join("shaders").join(name);
                    let (rgba, size) = Self::load_image_rgba(&full_path)?;
                    let tex = Self::create_gl_texture(&gl, &rgba, size)?;
                    self.shader_channel_cache.insert(name.clone(), tex);
                    Some((tex, ChannelTarget::Image))
                }
                ShaderChannel::Cubemap(faces) => {
                    let cache_key = faces
                        .iter()
                        .map(|f| f.as_deref().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join("\u{1}");

                    if let Some(tex) = self.shader_channel_cache.get(&cache_key) {
                        return Some((*tex, ChannelTarget::Cubemap));
                    }

                    let tex = Self::create_gl_cubemap_texture(&gl, faces)?;
                    self.shader_channel_cache.insert(cache_key, tex);
                    Some((tex, ChannelTarget::Cubemap))
                }
            })
            .collect()
    }

    fn load_image_rgba(path: &Path) -> Option<(Vec<u8>, (u32, u32))> {
        match image::open(path) {
            Ok(img) => {
                let size = img.dimensions();
                let rgba = img.to_rgba8().into_raw();
                Some((rgba, size))
            }
            Err(e) => {
                eprintln!("No se pudo abrir la imagen {:?}: {:?}", path, e);
                None
            }
        }
    }

    fn create_egui_texture(
        ctx: &Context,
        key: &str,
        data: &[u8],
        size: (u32, u32),
    ) -> TextureHandle {
        let color_image =
            ColorImage::from_rgba_unmultiplied([size.0 as usize, size.1 as usize], data);
        ctx.load_texture(key.to_string(), color_image, Default::default())
    }

    fn create_gl_texture(
        gl: &glow::Context,
        data: &[u8],
        size: (u32, u32),
    ) -> Option<glow::NativeTexture> {
        unsafe {
            let tex_id = gl.create_texture().ok()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(tex_id));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                size.0 as i32,
                size.1 as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                PixelUnpackData::Slice(Some(data)),
            );

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );

            Some(tex_id)
        }
    }

    /// Uploads a 6-face cubemap in GL's `POSITIVE_X..NEGATIVE_Z` order
    /// (matching `CUBEMAP_FACE_LABELS`). All 6 faces must be set and load
    /// successfully — a partially-configured cubemap has no coherent
    /// "missing face" fallback, so it's simplest to just not bind anything
    /// until every face is picked.
    fn create_gl_cubemap_texture(
        gl: &glow::Context,
        faces: &[Option<String>; 6],
    ) -> Option<glow::NativeTexture> {
        let mut loaded = Vec::with_capacity(6);
        for face in faces {
            let name = face.as_ref()?;
            let full_path = config_base_dir().join("shaders").join(name);
            loaded.push(Self::load_image_rgba(&full_path)?);
        }

        unsafe {
            let tex_id = gl.create_texture().ok()?;
            gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(tex_id));

            for (i, (rgba, size)) in loaded.iter().enumerate() {
                gl.tex_image_2d(
                    glow::TEXTURE_CUBE_MAP_POSITIVE_X + i as u32,
                    0,
                    glow::RGBA as i32,
                    size.0 as i32,
                    size.1 as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    PixelUnpackData::Slice(Some(rgba)),
                );
            }

            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_CUBE_MAP,
                glow::TEXTURE_WRAP_R,
                glow::CLAMP_TO_EDGE as i32,
            );

            Some(tex_id)
        }
    }
}
