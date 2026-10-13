use crate::{Session, ViewInfo};
use serde_json::json;
use vectorcraft_doc::NodeId;
use vectorcraft_tools::{Mods, PointerEvent, PointerKind, ToolKey};

fn session() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":200,"height":100})).unwrap();
    s.paint.stroke_width = 20.0;
    s.select_tool("paintbrush", ViewInfo::default()).unwrap();
    s.set_tool_option_cmd(&json!({"values":{"pressureWidth":true,"tiltNib":true,"closeWithin":0}})).unwrap();
    s
}
fn draw(s: &mut Session, bypass: bool, tilt: Option<[f64; 2]>) -> NodeId {
    for i in 0..=20 {
        let mut e = PointerEvent::new(if i == 0 { PointerKind::Down } else { PointerKind::Drag }, 20.0 + i as f64 * 5.0, 50.0);
        e.pressure = if i < 10 { 0.2 } else { 0.9 };
        e.tilt = tilt;
        e.time = Some(i as f64 / 120.0);
        e.mods = Mods { ctrl: bypass, ..Default::default() };
        s.pointer(&e, ViewInfo::default()).unwrap();
    }
    s.pointer(&PointerEvent::new(PointerKind::Up, 120.0, 50.0), ViewInfo::default()).unwrap();
    s.active().unwrap().selection.objects[0]
}

#[test]
fn measured_pressure_and_tilt_are_editable_saveable_and_one_undo() {
    let mut s = session();
    let id = draw(&mut s, false, Some([60.0, 0.0]));
    let st = s.active().unwrap().doc.node(id).unwrap().appearance.stroke().unwrap();
    assert_eq!(st.brush.as_deref(), Some(vectorcraft_brush::EXPRESSIVE_PEN));
    assert!((st.profile.as_ref().unwrap().points[0].1 - 0.28).abs() < 1e-6);
    assert!((st.nib.as_ref().unwrap().points[0].2 - 0.5).abs() < 1e-6);
    assert_eq!(s.active().unwrap().history.undo.len(), 1);
    let original = vectorcraft_format::save(&s.active().unwrap().doc, false);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.active().unwrap().doc).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.active().unwrap().doc.art_bounds().is_none());
    s.execute("edit.redo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.active().unwrap().doc, false), original);
    s.execute("stroke.setAdvanced", &json!({"ids":[id.0],"nib":[[0,90,0.2],[1,90,0.2]]})).unwrap();
    assert_eq!(s.active().unwrap().doc.node(id).unwrap().appearance.stroke().unwrap().nib.as_ref().unwrap().at(0.5), (90.0, 0.2));
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.active().unwrap().doc, false), original);
}

#[test]
fn stationary_pressure_updates_do_not_add_duplicate_geometry() {
    let mut s = session();
    for (kind, x, pressure) in
        [(PointerKind::Down, 20.0, 0.2), (PointerKind::Drag, 80.0, 0.4), (PointerKind::Drag, 80.0, 0.9), (PointerKind::Up, 80.0, 0.0)]
    {
        let mut e = PointerEvent::new(kind, x, 50.0);
        e.pressure = pressure;
        s.pointer(&e, ViewInfo::default()).unwrap();
    }
    let id = s.active().unwrap().selection.objects[0];
    let n = s.active().unwrap().doc.node(id).unwrap();
    assert_eq!(n.path_data().unwrap().subpaths[0].anchors.len(), 2);
    assert!((n.appearance.stroke().unwrap().profile.as_ref().unwrap().at(1.0).0 - 0.91).abs() < 1e-6);
}

#[test]
fn ctrl_start_bypasses_and_escape_restores_without_committing() {
    let mut s = session();
    let id = draw(&mut s, true, Some([60.0, 0.0]));
    let st = s.active().unwrap().doc.node(id).unwrap().appearance.stroke().unwrap();
    assert!(st.nib.is_none() && st.profile.is_none());
    let saved = vectorcraft_format::save(&s.active().unwrap().doc, false);
    s.pointer(&PointerEvent::new(PointerKind::Down, 20.0, 70.0), ViewInfo::default()).unwrap();
    s.pointer(&PointerEvent::new(PointerKind::Drag, 120.0, 70.0), ViewInfo::default()).unwrap();
    s.tool_key(ToolKey::Escape, Mods::default(), ViewInfo::default()).unwrap();
    assert_eq!(vectorcraft_format::save(&s.active().unwrap().doc, false), saved);
    assert_eq!(s.active().unwrap().history.undo.len(), 1);
}

#[test]
fn missing_channels_have_uniform_width_and_fixed_nib_not_fake_tilt() {
    let mut s = session();
    let result = s
        .execute(
            "path.freehand",
            &json!({"points":[[20,50],[120,50]],"penSamples":[{},{}],
        "expressive":{"pressureWidth":true,"tiltNib":true,"nibRoundness":0.3,"nibAngle":45}}),
        )
        .unwrap();
    let st = s.active().unwrap().doc.node(NodeId(result["id"].as_u64().unwrap())).unwrap().appearance.stroke().unwrap();
    assert_eq!(st.profile.as_ref().unwrap().at(0.5), (1.0, 1.0));
    assert_eq!(st.nib.as_ref().unwrap().at(0.5), (45.0, 0.3));
}

#[test]
fn malformed_pen_requests_fail_without_editing_or_changing_history() {
    let mut s = session();
    let before = vectorcraft_format::save(&s.active().unwrap().doc, false);
    for samples in [json!([{}]), json!([{"pressure":2},{}]), json!([{"tilt":[91,0]},{}])] {
        assert!(s.execute("path.freehand", &json!({"points":[[20,50],[120,50]],"penSamples":samples,"expressive":{"pressureWidth":true}})).is_err());
        assert_eq!(vectorcraft_format::save(&s.active().unwrap().doc, false), before);
        assert!(s.active().unwrap().history.undo.is_empty());
    }
}

#[test]
fn expressive_ink_exports_through_shared_geometry_to_png_svg_and_pdf() {
    let mut s = session();
    let id = draw(&mut s, false, Some([0.0, 60.0]));
    let doc = &s.active().unwrap().doc;
    let pieces = vectorcraft_brush::node_pieces(doc, doc.node(id).unwrap()).unwrap();
    assert!(!pieces.is_empty());
    for format in ["png", "svg", "pdf"] {
        let export = s.execute("document.export", &json!({"format":format,"useArtboards":true})).unwrap();
        let bytes = vectorcraft_format::base64_decode(export["dataBase64"].as_str().unwrap()).unwrap();
        assert!(!bytes.is_empty(), "{format}");
        if format == "png" {
            let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
            assert_eq!(image.dimensions(), (200, 100));
            assert!(image.get_pixel(110, 58)[3] > 0, "wide pressure end is rendered");
            assert_eq!(image.get_pixel(25, 58)[3], 0, "thin pressure end is retained");
        }
        if format == "pdf" {
            assert_eq!(vectorcraft_pdf::info(&bytes, None).unwrap().pages.len(), 1);
        }
        if format == "svg" {
            assert!(String::from_utf8(bytes).unwrap().contains("<path"));
        }
    }
}

#[test]
fn expressive_options_are_discoverable_and_persistent_per_tool() {
    let s = session();
    let options = s.tool_options();
    assert_eq!(options["pressureWidth"], true);
    assert_eq!(s.tool_options_of("pencil")["pressureWidth"], false);
    let prefs = serde_json::to_value(&s.prefs).unwrap();
    let mut reopened = session();
    reopened.prefs = serde_json::from_value(prefs).unwrap();
    reopened.select_tool("pencil", ViewInfo::default()).unwrap();
    reopened.select_tool("paintbrush", ViewInfo::default()).unwrap();
    for k in ["pressureWidth", "pressureGamma", "pressureMin", "tiltNib", "nibRoundness", "nibAngle"] {
        assert_eq!(reopened.tool_options()[k], options[k]);
    }
}

#[test]
fn a_native_frame_batch_preserves_channels_and_matches_packet_replay() {
    let mut events = vec![];
    for i in 0..60 {
        let mut e =
            PointerEvent::new(if i == 0 { PointerKind::Down } else { PointerKind::Drag }, 10.0 + i as f64 * 2.0, 50.0 + (i as f64 * 0.1).sin() * 5.0);
        e.time = Some(i as f64 / 240.0);
        e.pressure = 0.1 + (i as f32 / 60.0) * 0.8;
        e.tilt = Some([i as f64, 10.0]);
        events.push(e);
    }
    events.push(PointerEvent { kind: PointerKind::Up, ..*events.last().unwrap() });
    let mut packet = session();
    let mut batch = session();
    for e in &events {
        packet.pointer(e, ViewInfo::default()).unwrap();
    }
    batch.pointer_batch(&events, ViewInfo::default()).unwrap();
    assert_eq!(vectorcraft_format::save(&packet.active().unwrap().doc, false), vectorcraft_format::save(&batch.active().unwrap().doc, false));
    assert_eq!(batch.active().unwrap().history.undo.len(), 1);
}

#[test]
fn windows_ctrl_command_mapping_still_bypasses_expressive_ink() {
    let mut s = session();
    for kind in [PointerKind::Down, PointerKind::Drag, PointerKind::Up] {
        let mut e = PointerEvent::new(kind, if kind == PointerKind::Down { 20.0 } else { 120.0 }, 50.0);
        e.mods = Mods { ctrl: true, cmd: true, ..Default::default() };
        s.pointer(&e, ViewInfo::default()).unwrap();
    }
    let id = s.active().unwrap().selection.objects[0];
    let st = s.active().unwrap().doc.node(id).unwrap().appearance.stroke().unwrap();
    assert!(st.profile.is_none() && st.nib.is_none());
}

#[test]
#[ignore = "requires the owner's local Windows Ink recording; never substitutes a synthetic fixture"]
fn owner_cintiq_recording_retains_pressure_nib_and_editability() {
    use vectorcraft_tools::pen_input::{Phase, RawPenPacket};
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/cintiq-input-probe/captures/cintiq-1791713023544-99848.jsonl");
    let text = std::fs::read_to_string(path).expect("owner capture must be present for this explicit test");
    let mut s = session();
    let mut events = Vec::new();
    let mut completed = 0;
    let mut eraser = false;
    for line in text.lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        if v["record"] != "sample" {
            continue;
        }
        let raw: RawPenPacket = serde_json::from_value(v["raw"].clone()).unwrap();
        let sample = raw.normalize(false).unwrap();
        if sample.phase == Phase::Down {
            events.clear();
            eraser = sample.eraser || sample.inverted;
        }
        let kind = match sample.phase {
            Phase::Down => PointerKind::Down,
            Phase::Drag => PointerKind::Drag,
            Phase::Up => PointerKind::Up,
            _ => continue,
        };
        if eraser {
            continue;
        }
        let [x, y] = raw.client_px.expect("recorded coordinate mapping");
        let dpi = raw.window_dpi as f64 / 96.0;
        assert!(dpi > 0.0);
        let mut e = PointerEvent::new(kind, x as f64 / dpi, y as f64 / dpi);
        e.pressure = sample.pressure.expect("measured pressure");
        e.time = sample.time_seconds;
        e.tilt = match sample.tilt_degrees {
            [Some(x), Some(y)] => Some([x as f64, y as f64]),
            _ => None,
        };
        events.push(e);
        if sample.phase == Phase::Up {
            s.pointer_batch(&events, ViewInfo::default()).unwrap();
            let id = s.active().unwrap().selection.objects[0];
            let st = s.active().unwrap().doc.node(id).unwrap().appearance.stroke().unwrap();
            assert!(st.profile.is_some() && st.nib.is_some());
            completed += 1;
            events.clear();
        }
    }
    assert_eq!(completed, 17, "22 complete recorded contacts, five eraser/inverted excluded");
    assert_eq!(s.active().unwrap().history.undo.len(), completed);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.active().unwrap().doc).unwrap();
    eprintln!("owner-recording replay: {completed} strokes, measured pressure/tilt, one undo each, exact native roundtrip");
}
