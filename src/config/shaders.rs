use eframe::glow;
use glow::HasContext;
use std::fs;
use std::path::Path;
use std::sync::Arc;

const TIME_NAMES: &[&str] = &["u_time", "time", "iTime"];
const RESOLUTION_NAMES: &[&str] = &["u_resolution", "resolution", "iResolution"];
const MOUSE_NAMES: &[&str] = &["u_mouse", "mouse", "iMouse"];
const DATE_NAMES: &[&str] = &["u_date", "date", "iDate"];
const DELTA_TIME_NAMES: &[&str] = &["deltaTime", "u_deltaTime", "iTimeDelta"];
const TEXTURE_NAMES: &[&str] = &["u_texture", "iChannel0", "image"];
const CURRENT_SPLIT_NAMES: &[&str] = &["current_split", "u_current_split", "iCurrentSplit"];
const TOTAL_SPLITS_NAMES: &[&str] = &["total_splits", "u_total_splits", "iTotalSplits"];
const ELAPSED_TIME_NAMES: &[&str] = &["elapsed_time", "u_elapsed_time", "iElapsedTime"];
const ELAPSED_SPLIT_TIME_NAMES: &[&str] =
    &["elapsed_split_time", "u_elapsed_split_time", "iElapsedSplitTime"];
const TIMER_STATE_NAMES: &[&str] = &["timer_state", "u_timer_state", "iTimerState"];
const ATTEMPT_COUNT_NAMES: &[&str] = &["attempt_count", "u_attempt_count", "iAttemptCount"];
const IS_GOLD_SPLIT_NAMES: &[&str] = &["is_gold_split", "u_is_gold_split", "iGoldSplit"];
const IS_NEW_PB_NAMES: &[&str] = &["is_new_pb", "u_is_new_pb", "iNewPB"];
const IGT_TIME_NAMES: &[&str] = &["igt_time", "u_igt_time", "iGameTime"];
const IGT_PAUSED_NAMES: &[&str] = &["igt_paused", "u_igt_paused", "iGameTimePaused"];
const LIVE_DELTA_NAMES: &[&str] = &["live_delta", "u_live_delta", "iLiveDelta"];
const BEST_POSSIBLE_TIME_NAMES: &[&str] =
    &["best_possible_time", "u_best_possible_time", "iBestPossibleTime"];
const PB_TIME_NAMES: &[&str] = &["pb_time", "u_pb_time", "iPBTime"];

/// Documents a uniform a shader may declare, under any of its accepted names.
pub struct UniformDoc {
    /// Accepted spellings for this uniform; any one of them is picked up.
    pub names: &'static [&'static str],
    pub glsl_type: &'static str,
    pub description: &'static str,
}

/// Reference list of every uniform `ShaderBackground` will bind, shown to
/// users in the shader editor. Kept next to `ShaderUniforms` so the two
/// can't drift apart.
pub const UNIFORM_DOCS: &[UniformDoc] = &[
    UniformDoc {
        names: TIME_NAMES,
        glsl_type: "float",
        description: "Seconds elapsed since the shader started.",
    },
    UniformDoc {
        names: RESOLUTION_NAMES,
        glsl_type: "vec2",
        description: "Size of the render surface, in pixels.",
    },
    UniformDoc {
        names: MOUSE_NAMES,
        glsl_type: "vec2",
        description: "Mouse position. Currently always (0, 0).",
    },
    UniformDoc {
        names: DATE_NAMES,
        glsl_type: "vec4",
        description: "(year, month, day, seconds since midnight).",
    },
    UniformDoc {
        names: DELTA_TIME_NAMES,
        glsl_type: "float",
        description: "Seconds since the previous frame.",
    },
    UniformDoc {
        names: TEXTURE_NAMES,
        glsl_type: "sampler2D",
        description: "Background image texture, when enabled in the theme.",
    },
    UniformDoc {
        names: CURRENT_SPLIT_NAMES,
        glsl_type: "int",
        description: "Index of the current split.",
    },
    UniformDoc {
        names: TOTAL_SPLITS_NAMES,
        glsl_type: "int",
        description: "Total number of splits in the run.",
    },
    UniformDoc {
        names: ELAPSED_TIME_NAMES,
        glsl_type: "float",
        description: "Total elapsed run time, in seconds.",
    },
    UniformDoc {
        names: ELAPSED_SPLIT_TIME_NAMES,
        glsl_type: "float",
        description: "Elapsed time in the current split, in seconds.",
    },
    UniformDoc {
        names: TIMER_STATE_NAMES,
        glsl_type: "int",
        description: "0 = not started, 1 = running, 2 = paused, 3 = ended.",
    },
    UniformDoc {
        names: ATTEMPT_COUNT_NAMES,
        glsl_type: "int",
        description: "Number of attempts made on this run so far.",
    },
    UniformDoc {
        names: IS_GOLD_SPLIT_NAMES,
        glsl_type: "int",
        description: "1 if the most recently completed split beat its Best Segment, else 0. Sticky until the next split.",
    },
    UniformDoc {
        names: IS_NEW_PB_NAMES,
        glsl_type: "int",
        description: "1 if the most recently finished run beat the Personal Best, else 0. Sticky until the next run finishes.",
    },
    UniformDoc {
        names: IGT_TIME_NAMES,
        glsl_type: "float",
        description: "Elapsed in-game (manual) time, in seconds — independent from the real-time clock.",
    },
    UniformDoc {
        names: IGT_PAUSED_NAMES,
        glsl_type: "int",
        description: "1 while the in-game time clock is paused (a load is in progress), else 0.",
    },
    UniformDoc {
        names: LIVE_DELTA_NAMES,
        glsl_type: "float",
        description: "Seconds ahead (negative) or behind (positive) the selected comparison, live-updating through the current split.",
    },
    UniformDoc {
        names: BEST_POSSIBLE_TIME_NAMES,
        glsl_type: "float",
        description: "Sum of every split's Best Segment, in seconds (0 if incomplete).",
    },
    UniformDoc {
        names: PB_TIME_NAMES,
        glsl_type: "float",
        description: "Total Personal Best time, in seconds (0 if not set).",
    },
];

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
    timer_state: Option<glow::UniformLocation>,
    attempt_count: Option<glow::UniformLocation>,
    is_gold_split: Option<glow::UniformLocation>,
    is_new_pb: Option<glow::UniformLocation>,
    igt_time: Option<glow::UniformLocation>,
    igt_paused: Option<glow::UniformLocation>,
    live_delta: Option<glow::UniformLocation>,
    best_possible_time: Option<glow::UniformLocation>,
    pb_time: Option<glow::UniformLocation>,
}

impl ShaderUniforms {
    fn resolve(gl: &glow::Context, program: glow::NativeProgram) -> Self {
        Self {
            time: ShaderBackground::get_uniform_location_any(gl, program, TIME_NAMES),
            resolution: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                RESOLUTION_NAMES,
            ),
            mouse: ShaderBackground::get_uniform_location_any(gl, program, MOUSE_NAMES),
            date: ShaderBackground::get_uniform_location_any(gl, program, DATE_NAMES),
            delta_time: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                DELTA_TIME_NAMES,
            ),
            texture: ShaderBackground::get_uniform_location_any(gl, program, TEXTURE_NAMES),
            current_split: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                CURRENT_SPLIT_NAMES,
            ),
            total_splits: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                TOTAL_SPLITS_NAMES,
            ),
            elapsed_time: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                ELAPSED_TIME_NAMES,
            ),
            elapsed_split_time: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                ELAPSED_SPLIT_TIME_NAMES,
            ),
            timer_state: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                TIMER_STATE_NAMES,
            ),
            attempt_count: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                ATTEMPT_COUNT_NAMES,
            ),
            is_gold_split: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                IS_GOLD_SPLIT_NAMES,
            ),
            is_new_pb: ShaderBackground::get_uniform_location_any(gl, program, IS_NEW_PB_NAMES),
            igt_time: ShaderBackground::get_uniform_location_any(gl, program, IGT_TIME_NAMES),
            igt_paused: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                IGT_PAUSED_NAMES,
            ),
            live_delta: ShaderBackground::get_uniform_location_any(gl, program, LIVE_DELTA_NAMES),
            best_possible_time: ShaderBackground::get_uniform_location_any(
                gl,
                program,
                BEST_POSSIBLE_TIME_NAMES,
            ),
            pb_time: ShaderBackground::get_uniform_location_any(gl, program, PB_TIME_NAMES),
        }
    }
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

        let (program, vao) = match Self::init_gl(&gl, &fragment_shader_src, &vertex_shader_src) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("⚠ Failed to initialize shader '{shader_path}':\n{e}");
                return None;
            }
        };

        let uniforms = ShaderUniforms::resolve(&gl, program);

        Some(Self {
            gl,
            program,
            vao,
            shader_path,
            vertex_shader_path,
            uniforms,
        })
    }

    /// Compiles and links a fragment/vertex shader pair without setting up
    /// any GPU resources beyond the program, so it can be used purely to
    /// validate shader source (e.g. from the shader editor).
    ///
    /// The caller is responsible for deleting the returned program.
    pub fn compile_and_link(
        gl: &glow::Context,
        fragment_shader_src: &str,
        vertex_shader_src: &str,
    ) -> Result<glow::NativeProgram, String> {
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

            Ok(program)
        }
    }

    /// Tries to compile and link a fragment/vertex shader pair, reporting
    /// the compiler's error log on failure. Used by the shader editor to
    /// validate shaders without needing to run the main app.
    pub fn validate(
        gl: &glow::Context,
        fragment_shader_src: &str,
        vertex_shader_src: &str,
    ) -> Result<(), String> {
        let program = Self::compile_and_link(gl, fragment_shader_src, vertex_shader_src)?;
        unsafe { gl.delete_program(program) };
        Ok(())
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
    ) -> Result<(glow::NativeProgram, glow::NativeVertexArray), String> {
        unsafe {
            let program = Self::compile_and_link(gl, fragment_shader_src, vertex_shader_src)?;

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

            Ok((program, vao))
        }
    }

    fn get_uniform_location_any(
        gl: &glow::Context,
        program: glow::NativeProgram,
        names: &[&str],
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
        timer_state: i32,
        attempt_count: i32,
        is_gold_split: i32,
        is_new_pb: i32,
        igt_time: f32,
        igt_paused: i32,
        live_delta: f32,
        best_possible_time: f32,
        pb_time: f32,
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

            gl.uniform_1_i32(u.timer_state.as_ref(), timer_state);
            gl.uniform_1_i32(u.attempt_count.as_ref(), attempt_count);
            gl.uniform_1_i32(u.is_gold_split.as_ref(), is_gold_split);
            gl.uniform_1_i32(u.is_new_pb.as_ref(), is_new_pb);
            gl.uniform_1_f32(u.igt_time.as_ref(), igt_time);
            gl.uniform_1_i32(u.igt_paused.as_ref(), igt_paused);
            gl.uniform_1_f32(u.live_delta.as_ref(), live_delta);
            gl.uniform_1_f32(u.best_possible_time.as_ref(), best_possible_time);
            gl.uniform_1_f32(u.pb_time.as_ref(), pb_time);

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
