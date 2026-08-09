use std::collections::HashSet;
use std::rc::Rc;
use crate::engine::collision::{CollisionInfo, LayerMask, CollisionType, ColliderID, DebugMesh, CollisionSpace};
use crate::engine::world::World;
use crate::engine::EntityID;
use glam::{Vec2, Vec4};
use glow::HasContext;
use crate::engine::collision::raycast::RaycastHit;
use crate::engine::collision::shape::{sat_collide, Shape, ShapeTransform};
use crate::engine::component::SlotID;
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::rendering::shader::Shader;

const DEBUG_CIRCLE_SEGMENTS: usize = 24;

pub enum LayerType {
    On(u32),
    Collided(u32)
}

#[derive(Copy, Clone, Debug)]
pub enum ColliderType {
    Solid,
    Trigger,
}

pub struct Collider {
    pub entity: EntityID,
    pub id: SlotID,
    pub layers_on: LayerMask,
    pub layers_collided: LayerMask,
    pub collision_space: CollisionSpace,
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
    
    /// Creates a new Collider with default attributes to be optionally modified via the builder functions.
    /// Defaults:
    /// layers_on = 0,
    /// layers_collided = 0,
    /// collider_type = Solid,
    /// is_static = false,
    /// is_active = true,
    /// position_offset = Vec2::ZERO,
    /// rotation_offset = 0.0,
    /// debug_color = None
    pub fn new(
        entity: EntityID,
        id: SlotID,
        shapes: Vec<Shape>,
        gl: Rc<glow::Context>,
    ) -> Self {
        Self {
            entity,
            id,
            layers_on: 0,
            layers_collided: 0,
            collision_space: 0,
            collider_type: ColliderType::Solid,
            is_static: false,
            is_active: true,
            shapes,
            position_offset: Vec2::ZERO,
            rotation_offset: 0.0,
            debug_color: None,
            debug_meshes: Vec::new(),
            gl,
        }
    }
    
    /// Sets the collision layer masks for the collider
    pub fn with_layers(mut self, layers_on: LayerMask, layers_collided: LayerMask) -> Self {
        self.layers_on = layers_on;
        self.layers_collided = layers_collided;
        self
    }
    
    /// Sets the collision space for the collider
    pub fn with_collision_space(mut self, collision_space: CollisionSpace) -> Self {
        self.collision_space = collision_space;
        self
    }
    
    /// Sets the position and rotation offsets for the collider
    pub fn with_offsets(mut self, position_offset: Vec2, rotation_offset: f32) -> Self {
        self.position_offset = position_offset;
        self.rotation_offset = rotation_offset;
        self
    }
    
    /// Sets the debug color of the collider and lazily creates the meshes for the shapes
    pub fn with_debug_color(mut self, debug_color: Vec4) -> Self {
        self.debug_color = Some(debug_color);
        
        if !self.debug_meshes.is_empty() {
            return self;
        }
        
        self.debug_meshes = self.shapes
            .iter()
            .map(|_| unsafe {
                let vao = self.gl.create_vertex_array().unwrap();
                let vbo = self.gl.create_buffer().unwrap();

                self.gl.bind_vertex_array(Some(vao));
                self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

                self.gl.enable_vertex_attrib_array(0);
                self.gl.vertex_attrib_pointer_f32(
                    0,
                    2,
                    glow::FLOAT,
                    false,
                    2 * size_of::<f32>() as i32,
                    0
                );

                self.gl.bind_vertex_array(None);

                Some(DebugMesh {vao, vbo})
            })
            .collect();
        
        self
    }
    
    /// Sets the type for the collider
    pub fn with_type(mut self, collider_type: ColliderType) -> Self {
        self.collider_type = collider_type;
        self
    }
    
    /// Makes the collider static
    pub fn make_static(mut self) -> Self {
        self.is_static = true;
        self
    }

    /// Computes the broad phase sections the collider overlaps.
    pub fn get_sections(&self, world: &World) -> Vec<(i32, i32)> {
        let mut sections = HashSet::new();

        let shape_transform = self.build_shape_transform(world).unwrap();

        let section_size = (
            config().collision_sector_size.0 as f32, 
            config().collision_sector_size.1 as f32
        );

        for shape in self.shapes.iter() {
            let (bl, tr) = shape.bounding_box(&shape_transform);

            let bl_section = ((bl.x / section_size.0).floor() as i32, (bl.y / section_size.1).floor() as i32);
            let tr_section = ((tr.x/ section_size.0).floor() as i32, (tr.y / section_size.1).floor() as i32);

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
    
    /// Runs collision detection from this collider's layers_collided towards other.layers_on,
    /// if both are in the same collision space.
    pub fn collide_against(&self, other: &Collider, world: &World) -> Result<Option<CollisionInfo>, String> {
        if (self.layers_collided & other.layers_on) == 0 || !other.is_active || !self.is_active || (self.collision_space & other.collision_space) == 0 {
            return Ok(None);
        }

        let self_transform = self.build_shape_transform(world)?;

        let other_transform = other.build_shape_transform(world)?;

        let mut best = None;

        for i in 0..self.shapes.len() {
            for j in 0..other.shapes.len() {
                if let Some((normal, penetration)) = sat_collide(
                    &self.shapes[i], &self_transform,
                    &other.shapes[j], &other_transform,
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

    /// Checks a ray against itself for a potential hit. Returns the closest hit.
    pub fn raycast(
        &self,
        world: &World,
        origin: Vec2,
        direction: Vec2,
        max_distance: f32,
    ) -> Result<Option<RaycastHit>, String> {
        if !self.is_active {
            return Ok(None);
        }

        if direction == Vec2::ZERO {
            return Ok(None);
        }

        let shape_transform = self.build_shape_transform(world)?;
        let mut best: Option<(f32, Vec2)> = None;

        for shape in &self.shapes {
            if let Some((t, normal)) = shape.raycast(&shape_transform, origin, direction, max_distance) {
                let is_better = match best {
                    Some((best_t, _)) => t < best_t,
                    None => true
                };
                if is_better {
                    best = Some((t, normal));
                }
            }
        }
        
        Ok(best.map(|(t, normal)| RaycastHit {
            collider: ColliderID { entity: self.entity, collider: self.id },
            collider_type: self.collider_type,
            point: origin + direction * t,
            normal,
            distance: t
        }))
    }

    /// Used for debug rendering.
    pub fn draw(&self, gl: Rc<glow::Context>, shader: &Shader, world: &World) {
        if !self.is_active {return;}
        let Some(color) = self.debug_color else { return; };

        let shape_transform = self.build_shape_transform(world).unwrap();

        shader.set_vec4("uColor", &color);
        shader.set_float("uDepth", -1.0);

        for (shape, mesh) in self.shapes.iter().zip(self.debug_meshes.iter()) {
            let Some(mesh) = mesh else { return; };

            let points: Vec<f32> = shape
                .debug_outline_points(&shape_transform, DEBUG_CIRCLE_SEGMENTS)
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

    /// Generates the transform information for the collider's shapes to operate with
    fn build_shape_transform(&self, world: &World) -> Result<ShapeTransform, String> {
        let transform = world.get_component::<Transform>(self.entity)
            .ok_or_else(|| format!("Could not get transform for entity {}", self.entity))?;

        Ok(ShapeTransform{
            world_offset: transform.position() + self.position_offset,
            world_rotation: transform.rotation().to_radians() + self.rotation_offset,
            pivot: transform.position().clone(),
            reflect_axis: transform.get_mirror_logic()
        })
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