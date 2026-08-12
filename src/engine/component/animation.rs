use std::collections::HashMap;
use std::rc::Rc;
use glam::Vec2;
use crate::engine::rendering::{TextureProvider, TextureRenderData};
use crate::engine::rendering::texture::Texture2D;
use crate::engine::world::World;

/// Defines whether an animation that is being created is going to reference an already instanced
/// Texture2D, or if it is going to have to load it from a path.
pub enum AtlasOption {
    Direct(Rc<Texture2D>),
    Path(String),
}
/// Specifics about a frame of an animation
pub struct FrameInfo {
    /// How many grid spaces in the atlas this animation frame is offset by
    pub(crate) texture_offset: Vec2,
    /// How many times this frame needs to be repeated consecutively when reached.
    /// Equivalent to setting the exact same FrameInfo multiple times, each with repeat = 1.
    pub(crate) repeat: u32,
}
/// Models an animation, composed of same-size frames stored in a single texture atlas.
pub struct Animation {
    //Pixel dimensions of the rendered quad, only informational.
    width: u32,
    height: u32,

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
/// Component used to store multiple animations to the same Entity, allows one to manage many
/// Animation-s as if they were just one.
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
        let tm = world.texture_manager_mut();
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

        let region_size_uv = Vec2::new(
            width as f32 / atlas.width() as f32,
            height as f32 / atlas.height() as f32,
        );

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
            width, height,
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
}

impl TextureProvider for Animation {
    fn get_texture_render_data(&self) -> Option<TextureRenderData> {
        let frame = &self.frame_info[self.current_frame as usize];
        Some(TextureRenderData {
            texture_id: self.atlas.id(),
            tex_offset: frame.texture_offset,
            region_size: self.region_size_uv,
            max_uvs: Vec2::ONE,
        })
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

    /// Adds a new animation to the manager
    pub fn add_animation(&mut self, name: &str, animation: Animation) {
        self.animations.insert(name.to_string(), animation);
    }
    /// Advances the currently active animation, if there is one.
    pub fn update(&mut self, delta_time: f32) {
        self.finished = if let Some(animation) = self.animations.get_mut(&self.active_animation) {
            animation.update(delta_time)
        } else {
            false
        };
    }
    /// Sets the animation matching the given name to be the currently active one.
    /// If attempting to set the animation which is already active, nothing is changed internally and
    /// the animation continues playing without restarting.
    pub fn set_animation(&mut self, name: &str) {
        if self.active_animation == name {
            return
        }

        if let Some(current) = self.animations.get_mut(self.active_animation.as_str()) {
            current.stop();
        }

        if let Some(animation) = self.animations.get_mut(name) {
            animation.play(0);
            self.active_animation = name.to_string();
        }
    }
    /// Plays the set animation, starting from the specified frame.
    /// If start_frame is past the number of frames in the animation, it will immediately end or loop,
    /// based on its looping logic.
    pub fn play(&mut self, start_frame: u32) {
        if let Some(animation) = self.animations.get_mut(self.active_animation.as_str()) {
            animation.play(start_frame);
        }
    }
    /// Stops the current animation in a resumable state. The current frame keeps being rendered
    /// and calling resume() will reprise the animation.
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
    /// Whether the current animation is playing.
    pub fn is_playing(&self) -> bool {
        if let Some(animation) = self.animations.get(self.active_animation.as_str()) {
            animation.playing
        } else {
            false
        }
    }
}

impl TextureProvider for AnimationManager {
    fn get_texture_render_data(&self) -> Option<TextureRenderData> {
        if let Some(anim) = self.animations.get(&self.active_animation) {
            anim.get_texture_render_data()
        } else {
            None
        }
    }
}