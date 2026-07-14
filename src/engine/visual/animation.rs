use std::collections::HashMap;
use std::rc::Rc;
use glam::Vec2;
use glow::HasContext;
use crate::engine::rendering::shader::Shader;
use crate::engine::visual::texture::{Texture2D, TileMode};
use crate::engine::visual::UVData;
use crate::engine::world::World;

pub enum AtlasOption {
    Direct(Rc<Texture2D>),
    Path(String),
}

pub struct FrameInfo {
    pub(crate) texture_offset: Vec2,
    pub(crate) repeat: u32,
}

pub struct Animation {
    gl: Rc<glow::Context>,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    //these are the dimensions to use for the mesh size (so projected pixel size, NOT atlas image size)
    width: u32,
    height: u32,

    region_size: Vec2,
    region_size_uv: Vec2,

    current_frame: u32,
    current_repeats: u32,
    frame_info: Vec<FrameInfo>,

    fps: f32,
    timer: f32,
    frame_progress: f32,

    playing: bool,
    looping: bool,

    atlas: Rc<Texture2D>,
}

pub struct AnimationManager {
    pub active_animation: String,
    pub finished: bool,
    animations: HashMap<String, Animation>,
}

impl Animation {
    pub fn new(
        atlas: AtlasOption,
        width: u32,
        height: u32,
        fps: f32,
        looping: bool,
        frame_info: Vec<FrameInfo>,
        world: &mut World,
    ) -> Self {
        let gl = world.gl();
        let tm = world.texture_manager();
        let atlas = match atlas {
            AtlasOption::Path(path) => {
                let atlas = tm.load(gl, &path)
                    .expect(format!("Could not load animation atlas texture at {path}").as_str());
                atlas
            },
            AtlasOption::Direct(texture) => {
                texture
            }
        };


        let vertices: [f32; 16];
        let region_size = Vec2::new(
            width as f32,
            height as f32
        );
        let region_size_uv = Vec2::new(
            width as f32 / atlas.width() as f32,
            height as f32 / atlas.height() as f32,
        );

        {
            let width = width as f32;
            let height = height as f32;

            let u_max = width / region_size.x;
            let v_max = height / region_size.y;

            vertices = [
                width, height, u_max, v_max,  //TR
                width, 0.0,    u_max, 0.0,    //BR
                0.0,   0.0,    0.0,   0.0,    //BL
                0.0,   height, 0.0,   v_max   //TL
            ];
        }

        let indices: [u16; 6] = [0, 1, 3, 1, 2, 3];
        let gl = world.gl();

        let (vao, vbo, ebo) = unsafe {
            let vao = gl.create_vertex_array().map_err(|e| format!("Unable to create VAO: {}", e)).unwrap();
            let vbo = gl.create_buffer().map_err(|e| format!("Unable to create VBO: {}", e)).unwrap();
            let ebo = gl.create_buffer().map_err(|e| format!("Unable to create EBO: {}", e)).unwrap();

            gl.bind_vertex_array(Some(vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices),
                glow::STATIC_DRAW,
            );

            let stride = (4 * size_of::<f32>()) as i32;

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);

            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 2 * size_of::<f32>() as i32);

            gl.bind_vertex_array(None);

            (vao, vbo, ebo)
        };

        let mut frame_info_new = Vec::new();

        for info in frame_info {
            let texture_offset = Vec2::new(
                info.texture_offset.x * width as f32 / atlas.width() as f32,
                info.texture_offset.y * height as f32 / atlas.height() as f32,
            );

            frame_info_new.push(FrameInfo {
                texture_offset,
                repeat: info.repeat,
            });
        }

        Animation {
            gl: gl.clone(),
            vao, vbo, ebo,
            width, height,
            region_size,
            region_size_uv,
            current_frame: 0,
            current_repeats: 1,
            frame_info: frame_info_new,
            fps,
            timer: 0.0,
            frame_progress: 0.0,
            playing: false,
            looping,
            atlas,
        }
    }

    pub fn play(&mut self, start_frame: u32) {
        self.playing = true;
        self.timer = 0.0;
        self.frame_progress = 0.0;
        self.current_frame = start_frame;
        self.current_repeats = 1;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.frame_progress = self.timer;
    }

    pub fn resume(&mut self) {
        self.playing = true;
    }

    pub fn update(&mut self, delta_time: f32) -> bool {
        if !self.playing {
            return false;
        }

        let mut finished = false;

        self.timer += delta_time;
        let spf = 1.0 / self.fps;

        while self.timer >= spf {
            if self.frame_info[self.current_frame as usize].repeat > self.current_repeats {
                self.current_repeats += 1;
            } else {
                self.current_frame += 1;
                self.current_repeats = 1;

                if self.current_frame >= self.frame_info.len() as u32
                {
                    if self.looping {
                        self.current_frame = 0;
                        self.current_repeats = 1;
                        finished = true;
                    } else {
                        self.stop();
                        return true;
                    }
                }
            }
            //println!("Time elapsed on frame change: {}", self.timer);
            self.timer -= spf;
        }

        finished
    }

    pub fn draw(&self, shader: &Shader) {
        let frame = &self.frame_info[self.current_frame as usize];

        unsafe {
            shader.set_int("uTexture", 0);
            shader.set_vec2("uTexOffset", &frame.texture_offset);
            shader.set_vec2("uRegionSize", &self.region_size_uv);
            self.atlas.bind(0);
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
            self.gl.bind_vertex_array(None);
        }
    }
}

impl Drop for Animation {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_buffer(self.ebo);
            self.gl.delete_vertex_array(self.vao);
        }
    }
}

impl AnimationManager {
    pub fn new() -> Self {
        AnimationManager {
            active_animation: "".to_string(),
            finished: false,
            animations: HashMap::new(),
        }
    }

    pub fn add_animation(&mut self, name: &str, animation: Animation) {
        self.animations.insert(name.to_string(), animation);
    }

    pub fn update(&mut self, delta_time: f32) {
        self.finished = if let Some(animation) = self.animations.get_mut(&self.active_animation) {
            animation.update(delta_time)
        } else {
            false
        };
    }

    pub fn draw(&self, shader: &Shader) {
        if let Some(animation) = self.animations.get(&self.active_animation) {
            animation.draw(shader);
        }
    }

    pub fn set_animation(&mut self, name: &str) {
        let changed = if self.active_animation != name {
            if let Some(current) = self.animations.get_mut(self.active_animation.as_str()) {
                current.stop();
            }
            true
        } else {
            false
        };

        if let Some(animation) = self.animations.get_mut(name) {
            if changed {
                animation.play(0);
            }
            self.active_animation = name.to_string();
        }
    }

    pub fn play(&mut self, start_frame: u32) {
        if let Some(animation) = self.animations.get_mut(self.active_animation.as_str()) {
            animation.play(start_frame);
        }
    }

    pub fn stop(&mut self) {
        if let Some(animation) = self.animations.get_mut(self.active_animation.as_str()) {
            animation.stop();
        }
    }

    pub fn resume(&mut self) {
        if let Some(animation) = self.animations.get_mut(self.active_animation.as_str()) {
            animation.resume();
        }
    }
    
    pub fn is_playing(&self) -> bool {
        if let Some(animation) = self.animations.get(self.active_animation.as_str()) {
            animation.playing
        } else {
            false
        }
    }
}