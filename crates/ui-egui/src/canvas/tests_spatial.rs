use super::*;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Document};

#[test]
fn rotated_rectangle_bounds_include_all_corners_and_coordinate_roundtrips() {
    for zoom in [0.001, 0.1, 1.0, 64.0, 1000.0] {
        for rotation in [0.0f64, 30.0, 45.0, 90.0, 177.0] {
            let xf = Xf {
                rect: egui::Rect::from_min_size(pos2(30.0, 50.0), vec2(900.0, 600.0)),
                zoom,
                center: Point::new(-1400.0, 3000.0),
                rot: rotation.to_radians(),
            };
            let r = Rect::new(-1500.0, 2900.0, -1300.0, 3100.0);
            let b = xf.rect_to_screen(r);
            for p in xf.quad(r) {
                assert!(b.contains(p), "{rotation} {zoom} {b:?} {p:?}");
            }
            for p in [r.center(), Point::new(r.x0, r.y1)] {
                let back = xf.to_doc(xf.to_screen(p));
                assert!(back.distance(p) < 0.1 / zoom, "{rotation} {zoom} {p:?} {back:?}");
            }
            let delta = xf.delta_to_doc(vec2(100.0, -80.0));
            let a = xf.to_screen(xf.center);
            let b = xf.to_screen(xf.center + delta);
            assert!((b - a - vec2(100.0, -80.0)).length() < 0.001);
        }
    }
}

/// Rasterize only the actual grid painter's line/dot shapes for visual fixtures.
/// This is headless grid evidence, not a claim about a mounted native UI.
fn raster_grid(shapes: &[egui::epaint::ClippedShape], size: u32, ppp: f32) -> image::RgbaImage {
    let mut d = Document::new(size as f64, size as f64);
    let layer = d.layers[0].id;
    let mut append = |path: vectorcraft_geom::PathData, color: Color32, stroke: Option<f64>| {
        let [r, g, b, a] = color.to_srgba_unmultiplied();
        let paint = Paint::solid(Color::rgb8(r, g, b));
        let mut ap = if let Some(w) = stroke { Appearance::basic(Paint::None, paint, w) } else { Appearance::basic(paint, Paint::None, 0.0) };
        if let Some(f) = ap.fill_mut() {
            f.opacity = f32::from(a) / 255.0;
        }
        if let Some(s) = ap.stroke_mut() {
            s.opacity = f32::from(a) / 255.0;
        }
        let id = d.alloc_id();
        d.insert(Some(layer), usize::MAX, Node::path(id, path, ap)).unwrap();
    };
    for clipped in shapes {
        match &clipped.shape {
            Shape::LineSegment { points: [a, b], stroke } => {
                append(
                    vectorcraft_geom::shapes::line(Point::new(f64::from(a.x), f64::from(a.y)), Point::new(f64::from(b.x), f64::from(b.y))),
                    stroke.color,
                    Some(f64::from(stroke.width)),
                );
            }
            Shape::Mesh(m) => {
                for v in m.vertices.chunks_exact(4) {
                    let b = v.iter().fold(egui::Rect::NOTHING, |b, v| b.union(egui::Rect::from_min_max(v.pos, v.pos)));
                    append(
                        vectorcraft_geom::shapes::rectangle(Rect::new(
                            f64::from(b.min.x),
                            f64::from(b.min.y),
                            f64::from(b.max.x),
                            f64::from(b.max.y),
                        )),
                        v[0].color,
                        None,
                    );
                }
            }
            _ => {}
        }
    }
    let px = (size as f32 * ppp) as u32;
    let img = vectorcraft_render::Renderer::new().render(
        &d,
        px,
        px,
        Affine::scale(f64::from(ppp)),
        &vectorcraft_render::RenderOptions { background: Some([250, 248, 244, 255]), ..Default::default() },
    );
    image::RgbaImage::from_raw(px, px, img.to_straight()).unwrap()
}

#[test]
fn adaptive_visual_fixtures_cover_dpi_zoom_and_both_board_modes() {
    for ppp in [1.0, 2.0] {
        for zoom in [0.001, 1.0, 1000.0] {
            for infinite in [false, true] {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(ppp);
                let rect = egui::Rect::from_min_size(Pos2::ZERO, vec2(256.0, 256.0));
                let xf = Xf { rect, zoom, center: Point::new(-137.3, 92.7), rot: 0.0 };
                let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(rect), ..Default::default() }, |ui| {
                    let painter = ui.painter().with_clip_rect(rect);
                    if infinite {
                        paint_infinite_grid(&painter, &xf, [250, 248, 244]);
                    } else {
                        grid(&painter, &xf, 72.0, 8, LineLook { color: Color32::from_gray(180), dots: false });
                    }
                });
                out.textures_delta.clear();
                let lines: Vec<_> =
                    out.shapes.iter().filter_map(|s| if let Shape::LineSegment { stroke, .. } = &s.shape { Some(stroke) } else { None }).collect();
                assert!(!lines.is_empty());
                assert!(lines.iter().all(|s| (s.width * ppp - 1.0).abs() < 1e-6));
                assert!(out.shapes.len() < 1000);
                if let Some(dir) = std::env::var_os("VECTORCRAFT_SPATIAL_FIXTURES") {
                    let dir = std::path::PathBuf::from(dir);
                    std::fs::create_dir_all(&dir).unwrap();
                    let path = dir.join(format!("grid-{}-dpi{}-zoom{zoom}.png", if infinite { "infinite" } else { "finite" }, ppp as u32));
                    raster_grid(&out.shapes, 256, ppp).save(path).unwrap();
                }
            }
        }
    }
}

#[test]
fn dot_style_and_curved_perspective_visual_fixtures() {
    use vectorcraft_tools::distort::perspective::{PerspectiveGrid, grid_overlays_in};
    for ppp in [1.0, 2.0] {
        for mode in [0, 4, 5] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(ppp);
            let rect = egui::Rect::from_min_size(Pos2::ZERO, vec2(256.0, 256.0));
            let xf = Xf { rect, zoom: 1.0, center: Point::new(128.0, 128.0), rot: 0.0 };
            let mut doc = Document::new(256.0, 256.0);
            let mut g = PerspectiveGrid::preset(mode.max(1), Rect::new(0.0, 0.0, 256.0, 256.0));
            g.origin = [128.0, 180.0];
            g.horizon = 128.0;
            g.distance = 32.0;
            g.cell = 8.0;
            g.store(&mut doc);
            let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(rect), ..Default::default() }, |ui| {
                let painter = ui.painter().with_clip_rect(rect);
                if mode == 0 {
                    grid(&painter, &xf, 72.0, 8, LineLook { color: Color32::from_gray(140), dots: true });
                } else {
                    for o in grid_overlays_in(&doc, 1.0, None) {
                        if let Overlay::GridLine { a, b, color } = o {
                            painter.line_segment(
                                [xf.to_screen(a), xf.to_screen(b)],
                                Stroke::new(1.0 / ppp, Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])),
                            );
                        }
                    }
                }
            });
            out.textures_delta.clear();
            assert!(!out.shapes.is_empty());
            if let Some(dir) = std::env::var_os("VECTORCRAFT_SPATIAL_FIXTURES") {
                let path = std::path::PathBuf::from(dir).join(format!("grid-mode{mode}-dpi{}.png", ppp as u32));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                raster_grid(&out.shapes, 256, ppp).save(path).unwrap();
            }
        }
    }
}
