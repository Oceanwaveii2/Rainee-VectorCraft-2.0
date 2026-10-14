//! 3D Multi-Canvas Raycasting Input Router, Local UV Homography Jacobian,
//! and Non-Destructive Document Switching Pre-Pass (`vectorcraft_engine::spatial_router`).
//!
//! Maps 2D viewport pointer coordinates through a 3D perspective or orthographic camera ray,
//! intersects Type 1 (Z-Depth Billboard), Type 2 (Expandable Planar), and Type 3 (Freeform
//! Circle/Polygon/Bézier Bounded) canvases, and computes the local 2D Jacobian scale so
//! existing 2D tools and `Learned` 5-DOF stylus predictors operate without modification.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use vectorcraft_doc::spatial::{
    CanvasModality, CanvasNodeId, SpatialCanvasNode, SpatialStageManifest, Vec3,
};
use vectorcraft_geom::Point;

use crate::Session;

/// 3D stage camera parameters (perspective or orthographic) for raycasting pointer input.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageCamera3D {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    /// Vertical field of view in radians (used when `orthographic_scale` is `None`).
    pub fov_y_rad: f64,
    /// When `Some(height_world)`, the camera uses orthographic projection of `height_world` units.
    pub orthographic_scale: Option<f64>,
    pub near: f64,
    pub far: f64,
}

impl Default for StageCamera3D {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 0.0, 1000.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y_rad: std::f64::consts::FRAC_PI_3,
            orthographic_scale: None,
            near: 0.1,
            far: 100_000.0,
        }
    }
}

/// A 3D ray with origin and unit direction.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ray3D {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray3D {
    #[must_use]
    pub fn at(&self, t: f64) -> Vec3 {
        self.origin + self.dir * t
    }
}

impl StageCamera3D {
    /// Unit forward direction from `eye` toward `target`.
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        let d = self.target - self.eye;
        if d.length_squared() > 1e-18 {
            d.normalized()
        } else {
            Vec3::new(0.0, 0.0, -1.0)
        }
    }

    /// Unit right basis vector of the camera frame.
    #[must_use]
    pub fn right(&self) -> Vec3 {
        let f = self.forward();
        let r = f.cross(self.up);
        if r.length_squared() > 1e-18 {
            r.normalized()
        } else {
            Vec3::X
        }
    }

    /// Unit orthonormal up vector of the camera frame.
    #[must_use]
    pub fn ortho_up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalized()
    }

    /// Unprojects a 2D viewport pixel `screen_px` within `viewport_size = (width, height)` into a world-space `Ray3D`.
    #[must_use]
    pub fn screen_ray(&self, screen_px: Point, viewport_size: (f64, f64)) -> Ray3D {
        let vw = viewport_size.0.max(1.0);
        let vh = viewport_size.1.max(1.0);
        let aspect = vw / vh;
        // Normalized device coordinates in [-1, 1], with +X right and +Y up
        let ndc_x = (2.0 * screen_px.x / vw) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_px.y / vh);

        let fwd = self.forward();
        let right = self.right();
        let up = self.ortho_up();

        if let Some(ortho_h) = self.orthographic_scale {
            let half_h = ortho_h * 0.5;
            let half_w = half_h * aspect;
            let origin = self.eye + right * (ndc_x * half_w) + up * (ndc_y * half_h);
            Ray3D { origin, dir: fwd }
        } else {
            let tan_half_fov = (self.fov_y_rad.clamp(0.05, 2.8) * 0.5).tan();
            let dir = (fwd + right * (ndc_x * aspect * tan_half_fov) + up * (ndc_y * tan_half_fov)).normalized();
            Ray3D { origin: self.eye, dir }
        }
    }
}

/// Result of intersecting a 3D pointer ray against a `SpatialCanvasNode`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialRayHit {
    pub canvas_id: CanvasNodeId,
    pub doc_uid: u64,
    pub distance_t: f64,
    pub world_hit: Vec3,
    /// Exact 2D point in the target canvas's `Document` coordinate space.
    pub local_pt: Point,
}

/// Intersects `ray` with a single `SpatialCanvasNode` (handling Type 1, Type 2, and Type 3 modalities).
/// When `ignore_boundary` is `true` (used during an active stroke drag), skips the 2D enclosure clip
/// so fast drags past a canvas edge continue projecting onto the active plane.
#[must_use]
pub fn intersect_canvas_plane(
    ray: &Ray3D,
    camera: &StageCamera3D,
    node: &SpatialCanvasNode,
    ignore_boundary: bool,
) -> Option<SpatialRayHit> {
    if !node.visible {
        return None;
    }
    match &node.modality {
        CanvasModality::Type1ZDepthBillboard { z_depth, perspective_scale, bounds } => {
            // Billboard plane is orthogonal to camera.forward() at distance z_depth along forward axis from target
            let fwd = camera.forward();
            let right = camera.right();
            let up = camera.ortho_up();
            let plane_origin = camera.target + fwd * (*z_depth);
            let plane_normal = Vec3::new(-fwd.x, -fwd.y, -fwd.z);

            let denom = ray.dir.dot(plane_normal);
            if denom.abs() <= 1e-9 {
                return None;
            }
            let t = (plane_origin - ray.origin).dot(plane_normal) / denom;
            if t < camera.near || t > camera.far || !t.is_finite() {
                return None;
            }
            let hit = ray.at(t);
            let rel = hit - plane_origin;
            let scale = if *perspective_scale {
                1.0
            } else {
                let dist = (plane_origin - camera.eye).length().max(1.0);
                let ref_dist = (camera.target - camera.eye).length().max(1.0);
                (dist / ref_dist).clamp(0.05, 20.0)
            };
            // Local 2D document coordinates (X right, Y down relative to bounds center)
            let center = bounds.center();
            let u = center.x + rel.dot(right) / scale;
            let v = center.y - rel.dot(up) / scale;
            let local_pt = Point::new(u, v);
            if !ignore_boundary && (u < bounds.x0 || u > bounds.x1 || v < bounds.y0 || v > bounds.y1) {
                return None;
            }
            Some(SpatialRayHit {
                canvas_id: node.id,
                doc_uid: node.doc_uid,
                distance_t: t,
                world_hit: hit,
                local_pt,
            })
        }
        CanvasModality::Type2ExpandablePlanar { transform, materialized_bounds, auto_expand } => {
            let normal = transform.plane_normal();
            let denom = ray.dir.dot(normal);
            if denom.abs() <= 1e-9 {
                return None;
            }
            let t = (transform.translation - ray.origin).dot(normal) / denom;
            if t < camera.near || t > camera.far || !t.is_finite() {
                return None;
            }
            let hit = ray.at(t);
            let local_3d = transform.world_to_local_3d(hit);
            let local_pt = Point::new(local_3d.x, local_3d.y);
            if !ignore_boundary
                && !*auto_expand
                && (local_pt.x < materialized_bounds.x0
                    || local_pt.x > materialized_bounds.x1
                    || local_pt.y < materialized_bounds.y0
                    || local_pt.y > materialized_bounds.y1)
            {
                return None;
            }
            Some(SpatialRayHit {
                canvas_id: node.id,
                doc_uid: node.doc_uid,
                distance_t: t,
                world_hit: hit,
                local_pt,
            })
        }
        CanvasModality::Type3FreeformBounded { transform, boundary, double_sided } => {
            let normal = transform.plane_normal();
            let denom = ray.dir.dot(normal);
            if denom.abs() <= 1e-9 {
                return None;
            }
            if !*double_sided && denom > 0.0 {
                // Back-face culled
                return None;
            }
            let t = (transform.translation - ray.origin).dot(normal) / denom;
            if t < camera.near || t > camera.far || !t.is_finite() {
                return None;
            }
            let hit = ray.at(t);
            let local_3d = transform.world_to_local_3d(hit);
            let local_pt = Point::new(local_3d.x, local_3d.y);
            if !ignore_boundary && !boundary.contains(local_pt) {
                return None;
            }
            Some(SpatialRayHit {
                canvas_id: node.id,
                doc_uid: node.doc_uid,
                distance_t: t,
                world_hit: hit,
                local_pt,
            })
        }
    }
}

/// Raycasts through `stage` to locate the closest hit canvas (or locks to `locked_canvas` during an active stroke drag).
#[must_use]
pub fn raycast_stage(
    screen_px: Point,
    viewport_size: (f64, f64),
    camera: &StageCamera3D,
    stage: &SpatialStageManifest,
    locked_canvas: Option<CanvasNodeId>,
) -> Option<SpatialRayHit> {
    let ray = camera.screen_ray(screen_px, viewport_size);
    if let Some(lock_id) = locked_canvas
        && let Some(node) = stage.canvases.iter().find(|c| c.id == lock_id)
    {
        return intersect_canvas_plane(&ray, camera, node, true);
    }

    let mut best: Option<SpatialRayHit> = None;
    for node in &stage.canvases {
        if node.locked {
            continue;
        }
        if let Some(hit) = intersect_canvas_plane(&ray, camera, node, false)
            && best.is_none_or(|b| hit.distance_t < b.distance_t)
        {
            best = Some(hit);
        }
    }
    best
}

/// Computes the effective local 2D zoom factor (screen pixels per document point, $\det(J_{uv})^{-1/2}$)
/// at `screen_px` on `node` so 2D stroke tools and `Learned::prediction(zoom)` remain calibrated under 3D tilt.
#[must_use]
pub fn local_homography_zoom(
    screen_px: Point,
    viewport_size: (f64, f64),
    camera: &StageCamera3D,
    node: &SpatialCanvasNode,
) -> f64 {
    let r0 = camera.screen_ray(screen_px, viewport_size);
    let rx = camera.screen_ray(Point::new(screen_px.x + 1.0, screen_px.y), viewport_size);
    let ry = camera.screen_ray(Point::new(screen_px.x, screen_px.y + 1.0), viewport_size);

    let (Some(h0), Some(hx), Some(hy)) = (
        intersect_canvas_plane(&r0, camera, node, true),
        intersect_canvas_plane(&rx, camera, node, true),
        intersect_canvas_plane(&ry, camera, node, true),
    ) else {
        return 1.0;
    };
    let du = hx.local_pt - h0.local_pt;
    let dv = hy.local_pt - h0.local_pt;
    let det = (du.x * dv.y - du.y * dv.x).abs();
    if det > 1e-12 && det.is_finite() {
        (1.0 / det.sqrt()).clamp(1e-3, 1e4)
    } else {
        1.0
    }
}

/// Evaluates whether the active document in `session` has an open interaction or undo group that
/// must prevent switching to another spatial canvas mid-gesture.
#[must_use]
pub fn can_switch_spatial_canvas(session: &Session) -> bool {
    session
        .active()
        .is_none_or(|doc_state| doc_state.interaction.is_none() && doc_state.undo_group.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_doc::spatial::{CanvasBoundary2D, Quat, Transform3D};
    use vectorcraft_geom::Rect;

    #[test]
    fn raycast_stage_selects_frontmost_canvas_and_respects_type3_hole() {
        let cam = StageCamera3D::default();
        let viewport = (1000.0, 1000.0);

        // Front canvas at Z = 200 with a small circle radius 50 around (0, 0)
        let front = SpatialCanvasNode::new(
            CanvasNodeId(1),
            "Front Circle",
            101,
            CanvasModality::Type3FreeformBounded {
                transform: Transform3D {
                    translation: Vec3::new(0.0, 0.0, 200.0),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::new(1.0, 1.0, 1.0),
                },
                boundary: CanvasBoundary2D::Circle { center: Point::ZERO, radius: 50.0 },
                double_sided: true,
            },
        );
        // Back canvas at Z = 0 covering [-500..500]
        let back = SpatialCanvasNode::new(
            CanvasNodeId(2),
            "Back Sheet",
            102,
            CanvasModality::Type2ExpandablePlanar {
                transform: Transform3D::IDENTITY,
                materialized_bounds: Rect::new(-500.0, -500.0, 500.0, 500.0),
                auto_expand: false,
            },
        );
        let stage = SpatialStageManifest {
            active_canvas: CanvasNodeId(1),
            canvases: vec![back, front],
        };

        // Center ray (500, 500) hits the front circle at Z = 200
        let hit_center = raycast_stage(Point::new(500.0, 500.0), viewport, &cam, &stage, None).unwrap();
        assert_eq!(hit_center.canvas_id, CanvasNodeId(1));
        assert!(hit_center.local_pt.distance(Point::ZERO) < 1e-6);

        // Off-center ray (700, 500) misses the front circle and hits the back sheet!
        let hit_off = raycast_stage(Point::new(700.0, 500.0), viewport, &cam, &stage, None).unwrap();
        assert_eq!(hit_off.canvas_id, CanvasNodeId(2));

        // When locked_canvas is Some(CanvasNodeId(1)) (active stroke drag), off-center ray stays on Canvas 1
        let hit_locked = raycast_stage(Point::new(700.0, 500.0), viewport, &cam, &stage, Some(CanvasNodeId(1))).unwrap();
        assert_eq!(hit_locked.canvas_id, CanvasNodeId(1));

        let zoom = local_homography_zoom(Point::new(500.0, 500.0), viewport, &cam, &stage.canvases[0]);
        assert!(zoom.is_finite() && zoom > 0.1);
    }
}
