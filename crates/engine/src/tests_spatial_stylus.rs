//! Acceptance fixtures for the existing board/export pipeline and new input options.
use super::*;
use serde_json::json;
use vectorcraft_geom::Point;
use vectorcraft_tools::distort::perspective::{PerspectiveGrid, Plane};
use vectorcraft_tools::{Mods, Overlay, PointerEvent, PointerKind};

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":100,"height":80})).unwrap();
    s
}

#[test]
fn infinite_board_roundtrip_keeps_colors_offboard_art_and_finite_export_frames() {
    let mut s = session();
    s.execute("paint.setStroke", &json!({"none":true})).unwrap();
    let id = s.execute("shape.rectangle", &json!({"x":-20,"y":20,"width":50,"height":20})).unwrap()["id"].as_u64().unwrap();
    s.execute("shape.rectangle", &json!({"x":2000,"y":-1000,"width":40,"height":30})).unwrap();
    {
        let d = Arc::make_mut(&mut s.doc_mut().unwrap().doc);
        d.setup.infinite_canvas = true;
        d.setup.backdrop = Some("#123456".into());
        d.setup.artboard_color = Some("#fdf5e6".into());
    }
    let original = s.doc().unwrap().doc.clone();
    for compressed in [false, true] {
        let back = vectorcraft_format::load(&vectorcraft_format::save(&original, compressed)).unwrap();
        assert!(back.is_infinite_canvas());
        assert_eq!(back.setup, original.setup);
        assert_eq!(back.art_bounds(), original.art_bounds());
        assert_eq!(back.node(vectorcraft_doc::NodeId(id)).unwrap().geometric_bounds().unwrap().x0, -20.0);
    }
    let png = s.execute("document.export", &json!({"format":"png","useArtboards":true})).unwrap();
    let bytes = vectorcraft_format::base64_decode(png["dataBase64"].as_str().unwrap()).unwrap();
    let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(img.dimensions(), (100, 80));
    assert!(img.get_pixel(0, 25)[3] > 0, "cross-frame artwork is retained");
    let pdf = s.execute("document.export", &json!({"format":"pdf","useArtboards":true})).unwrap();
    let bytes = vectorcraft_format::base64_decode(pdf["dataBase64"].as_str().unwrap()).unwrap();
    let pages = vectorcraft_pdf::info(&bytes, None).unwrap().pages;
    assert_eq!(pages.len(), 1);
    assert_eq!((pages[0].width, pages[0].height), (100.0, 80.0));
}

#[test]
fn all_five_perspective_modes_save_reopen_and_undo_with_horizon_controls() {
    let mut s = session();
    for kind in 1..=5 {
        s.execute("perspective.grid.preset", &json!({"kind":kind})).unwrap();
        let old = PerspectiveGrid::current(&s.doc().unwrap().doc);
        s.execute("perspective.grid.define", &json!({"horizonHeight":35,"distance":100})).unwrap();
        let g = PerspectiveGrid::current(&s.doc().unwrap().doc);
        assert_eq!(g.kind, kind);
        assert!((g.origin[1] - g.horizon - 35.0).abs() < 1e-8);
        let back = vectorcraft_format::load(&vectorcraft_format::save(&s.doc().unwrap().doc, true)).unwrap();
        assert_eq!(PerspectiveGrid::current(&back), g);
        assert!(!g.lines(Plane::Left).is_empty());
        assert_eq!(g.homography(Plane::Left).is_some(), kind <= 3);
        if kind > 3 {
            let e = s.execute("perspective.attach", &json!({})).unwrap_err().to_string();
            assert!(e.contains("curved reference"), "{e}");
            assert!(g.widget_hit(&s.doc().unwrap().doc, 1.0, Point::new(10.0, 20.0)).is_none());
        }
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(PerspectiveGrid::current(&s.doc().unwrap().doc), old);
        s.execute("edit.redo", &json!({})).unwrap();
        assert_eq!(PerspectiveGrid::current(&s.doc().unwrap().doc), g);
    }
}

fn stroke(prediction: f64, stabilize: bool, bypass: bool) -> (Vec<u8>, bool) {
    let mut s = session();
    let view = ViewInfo::default();
    s.select_tool("paintbrush", view).unwrap();
    s.set_tool_option_cmd(&json!({"values":{"predictionMs":prediction,"stabilize":stabilize,"closeWithin":0}})).unwrap();
    let mut seen = false;
    for i in 0..60 {
        let t = i as f64 / 120.0;
        let mut ev =
            PointerEvent::new(if i == 0 { PointerKind::Down } else { PointerKind::Drag }, 5.0 + i as f64 * 1.2, 40.0 + (i as f64 * 0.2).sin() * 4.0);
        ev.time = Some(t);
        ev.mods = Mods { ctrl: bypass, ..Default::default() };
        s.pointer(&ev, view).unwrap();
        seen |= s.overlays(view).iter().any(|o| matches!(o, Overlay::Line { .. }));
        s.tool_tick(1.0 / 120.0, view).unwrap();
    }
    let mut ev = PointerEvent::new(PointerKind::Up, 75.8, 40.0 + (59.0f64 * 0.2).sin() * 4.0);
    ev.time = Some(0.5);
    ev.mods = Mods { ctrl: bypass, ..Default::default() };
    s.pointer(&ev, view).unwrap();
    assert_eq!(s.doc().unwrap().history.undo.len(), 1);
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.art_bounds().is_none());
    s.execute("edit.redo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.doc().unwrap().doc).unwrap();
    (before, seen)
}

#[test]
fn prediction_changes_only_feedback_not_committed_geometry_and_ctrl_bypasses_filter() {
    let (off, _) = stroke(0.0, false, false);
    let (on, seen) = stroke(14.0, false, false);
    assert!(seen, "the display head actually appeared");
    assert_eq!(off, on, "no extrapolated geometry saved");
    assert_eq!(off, stroke(14.0, true, true).0, "Ctrl bypass preserves original input");
    assert_ne!(off, stroke(0.0, true, false).0, "the optional stabilizer changes real samples");
}

#[test]
fn options_are_per_tool_persistent_and_discoverable() {
    let mut s = session();
    let v = s
        .set_tool_option_cmd(
            &json!({"tool":"pencil","values":{"stabilize":true,"predictionMs":14,"magnetic":true,"snapStrength":0.6,"angleLock":30}}),
        )
        .unwrap();
    assert_eq!(v["predictionMs"], json!(14.0));
    assert_eq!(s.tool_options_of("paintbrush")["stabilize"], json!(false));
    let prefs = serde_json::to_value(&s.prefs).unwrap();
    let mut reopened = session();
    reopened.prefs = serde_json::from_value(prefs).unwrap();
    reopened.select_tool("pencil", ViewInfo::default()).unwrap();
    for key in ["stabilize", "predictionMs", "magnetic", "snapStrength", "angleLock"] {
        assert_eq!(reopened.tool_options()[key], v[key], "{key}");
    }
}
