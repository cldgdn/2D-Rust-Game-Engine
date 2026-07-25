use std::collections::HashSet;
use std::rc::Rc;
use crate::engine::collision::{CollisionInfo, CollisionLayer, CollisionType};
use crate::engine::world::World;
use crate::engine::EntityID;
use glam::{Mat4, Vec2, Vec3, Vec4};
use glow::HasContext;
use crate::engine::collision::shape::{sat_collide, Shape};
use crate::engine::component::SlotID;
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::rendering::shader::Shader;

const DEBUG_CIRCLE_SEGMENTS: usize = 24;

pub enum LayerType {
    On(u32),
    Collided(u32)
}

#[derive(Copy, Clone)]
pub enum ColliderType {
    Solid,
    Trigger,
}

struct DebugMesh {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
}

pub struct Collider {
    pub entity: EntityID,
    pub id: SlotID,
    pub layers_on: CollisionLayer,
    pub layers_collided: CollisionLayer,
    pub collider_type: ColliderType,
    pub is_static: bool,
    pub is_active: bool,
    pub shapes: Vec<Shape>,
    pub position_offset: Vec2,
    pub rotation_offset: f32,
    pub debug_color: Option<Vec4>,
    debug_meshes: Vec<Option<DebugMesh>>,
    gl: Rc<glow::Context>,
}

impl Collider {
    pub fn new(
        entity: EntityID,
        id: SlotID,
        layers_on: CollisionLayer,
        layers_collided: CollisionLayer,
        collider_type: ColliderType,
        is_static: bool,
        is_active: bool,
        shapes: Vec<Shape>,
        position_offset: Vec2,
        rotation_offset: f32,
        debug_color: Option<Vec4>,
        gl: Rc<glow::Context>,
    ) -> Self {
        let debug_meshes = shapes
            .iter()
            .map(|_| match debug_color {
                Some(_) => unsafe {
                    let vao = gl.create_vertex_array().unwrap();
                    let vbo = gl.create_buffer().unwrap();

                    gl.bind_vertex_array(Some(vao));
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(
                        0,
                        2,
                        glow::FLOAT,
                        false,
                        2 * std::mem::size_of::<f32>() as i32,
                        0
                    );

                    gl.bind_vertex_array(None);

                    Some(DebugMesh {vao, vbo})
                },
                None => None
            })
            .collect();

        Collider {
            entity, layers_on, layers_collided, collider_type, is_static, is_active,
            shapes,
            position_offset,
            rotation_offset,
            id,
            debug_color,
            debug_meshes,
            gl
        }
    }

    pub fn get_sections(&self, world: &World) -> Vec<(i32, i32)> {
        let transform = world.get_component::<Transform>(self.entity).unwrap();
        let mut sections = HashSet::new();
        
        let section_size = (config().collision_sector_size.0 as i32, config().collision_sector_size.1 as i32);
        let pos = transform.position() + self.position_offset;
        let rot = transform.rotation().to_radians() + self.rotation_offset;

        for shape in self.shapes.iter() {
            let (bl, tr) = shape.bounding_box(pos, rot);

            let bl_section = (bl.x as i32 / section_size.0, bl.y as i32 / section_size.1);
            let tr_section = (tr.x as i32 / section_size.0, tr.y as i32 / section_size.1);

            let dx = bl_section.0 - tr_section.0;
            let dy = bl_section.1 - tr_section.1;

            for i in 0..=dx.abs() {
                for j in 0..=dy.abs() {
                    sections.insert((bl_section.0 + i, bl_section.1 + j));
                }
            }
        }

        sections.into_iter().collect()
    }
    
    pub fn collide_against(&self, other: &Collider, world: &World) -> Result<Option<CollisionInfo>, String> {
        if (self.layers_collided & other.layers_on) == 0 || !other.is_active || !self.is_active {
            return Ok(None);
        }

        let self_t;
        let other_t;

        if let Some(t) = world.get_component::<Transform>(self.entity) {
            self_t = t;
        } else {
            return Err(format!("Could not get transform for entity {}", self.entity));
        }

        if let Some(t) = world.get_component::<Transform>(other.entity) {
            other_t = t;
        } else {
            return Err(format!("Could not get transform for entity {}", other.entity));
        }

        let self_pos = self_t.position() + self.position_offset;
        let self_rot = self_t.rotation().to_radians() + self.rotation_offset;
        let other_pos = other_t.position() + other.position_offset;
        let other_rot = other_t.rotation().to_radians() + self.rotation_offset;

        let mut best = None;

        for i in 0..self.shapes.len() {
            for j in 0..other.shapes.len() {
                if let Some((normal, penetration)) = sat_collide(
                    &self.shapes[i], (self_pos, self_rot),
                    &other.shapes[j], (other_pos, other_rot),
                ) {
                    let is_better = match best {
                        Some((_, best_penetration)) => penetration > best_penetration,
                        None => true
                    };


                    if is_better {
                        best = Some((normal, penetration));
                    }
                }
            }
        }

        let (normal, penetration) = match best {
            Some(mtv) => mtv,
            None => return Ok(None)
        };

        let collision_type = match (self.collider_type, other.collider_type) {
            (ColliderType::Solid, ColliderType::Solid) => {
                CollisionType::Solid(self.layers_collided & other.layers_on)
            }
            _ => CollisionType::Trigger(self.layers_collided & other.layers_on)
        };

        Ok(Some(CollisionInfo {
            collision_type,
            normal,
            penetration
        }))
    }

    pub fn draw(&self, gl: Rc<glow::Context>, shader: &Shader, world: &World) {
        if !self.is_active {return;}
        let Some(color) = self.debug_color else { return; };

        let transform = match world.get_component::<Transform>(self.entity) {
            Some(t) => t,
            None => return
        };

        let world_pos = transform.position() + self.position_offset;
        let world_rot = transform.rotation().to_radians() + self.rotation_offset;

        shader.set_vec4("uColor", &color);
        shader.set_float("uDepth", -1.0);

        for (shape, mesh) in self.shapes.iter().zip(self.debug_meshes.iter()) {
            let Some(mesh) = mesh else { return; };

            let points: Vec<f32> = shape
                .debug_outline_points(world_pos, world_rot, DEBUG_CIRCLE_SEGMENTS)
                .into_iter()
                .flat_map(|p| {
                    let scaled = p * config().unit_pixel_size as f32;
                    [scaled.x, scaled.y]
                })
                .collect();

            let vertex_count = points.len() / 2;

            unsafe {
                gl.bind_vertex_array(Some(mesh.vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh.vbo));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&points),
                    glow::DYNAMIC_DRAW,
                );

                gl.draw_arrays(glow::LINE_LOOP, 0, vertex_count as i32);
            }
        }

        unsafe { gl.bind_vertex_array(None); }
    }
}

impl Drop for Collider {
    fn drop(&mut self) {
        for option in self.debug_meshes.iter() {
            if let Some(mesh) = option {
                unsafe {
                    self.gl.delete_vertex_array(mesh.vao);
                    self.gl.delete_buffer(mesh.vbo);
                }
            }
        }
    }
}