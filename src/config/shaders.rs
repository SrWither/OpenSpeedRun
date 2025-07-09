use std::fs;
use std::sync::Arc;
use eframe::glow;
use glow::HasContext;

pub struct ShaderBackground {
    pub gl: Arc<glow::Context>,
    pub program: glow::NativeProgram,
    pub vao: glow::NativeVertexArray,
    pub shader_path: String,
    pub vertex_shader_path: String,
}

impl ShaderBackground {
    pub fn new(gl: Arc<glow::Context>, shader_path: String, vertex_shader_path: String) -> Self {
        let fragment_shader_src = fs::read_to_string(&shader_path)
            .expect("Failed to read fragment shader file");

        let vertex_shader_src = fs::read_to_string(&vertex_shader_path)
            .expect("Failed to read vertex shader file");

        let (program, vao) = Self::init_gl(&gl, &fragment_shader_src, &vertex_shader_src);
        Self { gl, program, vao, shader_path, vertex_shader_path }
    }

    fn init_gl(gl: &glow::Context, fragment_shader_src: &str, vertex_shader_src: &str) -> (glow::NativeProgram, glow::NativeVertexArray) {
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
                -1.0, -1.0, 1.0, -1.0, -1.0, 1.0,
                -1.0,  1.0, 1.0, -1.0, 1.0,  1.0,
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

    pub fn render(&self, time: f32, width: f32, height: f32) {
        unsafe {
            let gl = &*self.gl;

            gl.viewport(0, 0, width as i32, height as i32);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));

            if let Some(loc) = gl.get_uniform_location(self.program, "u_time") {
                gl.uniform_1_f32(Some(&loc), time);
            }
            if let Some(loc) = gl.get_uniform_location(self.program, "u_resolution") {
                gl.uniform_2_f32(Some(&loc), width, height);
            }

            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }
}
