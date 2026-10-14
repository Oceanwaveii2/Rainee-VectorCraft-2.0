//! Thin controller: canvas mutation and job lifecycle are shared engine commands.
use crate::VectorcraftApp;
use serde_json::json;

fn run_cleanup(app: &mut VectorcraftApp, ctx: &egui::Context, command: &str, params: serde_json::Value) {
    let result = app.run(command, params);
    ctx.data_mut(|d| match result {
        Ok(_) => {
            d.remove::<String>(egui::Id::new("cleanup-error"));
        }
        Err(error) => {
            d.insert_temp(egui::Id::new("cleanup-error"), error);
        }
    });
}

pub fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.cleanup_open {
        return;
    }
    let mut open = true;
    let state = app.session.cleanup.summary();
    let settings = ctx.data(|d| {
        json!({
        "tolerance":d.get_temp::<f64>(egui::Id::new("cleanup-tolerance")).unwrap_or(0.25),
        "maxDeviation":d.get_temp::<f64>(egui::Id::new("cleanup-deviation")).unwrap_or(4.0),
        "timeout":d.get_temp::<f64>(egui::Id::new("cleanup-timeout")).unwrap_or(30.0)})
    });
    let stamp = json!({"document":app.session.active().map(|d|d.uid),
        "revision":app.session.active().map(|d|d.revision),
        "selection":app.session.active().map(|d|&d.selection.objects),"busy":app.session.tool_busy(),"settings":settings});
    let cache_id = egui::Id::new("cleanup-selection-preflight");
    let cached = ctx.data(|d| d.get_temp::<(serde_json::Value, serde_json::Value)>(cache_id));
    let admission = if let Some((old, result)) = cached.filter(|(old, _)| old == &stamp) {
        let _ = old;
        result
    } else {
        let result = app.run("sketch.cleanup.inspect", settings).unwrap_or_else(|reason| json!({"eligible":false,"reason":reason}));
        ctx.data_mut(|d| d.insert_temp(cache_id, (stamp, result.clone())));
        result
    };
    egui::Window::new("Editable Sketch Cleanup · Experimental").drag_area(egui::WindowDrag::TitleBar).open(&mut open).default_width(380.0).show(ctx, |ui| {
        ui.label("Select consecutive uniform open strokes with the same appearance.");
        ui.label("Cyan overlay = proposed centerlines. Originals stay untouched until Accept.");
        ui.small("Uniform Art/Natural brush centerlines are supported. Pressure/nib strokes, scatter/pattern/bristle effects, fills and masks remain protected. This is not Smart Inker.");
        ui.separator();
        let selected = app.session.active().map(|d| d.selection.objects.len()).unwrap_or(0);
        ui.label(format!("{selected} strokes selected (requires 2–64)"));
        if let Some(reason)=admission["reason"].as_str() { ui.label(format!("Selection not supported: {reason}")); }
        if let Some(error) = ctx.data(|d| d.get_temp::<String>(egui::Id::new("cleanup-error"))) {
            ui.label(format!("Cleanup error: {error}. Originals unchanged."));
        }
        let mut tolerance=ctx.data_mut(|d|d.get_temp::<f64>(egui::Id::new("cleanup-tolerance")).unwrap_or(0.25));
        let mut deviation=ctx.data_mut(|d|d.get_temp::<f64>(egui::Id::new("cleanup-deviation")).unwrap_or(4.0));
        let mut timeout=ctx.data_mut(|d|d.get_temp::<f64>(egui::Id::new("cleanup-timeout")).unwrap_or(30.0));
        ui.add(egui::Slider::new(&mut tolerance,0.1..=2.0).text("Curve sampling (pt)"));
        ui.add(egui::Slider::new(&mut deviation,0.25..=32.0).text("Maximum deviation (pt)"));
        ui.add(egui::Slider::new(&mut timeout,1.0..=90.0).text("Timeout (seconds)"));
        ctx.data_mut(|d|{d.insert_temp(egui::Id::new("cleanup-tolerance"),tolerance);d.insert_temp(egui::Id::new("cleanup-deviation"),deviation);d.insert_temp(egui::Id::new("cleanup-timeout"),timeout);});
        ui.label(format!("State: {}",state["state"].as_str().unwrap_or("idle")));
        if let Some(message)=state["message"].as_str().filter(|m|!m.is_empty()) {ui.label(message);}
        if state["backend"]["installed"]==false {ui.label(state["backend"]["reason"].as_str().unwrap_or("Worker unavailable"));}
        ui.horizontal(|ui| {
            if ui.add_enabled(!app.session.cleanup.running() && admission["eligible"]==true,egui::Button::new("Generate Preview")).clicked() {
                run_cleanup(app,ctx,"sketch.cleanup.start",json!({"tolerance":tolerance,"maxDeviation":deviation,"timeout":timeout}));
            }
            if ui.add_enabled(app.session.cleanup.running(),egui::Button::new("Cancel")).clicked(){run_cleanup(app,ctx,"sketch.cleanup.cancel",json!({}));}
        });
        ui.horizontal(|ui| {
            let ready=state["state"]=="ready" && state["bundleCount"].as_u64().unwrap_or(0)>0;
            if ui.add_enabled(ready,egui::Button::new("Accept (one undo step)")).clicked(){run_cleanup(app,ctx,"sketch.cleanup.accept",json!({"job":state["job"]}));}
            if ui.add_enabled(state["state"]=="ready",egui::Button::new("Reject")).clicked(){run_cleanup(app,ctx,"sketch.cleanup.reject",json!({}));}
        });
        if let Some(groups)=state["groupCount"].as_u64(){ui.label(format!("{groups} proposed groups; singleton strokes remain exact"));}
        ui.small("Acceptance retains source IDs inside hidden Cleanup Originals groups. Pan and zoom remain available. Content changes invalidate the preview.");
    });
    app.cleanup_open = open;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_preflight_cache_refreshes_after_selection_change() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width":240,"height":160})).unwrap();
        app.cleanup_open = true;
        let ctx = egui::Context::default();
        let frame = |app: &mut VectorcraftApp| {
            ctx.run_ui(egui::RawInput::default(), |ui| show(app, ui.ctx())).textures_delta.clear();
        };
        frame(&mut app);
        let key = egui::Id::new("cleanup-selection-preflight");
        assert_eq!(ctx.data(|d| d.get_temp::<(serde_json::Value, serde_json::Value)>(key)).unwrap().1["eligible"], false);
        app.run("paint.setFill", json!({"none":true})).unwrap();
        let mut ids = vec![];
        for y in [50, 51] {
            ids.push(app.run("path.create", json!({"d":format!("M20 {y} L100 {y}")})).unwrap()["id"].clone());
        }
        app.run("select.set", json!({"ids":ids})).unwrap();
        frame(&mut app);
        assert_eq!(ctx.data(|d| d.get_temp::<(serde_json::Value, serde_json::Value)>(key)).unwrap().1["eligible"], true);
        app.run("select.none", json!({})).unwrap();
        frame(&mut app);
        assert_eq!(ctx.data(|d| d.get_temp::<(serde_json::Value, serde_json::Value)>(key)).unwrap().1["eligible"], false);
    }
}
