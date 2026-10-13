use crate::{
    Session,
    cleanup::{self, Group, Plan},
};
use serde_json::json;
use vectorcraft_doc::NodeId;
use vectorcraft_geom::Point;

fn fixture() -> (Session, Vec<NodeId>) {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    std::sync::Arc::make_mut(&mut s.doc_mut().unwrap().doc).metadata.created = Some(0);
    s.paint.fill = vectorcraft_color::Paint::None;
    s.paint.stroke_width = 2.0;
    let mut ids = Vec::new();
    for d in
        ["M20 50 L40 46 L60 44 L80 46 L100 50 L120 56", "M20 51 L40 47 L60 45 L80 47 L100 51 L120 57", "M120 55 L100 49 L80 45 L60 43 L40 45 L20 49"]
    {
        let r = s.execute("path.create", &json!({"d":d})).unwrap();
        ids.push(NodeId(r["id"].as_u64().unwrap()));
    }
    s.doc_mut().unwrap().selection.set(ids.clone());
    (s, ids)
}
fn plan(s: &Session, ids: &[NodeId]) -> Plan {
    let input = cleanup::capture(s, ids, 0.25, 4.0, 30.0).unwrap();
    let mut plan = input.plan;
    plan.groups = vec![Group { originals: ids.to_vec(), points: input.strokes[0].clone() }];
    plan
}
#[test]
fn acceptance_undo_redo_native_svg_and_journal_replay() {
    let (mut s, ids) = fixture();
    let p = plan(&s, &ids);
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let n = s.doc().unwrap().history.undo.len();
    let command = json!({"proposal":p});
    let r = s.execute("sketch.cleanup.accept", &command).unwrap();
    assert!(s.journal.last().unwrap().1.get("proposal").is_some());
    assert_eq!(s.doc().unwrap().history.undo.len(), n + 1);
    for id in ids {
        assert!(s.doc().unwrap().doc.node(id).is_some());
        assert!(!s.doc().unwrap().doc.is_visible(id));
    }
    let fit = NodeId(r["ids"][0].as_u64().unwrap());
    assert_eq!(s.doc().unwrap().doc.node(fit).unwrap().path_data().unwrap().subpaths.len(), 1);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.doc().unwrap().doc).unwrap();
    let after = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let reopened = vectorcraft_format::load(&after).unwrap();
    assert_eq!(vectorcraft_format::save(&reopened, false), after);
    let svg = vectorcraft_svg::export(&reopened, &Default::default());
    assert!(svg.contains("<path"));
    assert!(!svg.contains("NaN"));
    let pdf = vectorcraft_pdf::export(&reopened, &Default::default()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    s.execute("edit.redo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), after);
    let (mut replay, _) = fixture();
    replay.execute("sketch.cleanup.accept", &command).unwrap();
    assert_eq!(vectorcraft_format::save(&replay.doc().unwrap().doc, false), after);
}
#[test]
fn reject_cancel_and_preview_do_not_mutate_art() {
    let (mut s, ids) = fixture();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    s.cleanup.uid = s.doc().unwrap().uid;
    s.cleanup.snapshot = Some(s.doc().unwrap().doc.clone());
    s.cleanup.plan = Some(plan(&s, &ids));
    s.cleanup.state = "ready".into();
    assert_eq!(s.execute("sketch.cleanup.status", &json!({})).unwrap()["state"], "ready");
    s.execute("sketch.cleanup.reject", &json!({})).unwrap();
    assert!(s.execute("sketch.cleanup.accept", &json!({})).is_err());
    s.execute("sketch.cleanup.cancel", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
}
#[test]
fn content_changes_revoke_preview() {
    let (mut s, ids) = fixture();
    s.cleanup.uid = s.doc().unwrap().uid;
    s.cleanup.snapshot = Some(s.doc().unwrap().doc.clone());
    s.cleanup.plan = Some(plan(&s, &ids));
    s.execute("path.create", &json!({"d":"M0 0 L10 10"})).unwrap();
    assert_eq!(s.execute("sketch.cleanup.status", &json!({})).unwrap()["state"], "stale");
    assert!(s.execute("sketch.cleanup.accept", &json!({})).is_err());
}
#[test]
fn malformed_unsafe_unknown_and_duplicate_targets_rejected_atomically() {
    let (mut s, ids) = fixture();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let original = plan(&s, &ids);
    for variant in 0..4 {
        let mut p = original.clone();
        match variant {
            0 => p.groups[0].points[0] = Point::new(1e8, 0.0),
            1 => p.groups[0].originals.push(ids[0]),
            2 => p.groups[0].originals.pop().map(|_| ()).unwrap(),
            _ => p.groups[0].originals[0] = NodeId(999999),
        }
        assert!(s.execute("sketch.cleanup.accept", &json!({"proposal":p})).is_err());
        assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    }
}
#[test]
fn expressive_strokes_are_not_silently_flattened() {
    let (mut s, ids) = fixture();
    s.edit("Expressive fixture", |d, _| {
        let st = d.node_mut(ids[0]).unwrap().appearance.stroke_mut().unwrap();
        st.brush = Some(vectorcraft_brush::EXPRESSIVE_PEN.into());
        st.nib = Some(Default::default());
        Ok(())
    })
    .unwrap();
    assert!(cleanup::capture(&s, &ids, 0.25, 4.0, 30.0).is_err());
}
#[test]
fn discovery_accounts_for_cleanup_commands() {
    for id in ["inspect", "start", "status", "accept", "reject", "cancel"] {
        assert!(crate::cmd::command_specs().iter().any(|c| c.id == format!("sketch.cleanup.{id}") && !c.params.is_empty()));
    }
}

#[test]
fn cleanup_consolidates_pressure_tilt_and_retains_each_original() {
    let (mut s, ids) = fixture();
    s.execute("brush.apply", &json!({"name":vectorcraft_brush::EXPRESSIVE_PEN,"ids":ids})).unwrap();
    s.edit("Dynamics fixture", |d, _| {
        for (i, id) in ids.iter().enumerate() {
            let st = d.node_mut(*id).unwrap().appearance.stroke_mut().unwrap();
            st.profile = Some(vectorcraft_doc::WidthProfile { points: vec![(0.0, 0.2, 0.2), (0.4, 1.0 + i as f64 * 0.1, 1.0), (1.0, 0.1, 0.1)] });
            st.nib = Some(vectorcraft_doc::NibProfile { points: vec![(0.0, 30.0, 0.5), (1.0, 60.0, 0.7)] });
        }
        Ok(())
    })
    .unwrap();
    let originals: Vec<_> = ids.iter().map(|id| s.doc().unwrap().doc.node(*id).unwrap().clone()).collect();
    assert_eq!(s.execute("sketch.cleanup.inspect", &json!({})).unwrap()["eligible"], true);
    let p = plan(&s, &ids);
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let result = s.execute("sketch.cleanup.accept", &json!({"proposal":p})).unwrap();
    let cleaned = NodeId(result["ids"][0].as_u64().unwrap());
    let st = s.doc().unwrap().doc.node(cleaned).unwrap().appearance.stroke().unwrap();
    assert_eq!(st.profile.as_ref().unwrap().points.len(), 3);
    assert_eq!(st.profile.as_ref().unwrap().points[1].1, 1.0);
    assert_eq!(st.nib.as_ref().unwrap().points[1].1, 60.0);
    for original in originals {
        assert_eq!(*s.doc().unwrap().doc.node(original.id).unwrap(), original);
    }
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.doc().unwrap().doc).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    s.execute("edit.redo", &json!({})).unwrap();
    assert!(s.doc().unwrap().doc.node(cleaned).unwrap().appearance.stroke().unwrap().nib.is_some());
}

#[test]
fn cleanup_preflight_matches_admission_without_starting_or_mutating() {
    let (mut s, ids) = fixture();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    assert_eq!(s.execute("sketch.cleanup.inspect", &json!({})).unwrap()["eligible"], true);
    let denied = s.execute("sketch.cleanup.inspect", &json!({"ids":[ids[0]]})).unwrap();
    assert_eq!(denied["eligible"], false);
    assert!(denied["reason"].as_str().unwrap().contains("2–64"));
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    assert!(!s.cleanup.running());
}

#[test]
fn cleanup_preserves_uniform_natural_brush_identity_and_originals() {
    let (mut s, ids) = fixture();
    s.execute("brush.apply", &json!({"name":"Rainee Dry Ink","ids":ids})).unwrap();
    assert_eq!(s.execute("sketch.cleanup.inspect", &json!({})).unwrap()["eligible"], true);
    let p = plan(&s, &ids);
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    let result = s.execute("sketch.cleanup.accept", &json!({"proposal":p})).unwrap();
    let cleaned = NodeId(result["ids"][0].as_u64().unwrap());
    assert_eq!(s.doc().unwrap().doc.node(cleaned).unwrap().appearance.stroke().unwrap().brush.as_deref(), Some("Rainee Dry Ink"));
    for id in ids {
        assert!(s.doc().unwrap().doc.node(id).is_some());
        assert!(!s.doc().unwrap().doc.is_visible(id));
    }
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.doc().unwrap().doc).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
}

#[test]
fn invalid_selection_is_reported_in_cleanup_status() {
    let (mut s, ids) = fixture();
    assert!(s.execute("sketch.cleanup.start", &json!({"ids":[ids[0]]})).is_err());
    let status = s.execute("sketch.cleanup.status", &json!({})).unwrap();
    assert_eq!(status["state"], "failed");
    assert!(status["message"].as_str().unwrap().contains("2–64"));
}

#[test]
fn unsafe_fit_preserves_sources_without_weakening_validation() {
    let (s, ids) = fixture();
    let input = cleanup::capture(&s, &ids, 0.25, 4.0, 30.0).unwrap();
    let mut p = plan(&s, &ids);
    p.groups[0].points[0] = Point::new(400.0, 400.0);
    assert!(cleanup::validate(&input, &p).is_err());
    cleanup::preserve_unsafe_groups(&input, &mut p).unwrap();
    assert_eq!(p.groups.len(), ids.len());
    for (i, group) in p.groups.iter().enumerate() {
        assert_eq!(group.originals, vec![ids[i]]);
        assert_eq!(group.points, input.strokes[i]);
    }
    cleanup::validate(&input, &p).unwrap();
    p.groups[0].originals[0] = NodeId(999999);
    assert!(cleanup::preserve_unsafe_groups(&input, &mut p).is_err());
}

#[test]
fn valid_bundle_survives_unrelated_bad_fit() {
    let (s, ids) = fixture();
    let input = cleanup::capture(&s, &ids, 0.25, 4.0, 30.0).unwrap();
    let mut p = input.plan.clone();
    p.groups = vec![
        Group { originals: vec![ids[0], ids[1]], points: input.strokes[0].clone() },
        Group { originals: vec![ids[2]], points: vec![Point::new(500.0, 500.0), Point::new(600.0, 600.0)] },
    ];
    cleanup::preserve_unsafe_groups(&input, &mut p).unwrap();
    assert_eq!(p.groups[0].originals.len(), 2);
    assert_eq!(p.groups[1].points, input.strokes[2]);
    cleanup::validate(&input, &p).unwrap();
}
#[test]
#[ignore = "Runs the optional pinned Windows research worker on this machine"]
fn cleanup_real_worker_bundle() {
    let (mut s, ids) = fixture();
    s.execute("sketch.cleanup.start", &json!({"ids":ids})).unwrap();
    let started = std::time::Instant::now();
    loop {
        let status = s.execute("sketch.cleanup.status", &json!({})).unwrap();
        if status["state"] != "running" {
            assert_eq!(status["state"], "ready", "{status}");
            break;
        }
        assert!(started.elapsed().as_secs() < 40);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(s.cleanup.plan.as_ref().unwrap().groups.len(), 1);
    let r = s.execute("sketch.cleanup.accept", &json!({})).unwrap();
    assert_eq!(r["ids"].as_array().unwrap().len(), 1);
}

#[test]
#[ignore = "Runs optional Windows worker on separation and hatching artwork fixtures"]
fn cleanup_real_worker_intent_fixtures() {
    for paths in [
        ["M20 40 L35 35 L50 30 L65 25 L80 20", "M100 40 L115 35 L130 30 L145 25 L160 20", "M20 110 L35 105 L50 100 L65 95 L80 90"],
        ["M20 30 L40 50 L60 70", "M20 60 L40 80 L60 100", "M20 90 L40 110 L60 130"],
    ] {
        let mut s = Session::new();
        s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
        s.paint.fill = vectorcraft_color::Paint::None;
        s.paint.stroke_width = 2.0;
        let mut ids = Vec::new();
        for d in paths {
            ids.push(s.execute("path.create", &json!({"d":d})).unwrap()["id"].as_u64().unwrap());
        }
        let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
        s.execute("sketch.cleanup.start", &json!({"ids":ids})).unwrap();
        let start = std::time::Instant::now();
        loop {
            let r = s.execute("sketch.cleanup.status", &json!({})).unwrap();
            if r["state"] != "running" {
                assert_eq!(r["state"], "ready", "{r}");
                break;
            }
            assert!(start.elapsed().as_secs() < 40);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(s.cleanup.plan.as_ref().unwrap().groups.len(), 3);
        assert!(s.execute("sketch.cleanup.accept", &json!({})).is_err());
        assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
    }
}
