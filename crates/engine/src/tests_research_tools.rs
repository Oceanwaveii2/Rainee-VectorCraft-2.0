use crate::{
    Session,
    research_tools::{Junction, Proposal},
};
use serde_json::json;
use vectorcraft_doc::NodeId;
use vectorcraft_geom::{Point, Rect};
fn fixture() -> (Session, Vec<NodeId>) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    std::sync::Arc::make_mut(&mut s.doc_mut().unwrap().doc).metadata.created = Some(0);
    s.paint.fill = vectorcraft_color::Paint::None;
    s.paint.stroke_width = 4.0;
    let ids = ["M20 60 L60 60", "M64 60 L110 60"]
        .iter()
        .map(|d| NodeId(s.execute("path.create", &json!({"d":d})).unwrap()["id"].as_u64().unwrap()))
        .collect();
    (s, ids)
}
fn proposal(s: &Session, ids: &[NodeId]) -> Proposal {
    Proposal {
        max_gap: 8.0,
        kind: "junction".into(),
        originals: ids.iter().map(|id| s.doc().unwrap().doc.node(*id).unwrap().clone()).collect(),
        junctions: vec![Junction { a: ids[0], tail: true, b: ids[1], point: Point::new(64.0, 60.0), confidence: 0.95, kind: "endpoint".into() }],
        png: vec![],
        region: Rect::ZERO,
        size: [128, 128],
        telemetry: json!({}),
    }
}
#[test]
fn accepted_join_is_undoable_and_journal_replays_without_worker() {
    let (mut s, ids) = fixture();
    let p = proposal(&s, &ids);
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    s.execute("assist.accept", &json!({"proposal":p})).unwrap();
    let command = s.journal.last().unwrap().1.clone();
    let after = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    assert_ne!(before, after);
    let (mut replay, _) = fixture();
    replay.execute("assist.accept", &command).unwrap();
    assert_eq!(vectorcraft_format::save(&replay.doc().unwrap().doc, false), after);
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    s.execute("edit.redo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), after);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.doc().unwrap().doc).unwrap();
}
#[test]
fn malformed_join_does_not_change_art() {
    let (mut s, ids) = fixture();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let mut p = proposal(&s, &ids);
    p.junctions[0].point = Point::new(1e6, 1e6);
    assert!(s.execute("assist.accept", &json!({"proposal":p})).is_err());
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
}
#[test]
fn cancel_reject_and_discovery_cover_both_workers() {
    let (mut s, _) = fixture();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    s.execute("assist.cancel", &json!({})).unwrap();
    s.execute("assist.reject", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    for name in [
        "assist.junction.preview",
        "assist.brush.preview",
        "assist.status",
        "assist.accept",
        "assist.cancel",
        "assist.reject",
        "assist.brush.release",
    ] {
        assert!(crate::cmd::command_specs().iter().any(|c| c.id == name && !c.params.is_empty()));
    }
}
#[test]
fn telemetry_retains_accepted_timing_but_never_attributes_it_to_new_jobs() {
    let (mut s, ids) = fixture();
    let mut p = proposal(&s, &ids);
    p.telemetry = json!({"seconds":0.05,"backend":"fixture"});
    s.research_tools.proposal = Some(p);
    s.research_tools.job = 1;
    s.research_tools.clear("accepted");
    assert_eq!(s.research_tools.status()["telemetry"]["seconds"], 0.05);
    s.research_tools.clear("running");
    assert!(s.research_tools.status()["telemetry"].is_null());
    assert_eq!(s.research_tools.telemetry()["lastCompleted"]["seconds"], 0.05);
    for _ in 0..40 {
        s.research_tools.clear("rejected");
    }
    assert_eq!(s.research_tools.telemetry()["history"].as_array().unwrap().len(), 32);
}
#[test]
fn raster_appearance_keeps_editable_sources_release_undo_and_exports() {
    let (mut s, ids) = fixture();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let mut p = proposal(&s, &ids);
    p.kind = "brush".into();
    p.region = Rect::new(0.0, 0.0, 128.0, 128.0);
    p.junctions.clear();
    p.png = vectorcraft_render::Rendered { width: 128, height: 128, pixels: vec![128; 128 * 128 * 4] }.to_png().unwrap();
    let result = s.execute("assist.accept", &json!({"proposal":p})).unwrap();
    let after = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let back = vectorcraft_format::load(&after).unwrap();
    assert_eq!(vectorcraft_format::save(&back, false), after);
    for id in &ids {
        assert!(back.node(*id).is_some());
        assert!(!back.is_visible(*id));
    }
    assert!(vectorcraft_svg::export(&back, &Default::default()).contains("image"));
    assert!(vectorcraft_pdf::export(&back, &Default::default()).unwrap().starts_with(b"%PDF-"));
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    s.execute("edit.redo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), after);
    s.execute("assist.brush.release", &json!({"id":result["group"]})).unwrap();
    for id in &ids {
        assert!(s.doc().unwrap().doc.is_visible(*id));
    }
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), after);
}
#[test]
fn learning_is_optional_and_does_not_change_committed_strokes() {
    fn draw(enabled: bool) -> Vec<u8> {
        let (mut s, _) = fixture();
        s.select_tool("paintbrush", Default::default()).unwrap();
        s.set_tool_options(None, json!({"learnedPrediction":enabled,"predictionMs":14.0}).as_object().unwrap());
        for i in 0..24 {
            let kind = if i == 0 {
                vectorcraft_tools::PointerKind::Down
            } else if i == 23 {
                vectorcraft_tools::PointerKind::Up
            } else {
                vectorcraft_tools::PointerKind::Drag
            };
            let mut ev = vectorcraft_tools::PointerEvent::new(kind, 20.0 + i as f64 * 3.0, 100.0);
            ev.time = Some(i as f64 / 240.0);
            s.pointer(&ev, Default::default()).unwrap();
        }
        vectorcraft_format::save(&s.doc().unwrap().doc, false)
    }
    assert_eq!(draw(false), draw(true));
}
