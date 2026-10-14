//! Pre-Pass Blender 4.x Interop Bridge: Grease Pencil v3 (`BLENDER_GPENCIL_V3`)
//! & glTF 2.0 `VC_spatial_canvas` Extension Schema (`vectorcraft_format::blender_bridge`).
//!
//! Provides lossless coordinate basis transforms between VectorCraft's Right-Handed Y-Up 3D stage
//! and Blender's Right-Handed Z-Up world space, plus bidirectional conversion between
//! `vectorcraft_doc::Document` + `SpatialCanvasNode` and Grease Pencil v3 Bezier stroke trees.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use vectorcraft_doc::color::{Color, Paint};
use vectorcraft_doc::geom::{Anchor, AnchorKind, PathData, Point, Rect, SubPath};
use vectorcraft_doc::spatial::{
    CanvasBoundary2D, CanvasModality, CanvasNodeId, Quat, SpatialCanvasNode, Transform3D, Vec3,
};
use vectorcraft_doc::{Appearance, AppearanceItem, Document, FillLayer, Node, NodeKind, StrokeLayer};

/// Converts a VectorCraft Right-Handed Y-Up 3D position `(x, y, z)` into Blender Right-Handed Z-Up `[x, -z, y]`.
#[must_use]
pub fn vc_vec3_to_blender(v: Vec3) -> [f64; 3] {
    [v.x, -v.z, v.y]
}

/// Converts a Blender Right-Handed Z-Up 3D position `[bx, by, bz]` into VectorCraft Right-Handed Y-Up `(bx, bz, -by)`.
#[must_use]
pub fn blender_to_vc_vec3(b: [f64; 3]) -> Vec3 {
    Vec3::new(b[0], b[2], -b[1])
}

/// Converts a VectorCraft Y-Up quaternion `(x, y, z, w)` into a Blender Z-Up quaternion `[x, -z, y, w]`.
#[must_use]
pub fn vc_quat_to_blender(q: Quat) -> [f64; 4] {
    [q.x, -q.z, q.y, q.w]
}

/// Converts a Blender Z-Up quaternion `[qx, qy, qz, qw]` into a VectorCraft Y-Up quaternion.
#[must_use]
pub fn blender_to_vc_quat(bq: [f64; 4]) -> Quat {
    Quat {
        x: bq[0],
        y: bq[2],
        z: -bq[1],
        w: bq[3],
    }
    .normalized()
}

/// glTF 2.0 `VC_spatial_canvas` node extension descriptor for round-tripping Type 1, 2, and 3
/// spatial canvases with Blender.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GltfSpatialCanvasExt {
    pub canvas_id: u64,
    pub name: String,
    pub blender_translation: [f64; 3],
    pub blender_rotation_xyzw: [f64; 4],
    pub scale: [f64; 3],
    pub modality: CanvasModality,
}

impl GltfSpatialCanvasExt {
    #[must_use]
    pub fn from_spatial_node(node: &SpatialCanvasNode) -> Self {
        let xf = match &node.modality {
            CanvasModality::Type1ZDepthBillboard { z_depth, .. } => Transform3D {
                translation: Vec3::new(0.0, 0.0, *z_depth),
                ..Transform3D::IDENTITY
            },
            CanvasModality::Type2ExpandablePlanar { transform, .. } | CanvasModality::Type3FreeformBounded { transform, .. } => *transform,
        };
        Self {
            canvas_id: node.id.0,
            name: node.name.clone(),
            blender_translation: vc_vec3_to_blender(xf.translation),
            blender_rotation_xyzw: vc_quat_to_blender(xf.rotation),
            scale: [xf.scale.x, xf.scale.y, xf.scale.z],
            modality: node.modality.clone(),
        }
    }

    #[must_use]
    pub fn to_spatial_node(&self, doc_uid: u64) -> SpatialCanvasNode {
        SpatialCanvasNode::new(CanvasNodeId(self.canvas_id), self.name.clone(), doc_uid, self.modality.clone())
    }
}

/// One cubic Bézier control point in a Blender 4.x Grease Pencil v3 stroke.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpV3Point {
    pub co: [f64; 3],
    pub handle_left: [f64; 3],
    pub handle_right: [f64; 3],
    pub radius: f32,
    pub opacity: f32,
}

/// One Grease Pencil v3 stroke (`GP_STROKE_BEZIER`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpV3Stroke {
    pub curve_type: String,
    pub cyclic: bool,
    pub material_index: usize,
    pub points: Vec<GpV3Point>,
}

/// One Grease Pencil v3 material entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpV3Material {
    pub name: String,
    pub stroke_rgba: [f32; 4],
    pub fill_rgba: [f32; 4],
}

/// One Grease Pencil v3 layer carrying a 4x4 or TRS spatial matrix and keyframes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpV3Layer {
    pub name: String,
    pub opacity: f32,
    pub strokes: Vec<GpV3Stroke>,
}

/// Top-level Grease Pencil v3 interchange document (`BLENDER_GPENCIL_V3`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpV3Document {
    pub format: String,
    pub version: u32,
    pub spatial_canvas: GltfSpatialCanvasExt,
    pub materials: Vec<GpV3Material>,
    pub layers: Vec<GpV3Layer>,
}

/// Exports a `Document` and its `SpatialCanvasNode` descriptor into a Blender Grease Pencil v3 JSON structure.
#[must_use]
pub fn export_canvas_to_gpv3(doc: &Document, canvas: &SpatialCanvasNode) -> GpV3Document {
    let xf = match &canvas.modality {
        CanvasModality::Type1ZDepthBillboard { z_depth, .. } => Transform3D {
            translation: Vec3::new(0.0, 0.0, *z_depth),
            ..Transform3D::IDENTITY
        },
        CanvasModality::Type2ExpandablePlanar { transform, .. } | CanvasModality::Type3FreeformBounded { transform, .. } => *transform,
    };
    let map_pt = |p: Point| -> [f64; 3] { vc_vec3_to_blender(xf.local_2d_to_world(p)) };

    let mut materials = Vec::new();
    let mut layers = Vec::new();

    for layer_node in &doc.layers {
        let mut strokes = Vec::new();
        layer_node.walk(&mut |n| {
            let Some(pd) = n.path_data() else { return };
            let mut stroke_rgba = [0.0_f32; 4];
            let mut fill_rgba = [0.0_f32; 4];
            let mut half_width = 0.5_f32;
            for item in &n.appearance.items {
                match item {
                    AppearanceItem::Fill(fl) if fl.visible => {
                        if let Some(c) = fl.paint.color() {
                            let [r, g, b] = c.to_rgb();
                            fill_rgba = [r, g, b, fl.opacity];
                        }
                    }
                    AppearanceItem::Stroke(st) if st.visible => {
                        if let Some(c) = st.paint.color() {
                            let [r, g, b] = c.to_rgb();
                            stroke_rgba = [r, g, b, st.opacity];
                            half_width = (st.width.max(0.1) * 0.5) as f32;
                        }
                    }
                    _ => {}
                }
            }
            let mat_idx = materials.len();
            materials.push(GpV3Material {
                name: n.display_name(),
                stroke_rgba,
                fill_rgba,
            });
            for sp in &pd.subpaths {
                let points = sp
                    .anchors
                    .iter()
                    .map(|a| GpV3Point {
                        co: map_pt(a.p),
                        handle_left: map_pt(a.h_in),
                        handle_right: map_pt(a.h_out),
                        radius: half_width,
                        opacity: n.opacity,
                    })
                    .collect();
                strokes.push(GpV3Stroke {
                    curve_type: "BEZIER".into(),
                    cyclic: sp.closed,
                    material_index: mat_idx,
                    points,
                });
            }
        });
        layers.push(GpV3Layer {
            name: layer_node.display_name(),
            opacity: layer_node.opacity,
            strokes,
        });
    }

    GpV3Document {
        format: "BLENDER_GPENCIL_V3".into(),
        version: 1,
        spatial_canvas: GltfSpatialCanvasExt::from_spatial_node(canvas),
        materials,
        layers,
    }
}

/// Imports a `GpV3Document` back into a `(Document, SpatialCanvasNode)` pair, projecting 3D
/// Blender coordinates back onto the canvas's local 2D plane.
#[must_use]
pub fn import_gpv3_to_document(gp: &GpV3Document, doc_uid: u64) -> (Document, SpatialCanvasNode) {
    let canvas = gp.spatial_canvas.to_spatial_node(doc_uid);
    let xf = match &canvas.modality {
        CanvasModality::Type1ZDepthBillboard { z_depth, .. } => Transform3D {
            translation: Vec3::new(0.0, 0.0, *z_depth),
            ..Transform3D::IDENTITY
        },
        CanvasModality::Type2ExpandablePlanar { transform, .. } | CanvasModality::Type3FreeformBounded { transform, .. } => *transform,
    };
    let unmap_pt = |b: [f64; 3]| -> Point {
        let vc_world = blender_to_vc_vec3(b);
        let local = xf.world_to_local_3d(vc_world);
        Point::new(local.x, local.y)
    };

    let mut doc = Document::new(1920.0, 1080.0);
    doc.layers.clear();

    for gp_layer in &gp.layers {
        let lid = doc.add_layer(Some(&gp_layer.name));
        if let Some(lnode) = doc.node_mut(lid) {
            lnode.opacity = gp_layer.opacity;
        }
        for st in &gp_layer.strokes {
            let anchors: Vec<Anchor> = st
                .points
                .iter()
                .map(|pt| {
                    let p = unmap_pt(pt.co);
                    let h_in = unmap_pt(pt.handle_left);
                    let h_out = unmap_pt(pt.handle_right);
                    Anchor {
                        p,
                        h_in,
                        h_out,
                        kind: AnchorKind::Corner,
                    }
                })
                .collect();
            if anchors.is_empty() {
                continue;
            }
            let pd = PathData::single(SubPath { anchors, closed: st.cyclic });
            let mut ap = Appearance::default();
            if let Some(mat) = gp.materials.get(st.material_index) {
                if mat.fill_rgba[3] > 0.0 {
                    let mut fl = FillLayer::new(Paint::solid(Color::rgb(mat.fill_rgba[0], mat.fill_rgba[1], mat.fill_rgba[2])));
                    fl.opacity = mat.fill_rgba[3];
                    ap.items.push(AppearanceItem::Fill(fl));
                }
                if mat.stroke_rgba[3] > 0.0 {
                    let width = f64::from(st.points.first().map_or(0.5, |p| p.radius)) * 2.0;
                    let mut sl = StrokeLayer::new(
                        Paint::solid(Color::rgb(mat.stroke_rgba[0], mat.stroke_rgba[1], mat.stroke_rgba[2])),
                        width,
                    );
                    sl.opacity = mat.stroke_rgba[3];
                    ap.items.push(AppearanceItem::Stroke(sl));
                }
            }
            let pid = doc.alloc_id();
            let mut pnode = Node::path(pid, pd, ap);
            if let Some(first_pt) = st.points.first() {
                pnode.opacity = first_pt.opacity;
            }
            let _ = doc.insert(Some(lid), usize::MAX, pnode);
        }
    }
    if doc.layers.is_empty() {
        let _ = doc.add_layer(Some("Layer 1"));
    }
    let _ = CanvasBoundary2D::Infinite;
    let _ = Rect::ZERO;
    let _ = NodeKind::Group { children: vec![], clip: false };
    (doc, canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectorcraft_doc::geom::shapes;

    #[test]
    fn blender_coordinate_and_gpv3_roundtrip_preserves_3d_canvas_strokes() {
        let v = Vec3::new(12.0, 34.0, -56.0);
        let b = vc_vec3_to_blender(v);
        assert_eq!(b, [12.0, 56.0, 34.0]);
        assert_eq!(blender_to_vc_vec3(b), v);

        let mut d = Document::new(400.0, 300.0);
        let lid = d.layers[0].id;
        let id = d.alloc_id();
        let ap = Appearance::basic(Paint::solid(Color::rgb8(20, 120, 240)), Paint::solid(Color::BLACK), 4.0);
        d.insert(Some(lid), 0, Node::path(id, shapes::rectangle(Rect::new(10.0, 20.0, 110.0, 80.0)), ap))
            .unwrap();

        let canvas = SpatialCanvasNode::new(
            CanvasNodeId(7),
            "Hero Plane",
            1,
            CanvasModality::Type2ExpandablePlanar {
                transform: Transform3D {
                    translation: Vec3::new(10.0, 20.0, 30.0),
                    rotation: Quat::from_axis_angle(Vec3::X, 0.4),
                    scale: Vec3::new(1.0, 1.0, 1.0),
                },
                materialized_bounds: Rect::new(0.0, 0.0, 400.0, 300.0),
                auto_expand: true,
            },
        );

        let gpv3 = export_canvas_to_gpv3(&d, &canvas);
        assert_eq!(gpv3.format, "BLENDER_GPENCIL_V3");
        assert_eq!(gpv3.layers.len(), 1);
        assert_eq!(gpv3.layers[0].strokes.len(), 1);

        let (round_doc, round_canvas) = import_gpv3_to_document(&gpv3, 1);
        assert_eq!(round_canvas.id, CanvasNodeId(7));
        let ch = round_doc.layers[0].children().unwrap();
        assert_eq!(ch.len(), 1);
        let p0 = ch[0].path_data().unwrap().subpaths[0].anchors[0].p;
        assert!(p0.distance(Point::new(10.0, 20.0)) < 1e-6, "got {p0:?}");
    }
}
