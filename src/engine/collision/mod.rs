pub mod shape;
pub mod raycast;
pub mod math_utilities;

use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use glam::{Mat4, Vec2};
use glow::HasContext;
use crate::engine::collision::raycast::{RaycastHit, RaycastMode, RaycastRequest};
use crate::engine::component::{Multi, SlotID};
use crate::engine::component::collider::{Collider, ResolutionBehavior};
use crate::engine::component::transform::Transform;
use crate::engine::config::config;
use crate::engine::{BitMask, Entity, EntityID};
use crate::engine::rendering::pass::RenderPass;
use crate::engine::world::{FixedIntention, World};

/// Identifies which layers a `Collider` or `RayCast` may interact with
pub type LayerMask = BitMask;
/// Identifies which spaces a `Collider` or `RayCast` may interact with. Functionally like a
/// secondary `LayerMask`
pub type CollisionSpace = BitMask;

/// Simple mesh used for debug rendering of `Collider` shapes
pub struct DebugMesh {
    pub vao: glow::VertexArray,
    pub vbo: glow::Buffer,
}

/// Encodes what kind of collision happened, and on which layers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CollisionType {
    /// The collision happened between a pair of `Solid` colliders
    Solid(LayerMask),
    /// The collision happened between a pair of colliders of which at least one is a `Trigger`
    Trigger(LayerMask),
}

/// Carrier for the information about a collision, used for resolution.
#[derive(Clone, Copy, Debug)]
pub struct CollisionInfo {
    pub collision_type: CollisionType,
    /// Direction to push the A collider during resolution
    pub normal: Vec2,
    /// Distance to push the A collider during resolution
    pub penetration: f32,   
}

/// An identifier for a specific Collider
#[derive(Clone, Debug, Eq, Hash, PartialEq, Copy)]
pub struct ColliderID {
    /// The collider's owner `Entity`
    pub entity: EntityID,
    /// The collider's own id, local to the `Entity`
    pub collider: SlotID,
}

/// Internal carrier for collision resolution data
#[derive(Clone, Debug)]
pub struct Collision {
    pub a: ColliderID,
    pub b: ColliderID,
    pub collision_info: CollisionInfo,
}

/// Collision Event produced by the resolution stage and passed out to `Entity` a.
#[derive(Clone, Debug)]
pub struct CollisionEvent {
    pub a: ColliderID,
    pub b: ColliderID,
    pub collision_type: CollisionType,
    pub normal: Vec2,
    pub penetration: f32,
}

/// Struct defining the logic for collision detection, ray-casting and collision resolution
pub struct CollisionSystem {
    /// Stores the broad-phase sections for static colliders. Because static colliders are assumed to
    /// never move, these are precomputed once when the colliders are registered and never updated
    /// until they need to be removed because of an `Entity` being destroyed.
    sections_static: HashMap<(i32, i32), Vec<ColliderID>>,
    /// Stores the broad-phase sections for dynamic colliders. Because dynamic colliders are allowed
    /// to mode, these are recomputed on each broad-phase pass.
    sections_dynamic: HashMap<(i32, i32), Vec<ColliderID>>,
    /// Used by ray-casting to check whether there already has been a raycast request. This avoids
    /// recomputing `sections_dynamic` on each raycast, since those are performed after resolution.
    dynamic_sections_ready: bool,
    /// Shared mesh used for debug draw of rays.
    ray_debug_mesh: Option<DebugMesh>
}

/// `Entity` sub-trait which exposes collision behavior.
pub trait Collides {
    /// Used to instantiate and submit any static colliders to the `World` and `CollisionSystem`.
    fn submit_colliders(
        &self, 
        collision_system: &mut CollisionSystem, 
        world: &mut World
    );
    /// Called by the engine after collision resolution on each `Entity` which `Collides` and whose `EntityID`
    /// is found in the issued `CollisionEvent`
    fn react_to_collision(
        &mut self,
        collision: CollisionEvent,
        world: &mut World
    ) {
        ()
    }
    /// Called by the engine after collision resolution and ray-casting phases, on each `Entity` which
    /// `Collides` and whose `EntityID` is the owner of the matching `RayCastRequest`
    fn react_to_raycast(
        &mut self,
        hits: Vec<RaycastHit>,
        world: &mut World
    ) {
        ()
    }
}

impl CollisionSystem {
    pub fn new() -> CollisionSystem {
        CollisionSystem {
            sections_static: HashMap::new(),
            sections_dynamic: HashMap::new(),
            dynamic_sections_ready: false,
            ray_debug_mesh: None,
        }
    }
    
    pub fn run_collisions(&mut self, world: &World) -> (HashSet<Collision>, u32) {
        let mut collisions = HashSet::new();
        let mut checks = 0;

        // BROAD PHASE -----------------------------------------------------------------------------

        self.build_dynamic_sections(world);

        // NARROW PHASE ----------------------------------------------------------------------------

        for (section, colliders) in self.sections_dynamic.iter() {
            let static_colliders = self.sections_static.get(section);

            for (i, collider_id) in colliders.iter().enumerate() {
                let a = world
                    .get_component::<Multi<Collider>>(collider_id.entity)
                    .unwrap()
                    .get(&collider_id.collider)
                    .unwrap();

                if let Some(static_colliders) = static_colliders {
                    for static_collider in static_colliders {
                        let b = world
                            .get_component::<Multi<Collider>>(static_collider.entity)
                            .unwrap()
                            .get(&static_collider.collider)
                            .unwrap();

                        let t = a.collide_against(&b, world).unwrap();
                        if let Some(collision_info) = t {
                            collisions.insert(Collision {
                                a: collider_id.clone(),
                                b: static_collider.clone(),
                                collision_info
                            });
                        }

                        checks += 1;
                    }
                }

                for j in (i + 1)..colliders.len() {
                    let other = colliders.get(j).unwrap();

                    if collider_id.entity == other.entity {
                        continue;
                    } else {
                        checks += 2;
                    }

                    let b = world
                        .get_component::<Multi<Collider>>(other.entity)
                        .unwrap()
                        .get(&other.collider)
                        .unwrap();

                    let t = a.collide_against(&b, world).unwrap();
                    if let Some(collision_info) = t {
                        collisions.insert(Collision {
                            a: collider_id.clone(),
                            b: other.clone(),
                            collision_info
                        });
                    }

                    let t = b.collide_against(&a, world).unwrap();
                    if let Some(collision_info) = t {
                        collisions.insert(Collision {
                            a: other.clone(),
                            b: collider_id.clone(),
                            collision_info
                        });
                    }
                }
            }

        }

        self.sections_dynamic.clear();

        (collisions, checks)
    }

    pub fn resolve_collisions(&self, collisions: HashSet<Collision>, world: &mut World) -> Vec<CollisionEvent> {
        let mut events = Vec::new();

        for collision in collisions {
            let b_collider = world
                .get_component::<Multi<Collider>>(collision.b.entity)
                .unwrap()
                .get(&collision.b.collider)
                .unwrap();
            let info = collision.collision_info;

            if let CollisionType::Solid(_) = info.collision_type {
                let translation = if !b_collider.is_static && b_collider.resolution_behavior == ResolutionBehavior::Movable {
                    info.normal * (info.penetration * 0.5)
                } else {
                    info.normal * info.penetration
                };

                let transform = world
                    .get_component_mut::<Transform>(collision.a.entity)
                    .unwrap();

                transform.translate(translation);
            }

            let event = CollisionEvent {
                a: collision.a,
                b: collision.b,
                collision_type: info.collision_type,
                penetration: info.penetration,
                normal: info.normal,
            };

            events.push(event);
        }

        events
    }

    pub fn add_static_colliders(&mut self, entity: EntityID, world: &World) {
        let colliders = world.get_component::<Multi<Collider>>(entity).unwrap();

        for (_, collider) in colliders.iter() {
            if collider.is_static {
                let sections = collider.get_sections(world);

                for section in sections {
                    let section_colliders = self.sections_static.entry(section).or_insert(Vec::new());
                    section_colliders.push(ColliderID {
                        entity,
                        collider: collider.id,
                    });
                }
            }
        }
    }

    pub fn remove_static_colliders(&mut self, entity: EntityID, world: &mut World) {
        if let Some(colliders) = world.get_component::<Multi<Collider>>(entity) {
            for (_, collider) in colliders.iter() {
                if collider.is_static {
                    let sections = collider.get_sections(world);

                    for section in sections {
                        if let Some(section_colliders) = self.sections_static.get_mut(&section) {
                            section_colliders.retain(|x| x.entity != entity);
                        }
                    }
                }
            }
        }
    }

    pub fn debug_draw(&mut self, world: &World, passes: &Vec<RenderPass>, entities: &HashMap<EntityID, Box<dyn Entity>>) {
        let debug_shader = world.debug_shader();
        let camera = world.camera(1).unwrap();
        let view_projection = camera.get_view_projection(0.0);
        let intentions = &world.fixed_intention_queue;

        debug_shader.use_program();

        debug_shader.set_mat4("uViewProjection", &view_projection);

        let multis: Vec<(EntityID, &Multi<Collider>)> = world.get_all_components::<Multi<Collider>>().collect();
        let mut colliders = Vec::new();

        for (_, multi) in multis.iter() {
            for (_, collider) in multi.iter() {
                colliders.push(collider);
            }
        }

        for collider in colliders {
            if let Some(entity) = entities.get(&collider.entity).unwrap().as_drawable() {
                let pass_mask = entity.pass_mask();
                let passes = passes.iter().filter(|p| p.matches(pass_mask) && p.enabled).collect::<Vec<_>>();

                if !passes.is_empty() {
                    for pass in passes {
                        let camera = world.camera(pass.camera_id).unwrap();

                        debug_shader.set_mat4("uViewProjection", &camera.get_view_projection(0.0));

                        collider.draw(world.gl(), debug_shader, world);
                    }
                }
            } else {
                debug_shader.set_mat4("uViewProjection", &view_projection);

                collider.draw(world.gl(), debug_shader, world);
            };
        }

        let gl = world.gl();

        let mesh = self.ray_debug_mesh.get_or_insert_with(|| unsafe {
            let vao = gl.create_vertex_array().unwrap();
            let vbo = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0, 2, glow::FLOAT, false,
                2 * size_of::<f32>() as i32, 0
            );

            gl.bind_vertex_array(None);

            DebugMesh { vao, vbo}
        });

        for intention in intentions {
            match intention {
                FixedIntention::RaycastRequest(request) => {
                    let Some(color) = request.debug_color else { continue; };

                    let direction = request.direction.normalize_or_zero();
                    if direction == Vec2::ZERO {
                        continue;
                    }

                    let end = request.origin + direction * request.max_distance;
                    let scale = config().unit_pixel_size as f32;

                    let points: [f32; 4] = [
                        request.origin.x * scale,
                        request.origin.y * scale,
                        end.x * scale,
                        end.y * scale,
                    ];

                    debug_shader.set_vec4("uColor", &color);
                    debug_shader.set_float("uDepth", -1.0);

                    unsafe {
                        gl.bind_vertex_array(Some(mesh.vao));
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh.vbo));
                        gl.buffer_data_u8_slice(
                            glow::ARRAY_BUFFER,
                            bytemuck::cast_slice(&points),
                            glow::DYNAMIC_DRAW,
                        );

                        gl.draw_arrays(glow::LINES, 0, 2);
                    }
                }
            }
        }

        unsafe { gl.bind_vertex_array(None); }
    }

    pub fn get_hits(&mut self, world: &World, request: &RaycastRequest) -> Vec<RaycastHit> {
        self.ensure_dynamic_sections_for_raycast(world);

        let origin = request.origin;
        let direction = request.direction.normalize_or_zero();

        if direction == Vec2::ZERO || request.max_distance <= 0.0 {
            return Vec::new();
        }

        let section_size = (
            config().collision_sector_size.0 as f32,
            config().collision_sector_size.1 as f32
        );

        let mut cell = (
            (origin.x / section_size.0).floor() as i32,
            (origin.y / section_size.1).floor() as i32,
        );

        let step_x: i32 = if direction.x > 0.0 { 1 } else if direction.x < 0.0 { -1 } else { 0 };
        let step_y: i32 = if direction.y > 0.0 { 1 } else if direction.y < 0.0 { -1 } else { 0 };

        let next_boundary_x = if step_x > 0 { (cell.0 + 1) as f32 * section_size.0 } else { cell.0 as f32 * section_size.0 };
        let next_boundary_y = if step_y > 0 { (cell.1 + 1) as f32 * section_size.1 } else { cell.1 as f32 * section_size.1 };

        let mut t_max_x = if direction.x != 0.0 { (next_boundary_x - origin.x) / direction.x } else { f32::INFINITY };
        let mut t_max_y = if direction.y != 0.0 { (next_boundary_y - origin.y) / direction.y } else { f32::INFINITY };

        let t_delta_x = if direction.x != 0.0 { section_size.0 / direction.x.abs() } else { f32::INFINITY };
        let t_delta_y = if direction.y != 0.0 { section_size.1 / direction.y.abs() } else { f32::INFINITY };

        let mut tested: HashSet<ColliderID> = HashSet::new();
        let mut hits: Vec<RaycastHit> = Vec::new();
        let mut t_entered = 0.0_f32;

        loop {
            if t_entered > request.max_distance {
                break;
            }

            if request.mode == RaycastMode::Nearest {
                if let Some(best) = hits.first() {
                    if t_entered >= best.distance {
                        break;
                    }
                }
            }

            let mut candidates: Vec<&ColliderID> = Vec::new();
            if let Some(ids) = self.sections_static.get(&cell) {
                candidates.extend(ids.iter());
            }
            if let Some(ids) = self.sections_dynamic.get(&cell) {
                candidates.extend(ids.iter());
            }

            for id in candidates {
                if id.entity == request.source_id {
                    continue;
                }
                if !tested.insert(id.clone()) {
                    continue;
                }

                let Some(collider) = world
                    .get_component::<Multi<Collider>>(id.entity)
                    .and_then(|multi| multi.get(&id.collider))
                else {
                    continue;
                };

                if collider.layers_on & request.layers_collided == 0 {
                    continue;
                }

                let hit = match collider.raycast(world, origin, direction, request.max_distance) {
                    Ok(Some(hit)) => hit,
                    Ok(None) => continue,
                    Err(_) => panic!("Raycast requested by entity {} failed to retrieve entity {}'s transform", request.source_id, collider.entity),
                };

                match request.mode {
                    RaycastMode::Nearest => {
                        let is_better = hits.first().map_or(true, |best| hit.distance < best.distance);
                        if is_better {
                            hits.clear();
                            hits.push(hit);
                        }
                    },
                    RaycastMode::All => hits.push(hit),
                }
            }

            if step_x == 0 && step_y == 0 {
                break; //stationary ray, guards against looping forever
            }

            if t_max_x < t_max_y {
                t_entered = t_max_x;
                t_max_x += t_delta_x;
                cell.0 += step_x;
            } else {
                t_entered = t_max_y;
                t_max_y += t_delta_y;
                cell.1 += step_y;
            }
        }

        if request.mode == RaycastMode::All {
            hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
        }

        hits
    }

    pub fn post_raycast_clear(&mut self) {
        self.dynamic_sections_ready = false;
        self.sections_dynamic.clear();
    }

    fn build_dynamic_sections(&mut self, world: &World) {
        let multis: Vec<(EntityID, &Multi<Collider>)> = world.get_all_components::<Multi<Collider>>().collect();
        let mut colliders = Vec::new();

        for (_, multi) in multis.iter() {
            for (_, collider) in multi.iter() {
                colliders.push(collider);
            }
        }

        for collider in colliders.iter() {
            if !collider.is_active || collider.is_static {
                continue;
            }

            let sections = collider.get_sections(world);

            for section in sections {
                let section_colliders = self.sections_dynamic.entry(section).or_insert(Vec::new());

                section_colliders.push(ColliderID {
                    entity: collider.entity,
                    collider: collider.id,
                });
            }
        }
    }

    fn ensure_dynamic_sections_for_raycast(&mut self, world: &World) {
        if self.dynamic_sections_ready {
            return;
        }

        self.build_dynamic_sections(world);
        self.dynamic_sections_ready = true;
    }
}

impl PartialEq for Collision {
    fn eq(&self, other: &Collision) -> bool {
        self.a == other.a && self.b == other.b
    }
}

impl Eq for Collision {}

impl Hash for Collision {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a.hash(state);
        self.b.hash(state);
    }
}