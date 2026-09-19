use glam::Vec2;

pub struct ShapeTransform {
    pub world_offset: Vec2,
    pub world_rotation: f32,
    pub pivot: Vec2,
    pub reflect_axis: (bool, bool),
    pub scale: Vec2,
}

/// A single convex primitive making up part of a `Collider`.
///
/// Every method takes `offset: Vec2`, the *world-space* position of the
/// owning collider (i.e. `transform.position() + collider.offset`). The
/// shape's own local `offset` field is added on top of that, so a compound
/// collider can place several shapes at different points relative to the
/// same entity.
///
/// `Rectangle` and `Triangle` are both "polygon" shapes and share almost
/// all of their SAT logic through `vertices()` - neither has any bespoke
/// per-shape geometry code. Only `Circle`, which has no fixed edges,
/// branches separately.
#[derive(Clone, Debug)]
pub enum Shape {
    Rectangle {
        /// Half-width / half-height, before rotation is applied.
        half_extents: Vec2,
        /// Rotation in radians, counter-clockwise.
        rotation_offset: f32,
        /// Local offset of the rectangle's center from the collider origin.
        offset: Vec2,
    },
    Triangle {
        /// Local vertex positions, relative to `offset`, before rotation.
        /// Winding order (CW/CCW) does not matter - SAT doesn't care, and
        /// the MTV's final direction is resolved separately in
        /// `sat_collide` regardless of which way normals initially point.
        vertices: [Vec2; 3],
        /// Rotation in radians, counter-clockwise.
        rotation_offset: f32,
        /// Local anchor point the vertices are positioned relative to.
        offset: Vec2,
    },
    Circle {
        radius: f32,
        rotation_offset: f32,
        /// Local offset of the circle's center from the collider origin.
        offset: Vec2,
    },
}

/// Computes the world-space point a shape's own local geometry (its
/// vertices, or a circle's radius) is positioned relative to.
///
/// The collider's own offset (`transform.world_offset - transform.pivot`,
/// recovered here since `ShapeTransform` only carries the already-summed
/// `world_offset`) and the shape's own local `offset` are combined into a
/// single vector *before* reflecting and rotating - reflection and
/// rotation are both linear, so reflecting/rotating them separately and
/// summing the results would give the identical answer. Both are rotated
/// by `world_rotation` alone: that field already bundles the entity's
/// rotation and the collider's own `rotation_offset` together, so this is
/// the one rotation that moves *where* every shape in the collider sits.
/// A shape's own `rotation_offset` never affects this - it only orients
/// that shape's vertices around the point this function returns.
fn shape_anchor(transform: &ShapeTransform, local_offset: Vec2) -> Vec2 {
    let collider_offset = transform.world_offset - transform.pivot;
    let combined = reflect_local(collider_offset + local_offset, transform.reflect_axis) * transform.scale;
    let (s, c) = transform.world_rotation.sin_cos();

    transform.pivot + Vec2::new(combined.x * c - combined.y * s, combined.x * s + combined.y * c)
}

impl Shape {
    /// World-space center of this shape.
    ///
    /// For polygons this is the true centroid (average of world-space
    /// vertices), not just `offset` - for `Rectangle` these coincide
    /// exactly since its vertices are symmetric, but for `Triangle` they
    /// generally won't, and using the real centroid keeps the
    /// inside/outside orientation check in `sat_collide` correct
    /// regardless of where the caller chose to place `offset`.
    pub fn center(&self, transform: &ShapeTransform) -> Vec2 {
        match self {
            Shape::Circle { offset, .. } => {
                // rotation_offset is deliberately unused here: it governs
                // orientation of a shape's own vertices, and a circle has
                // none - rotating a circle in place has no visible effect,
                // so this field is inert for this variant, not a bug.
                shape_anchor(transform, *offset)
            },
            _ => {
                let verts = self.vertices(transform);
                let sum: Vec2 = verts.iter().copied().sum();
                sum / verts.len() as f32
            }
        }
    }

    /// World-space vertices of this shape, in order, forming its boundary.
    /// Returns an empty vec for `Circle`, which has no vertices - callers
    /// must special-case circles rather than calling this on them.
    fn vertices(&self, transform: &ShapeTransform) -> Vec<Vec2> {
        match self {
            Shape::Rectangle { half_extents, rotation_offset: rotation, offset } => {
                let shape_center = shape_anchor(transform, *offset);

                // shape center -> corners: rotate by the combined angle,
                // so the shape's own rotation_offset layers on top of
                // whatever the collider (and entity) rotation already is.
                let (sr, cr) = rotation.sin_cos();
                let (sw, cw) = transform.world_rotation.sin_cos();
                let hx = half_extents.x;
                let hy = half_extents.y;

                [
                    Vec2::new(hx, hy),
                    Vec2::new(hx, -hy),
                    Vec2::new(-hx, -hy),
                    Vec2::new(-hx, hy),
                ]
                    .into_iter()
                    .map(|p| {
                        let p = reflect_local(p, transform.reflect_axis);

                        let rotated = Vec2::new(p.x * cr - p.y * sr, p.x * sr - p.y * cr);
                        let scaled = rotated * transform.scale;

                        shape_center + Vec2::new(scaled.x * cw - scaled.y * sw, scaled.x * sw + scaled.y * cw)
                    })
                    .collect()
            }
            Shape::Triangle { vertices, rotation_offset: rotation, offset } => {
                let anchor = shape_anchor(transform, *offset);

                // anchor -> vertices: rotate by the combined angle, so
                // rotation_offset layers on top of the collider (and
                // entity) rotation already folded into world_rotation.
                let (sr, cr) = rotation.sin_cos();
                let (sw, cw) = transform.world_rotation.sin_cos();

                vertices
                    .iter()
                    .map(|p| {
                        let p = reflect_local(*p, transform.reflect_axis);
                        let rotated = Vec2::new(p.x * cr - p.y * sr, p.x * sr + p.y * cr);
                        let scaled = rotated * transform.scale;
                        anchor + Vec2::new(scaled.x * cw - scaled.y * sw, scaled.x * sw + scaled.y * cw)
                    })
                    .collect()
            }
            Shape::Circle { .. } => Vec::new(),
        }
    }

    /// Effective world-space radius for a Circle after transform.scale.
    /// Non-uniform scale can't be represented exactly by a circle, so this approximates it as a
    /// uniformly scaled circle using the average of the two axis scales.
    fn scaled_circle_radius(radius: f32, transform: &ShapeTransform) -> f32 {
        radius * (transform.scale.x.abs() + transform.scale.y.abs()) * 0.5
    }

    /// World-space axis-aligned bounding box, used for section lookup.
    /// Returns the bottom-left and top-right coordinates of the box vertices.
    pub fn bounding_box(&self, transform: &ShapeTransform) -> (Vec2, Vec2) {
        match self {
            Shape::Circle { radius, .. } => {
                let center = self.center(transform);
                let r = Self::scaled_circle_radius(*radius, transform);
                (center - Vec2::splat(r), center + Vec2::splat(r))
            }
            _ => {
                let verts = self.vertices(transform);
                let mut min = Vec2::splat(f32::MAX);
                let mut max = Vec2::splat(f32::MIN);

                for v in verts {
                    min = min.min(v);
                    max = max.max(v);
                }

                (min, max)
            }
        }
    }

    /// Local-space (width, height) footprint of this shape at scale = (1.0, 1.0),
    /// ignoring `rotation_offset` and `offset` - only the shape's raw dimensions
    /// matter here, not its placement or orientation within the Collider.
    ///
    /// Rotation is deliberately ignored: a rotated shape's true axis-aligned
    /// footprint can only be equal to or larger than its unrotated one, so this
    /// can under-count how thin a shape is but never over-count it. The caller
    /// (deciding how many collision substeps a fast-moving Collider needs) is
    /// only harmed by overestimating thinness, never by underestimating it, so
    /// erring toward "looks thinner than it really is" is the safe direction.
    pub fn get_min_extent(&self) -> Vec2 {
        match self {
            Shape::Rectangle { half_extents, .. } => *half_extents * 2.0,
            Shape::Circle { radius, .. } => Vec2::splat(radius * 2.0),
            Shape::Triangle { vertices, .. } => {
                let mut min = Vec2::splat(f32::MAX);
                let mut max = Vec2::splat(f32::MIN);
                for v in vertices {
                    min = min.min(*v);
                    max = max.max(*v);
                }
                max - min
            }
        }
    }

    /// World-space points approximating this shape's boundary, for debug
    /// drawing as a closed line loop. Polygons return their exact `vertices()`
    /// - what you see is exactly what collision math is using. Circles have
    /// no discrete vertices, so they're tessellated into `segments` points;
    /// this is purely a rendering approximation and has no bearing on
    /// `sat_collide`, which treats circles exactly via center + radius.
    pub fn debug_outline_points(&self, transform: &ShapeTransform, segments: usize) -> Vec<Vec2> {
        match self {
            Shape::Circle { radius, .. } => {
                let segments = segments.max(3);
                let center = self.center(transform);

                (0..segments)
                    .map(|i| {
                        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
                        center + Vec2::new(theta.cos(), theta.sin()) * Self::scaled_circle_radius(*radius, transform)
                    })
                    .collect()
            }
            _ => self.vertices(transform),
        }
    }

    /// The closest point on this shape's boundary to `point` (world space).
    /// Used to derive the one extra separating axis a circle needs.
    ///
    /// For polygons this walks every edge, finds the closest point on
    /// each edge *segment* (clamped, not the infinite line), and keeps
    /// the closest overall - this is what lets it work for a triangle,
    /// a rectangle, or any future convex polygon without per-shape code.
    fn closest_point_to(&self, transform: &ShapeTransform, point: Vec2) -> Vec2 {
        match self {
            Shape::Circle { radius, .. } => {
                let center = self.center(transform);
                let dir = (point - center).normalize_or_zero();
                center + dir * Self::scaled_circle_radius(*radius, transform)
            }
            _ => {
                let verts = self.vertices(transform);
                let n = verts.len();

                let mut best = verts[0];
                let mut best_dist_sq = f32::MAX;

                for i in 0..n {
                    let a = verts[i];
                    let b = verts[(i + 1) % n];
                    let candidate = closest_point_on_segment(a, b, point);
                    let dist_sq = candidate.distance_squared(point);

                    if dist_sq < best_dist_sq {
                        best_dist_sq = dist_sq;
                        best = candidate;
                    }
                }

                best
            }
        }
    }

    /// Candidate separating axes this shape contributes when tested
    /// against `other`. Polygons contribute one axis per edge (the edge's
    /// normal) - for any convex polygon, this is always a complete and
    /// correct set of candidates, regardless of vertex count. Circles
    /// contribute a single axis derived from the closest point on `other`,
    /// since a circle has no fixed edges of its own.
    fn axes_against(&self, transform: &ShapeTransform, other: &Shape, other_transform: &ShapeTransform) -> Vec<Vec2> {
        match self {
            Shape::Circle { .. } => {
                let self_center = self.center(transform);
                let closest = other.closest_point_to(other_transform, self_center);
                let axis = closest - self_center;

                // Degenerate: circle center sits exactly on the closest
                // point (e.g. deeply inside `other`, dead center). No
                // useful axis here - the other shape's own edge normals
                // (or, for circle-vs-circle, the fallback in sat_collide)
                // cover this case instead.
                if axis.length_squared() < f32::EPSILON {
                    vec![]
                } else {
                    vec![axis.normalize()]
                }
            }
            _ => {
                let verts = self.vertices(transform);
                let n = verts.len();

                (0..n)
                    .filter_map(|i| {
                        let a = verts[i];
                        let b = verts[(i + 1) % n];
                        let edge = b - a;
                        let normal = Vec2::new(-edge.y, edge.x);
                        let normalized = normal.normalize_or_zero();

                        if normalized == Vec2::ZERO { None } else { Some(normalized) }
                    })
                    .collect()
            }
        }
    }

    /// Casts a ray against this shape and returns the nearest intersection
    /// within `[0, max_distance]`, as `(distance, world-space normal)`.
    ///
    /// `direction` must already be a unit vector - callers normalize once
    /// at the raycast entry point (`Collider::raycast`) rather than every
    /// shape repeating the work.
    pub fn raycast(
        &self,
        transform: &ShapeTransform,
        origin: Vec2,
        direction: Vec2,
        max_distance: f32,
    ) -> Option<(f32, Vec2)> {
        match self {
            Shape::Circle { radius, .. } => {
                let center = self.center(transform);
                let r = Self::scaled_circle_radius(*radius, transform);
                let to_origin = origin - center;
                let b = 2.0 * to_origin.dot(direction);
                let c = to_origin.length_squared() - r * r;
                let discriminant = b * b - 4.0 * c;

                if discriminant < 0.0 {
                    return None;
                }

                let sqrt_disc = discriminant.sqrt();
                let t0 = (-b - sqrt_disc) * 0.5;
                let t1 = (-b + sqrt_disc) * 0.5;

                if t0 >= 0.0 && t0 <= max_distance {
                    let point = origin + direction * t0;
                    let normal = (point - center).normalize_or_zero();
                    Some((t0, normal))
                } else if t1 >= 0.0 && t1 <= max_distance {
                    // Ray origin starts inside the circle - there's no
                    // real entry surface, so report an immediate hit at
                    // t=0 with the direction from center to origin as a
                    // stand-in normal, rather than the exit point.
                    let normal = to_origin.normalize_or_zero();
                    Some((0.0, normal))
                } else {
                    None
                }
            }
            _ => self.raycast_polygon(transform, origin, direction, max_distance),
        }
    }

    /// Ray-vs-convex-polygon via half-plane clipping: each edge defines a
    /// half-plane the ray's valid parameter range `[t_min, t_max]` must
    /// stay inside. Each edge either raises `t_min` (an "entering" plane)
    /// or lowers `t_max` (an "exiting" plane); if the range ever becomes
    /// empty, the ray misses entirely. Works identically for any vertex
    /// count.
    fn raycast_polygon(
        &self,
        transform: &ShapeTransform,
        origin: Vec2,
        direction: Vec2,
        max_distance: f32,
    ) -> Option<(f32, Vec2)> {
        let verts = self.vertices(transform);
        let n = verts.len();
        let centroid = self.center(transform);

        let mut t_min = 0.0_f32;
        let mut t_max = max_distance;
        let mut entry_normal: Option<Vec2> = None;

        for i in 0..n {
            let a = verts[i];
            let b = verts[(i + 1) % n];
            let edge = b - a;

            let mut normal = Vec2::new(-edge.y, edge.x).normalize_or_zero();
            if normal == Vec2::ZERO {
                continue; // degenerate zero-length edge
            }

            // Edge normals from vertex order alone could point either
            // way - force outward, away from the shape's own centroid,
            // regardless of winding.
            if normal.dot(centroid - a) > 0.0 {
                normal = -normal;
            }

            let denom = direction.dot(normal);
            let numer = (origin - a).dot(normal);

            if denom.abs() < f32::EPSILON {
                // Ray runs parallel to this edge: either always on the
                // inside of this half-plane (doesn't constrain anything)
                // or always outside it (misses the polygon entirely).
                if numer > 0.0 {
                    return None;
                }
                continue;
            }

            let t = -numer / denom;

            if denom < 0.0 {
                // Entering half-plane: ray moves from outside to inside
                // as t increases.
                if t > t_min {
                    t_min = t;
                    entry_normal = Some(normal);
                }
            } else {
                // Exiting half-plane: ray moves from inside to outside.
                if t < t_max {
                    t_max = t;
                }
            }

            if t_min > t_max {
                return None;
            }
        }

        let normal = entry_normal.unwrap_or_else(|| {
            // No edge ever pushed t_min past 0 - the ray started inside
            // the polygon. There's no real entry surface, so fall back
            // to the direction from centroid to origin, matching the
            // circle's inside-start convention above.
            (origin - centroid).normalize_or_zero()
        });

        Some((t_min, normal))
    }

    /// Projects this shape onto `axis` (which must be a unit vector),
    /// returning the (min, max) interval of the projection.
    pub fn project(&self, transform: &ShapeTransform, axis: Vec2) -> (f32, f32) {
        match self {
            Shape::Circle { radius, .. } => {
                let c = self.center(transform).dot(axis);
                let r = Self::scaled_circle_radius(*radius, transform);
                (c - r, c + r)
            }
            _ => {
                let mut min = f32::MAX;
                let mut max = f32::MIN;

                for v in self.vertices(transform) {
                    let p = v.dot(axis);
                    min = min.min(p);
                    max = max.max(p);
                }

                (min, max)
            }
        }
    }
}

/// Closest point on segment `a`-`b` to `point`, clamped to the segment
/// (not the infinite line through it).
fn closest_point_on_segment(a: Vec2, b: Vec2, point: Vec2) -> Vec2 {
    let ab = b - a;
    let len_sq = ab.length_squared();

    let t = if len_sq > f32::EPSILON {
        ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        // Degenerate zero-length edge - a and b coincide, treat as a point.
        0.0
    };

    a + ab * t
}

/// Runs the separating axis test between two shapes at their given
/// world-space collider offsets.
///
/// Returns `None` if a separating axis was found (no collision), or
/// `Some((normal, penetration))` where `normal` is a unit vector pointing
/// from `b` toward `a` along the axis of minimum penetration, and
/// `penetration` is the overlap depth along that axis.
///
/// This is shape-agnostic
pub fn sat_collide(a: &Shape, a_transform: &ShapeTransform, b: &Shape, b_transform: &ShapeTransform) -> Option<(Vec2, f32)> {
    let mut axes = a.axes_against(a_transform, b, b_transform);
    axes.extend(b.axes_against(b_transform, a, a_transform));

    // Circle-vs-circle: neither circle contributes an axis via
    // axes_against (each needs the *other's* closest point, and a
    // circle's closest point to a point outside it collapses to the same
    // center-to-center axis anyway). Handle it directly as a fallback
    // whenever no axis was found at all.
    if axes.is_empty() {
        let delta = b.center(b_transform) - a.center(a_transform);
        if delta.length_squared() < f32::EPSILON {
            // Centers coincide exactly - no well-defined normal.
            return None;
        }
        axes.push(delta.normalize());
    }

    let mut min_overlap = f32::MAX;
    let mut mtv_axis = Vec2::ZERO;

    for axis in axes {
        let (a_min, a_max) = a.project(a_transform, axis);
        let (b_min, b_max) = b.project(b_transform, axis);

        let overlap = a_max.min(b_max) - a_min.max(b_min);

        // Found a separating axis - shapes cannot be overlapping.
        if overlap <= 0.0 {
            return None;
        }

        if overlap < min_overlap {
            min_overlap = overlap;
            mtv_axis = axis;
        }
    }

    // The axis was picked without regard to direction (normalize() could
    // point either way); orient it to point from b toward a, matching
    // CollisionInfo's documented convention ("direction to push the A
    // collider during resolution").
    let center_delta = a.center(a_transform) - b.center(b_transform);
    if center_delta.dot(mtv_axis) < 0.0 {
        mtv_axis = -mtv_axis;
    }

    Some((mtv_axis, min_overlap))
}

fn reflect_local(v: Vec2, reflect: (bool, bool)) -> Vec2 {
    Vec2::new(
        if reflect.0 {-v.x} else {v.x},
        if reflect.1 {-v.y} else {v.y}
    )
}