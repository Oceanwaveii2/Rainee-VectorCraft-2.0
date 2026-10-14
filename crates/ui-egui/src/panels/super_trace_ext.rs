//! Super Live Trace Inspector Panel Extension

#![forbid(unsafe_code)]

use egui::Ui;
use serde_json::{Value, json};

pub fn show_embedded(ui: &mut Ui, params: &mut Value, retrace: &mut bool) {
    ui.add_space(6.0);
    ui.collapsing("Super Live Trace (SIGGRAPH 2022–2024 & Rebelle Paint Strata)", |ui| {
        let current_engine = params
            .get("superEngineMode")
            .and_then(Value::as_str)
            .unwrap_or("neuralUdf")
            .to_string();

        ui.label("Vectorization Engine:");
        ui.horizontal_wrapped(|ui| {
            for (id, label) in [
                ("classic", "Classic (<5ms)"),
                ("neuralUdf", "Neural UDF + QEF"),
                ("tcbSpline", "TCB Hard Corners"),
                ("rebelleStrata", "Rebelle Wash Strata"),
            ] {
                if ui.selectable_label(current_engine == id, label).clicked() {
                    params["superEngineMode"] = json!(id);
                    *retrace = true;
                }
            }
        });

        let mut tcb_tension = params.get("tcbTension").and_then(Value::as_f64).unwrap_or(0.35);
        if ui.add(egui::Slider::new(&mut tcb_tension, -1.0..=1.0).text("TCB Corner Tension")).changed() {
            params["tcbTension"] = json!(tcb_tension);
            *retrace = true;
        }

        let mut preserve_junctions = params.get("preserveStarJunctions").and_then(Value::as_bool).unwrap_or(true);
        if ui.checkbox(&mut preserve_junctions, "Preserve High-Valence Star Junctions (2D QEF)").changed() {
            params["preserveStarJunctions"] = json!(preserve_junctions);
            *retrace = true;
        }
    });
}
