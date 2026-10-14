//! Definition-backed desktop Brush Studio controls. Never synthesizes unsupported effects.
use super::*;

pub(super) const PAGES: [&str; 9] =
    ["Shape", "Stroke path", "Grain", "Rendering", "Dynamics", "Color dynamics", "Properties", "Pen input", "About this brush"];
pub(super) const ICONS: [&str; 9] = ["brush", "spline", "grid-3x3", "paintbrush", "waves", "palette", "settings", "pen-tool", "info"];

fn section(key: &str) -> usize {
    match key {
        "shape" | "size" | "roundness" | "angle" | "direction" => 0,
        "scale" | "spacing" | "fit" | "rotation_relative_to_path" | "flip_along" | "flip_across" => 1,
        "grain" | "fibers" | "paperScale" | "density" | "thickness" | "length" => 2,
        "media" | "inkLoad" | "edgePooling" | "breakup" | "opacity" | "stiffness" => 3,
        "variation" | "scatter" | "rotation" => 4,
        "colorization" => 5,
        "seed" | "width" => 6,
        _ => 8,
    }
}

fn limits(key: &str, natural: bool) -> (f64, f64, &'static str) {
    match key {
        "angle" | "rotation" => (-180.0, 180.0, "°"),
        "roundness" | "density" | "thickness" | "opacity" | "stiffness" => (1.0, 100.0, "%"),
        "length" => (25.0, 300.0, "%"),
        "grain" | "edgePooling" | "breakup" => (0.0, 1.0, ""),
        "inkLoad" => (0.0, 2.0, ""),
        "paperScale" => (0.125, 128.0, "×"),
        "fibers" => (1.0, 16.0, ""),
        "scatter" => (-1000.0, 1000.0, "%"),
        "spacing" => (0.0, 1000.0, "%"),
        "size" => (0.1, 1000.0, ""),
        "width" if natural => (0.1, 4096.0, "pt"),
        "width" | "scale" => (0.1, 1000.0, "%"),
        _ => (0.0, 100.0, ""),
    }
}

fn title(key: &str) -> &str {
    match key {
        "inkLoad" => "Ink load",
        "edgePooling" => "Edge pooling",
        "paperScale" => "Paper scale",
        "shape" => "Nib shape",
        "size" => "Size",
        "width" => "Width",
        "roundness" => "Roundness",
        "angle" => "Angle",
        "direction" => "Direction",
        "scale" => "Scaling",
        "spacing" => "Spacing",
        "fit" => "Fitting",
        "grain" => "Grain strength",
        "fibers" => "Fibers",
        "density" => "Density",
        "thickness" => "Thickness",
        "length" => "Bristle length",
        "media" => "Medium",
        "breakup" => "Dry breakup",
        "opacity" => "Opacity",
        "stiffness" => "Stiffness",
        "scatter" => "Scatter",
        "rotation" => "Rotation",
        "variation" => "Random variation",
        "flip_along" => "Flip along path",
        "flip_across" => "Flip across path",
        "rotation_relative_to_path" => "Rotate relative to path",
        _ => key,
    }
}

fn number(ui: &mut Ui, key: &str, value: &mut Value, natural: bool) {
    if key == "seed" {
        let mut seed = value.as_u64().unwrap_or(0);
        ui.horizontal(|ui| {
            ui.label("Pattern seed");
            if ui.add(egui::DragValue::new(&mut seed)).changed() {
                *value = json!(seed);
            }
        });
        return;
    }
    let Some(mut n) = value.as_f64() else { return };
    let (min, max, suffix) = limits(key, natural);
    ui.label(egui::RichText::new(title(key)).strong());
    let mut slider = egui::Slider::new(&mut n, min..=max).suffix(suffix).clamping(egui::SliderClamping::Never);
    if key == "fibers" {
        slider = slider.integer();
    }
    let response = ui.add(slider);
    remember(ui, key, response.rect);
    if response.changed() && n.is_finite() {
        let n = n.clamp(min, max);
        *value = if key == "fibers" { json!(n.round() as u64) } else { json!(n) };
    }
}

fn choice(ui: &mut Ui, key: &str, value: &mut Value, choices: &[&str]) {
    let mut selected = value.as_str().unwrap_or("").to_owned();
    ui.label(title(key));
    egui::ComboBox::from_id_salt(key).selected_text(&selected).width(ui.available_width().min(240.0)).show_ui(ui, |ui| {
        for c in choices {
            ui.selectable_value(&mut selected, (*c).into(), *c);
        }
    });
    if value.as_str() != Some(selected.as_str()) {
        *value = json!(selected);
    }
}

pub(super) fn show(app: &mut VectorcraftApp, ui: &mut Ui, studio: &mut Studio) {
    set_pstate(ui.ctx(), "studio-control-rects", std::collections::BTreeMap::<String, [f32; 4]>::new());
    let natural = studio.draft.get("type").and_then(Value::as_str) == Some("natural");
    ui.heading(PAGES[studio.page.min(PAGES.len() - 1)]);
    ui.add_space(12.0);
    if studio.page == 8 {
        ui.label("Brush name");
        let mut name = studio.draft.get("name").and_then(Value::as_str).unwrap_or(&studio.name).to_owned();
        let edit = ui.text_edit_singleline(&mut name);
        remember(ui, "name", edit.rect);
        if edit.changed() {
            studio.draft["name"] = json!(name);
        }
        ui.label(format!("Brush type: {}", studio.draft.get("type").and_then(Value::as_str).unwrap_or("unknown")));
        ui.separator();
        ui.label("The original vector path stays editable. Done updates the document's brush definition in one undoable operation; Cancel discards this draft.");
        ui.label("Reset draft restores your starting settings. Preview strokes never enter the document.");
        return;
    }
    if studio.page == 7 {
        ui.label(egui::RichText::new("Pressure & tilt").strong());
        ui.label("Pen response, stabilization and button assignments are configured for drawing tools in Preferences. The preview pad uses pointer position.");
        if ui.button("Pen settings…").clicked() {
            app.run("edit.preferences", json!({"category":"Pen & Wacom Buttons"})).ok();
        }
        return;
    }
    let Some(fields) = studio.draft.as_object_mut() else { return };
    // These neutral defaults are omitted by the serializer, but are real editable engine fields.
    if natural {
        for (key, default) in [("grain", 0.0), ("inkLoad", 1.0), ("edgePooling", 0.0), ("breakup", 0.0), ("paperScale", 1.0)] {
            fields.entry(key).or_insert(json!(default));
        }
    }
    let mut count = 0;
    for (key, value) in fields {
        if section(key) != studio.page {
            continue;
        }
        count += 1;
        ui.push_id(key, |ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 72.0).max(100.0);
            if value.is_number() {
                number(ui, key, value, natural);
            } else if let Some(mut b) = value.as_bool() {
                if widgets::toggle_switch(ui, title(key), &mut b).changed() {
                    *value = json!(b);
                }
            } else if let Some(values) = value.as_array_mut() {
                ui.label(title(key));
                for (i, v) in values.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        let component = if key == "variation" { ["angle", "roundness", "size"].get(i).copied().unwrap_or("variation") } else { key };
                        ui.small(if key == "variation" {
                            "Variation per stroke"
                        } else if i == 0 {
                            "Minimum"
                        } else {
                            "Maximum"
                        });
                        number(ui, component, v, natural);
                    });
                }
            } else {
                match key.as_str() {
                    "shape" => choice(
                        ui,
                        key,
                        value,
                        &[
                            "roundPoint",
                            "flatPoint",
                            "roundBlunt",
                            "flatBlunt",
                            "roundCurve",
                            "flatCurve",
                            "roundAngle",
                            "flatAngle",
                            "roundFan",
                            "flatFan",
                        ],
                    ),
                    "direction" => choice(ui, key, value, &["leftToRight", "rightToLeft", "topToBottom", "bottomToTop"]),
                    "fit" => choice(ui, key, value, &["stretch", "addSpace", "approximate"]),
                    "media" => choice(ui, key, value, &["pencil", "dryInk"]),
                    "scale" => {
                        if let Some(mode) = value.get_mut("mode") {
                            choice(ui, "Scaling", mode, &["stretch", "proportional", "betweenGuides"]);
                        }
                        if value.get("mode").and_then(Value::as_str) == Some("betweenGuides") {
                            for (k, default) in [("start", 0.2), ("end", 0.8)] {
                                let mut n = value.get(k).and_then(Value::as_f64).unwrap_or(default);
                                let range = if k == "start" {
                                    0.0..=value.get("end").and_then(Value::as_f64).unwrap_or(0.8)
                                } else {
                                    value.get("start").and_then(Value::as_f64).unwrap_or(0.2)..=1.0
                                };
                                if ui.add(egui::Slider::new(&mut n, range).text(k)).changed() || value.get(k).is_none() {
                                    value[k] = json!(n);
                                }
                            }
                        }
                    }
                    "colorization" => {
                        if let Some(method) = value.get_mut("method") {
                            // Hue Shift needs its existing key colour; do not destroy that payload.
                            choice(ui, "Colorization", method, &["none", "tints", "tintsAndShades"]);
                        }
                    }
                    _ => {
                        ui.label("Edit source artwork on the document canvas.");
                    }
                }
            }
            ui.add_space(10.0);
        });
    }
    if studio.page == 3 {
        super_studio_ext::show_embedded(ui, &mut studio.draft);
    }
    if count == 0 && studio.page != 3 {
        let btype = studio.draft.get("type").and_then(Value::as_str).unwrap_or("this");
        ui.vertical(|ui| {
            ui.add_space(8.0);
            match (btype, studio.page) {
                ("calligraphic", 1) => {
                    ui.label(egui::RichText::new("Continuous Vector Nib (No Stamp Spacing)").strong());
                    ui.add_space(4.0);
                    ui.label("Calligraphic brushes generate an unbroken, smooth mathematical envelope around the path.");
                    ui.add_space(4.0);
                    ui.label("Nib diameter, angle and roundness are in Shape.");
                    ui.label("Random variation is in Dynamics; pressure and tilt settings are in Pen input.");
                    ui.label("• For adjustable stamp spacing, select a Scatter or Pattern brush from the library.");
                }
                ("calligraphic", 2) => {
                    ui.label(egui::RichText::new("Pure Vector Ribbon (No Substrate Grain)").strong());
                    ui.add_space(4.0);
                    ui.label("Calligraphic nibs render smooth vector paths without simulated paper tooth.");
                    ui.label("• For fibrous paper tooth and grain, use a Natural Media brush (e.g. Rainee Pencil).");
                }
                ("art", 2) => {
                    ui.label(egui::RichText::new("Vector Art Mesh").strong());
                    ui.add_space(4.0);
                    ui.label("Art brushes deform vector geometry along the path.");
                    ui.label("Scale, stretch guides and fitting controls are in Stroke path.");
                }
                _ => {
                    ui.label(format!("The {} engine does not require parameters in this section.", btype));
                    ui.label("Active controls for this brush type are located in the other tabs.");
                }
            }
        });
    }
}

fn remember(ui: &Ui, key: &str, r: egui::Rect) {
    let mut fields = pstate::<std::collections::BTreeMap<String, [f32; 4]>>(ui.ctx(), "studio-control-rects");
    fields.insert(key.into(), [r.left(), r.top(), r.width(), r.height()]);
    set_pstate(ui.ctx(), "studio-control-rects", fields);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_studio_section_uses_an_existing_stock_icon() {
        assert!(ICONS.into_iter().all(crate::icons::exists));
    }
    #[test]
    fn rainee_studio_pages_preserve_definitions_and_expose_neutral_ink_defaults() {
        let mut app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({})).unwrap();
        app.run("brush.new", json!({"name":"Ink test", "type":"natural"})).unwrap();
        let original = app.run("brush.get", json!({"name":"Ink test"})).unwrap();
        let before = app.session.doc().unwrap().doc.clone();
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut studio = Studio { name: "Ink test".into(), draft: original.clone(), original, ..Default::default() };
        for page in 0..PAGES.len() {
            studio.page = page;
            ctx.run_ui(egui::RawInput::default(), |ui| show(&mut app, ui, &mut studio)).textures_delta.clear();
        }
        assert_eq!(studio.draft["inkLoad"], json!(1.0));
        assert_eq!(studio.draft["grain"], json!(0.0));
        assert_eq!(studio.draft["paperScale"], json!(1.0));
        assert_eq!(app.session.doc().unwrap().doc, before);
    }
}
