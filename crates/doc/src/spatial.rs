//! Pre-Pass 3D Multi-Canvas Spatial Data Model (Types 1, 2, and 3 Canvases).
//!
//! Defines the foundational 3D spatial primitives (`Vec3`, `Quat`, `Transform3D`),
//! 2D clipping enclosures (`CanvasBoundary2D`), and the 3 canvas modalities:
//! - **Type 1 (`Type1ZDepthBillboard`)**: Camera-orthogonal depth plane locked to Z translation.
//! - **Type 2 (`Type2ExpandablePlanar`)**: 6-DOF anchored planar sheet with dynamic 2D expansion.
//! - **Type 3 (`Type3FreeformBounded`)**: 6-DOF planar surface clipped by a Circle, Polygon, or
//!   arbitrary closed `PathData` Bézier contour.

#![forbid(unsafe_code)]

use std::ops::{Add, Mul, Sub};

use kurbo::Shape as _;
use serde::{Deserialize, Serialize};
use vectorcraft_geom::{BezPath, FillRule, PathData, Point, Rect, shapes};

use crate::Document;

/// Key in `Document::unknown` storing an optional `SpatialStageManifest` when a document participates
/// in a multi-canvas 3D stage scene.
pub const STAGE_DOC_KEY: &str = "spatialStage";

/// Stable identifier for a spatial canvas node inside a `SpatialStageManifest`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CanvasNodeId(pub u64);

/// Right-handed 3D vector (VectorCraft stage convention: +X right, +Y up, +Z toward viewer).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0 };

    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[must_use]
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    #[must_use]
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    #[must_use]
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 1e-12 && len.is_finite() {
            Self::new(self.x / len, self.y / len, self.z / len)
        } else {
            Self::Z
        }
    }

    #[must_use]
    pub fn lerp(self, rhs: Self, t: f64) -> Self {
        Self::new(
            self.x + (rhs.x - self.x) * t,
            self.y + (rhs.y - self.y) * t,
            self.z + (rhs.z - self.z) * t,
        )
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

/// Unit quaternion representing a 3D rotation `(x, y, z, w)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    #[must_use]
    pub fn from_axis_angle(axis: Vec3, angle_rad: f64) -> Self {
        let n = axis.normalized();
        let half = angle_rad * 0.5;
        let (s, c) = half.sin_cos();
        Self {
            x: n.x * s,
            y: n.y * s,
            z: n.z * s,
            w: c,
        }
        .normalized()
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        let n2 = self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
        if n2 > 1e-18 && n2.is_finite() {
            let inv = 1.0 / n2.sqrt();
            Self {
                x: self.x * inv,
                y: self.y * inv,
                z: self.z * inv,
                w: self.w * inv,
            }
        } else {
            Self::IDENTITY
        }
    }

    #[must_use]
    pub fn conjugate(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    #[must_use]
    pub fn mul_quat(self, rhs: Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }

    #[must_use]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let q = self.normalized();
        let u = Vec3::new(q.x, q.y, q.z);
        let uv = u.cross(v);
        let uuv = u.cross(uv);
        v + (uv * q.w + uuv) * 2.0
    }
}

/// 6-DOF 3D affine transform for positioning a 2D canvas plane in 3D stage space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform3D {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform3D {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::new(1.0, 1.0, 1.0),
    };

    /// Maps a local 2D canvas point `(u, v)` (where `z = 0`) into 3D world space.
    #[must_use]
    pub fn local_2d_to_world(&self, pt: Point) -> Vec3 {
        let scaled = Vec3::new(pt.x * self.scale.x, pt.y * self.scale.y, 0.0);
        self.translation + self.rotation.rotate_vec3(scaled)
    }

    /// Maps a 3D world point into local 3D canvas space `(u, v, w_normal_offset)`.
    #[must_use]
    pub fn world_to_local_3d(&self, world: Vec3) -> Vec3 {
        let rel = world - self.translation;
        let unrot = self.rotation.conjugate().rotate_vec3(rel);
        let sx = if self.scale.x.abs() > 1e-9 { self.scale.x } else { 1.0 };
        let sy = if self.scale.y.abs() > 1e-9 { self.scale.y } else { 1.0 };
        let sz = if self.scale.z.abs() > 1e-9 { self.scale.z } else { 1.0 };
        Vec3::new(unrot.x / sx, unrot.y / sy, unrot.z / sz)
    }

    /// Unit world-space normal of the canvas plane (`+Z` rotated by `self.rotation`).
    #[must_use]
    pub fn plane_normal(&self) -> Vec3 {
        self.rotation.rotate_vec3(Vec3::Z).normalized()
    }
}

/// 2D clipping boundary for a spatial canvas plane.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanvasBoundary2D {
    /// Unbounded infinite 2D plane (Type 2 dynamic sheet).
    #[default]
    Infinite,
    /// Axis-aligned rectangle in local 2D canvas points.
    Rect { rect: Rect },
    /// Analytical circle enclosure in local 2D canvas points (Type 3).
    Circle { center: Point, radius: f64 },
    /// Closed polygon enclosure in local 2D canvas points (Type 3).
    Polygon { vertices: Vec<Point> },
    /// Arbitrary closed cubic Bézier contour enclosure in local 2D canvas points (Type 3).
    BezierContour { path: PathData, fill_rule: FillRule },
}

impl CanvasBoundary2D {
    /// Exact point-in-boundary test in local 2D canvas coordinates.
    #[must_use]
    pub fn contains(&self, pt: Point) -> bool {
        if !pt.x.is_finite() || !pt.y.is_finite() {
            return false;
        }
        match self {
            Self::Infinite => true,
            Self::Rect { rect } => pt.x >= rect.x0 && pt.x <= rect.x1 && pt.y >= rect.y0 && pt.y <= rect.y1,
            Self::Circle { center, radius } => {
                let r = (*radius).max(0.0);
                pt.distance_squared(*center) <= r * r
            }
            Self::Polygon { vertices } => {
                if vertices.len() < 3 {
                    return false;
                }
                let mut inside = false;
                let mut j = vertices.len() - 1;
                for i in 0..vertices.len() {
                    if let (Some(&vi), Some(&vj)) = (vertices.get(i), vertices.get(j)) {
                        let intersects = ((vi.y > pt.y) != (vj.y > pt.y))
                            && (pt.x < (vj.x - vi.x) * (pt.y - vi.y) / (vj.y - vi.y + 1e-18) + vi.x);
                        if intersects {
                            inside = !inside;
                        }
                    }
                    j = i;
                }
                inside
            }
            Self::BezierContour { path, fill_rule } => {
                let bp = path.to_bezpath();
                let w = bp.winding(pt);
                match fill_rule {
                    FillRule::NonZero => w != 0,
                    FillRule::EvenOdd => w % 2 != 0,
                }
            }
        }
    }

    /// Converts bounded enclosures into a `(BezPath, FillRule)` pair suitable for `vello_cpu` clipping.
    #[must_use]
    pub fn to_clip_bezpath(&self) -> Option<(BezPath, FillRule)> {
        match self {
            Self::Infinite => None,
            Self::Rect { rect } => Some((shapes::rectangle(*rect).to_bezpath(), FillRule::NonZero)),
            Self::Circle { center, radius } => {
                let r = (*radius).max(0.0);
                let rect = Rect::new(center.x - r, center.y - r, center.x + r, center.y + r);
                Some((shapes::ellipse(rect).to_bezpath(), FillRule::NonZero))
            }
            Self::Polygon { vertices } => {
                if vertices.len() < 3 {
                    return None;
                }
                let mut bp = BezPath::new();
                if let Some(&first) = vertices.first() {
                    bp.move_to(first);
                    for &v in vertices.iter().skip(1) {
                        bp.line_to(v);
                    }
                    bp.close_path();
                }
                Some((bp, FillRule::NonZero))
            }
            Self::BezierContour { path, fill_rule } => Some((path.to_bezpath(), *fill_rule)),
        }
    }

    /// Bounding box of the 2D enclosure in local canvas points (`None` for `Infinite`).
    #[must_use]
    pub fn bounding_rect(&self) -> Option<Rect> {
        match self {
            Self::Infinite => None,
            Self::Rect { rect } => Some(*rect),
            Self::Circle { center, radius } => {
                let r = (*radius).max(0.0);
                Some(Rect::new(center.x - r, center.y - r, center.x + r, center.y + r))
            }
            Self::Polygon { vertices } => {
                let first = *vertices.first()?;
                let mut r = Rect::new(first.x, first.y, first.x, first.y);
                for &p in vertices.iter().skip(1) {
                    r = Rect::new(r.x0.min(p.x), r.y0.min(p.y), r.x1.max(p.x), r.y1.max(p.y));
                }
                Some(r)
            }
            Self::BezierContour { path, .. } => {
                let bp = path.to_bezpath();
                (!bp.is_empty()).then(|| bp.bounding_box())
            }
        }
    }
}

/// The three 3D Multi-Canvas spatial modalities specified for the Blender Bridge pre-pass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "modality", rename_all = "camelCase")]
pub enum CanvasModality {
    /// **Type 1: Z-Depth Locked Canvas (Planar Billboard / Depth Plane)**.
    /// Orthogonal to the active camera; translates strictly along camera Z-depth.
    Type1ZDepthBillboard {
        z_depth: f64,
        perspective_scale: bool,
        bounds: Rect,
    },
    /// **Type 2: Expandable Planar Canvas (X-Y Orthographic/Perspective Sheet)**.
    /// Anchored in 3D space with 6-DOF transform; its 2D X-Y boundary dynamically expands.
    Type2ExpandablePlanar {
        transform: Transform3D,
        materialized_bounds: Rect,
        auto_expand: bool,
    },
    /// **Type 3: Freeform Bounded Canvas (Custom Geometry / Projection Surface)**.
    /// Full 6-DOF 3D transform with a Circle, Polygon, or closed Bézier clipping contour.
    Type3FreeformBounded {
        transform: Transform3D,
        boundary: CanvasBoundary2D,
        double_sided: bool,
    },
}

impl Default for CanvasModality {
    fn default() -> Self {
        Self::Type1ZDepthBillboard {
            z_depth: 0.0,
            perspective_scale: true,
            bounds: Rect::new(0.0, 0.0, 1920.0, 1080.0),
        }
    }
}

impl CanvasModality {
    /// Expands `materialized_bounds` on a Type 2 canvas when new artwork is drawn outside the current sheet.
    pub fn expand_to_include_rect(&mut self, art_bounds: Rect, padding: f64) {
        if let Self::Type2ExpandablePlanar { materialized_bounds, auto_expand: true, .. } = self {
            let padded = art_bounds.inflate(padding.max(0.0), padding.max(0.0));
            *materialized_bounds = materialized_bounds.union(padded);
        }
    }
}

/// A single 2D VectorCraft document plane hosted inside the 3D Multi-Canvas Stage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialCanvasNode {
    pub id: CanvasNodeId,
    pub name: String,
    /// Links to the isolated `DocState::uid` (or embedded document index) owning this canvas's
    /// layer stack, undo/redo history, artboards, and selection state.
    pub doc_uid: u64,
    pub modality: CanvasModality,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
}

impl SpatialCanvasNode {
    #[must_use]
    pub fn new(id: CanvasNodeId, name: impl Into<String>, doc_uid: u64, modality: CanvasModality) -> Self {
        Self {
            id,
            name: name.into(),
            doc_uid,
            modality,
            visible: true,
            locked: false,
            opacity: 1.0,
        }
    }
}

/// Scene-level manifest describing multiple coexisting 2D canvases in 3D space.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialStageManifest {
    pub active_canvas: CanvasNodeId,
    pub canvases: Vec<SpatialCanvasNode>,
}

/// Reads the optional `SpatialStageManifest` stored in `doc.unknown["spatialStage"]`.
#[must_use]
pub fn stage_manifest(doc: &Document) -> Option<SpatialStageManifest> {
    doc.unknown
        .get(STAGE_DOC_KEY)
        .and_then(|v| serde_json::from_value::<SpatialStageManifest>(v.clone()).ok())
}

/// Stores `manifest` into `doc.unknown["spatialStage"]` (or removes the key if `canvases` is empty).
pub fn store_stage_manifest(doc: &mut Document, manifest: &SpatialStageManifest) {
    if manifest.canvases.is_empty() {
        doc.unknown.remove(STAGE_DOC_KEY);
    } else if let Ok(v) = serde_json::to_value(manifest) {
        doc.unknown.insert(STAGE_DOC_KEY.into(), v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_3d_roundtrips_local_2d_points() {
        let xf = Transform3D {
            translation: Vec3::new(100.0, -40.0, 250.0),
            rotation: Quat::from_axis_angle(Vec3::new(0.3, 0.8, 0.5), 0.72),
            scale: Vec3::new(2.0, 1.5, 1.0),
        };
        let p2 = Point::new(35.0, -18.0);
        let world = xf.local_2d_to_world(p2);
        let back = xf.world_to_local_3d(world);
        assert!((back.x - p2.x).abs() < 1e-9);
        assert!((back.y - p2.y).abs() < 1e-9);
        assert!(back.z.abs() < 1e-9);
    }

    #[test]
    fn type3_circle_polygon_and_bezier_boundaries_clip_accurately() {
        let circle = CanvasBoundary2D::Circle { center: Point::new(50.0, 50.0), radius: 25.0 };
        assert!(circle.contains(Point::new(60.0, 60.0)));
        assert!(!circle.contains(Point::new(80.0, 80.0)));
        assert!(circle.to_clip_bezpath().is_some());

        let poly = CanvasBoundary2D::Polygon {
            vertices: vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(50.0, 80.0)],
        };
        assert!(poly.contains(Point::new(50.0, 30.0)));
        assert!(!poly.contains(Point::new(90.0, 70.0)));

        let bez = CanvasBoundary2D::BezierContour {
            path: shapes::ellipse(Rect::new(10.0, 10.0, 90.0, 90.0)),
            fill_rule: FillRule::NonZero,
        };
        assert!(bez.contains(Point::new(50.0, 50.0)));
        assert!(!bez.contains(Point::new(12.0, 12.0)));
    }
}
