use eframe::glow;
use glow::HasContext;
use std::fs;
use std::path::Path;
use std::sync::Arc;

struct ShaderUniforms {
    time: Option<glow::UniformLocation>,
    resolution: Option<glow::UniformLocation>,
    mouse: Option<glow::UniformLocation>,
    date: Option<glow::UniformLocation>,
    delta_time: Option<glow::UniformLocation>,
    texture: Option<glow::UniformLocation>,
    current_split: Option<glow::UniformLocation>,
    total_splits: Option<glow::UniformLocation>,
    elapsed_time: Option<glow::UniformLocation>,
    elapsed_split_time: Option<glow::UniformLocation>,
}

pub struct ShaderBackground {
    pub gl: Arc<glow::Context>,
    pub program: glow::NativeProgram,
    pub vao: glow::NativeVertexArray,
    pub shader_path: String,
    pub vertex_shader_path: String,
    uniforms: ShaderUniforms,
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

        let (program, vao, uniforms) =
            match Self::init_gl(&gl, &fragment_shader_src, &vertex_shader_src) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("⚠ Failed to initialize shader '{shader_path}':\n{e}");
                    return None;
                }
            };

        Some(Self {
            gl,
            program,
            vao,
            shader_path,
            vertex_shader_path,
            uniforms,
        })
    }

    /// Compiles a shader, returning its info log on failure instead of panicking.
    unsafe fn compile_shader(
        gl: &glow::Context,
        kind: u32,
        src: &str,
    ) -> Result<glow::NativeShader, String> {
        unsafe {
            let shader = gl
                .create_shader(kind)
                .map_err(|e| format!("Failed to create shader: {e}"))?;
            gl.shader_source(shader, src);
            gl.compile_shader(shader);

            if gl.get_shader_compile_status(shader) {
                Ok(shader)
            } else {
                let log = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                Err(log)
            }
        }
    }

    fn init_gl(
        gl: &glow::Context,
        fragment_shader_src: &str,
        vertex_shader_src: &str,
    ) -> Result<(glow::NativeProgram, glow::NativeVertexArray, ShaderUniforms), String> {
        unsafe {
            let vs = Self::compile_shader(gl, glow::VERTEX_SHADER, vertex_shader_src)
                .map_err(|log| format!("Vertex shader failed to compile:\n{log}"))?;

            let fs = match Self::compile_shader(gl, glow::FRAGMENT_SHADER, fragment_shader_src) {
                Ok(fs) => fs,
                Err(log) => {
                    gl.delete_shader(vs);
                    return Err(format!("Fragment shader failed to compile:\n{log}"));
                }
            };

            let program = gl.create_program().map_err(|e| {
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                format!("Failed to create shader program: {e}")
            })?;
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);

            // The shaders are no longer needed once the program is linked.
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(format!("Shader program failed to link:\n{log}"));
            }

            let vao = gl.create_vertex_array().map_err(|e| {
                gl.delete_program(program);
                format!("Failed to create vertex array: {e}")
            })?;
            gl.bind_vertex_array(Some(vao));

            let vertices: [f32; 12] = [
                -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
            ];

            let vbo = gl.create_buffer().map_err(|e| {
                gl.delete_vertex_array(vao);
                gl.delete_program(program);
                format!("Failed to create vertex buffer: {e}")
            })?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            if let Some(a_pos) = gl.get_attrib_location(program, "a_pos") {
                gl.enable_vertex_attrib_array(a_pos);
                gl.vertex_attrib_pointer_f32(a_pos, 2, glow::FLOAT, false, 0, 0);
            } else {
                eprintln!(
                    "⚠ Vertex shader has no 'a_pos' attribute; the fullscreen quad will not be positioned."
                );
            }

            gl.bind_vertex_array(None);

            // Uniform locations only need to be looked up once, right after linking.
            let uniforms = ShaderUniforms {
                time: Self::get_uniform_location_any(gl, program, &["u_time", "time", "iTime"]),
                resolution: Self::get_uniform_location_any(
                    gl,
                    program,
                    &["u_resolution", "resolution", "iResolution"],
                ),
                mouse: Self::get_uniform_location_any(
                    gl,
                    program,
                    &["u_mouse", "mouse", "iMouse"],
                ),
                date: Self::get_uniform_location_any(gl, program, &["u_date", "date", "iDate"]),
                delta_time: Self::get_uniform_location_any(
                    gl,
                    program,
                    &["deltaTime", "u_deltaTime", "iTimeDelta"],
                ),
                texture: Self::get_uniform_location_any(
                    gl,
                    program,
                    &["u_texture", "iChannel0", "image"],
                ),
                current_split: Self::get_uniform_location_any(
                    gl,
                    program,
                    &["current_split", "u_current_split", "iCurrentSplit"],
                ),
                total_splits: Self::get_uniform_location_any(
                    gl,
                    program,
                    &["total_splits", "u_total_splits", "iTotalSplits"],
                ),
                elapsed_time: Self::get_uniform_location_any(
                    gl,
                    program,
                    &["elapsed_time", "u_elapsed_time", "iElapsedTime"],
                ),
                elapsed_split_time: Self::get_uniform_location_any(
                    gl,
                    program,
                    &[
                        "elapsed_split_time",
                        "u_elapsed_split_time",
                        "iElapsedSplitTime",
                    ],
                ),
            };

            Ok((program, vao, uniforms))
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
            let u = &self.uniforms;

            gl.viewport(0, 0, width as i32, height as i32);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));

            gl.uniform_1_f32(u.time.as_ref(), time);
            gl.uniform_2_f32(u.resolution.as_ref(), width, height);
            gl.uniform_2_f32(u.mouse.as_ref(), 0.0, 0.0);

            let (year, month, day, seconds) = date;
            gl.uniform_4_f32(u.date.as_ref(), year as f32, month as f32, day as f32, seconds);

            gl.uniform_1_f32(u.delta_time.as_ref(), delta_time);

            if let Some(tex) = background_gl_texture {
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(*tex));
                gl.uniform_1_i32(u.texture.as_ref(), 0); // sampler2D en unidad 0
            }

            gl.uniform_1_i32(u.current_split.as_ref(), current_split);
            gl.uniform_1_i32(u.total_splits.as_ref(), total_splits);
            gl.uniform_1_f32(u.elapsed_time.as_ref(), elapsed_time);
            gl.uniform_1_f32(u.elapsed_split_time.as_ref(), elapsed_split_time);

            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }
}

impl Drop for ShaderBackground {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
            self.gl.delete_vertex_array(self.vao);
        }
    }
}
