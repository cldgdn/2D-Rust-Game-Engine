use glam::Vec2;

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

impl Shape {
    /// World-space center of this shape.
    ///
    /// For polygons this is the true centroid (average of world-space
    /// vertices), not just `offset` - for `Rectangle` these coincide
    /// exactly since its vertices are symmetric, but for `Triangle` they
    /// generally won't, and using the real centroid keeps the
    /// inside/outside orientation check in `sat_collide` correct
    /// regardless of where the caller chose to place `offset`.
    pub fn center(&self, world_offset: Vec2, world_rotation: f32) -> Vec2 {
        match self {
            Shape::Circle { offset, rotation_offset, .. } => {
                let anchor = world_offset;
                let (s, c) = (rotation_offset + world_rotation).sin_cos();

                anchor + Vec2::new(offset.x * c - offset.y * s, offset.x * s + offset.y * c)
            },
            _ => {
                let verts = self.vertices(world_offset, world_rotation);
                let sum: Vec2 = verts.iter().copied().sum();
                sum / verts.len() as f32
            }
        }
    }

    /// World-space vertices of this shape, in order, forming its boundary.
    /// Returns an empty vec for `Circle`, which has no vertices - callers
    /// must special-case circles rather than calling this on them.
    fn vertices(&self, world_offset: Vec2, world_rotation: f32) -> Vec<Vec2> {
        match self {
            Shape::Rectangle { half_extents, rotation_offset: rotation, offset } => {
                let center = world_offset + *offset;
                let (s, c) = (rotation + world_rotation).sin_cos();
                let hx = half_extents.x;
                let hy = half_extents.y;
                let rotated_center = world_offset + Vec2::new(offset.x * c - offset.y * s, offset.x * s + offset.y * c);

                [
                    Vec2::new(hx, hy),
                    Vec2::new(hx, -hy),
                    Vec2::new(-hx, -hy),
                    Vec2::new(-hx, hy),
                ]
                    .into_iter()
                    .map(|p| rotated_center + Vec2::new(p.x * c - p.y * s, p.x * s + p.y * c))
                    .collect()
            }
            Shape::Triangle { vertices, rotation_offset: rotation, offset } => {
                let anchor = world_offset + *offset;
                let (s, c) = (rotation + world_rotation).sin_cos();

                vertices
                    .iter()
                    .map(|p| anchor + Vec2::new(p.x * c - p.y * s, p.x * s + p.y * c))
                    .collect()
            }
            Shape::Circle { .. } => Vec::new(),
        }
    }

    /// World-space axis-aligned bounding box, used for section lookup.
    /// Returns the bottom-left and top-right coordinates of the box vertices.
    pub fn bounding_box(&self, world_offset: Vec2, world_rotation: f32) -> (Vec2, Vec2) {
        match self {
            Shape::Circle { radius, .. } => {
                let center = self.center(world_offset, world_rotation);
                (center - Vec2::splat(*radius), center + Vec2::splat(*radius))
            }
            _ => {
                let verts = self.vertices(world_offset, world_rotation);
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

    /// World-space points approximating this shape's boundary, for debug
    /// drawing as a closed line loop. Polygons return their exact `vertices()`
    /// - what you see is exactly what collision math is using. Circles have
    /// no discrete vertices, so they're tessellated into `segments` points;
    /// this is purely a rendering approximation and has no bearing on
    /// `sat_collide`, which treats circles exactly via center + radius.
    pub fn debug_outline_points(&self, world_offset: Vec2, world_rotation: f32, segments: usize) -> Vec<Vec2> {
        match self {
            Shape::Circle { radius, .. } => {
                let segments = segments.max(3);
                let center = self.center(world_offset, world_rotation);

                (0..segments)
                    .map(|i| {
                        let theta = (i as f32 / segments as f32) * std::f32::consts::TAU;
                        center + Vec2::new(theta.cos(), theta.sin()) * *radius
                    })
                    .collect()
            }
            _ => self.vertices(world_offset, world_rotation),
        }
    }

    /// The closest point on this shape's boundary to `point` (world space).
    /// Used to derive the one extra separating axis a circle needs.
    ///
    /// For polygons this walks every edge, finds the closest point on
    /// each edge *segment* (clamped, not the infinite line), and keeps
    /// the closest overall - this is what lets it work for a triangle,
    /// a rectangle, or any future convex polygon without per-shape code.
    fn closest_point_to(&self, world_offset: Vec2, world_rotation: f32, point: Vec2) -> Vec2 {
        match self {
            Shape::Circle { radius, .. } => {
                let center = self.center(world_offset, world_rotation);
                let dir = (point - center).normalize_or_zero();
                center + dir * *radius
            }
            _ => {
                let verts = self.vertices(world_offset, world_rotation);
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
    fn axes_against(&self, world_offset: Vec2, world_rotation: f32, other: &Shape, other_offset: Vec2, other_rotation: f32) -> Vec<Vec2> {
        match self {
            Shape::Circle { .. } => {
                let self_center = self.center(world_offset, world_rotation);
                let closest = other.closest_point_to(other_offset, other_rotation, self_center);
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
                let verts = self.vertices(world_offset, world_rotation);
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

    /// Projects this shape onto `axis` (which must be a unit vector),
    /// returning the (min, max) interval of the projection.
    fn project(&self, world_offset: Vec2, world_rotation: f32, axis: Vec2) -> (f32, f32) {
        match self {
            Shape::Circle { radius, .. } => {
                let c = self.center(world_offset, world_rotation).dot(axis);
                (c - radius, c + radius)
            }
            _ => {
                let mut min = f32::MAX;
                let mut max = f32::MIN;

                for v in self.vertices(world_offset, world_rotation) {
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
pub fn sat_collide(a: &Shape, a_offset: (Vec2, f32), b: &Shape, b_offset: (Vec2, f32)) -> Option<(Vec2, f32)> {
    let (a_offset, a_rotation) = a_offset;
    let (b_offset, b_rotation) = b_offset;

    let mut axes = a.axes_against(a_offset, a_rotation, b, b_offset, b_rotation);
    axes.extend(b.axes_against(b_offset, b_rotation, a, a_offset, a_rotation));

    // Circle-vs-circle: neither circle contributes an axis via
    // axes_against (each needs the *other's* closest point, and a
    // circle's closest point to a point outside it collapses to the same
    // center-to-center axis anyway). Handle it directly as a fallback
    // whenever no axis was found at all.
    if axes.is_empty() {
        let delta = b.center(b_offset, b_rotation) - a.center(a_offset, a_rotation);
        if delta.length_squared() < f32::EPSILON {
            // Centers coincide exactly - no well-defined normal.
            return None;
        }
        axes.push(delta.normalize());
    }

    let mut min_overlap = f32::MAX;
    let mut mtv_axis = Vec2::ZERO;

    for axis in axes {
        let (a_min, a_max) = a.project(a_offset, a_rotation, axis);
        let (b_min, b_max) = b.project(b_offset, b_rotation, axis);

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
    let center_delta = a.center(a_offset, a_rotation) - b.center(b_offset, b_rotation);
    if center_delta.dot(mtv_axis) < 0.0 {
        mtv_axis = -mtv_axis;
    }

    Some((mtv_axis, min_overlap))
}