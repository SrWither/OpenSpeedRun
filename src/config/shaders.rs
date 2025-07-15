use eframe::glow;
use glow::HasContext;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub struct ShaderBackground {
    pub gl: Arc<glow::Context>,
    pub program: glow::NativeProgram,
    pub vao: glow::NativeVertexArray,
    pub shader_path: String,
    pub vertex_shader_path: String,
}

impl ShaderBackground {
    pub fn new(
        gl: Arc<glow::Context>,
        shader_path: String,
        vertex_shader_path: String,
    ) -> Option<Self> {
        let shader_exists = Path::new(&shader_path).exists();
        let vertex_exists = Path::new(&vertex_shader_path).exists();

        if !shader_exists || !vertex_exists {
            eprintln!(
                "⚠ Shader files not found:\n  Fragment: {}\n  Vertex: {}\nSkipping shader init.",
                shader_path, vertex_shader_path
            );
            return None;
        }

        let fragment_shader_src = fs::read_to_string(&shader_path).ok()?;
        let vertex_shader_src = fs::read_to_string(&vertex_shader_path).ok()?;

        let (program, vao) = Self::init_gl(&gl, &fragment_shader_src, &vertex_shader_src);

        Some(Self {
            gl,
            program,
            vao,
            shader_path,
            vertex_shader_path,
        })
    }

    fn init_gl(
        gl: &glow::Context,
        fragment_shader_src: &str,
        vertex_shader_src: &str,
    ) -> (glow::NativeProgram, glow::NativeVertexArray) {
        unsafe {
            let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            gl.shader_source(vs, vertex_shader_src);
            gl.compile_shader(vs);
            assert!(gl.get_shader_compile_status(vs), "Vertex shader failed");

            let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            gl.shader_source(fs, fragment_shader_src);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                let log = gl.get_shader_info_log(fs);
                panic!("Fragment shader failed:\n{log}");
            }

            let program = gl.create_program().unwrap();
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            assert!(gl.get_program_link_status(program), "Shader link failed");

            gl.delete_shader(vs);
            gl.delete_shader(fs);

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let vertices: [f32; 12] = [
                -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
            ];

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            let a_pos = gl.get_attrib_location(program, "a_pos").unwrap();
            gl.enable_vertex_attrib_array(a_pos as u32);
            gl.vertex_attrib_pointer_f32(a_pos as u32, 2, glow::FLOAT, false, 0, 0);

            gl.bind_vertex_array(None);

            (program, vao)
        }
    }

    fn get_uniform_location_any<'a>(
        gl: &glow::Context,
        program: glow::NativeProgram,
        names: &[&'a str],
    ) -> Option<glow::UniformLocation> {
        for name in names {
            if let Some(loc) = unsafe { gl.get_uniform_location(program, name) } {
                return Some(loc);
            }
        }
        None
    }

    pub fn render(
        &self,
        time: f32,
        width: f32,
        height: f32,
        date: (i32, i32, i32, f32),
        delta_time: f32,
        background_gl_texture: Option<&glow::NativeTexture>,
        current_split: i32,
        total_splits: i32,
        elapsed_time: f32,
        elapsed_split_time: f32,
    ) {
        unsafe {
            let gl = &*self.gl;

            gl.viewport(0, 0, width as i32, height as i32);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));

            // Pass uniforms to the shader

            if let Some(loc) =
                Self::get_uniform_location_any(gl, self.program, &["u_time", "time", "iTime"])
            {
                gl.uniform_1_f32(Some(&loc), time);
            }

            if let Some(loc) = Self::get_uniform_location_any(
                gl,
                self.program,
                &["u_resolution", "resolution", "iResolution"],
            ) {
                gl.uniform_2_f32(Some(&loc), width, height);
            }

            if let Some(loc) =
                Self::get_uniform_location_any(gl, self.program, &["u_mouse", "mouse", "iMouse"])
            {
                gl.uniform_2_f32(Some(&loc), 0.0, 0.0);
            }

            if let Some(loc) =
                Self::get_uniform_location_any(gl, self.program, &["u_date", "date", "iDate"])
            {
                let (year, month, day, seconds) = date;
                gl.uniform_4_f32(Some(&loc), year as f32, month as f32, day as f32, seconds);
            }

            if let Some(loc) = Self::get_uniform_location_any(
                gl,
                self.program,
                &["deltaTime", "u_deltaTime", "iTimeDelta"],
            ) {
                gl.uniform_1_f32(Some(&loc), delta_time);
            }

            if let Some(tex) = background_gl_texture {
                gl.active_texture(glow::TEXTURE0); // Usa la unidad 0
                gl.bind_texture(glow::TEXTURE_2D, Some(*tex));

                if let Some(loc) = Self::get_uniform_location_any(
                    gl,
                    self.program,
                    &["u_texture", "iChannel0", "image"],
                ) {
                    gl.uniform_1_i32(Some(&loc), 0); // sampler2D en unidad 0
                }
            }

            if let Some(loc) = Self::get_uniform_location_any(
                gl,
                self.program,
                &["current_split", "u_current_split", "iCurrentSplit"],
            ) {
                gl.uniform_1_i32(Some(&loc), current_split);
            }

            if let Some(loc) = Self::get_uniform_location_any(
                gl,
                self.program,
                &["total_splits", "u_total_splits", "iTotalSplits"],
            ) {
                gl.uniform_1_i32(Some(&loc), total_splits);
            }

            if let Some(loc) = Self::get_uniform_location_any(
                gl,
                self.program,
                &["elapsed_time", "u_elapsed_time", "iElapsedTime"],
            ) {
                gl.uniform_1_f32(Some(&loc), elapsed_time);
            }

            if let Some(loc) = Self::get_uniform_location_any(
                gl,
                self.program,
                &[
                    "elapsed_split_time",
                    "u_elapsed_split_time",
                    "iElapsedSplitTime",
                ],
            ) {
                gl.uniform_1_f32(Some(&loc), elapsed_split_time);
            }

            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }
}
