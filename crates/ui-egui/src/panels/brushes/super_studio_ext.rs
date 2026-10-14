//! Procreate-Style Rebelle Super Brush Studio & Vector Outliner Controls

#![forbid(unsafe_code)]

use egui::Ui;
use serde_json::{Value, json};

pub fn show_embedded(ui: &mut Ui, draft: &mut Value) {
    ui.add_space(8.0);
    ui.collapsing("Rebelle Vector-Controlled Fluid & Natural Media", |ui| {
        let current_media = draft.get("rebelleMedia").and_then(Value::as_str).unwrap_or("watercolorWash").to_string();

        ui.label(egui::RichText::new("Rebelle Natural Media Mode (Live Vector Spine):").strong());
        ui.horizontal_wrapped(|ui| {
            for (id, label) in [
                ("watercolorWash", "Watercolor Wash"),
                ("oilImpasto", "Oil Impasto"),
                ("sumiEInk", "Sumi-e Flying White"),
                ("acrylicGlaze", "Acrylic Glaze"),
                ("graphiteTooth", "Graphite Tooth"),
                ("charcoalGrain", "Charcoal Grain"),
            ] {
                if ui.selectable_label(current_media == id, label).clicked() {
                    draft["rebelleMedia"] = json!(id);
                }
            }
        });

        ui.separator();
        let mut viscosity = draft.get("lbmViscosity").and_then(Value::as_f64).unwrap_or(0.42);
        if ui.add(egui::Slider::new(&mut viscosity, 0.05..=1.0).text("LBM Fluid Viscosity")).changed() {
            draft["lbmViscosity"] = json!(viscosity);
        }

        let mut bleed = draft.get("capillaryBleed").and_then(Value::as_f64).unwrap_or(0.88);
        if ui.add(egui::Slider::new(&mut bleed, 0.0..=1.0).text("Darcy Capillary Bleed")).changed() {
            draft["capillaryBleed"] = json!(bleed);
        }

        let mut pooling = draft.get("coffeeRingPooling").and_then(Value::as_f64).unwrap_or(0.92);
        if ui.add(egui::Slider::new(&mut pooling, 0.0..=1.5).text("Coffee-Ring Edge Pooling")).changed() {
            draft["coffeeRingPooling"] = json!(pooling);
        }

        let mut bristles = draft.get("cosseratBristles").and_then(Value::as_u64).unwrap_or(48);
        if ui.add(egui::Slider::new(&mut bristles, 16..=64).text("Cosserat Bristle Count")).changed() {
            draft["cosseratBristles"] = json!(bristles);
        }
    });
}
