use crate::Session;
use serde_json::json;
use vectorcraft_doc::NodeId;
#[test]
fn expressive_input_keeps_current_vector_art_brush() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    s.execute("brush.setCurrent", &json!({"name":"Tapered Stroke"})).unwrap();
    let id=NodeId(s.execute("path.freehand",&json!({"style":"brush","points":[[20,80],[80,80],[140,80]],"penSamples":[{"pressure":0.2,"tilt":[10,20]},{"pressure":0.8,"tilt":[20,30]},{"pressure":0.3,"tilt":[10,10]}],"expressive":{"pressureWidth":true,"tiltNib":true}})).unwrap()["id"].as_u64().unwrap());
    let doc = &s.doc().unwrap().doc;
    let stroke = doc.node(id).unwrap().appearance.stroke().unwrap();
    assert_eq!(stroke.brush.as_deref(), Some("Tapered Stroke"));
    assert!(stroke.profile.is_some());
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(doc).unwrap();
}
#[test]
fn grain_controls_are_discoverable_editable_undoable_and_persistent() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    let id = NodeId(s.execute("path.create", &json!({"d":"M20 80 L180 80"})).unwrap()["id"].as_u64().unwrap());
    s.execute("brush.apply", &json!({"name":"Rainee Graphite Grain","ids":[id]})).unwrap();
    let source = s.doc().unwrap().doc.node(id).unwrap().kind.clone();
    let before = vectorcraft_brush::node_pieces(&s.doc().unwrap().doc, &s.doc().unwrap().doc.node(id).unwrap()).unwrap();
    s.execute("brush.options", &json!({"name":"Rainee Graphite Grain","params":{"grain":0.2}})).unwrap();
    let after = vectorcraft_brush::node_pieces(&s.doc().unwrap().doc, &s.doc().unwrap().doc.node(id).unwrap()).unwrap();
    assert_ne!(before, after);
    assert_eq!(source, s.doc().unwrap().doc.node(id).unwrap().kind);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.doc().unwrap().doc).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(before, vectorcraft_brush::node_pieces(&s.doc().unwrap().doc, &s.doc().unwrap().doc.node(id).unwrap()).unwrap());
}
#[test]
fn natural_freehand_keeps_pressure_brush_width_edit_and_native_roundtrip() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    s.execute("brush.setCurrent", &json!({"name":"Rainee Pencil"})).unwrap();
    let id=NodeId(s.execute("path.freehand",&json!({"style":"brush","points":[[20,80],[80,80],[140,80]],"penSamples":[{"pressure":0.2,"tilt":[10,20]},{"pressure":0.8,"tilt":[20,30]},{"pressure":0.3,"tilt":[10,10]}],"expressive":{"pressureWidth":true,"tiltNib":true}})).unwrap()["id"].as_u64().unwrap());
    let doc = &s.doc().unwrap().doc;
    let node = doc.node(id).unwrap();
    let stroke = node.appearance.stroke().unwrap();
    assert_eq!(stroke.brush.as_deref(), Some("Rainee Pencil"));
    assert!(stroke.profile.is_some());
    assert!(stroke.nib.is_some());
    let before = vectorcraft_brush::node_pieces(doc, node).unwrap();
    s.execute("stroke.set", &json!({"ids":[id],"weight":8})).unwrap();
    let doc = &s.doc().unwrap().doc;
    let after = vectorcraft_brush::node_pieces(doc, doc.node(id).unwrap()).unwrap();
    assert_ne!(before, after);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(doc).unwrap();
}
#[test]
fn erase_natural_centerline_retains_editable_brush_pieces_and_undo() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    s.execute("paint.setFill", &json!({"none":true})).unwrap();
    let id = NodeId(s.execute("path.create", &json!({"d":"M20 80 L180 80"})).unwrap()["id"].as_u64().unwrap());
    s.execute("brush.apply", &json!({"name":"Rainee Dry Ink","ids":[id]})).unwrap();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    s.execute("select.set", &json!({"ids":[id]})).unwrap();
    let r = s.execute("path.eraseRegion", &json!({"points":[[100,60],[100,100]],"size":12})).unwrap();
    let ids = r.get("ids").unwrap_or(&r).as_array().unwrap();
    assert!(ids.len() >= 2);
    for id in ids {
        let node = s.doc().unwrap().doc.node(NodeId(id.as_u64().unwrap())).unwrap();
        assert!(node.path_data().is_some());
        assert_eq!(node.appearance.stroke().unwrap().brush.as_deref(), Some("Rainee Dry Ink"));
    }
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
}

#[test]
fn advanced_ink_options_preserve_vector_source_and_validate_atomically() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    let id = NodeId(s.execute("path.create", &json!({"d":"M20 90 C60 10 120 10 190 90"})).unwrap()["id"].as_u64().unwrap());
    s.execute("brush.apply", &json!({"name":"Rainee Dry Ink","ids":[id]})).unwrap();
    let source = s.doc().unwrap().doc.node(id).unwrap().kind.clone();
    let before = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    s.execute("brush.options", &json!({"name":"Rainee Dry Ink","params":{"inkLoad":1.5,"edgePooling":0.8,"breakup":0.2,"paperScale":2.0}})).unwrap();
    assert_eq!(s.doc().unwrap().doc.node(id).unwrap().kind, source);
    vectorcraft_testkit::invariants::check_native_roundtrip_exact(&s.doc().unwrap().doc).unwrap();
    let edited = vectorcraft_format::save(&s.doc().unwrap().doc, false);
    assert!(s.execute("brush.options", &json!({"name":"Rainee Dry Ink","params":{"paperScale":0}})).is_err());
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), edited);
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(vectorcraft_format::save(&s.doc().unwrap().doc, false), before);
}
