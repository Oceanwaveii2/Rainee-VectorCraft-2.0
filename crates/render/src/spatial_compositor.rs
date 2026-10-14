//! Pre-Pass 3D Multi-Canvas BSP Shard Splitter, Back-to-Front Depth Sorter,
//! and Type 3 Bézier Contour Clipping Helper (`vectorcraft_render::spatial_compositor`).
//!
//! Resolves overlapping and intersecting 3D canvas planes via Sutherland-Hodgman plane clipping
//! so transparent vector canvases composite in exact back-to-front painter's order without
//! Z-fighting or alpha sorting artifacts.

#![forbid(unsafe_code)]

use vectorcraft_doc::spatial::{
    CanvasBoundary2D, CanvasModality, CanvasNodeId, SpatialCanvasNode, Vec3,
};
use vectorcraft_geom::{Affine, Point, Rect};
use vello_cpu::RenderContext;

use crate::fill_rule;

/// A 3D vertex on a spatial canvas polygon shard carrying world position and 2D document UV coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageVertex3D {
    pub pos: Vec3,
    pub uv: Point,
}

impl StageVertex3D {
    #[must_use]
    pub fn lerp(self, rhs: Self, t: f64) -> Self {
        Self {
            pos: self.pos.lerp(rhs.pos, t),
            uv: self.uv.lerp(rhs.uv, t),
        }
    }
}

/// A convex 3D polygon shard representing all or part of a `SpatialCanvasNode` for back-to-front compositing.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasShard3D {
    pub canvas_id: CanvasNodeId,
    pub doc_uid: u64,
    pub vertices: Vec<StageVertex3D>,
    pub normal: Vec3,
    /// Plane constant $d = n \cdot p_0$ for plane equation $n \cdot x - d = 0$.
    pub plane_d: f64,
    pub opacity: f32,
}

impl CanvasShard3D {
    /// Centroid of the shard in 3D world space.
    #[must_use]
    pub fn centroid(&self) -> Vec3 {
        if self.vertices.is_empty() {
            return Vec3::ZERO;
        }
        let mut sum = Vec3::ZERO;
        for v in &self.vertices {
            sum = sum + v.pos;
        }
        sum * (1.0 / (self.vertices.len() as f64))
    }

    /// Builds the initial 4-corner 3D quad shard for a `SpatialCanvasNode`.
    #[must_use]
    pub fn from_canvas_node(node: &SpatialCanvasNode, fallback_bounds: Rect) -> Self {
        let (xf, rect) = match &node.modality {
            CanvasModality::Type1ZDepthBillboard { z_depth, bounds, .. } => {
                let mut t = vectorcraft_doc::spatial::Transform3D::IDENTITY;
                t.translation = Vec3::new(0.0, 0.0, *z_depth);
                (t, *bounds)
            }
            CanvasModality::Type2ExpandablePlanar { transform, materialized_bounds, .. } => (*transform, *materialized_bounds),
            CanvasModality::Type3FreeformBounded { transform, boundary, .. } => {
                (*transform, boundary.bounding_rect().unwrap_or(fallback_bounds))
            }
        };
        let corners_2d = [
            Point::new(rect.x0, rect.y0),
            Point::new(rect.x1, rect.y0),
            Point::new(rect.x1, rect.y1),
            Point::new(rect.x0, rect.y1),
        ];
        let vertices: Vec<StageVertex3D> = corners_2d
            .into_iter()
            .map(|uv| StageVertex3D { pos: xf.local_2d_to_world(uv), uv })
            .collect();
        let normal = xf.plane_normal();
        let plane_d = normal.dot(xf.translation);
        Self {
            canvas_id: node.id,
            doc_uid: node.doc_uid,
            vertices,
            normal,
            plane_d,
            opacity: node.opacity,
        }
    }
}

/// Splits `shard` against the 3D plane `plane_normal · x - plane_d = 0` with thickness `eps`.
/// Returns `(back_shard, front_shard)` via Sutherland-Hodgman polygon clipping.
#[must_use]
pub fn split_shard_by_plane(
    shard: &CanvasShard3D,
    plane_normal: Vec3,
    plane_d: f64,
    eps: f64,
) -> (Option<CanvasShard3D>, Option<CanvasShard3D>) {
    if shard.vertices.len() < 3 {
        return (None, None);
    }
    let dist = |p: Vec3| plane_normal.dot(p) - plane_d;
    let mut has_front = false;
    let mut has_back = false;
    for v in &shard.vertices {
        let d = dist(v.pos);
        if d > eps {
            has_front = true;
        } else if d < -eps {
            has_back = true;
        }
    }
    if !has_front && !has_back {
        // Coplanar: keep on front side
        return (None, Some(shard.clone()));
    }
    if !has_back {
        return (None, Some(shard.clone()));
    }
    if !has_front {
        return (Some(shard.clone()), None);
    }

    let mut back_verts = Vec::with_capacity(shard.vertices.len() + 2);
    let mut front_verts = Vec::with_capacity(shard.vertices.len() + 2);
    let n = shard.vertices.len();

    for i in 0..n {
        let Some(&curr) = shard.vertices.get(i) else { continue };
        let Some(&prev) = shard.vertices.get((i + n - 1) % n) else { continue };
        let d_curr = dist(curr.pos);
        let d_prev = dist(prev.pos);

        let prev_in_front = d_prev >= 0.0;
        let curr_in_front = d_curr >= 0.0;

        if prev_in_front != curr_in_front {
            let denom = d_prev - d_curr;
            let t = if denom.abs() > 1e-12 { (d_prev / denom).clamp(0.0, 1.0) } else { 0.5 };
            let mid = prev.lerp(curr, t);
            front_verts.push(mid);
            back_verts.push(mid);
        }
        if curr_in_front {
            front_verts.push(curr);
        } else {
            back_verts.push(curr);
        }
    }

    let make = |verts: Vec<StageVertex3D>| {
        (verts.len() >= 3).then(|| CanvasShard3D {
            canvas_id: shard.canvas_id,
            doc_uid: shard.doc_uid,
            vertices: verts,
            normal: shard.normal,
            plane_d: shard.plane_d,
            opacity: shard.opacity,
        })
    };
    (make(back_verts), make(front_verts))
}

/// Resolves intersecting canvas shards by splitting mutually intersecting planes and sorting
/// the resulting non-intersecting shards in back-to-front painter's order relative to `camera_eye`.
#[must_use]
pub fn sort_shards_back_to_front(initial: &[CanvasShard3D], camera_eye: Vec3) -> Vec<CanvasShard3D> {
    let mut shards: Vec<CanvasShard3D> = Vec::new();
    for candidate in initial {
        let mut fragments = vec![candidate.clone()];
        for existing in &shards {
            // If planes are non-parallel, split fragments against existing plane
            if candidate.normal.cross(existing.normal).length_squared() > 1e-6 {
                let mut next_frags = Vec::new();
                for frag in fragments {
                    let (back, front) = split_shard_by_plane(&frag, existing.normal, existing.plane_d, 1e-3);
                    if let Some(b) = back {
                        next_frags.push(b);
                    }
                    if let Some(f) = front {
                        next_frags.push(f);
                    }
                }
                fragments = next_frags;
            }
        }
        shards.extend(fragments);
    }

    shards.sort_by(|a, b| {
        let da = (a.centroid() - camera_eye).length_squared();
        let db = (b.centroid() - camera_eye).length_squared();
        db.total_cmp(&da)
    });
    shards
}

/// Pushes a Type 3 `CanvasBoundary2D` clip path (`Circle`, `Polygon`, or `BezierContour`) onto
/// a `vello_cpu::RenderContext`. Returns `true` if a clip layer was pushed (caller must call `ctx.pop_clip_path()`).
pub fn push_boundary_clip(ctx: &mut RenderContext, view: Affine, boundary: &CanvasBoundary2D) -> bool {
    let Some((bp, rule)) = boundary.to_clip_bezpath() else {
        return false;
    };
    ctx.set_transform(view);
    ctx.set_fill_rule(fill_rule(rule));
    ctx.push_clip_path(&bp);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_doc::spatial::{Quat, Transform3D};

    #[test]
    fn intersecting_canvas_shards_split_along_intersection_line_and_sort_back_to_front() {
        let bounds = Rect::new(-100.0, -100.0, 100.0, 100.0);
        let node_a = SpatialCanvasNode::new(
            CanvasNodeId(1),
            "Plane A",
            1,
            CanvasModality::Type2ExpandablePlanar {
                transform: Transform3D::IDENTITY,
                materialized_bounds: bounds,
                auto_expand: false,
            },
        );
        let node_b = SpatialCanvasNode::new(
            CanvasNodeId(2),
            "Plane B Tilted",
            2,
            CanvasModality::Type2ExpandablePlanar {
                transform: Transform3D {
                    translation: Vec3::ZERO,
                    rotation: Quat::from_axis_angle(Vec3::Y, 0.6),
                    scale: Vec3::new(1.0, 1.0, 1.0),
                },
                materialized_bounds: bounds,
                auto_expand: false,
            },
        );
        let s_a = CanvasShard3D::from_canvas_node(&node_a, bounds);
        let s_b = CanvasShard3D::from_canvas_node(&node_b, bounds);
        let sorted = sort_shards_back_to_front(&[s_a, s_b], Vec3::new(0.0, 0.0, 500.0));
        assert_eq!(sorted.len(), 3, "tilted plane B splits across plane A into 2 shards + 1 shard for A");
        let d0 = (sorted[0].centroid() - Vec3::new(0.0, 0.0, 500.0)).length_squared();
        let d2 = (sorted[2].centroid() - Vec3::new(0.0, 0.0, 500.0)).length_squared();
        assert!(d0 >= d2, "sorted back-to-front: {d0} >= {d2}");
    }
}
