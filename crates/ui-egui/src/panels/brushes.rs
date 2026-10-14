//! Brushes panel: the document's brush library with rendered stroke previews. Clicking a brush
//! applies it to the selected paths (and makes it the Paintbrush's current brush).

use std::collections::HashMap;

use egui::{Sense, Ui, vec2};
use serde_json::{Value, json};
use vectorcraft_doc::{Appearance, Document, Node, color::Color, color::Paint};

use super::{first_selected, pstate, set_pstate};
use crate::VectorcraftApp;
use crate::theme::Tokens;
use crate::widgets::{self, PanelDrag, menu_item};

const KINDS: [(&str, &str); 6] = [
    ("calligraphic", "Calligraphic"),
    ("scatter", "Scatter"),
    ("art", "Art"),
    ("bristle", "Bristle"),
    ("pattern", "Pattern"),
    ("natural", "Natural"),
];

/// (name, type) of every brush in the active document's library.
pub fn brushes(app: &mut VectorcraftApp) -> (Vec<(String, String)>, Option<String>) {
    let Ok(v) = app.run("brush.list", json!({})) else { return (vec![], None) };
    let list = v["brushes"]
        .as_array()
        .map(|a| a.iter().map(|b| (b["name"].as_str().unwrap_or("").to_string(), b["type"].as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();
    (list, v["current"].as_str().map(str::to_string))
}

/// A stroke preview for brush definition `def`, rendered at `size` and cached by the definition's
/// JSON.
fn preview(ui: &Ui, def: &Value, size: egui::Vec2) -> Option<egui::TextureHandle> {
    preview_tone(ui, def, size, false)
}

fn preview_tone(ui: &Ui, def: &Value, size: egui::Vec2, light: bool) -> Option<egui::TextureHandle> {
    let build = |w: f64, h: f64| {
        let mut doc = Document::new(w, h);
        doc.unknown.insert("brushes".into(), json!([def]));
        let name = def["name"].as_str()?.to_string();
        let mut ap = Appearance::basic(Paint::None, Paint::solid(Color::BLACK), 1.0);
        ap.stroke_mut()?.brush = Some(name);
        // A gentle S-curve across the swatch.
        let mut bp = vectorcraft_geom::BezPath::new();
        let (x0, x1) = (h * 0.5, w - h * 0.5);
        bp.move_to((x0, h * 0.5));
        bp.curve_to((x0 + (x1 - x0) * 0.35, h * 0.1), (x0 + (x1 - x0) * 0.65, h * 0.9), (x1, h * 0.5));
        let id = doc.alloc_id();
        let l = doc.layers[0].id;
        doc.insert(Some(l), 0, Node::path(id, vectorcraft_geom::PathData::from_bezpath(&bp), ap)).ok()?;
        Some(doc)
    };
    if light {
        widgets::doc_preview_light(ui, &format!("brush:{def}"), size, build)
    } else {
        widgets::doc_preview(ui, &format!("brush:{def}"), size, build)
    }
}

/// The stroke preview of brush definition `def` on white in `r` (the chip a dragged brush shows at
/// the pointer).
pub(crate) fn chip(ui: &Ui, r: egui::Rect, def: &Value) {
    ui.painter().rect_filled(r, 0.0, egui::Color32::WHITE);
    if let Some(tex) = preview(ui, def, r.size()) {
        ui.painter().image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    }
}

fn selected_brush(app: &VectorcraftApp) -> Option<String> {
    first_selected(app).and_then(|n| n.appearance.stroke().and_then(|s| s.brush.clone()))
}

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    if app.ui.procreate_panels {
        show_library(app, ui);
        return;
    }
    let t = Tokens::get(ui.ctx());
    let (list, current) = brushes(app);
    let sel_brush = selected_brush(app);
    let list_view: bool = pstate(ui.ctx(), "br-list");
    let hidden: Vec<String> = pstate(ui.ctx(), "br-hidden");
    let defs: HashMap<String, Value> =
        list.iter().filter_map(|(n, _)| app.run("brush.get", json!({"name": n})).ok().map(|v| (n.clone(), v))).collect();
    let mut clicked: Option<String> = None;
    let list_rect = widgets::list_box(ui, |ui| {
        ui.set_min_height(110.0);
        ui.set_width(ui.available_width());
        if list.is_empty() {
            super::empty_state(ui, "paintbrush", tl!("No brushes"), tl!("Select art and use New Brush to make one."));
            return ui.min_rect();
        }
        for (ty, _) in KINDS {
            if hidden.iter().any(|h| h == ty) {
                continue;
            }
            let group: Vec<&(String, String)> = list.iter().filter(|b| b.1 == ty).collect();
            if group.is_empty() {
                continue;
            }
            let row = |ui: &mut Ui, name: &str, size: egui::Vec2, label: bool| -> egui::Response {
                let (r, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
                // Dragged onto a path, the brush is applied to it.
                widgets::drag_source(ui, &resp, || PanelDrag::Brush { name: name.to_string(), def: defs.get(name).cloned().unwrap_or_default() });
                let on = sel_brush.as_deref() == Some(name) || (sel_brush.is_none() && current.as_deref() == Some(name));
                if on {
                    ui.painter().rect_filled(r, 0.0, t.row_selected);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, 0.0, t.hover);
                }
                let pw = if label { 72.0 } else { size.x - 4.0 };
                let pr = egui::Rect::from_min_size(r.left_top() + vec2(2.0, 2.0), vec2(pw, size.y - 4.0));
                ui.painter().rect_filled(pr, 0.0, egui::Color32::WHITE);
                if let Some(def) = defs.get(name)
                    && let Some(tex) = preview(ui, def, pr.size())
                {
                    ui.painter().image(tex.id(), pr, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
                if label {
                    ui.painter().text(
                        r.left_center() + vec2(pw + 10.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        name,
                        egui::FontId::proportional(12.5),
                        t.text,
                    );
                }
                resp.on_hover_text(name)
            };
            if list_view {
                for (name, _) in group {
                    if row(ui, name, vec2(ui.available_width(), 26.0), true).clicked() {
                        clicked = Some(name.clone());
                    }
                }
            } else {
                ui.horizontal_wrapped(|ui| {
                    for (name, _) in group {
                        if row(ui, name, vec2(64.0, 28.0), false).clicked() {
                            clicked = Some(name.clone());
                        }
                    }
                });
            }
            ui.separator();
        }
        ui.min_rect()
    });
    // Art dragged off the canvas and dropped on the list becomes an Art brush, as New Brush makes.
    let zone = ui.interact(list_rect, ui.id().with("brushes-drop"), Sense::hover());
    if let Some(ids) = widgets::art_drop(ui, &zone)
        && let Err(e) = app.run("brush.new", json!({ "type": "art", "ids": ids }))
    {
        app.status(e);
    }
    if let Some(name) = clicked {
        let has_sel = app.session.active().is_some_and(|d| !d.selection.is_empty());
        if has_sel {
            app.run("brush.apply", json!({"name": name})).ok();
        } else {
            app.run("brush.setCurrent", json!({"name": name})).ok();
        }
    }
    let has_brush = sel_brush.is_some();
    let target = sel_brush.clone().or(current.clone());
    if let Some(name) = &target
        && let Some(def) = defs.get(name)
        && def["type"] == "natural"
    {
        ui.collapsing("Natural brush controls",|ui|{
            let mut media=def["media"].as_str().unwrap_or("pencil").to_owned();
            let mut width=def["width"].as_f64().unwrap_or(1.0);
            let mut fibers=def["fibers"].as_u64().unwrap_or(10).clamp(1,16);
            let mut seed=def["seed"].as_u64().unwrap_or(0);
            let mut grain=def["grain"].as_f64().unwrap_or(0.0);
            let mut ink_load=def["inkLoad"].as_f64().unwrap_or(1.0);
            let mut pooling=def["edgePooling"].as_f64().unwrap_or(0.0);
            let mut breakup=def["breakup"].as_f64().unwrap_or(0.0);
            let mut paper_scale=def["paperScale"].as_f64().unwrap_or(1.0);
            let old=(media.clone(),width,fibers,seed,grain,ink_load,pooling,breakup,paper_scale);
            egui::ComboBox::from_id_salt("natural-media").selected_text(&media).show_ui(ui,|ui|{ui.selectable_value(&mut media,"pencil".into(),"Pencil");ui.selectable_value(&mut media,"dryInk".into(),"Dry ink");});
            ui.add(egui::Slider::new(&mut width,0.1..=8.0).text("Nib scale"));
            ui.add(egui::Slider::new(&mut fibers,1..=16).text("Fibers"));
            ui.add(egui::Slider::new(&mut grain,0.0..=1.0).text("Paper grain"));
            ui.add(egui::Slider::new(&mut ink_load,0.0..=2.0).text("Ink load"));
            ui.add(egui::Slider::new(&mut pooling,0.0..=1.0).text("Edge pooling"));
            ui.add(egui::Slider::new(&mut breakup,0.0..=1.0).text("Broken bristles"));
            ui.add(egui::Slider::new(&mut paper_scale,0.125..=8.0).logarithmic(true).text("Paper scale"));
            ui.horizontal(|ui|{ui.label("Texture seed");ui.add(egui::DragValue::new(&mut seed));});
            if old!=(media.clone(),width,fibers,seed,grain,ink_load,pooling,breakup,paper_scale) && let Err(e)=app.run("brush.edit",json!({"name":name,"params":{"media":media,"width":width,"fibers":fibers,"seed":seed,"grain":grain,"inkLoad":ink_load,"edgePooling":pooling,"breakup":breakup,"paperScale":paper_scale}})){app.status(e);}
            ui.small("Edit stroke weight/profile with vector tools. Appearance regenerates from its centerline.");
            ui.small("Pooling is procedural edge darkening, not simulated flowing paint.");
        });
    }
    let has_sel = app.session.active().is_some_and(|d| !d.selection.is_empty());
    widgets::bottom_bar(ui, |ui| {
        widgets::icon_button_enabled(ui, "library", tl!("Brush Libraries (on the roadmap)"), false, false, 24.0);
        if widgets::icon_button_enabled(ui, "dc-remove-brush", tl!("Remove Brush Stroke"), false, has_brush, 24.0).clicked() {
            app.run("brush.remove", json!({})).ok();
        }
        if widgets::icon_button_enabled(ui, "dc-options", tl!("Expand Brush Strokes"), false, has_brush, 24.0).clicked() {
            app.run("object.expandBrush", json!({})).ok();
        }
        ui.add_space((ui.available_width() - 2.0 * 28.0).max(0.0));
        if widgets::icon_button_enabled(ui, "dc-new-item", tl!("New Art Brush from Selection"), false, has_sel, 24.0).clicked() {
            app.run("brush.new", json!({"type": "art"})).ok();
        }
        if widgets::icon_button_enabled(ui, "trash-2", tl!("Delete Brush"), false, target.is_some(), 24.0).clicked()
            && let Some(n) = &target
        {
            app.run("brush.delete", json!({"name": n})).ok();
        }
    });
}

fn show_library(app: &mut VectorcraftApp, ui: &mut Ui) {
    Tokens::style_rainee(ui);
    let skin = Tokens::get(ui.ctx());
    let (list, current) = brushes(app);
    let mut query: String = pstate(ui.ctx(), "rainee-brush-search");
    ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search brushes").desired_width(ui.available_width()));
    set_pstate(ui.ctx(), "rainee-brush-search", query.clone());
    let mut set = app.ui.brush_library_set.clone();
    let mut pinned = app.ui.brush_library_pinned.clone();
    let mut recent = app.ui.brush_library_recent.clone();
    let mut picked = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(86.0);
            for (key, label) in [
                ("", "All"),
                ("pinned", "Pinned"),
                ("recent", "Recent"),
                ("natural", "Natural"),
                ("art", "Vector Art"),
                ("calligraphic", "Inking"),
                ("bristle", "Bristle"),
                ("scatter", "Scatter"),
                ("pattern", "Pattern"),
            ] {
                if ui.selectable_label(set == key, label).clicked() {
                    set = key.into();
                }
            }
        });
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            for (name, ty) in &list {
                let in_set = set.is_empty() || set == *ty || (set == "pinned" && pinned.contains(name)) || (set == "recent" && recent.contains(name));
                if !in_set || !name.to_lowercase().contains(&query.to_lowercase()) {
                    continue;
                }
                let def = app.run("brush.get", json!({"name":name})).unwrap_or_default();
                let selected = current.as_ref() == Some(name);
                egui::Frame::NONE.fill(if selected { skin.row_selected } else { skin.input }).corner_radius(7).inner_margin(8).show(ui, |ui| {
                    ui.label(egui::RichText::new(name).size(12.5).strong());
                    let (r, hit) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
                    if let Some(tex) = preview_tone(ui, &def, r.size(), true) {
                        ui.painter().image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                    }
                    if hit.clicked() {
                        picked = Some(name.clone());
                    }
                    if hit.double_clicked() {
                        open_studio(app, ui.ctx(), name, &def);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("Use").clicked() {
                            picked = Some(name.clone());
                        }
                        if ui.small_button(if pinned.contains(name) { "★" } else { "☆" }).on_hover_text("Pin brush").clicked() {
                            if pinned.contains(name) {
                                pinned.retain(|n| n != name);
                            } else {
                                pinned.push(name.clone());
                            }
                        }
                        if ui.small_button("Edit").clicked() {
                            open_studio(app, ui.ctx(), name, &def);
                        }
                    });
                });
            }
        });
    });
    if let Some(name) = picked {
        let command = if app.session.active().is_some_and(|d| !d.selection.is_empty()) { "brush.apply" } else { "brush.setCurrent" };
        if let Err(e) = app.run(command, json!({"name":name})) {
            app.status(e);
        } else {
            recent.retain(|n| n != &name);
            recent.insert(0, name);
            recent.truncate(20);
        }
    }
    app.ui.brush_library_set = set;
    app.ui.brush_library_pinned = pinned;
    app.ui.brush_library_recent = recent;
    ui.collapsing("Brush actions / authoring", |ui| menu(app, ui));
    show_studio(app, ui.ctx());
}

mod studio_controls;
pub mod super_studio_ext;

#[derive(Clone, Default)]
struct Studio {
    document: u64,
    name: String,
    original: Value,
    draft: Value,
    strokes: Vec<Vec<[f32; 2]>>,
    drawing: bool,
    page: usize,
}

fn open_studio(app: &VectorcraftApp, ctx: &egui::Context, name: &str, def: &Value) {
    let Some(doc) = app.session.active() else { return };
    set_pstate(
        ctx,
        "rainee-brush-editor",
        Some(Studio { document: doc.uid, name: name.into(), original: def.clone(), draft: def.clone(), ..Default::default() }),
    );
}

fn studio_valid(app: &VectorcraftApp, studio: &Studio) -> bool {
    app.session.active().is_some_and(|d| d.uid == studio.document)
}

fn apply_studio(app: &mut VectorcraftApp, studio: &Studio) -> Result<(), String> {
    if !studio_valid(app, studio) {
        return Err("Brush Studio belongs to another document".into());
    }
    let current = app.run("brush.get", json!({"name":studio.name}))?;
    if current != studio.original {
        return Err("Brush changed outside Studio; cancel and reopen to avoid overwriting it".into());
    }
    app.run("brush.options", json!({"name":studio.name,"params":studio.draft}))?;
    Ok(())
}

fn pad_document(def: &Value, strokes: &[Vec<[f32; 2]>], w: f64, h: f64) -> Option<Document> {
    let name = def.get("name")?.as_str()?.to_owned();
    let mut doc = Document::new(w, h);
    doc.unknown.insert("brushes".into(), json!([def]));
    let layer = doc.layers.first()?.id;
    for samples in strokes.iter().take(8) {
        let Some(first) = samples.first() else { continue };
        let mut path = vectorcraft_geom::BezPath::new();
        path.move_to((first[0] as f64 * w, first[1] as f64 * h));
        for p in samples.iter().skip(1).take(511) {
            path.line_to((p[0] as f64 * w, p[1] as f64 * h));
        }
        let mut appearance = Appearance::basic(Paint::None, Paint::solid(Color::BLACK), 1.0);
        appearance.stroke_mut()?.brush = Some(name.clone());
        let id = doc.alloc_id();
        doc.insert(Some(layer), 0, Node::path(id, vectorcraft_geom::PathData::from_bezpath(&path), appearance)).ok()?;
    }
    Some(doc)
}

fn drawing_pad(ui: &mut Ui, studio: &mut Studio) {
    ui.horizontal(|ui| {
        ui.label("Drawing Pad");
        if ui.button("Clear").clicked() {
            studio.strokes.clear();
            studio.drawing = false;
        }
    });
    let size = vec2(ui.available_width().clamp(100.0, 500.0), 240.0);
    let (r, response) = ui.allocate_exact_size(size, Sense::drag());
    ui.painter().rect_filled(r, 8.0, Tokens::get(ui.ctx()).input);
    if response.drag_started() {
        if studio.strokes.len() >= 8 {
            studio.strokes.remove(0);
        }
        studio.strokes.push(Vec::new());
        studio.drawing = true;
    }
    if studio.drawing
        && let Some(p) = response.interact_pointer_pos()
        && r.contains(p)
    {
        let sample = [(p.x - r.left()) / r.width(), (p.y - r.top()) / r.height()];
        if let Some(stroke) = studio.strokes.last_mut()
            && stroke.len() < 512
            && stroke.last().is_none_or(|last| (last[0] - sample[0]).abs() + (last[1] - sample[1]).abs() > 0.002)
        {
            stroke.push(sample);
        }
    }
    if response.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
        studio.drawing = false;
    }
    let key = format!("studio-pad:{}:{:?}", studio.draft, studio.strokes);
    if let Some(tex) = widgets::doc_preview_light(ui, &key, r.size(), |w, h| pad_document(&studio.draft, &studio.strokes, w, h)) {
        ui.painter().image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    }
    ui.small("Disposable renderer-backed strokes. Mouse/pen position only; pressure/tilt calibration remains on the document canvas.");
}

fn show_studio(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let Some(mut studio) = pstate::<Option<Studio>>(ctx, "rainee-brush-editor") else { return };
    if !studio_valid(app, &studio) {
        set_pstate::<Option<Studio>>(ctx, "rainee-brush-editor", None);
        app.status("Brush Studio draft discarded: active document changed");
        return;
    }
    let mut open = true;
    let mut apply = false;
    let mut cancel = false;
    egui::Window::new(format!("Brush Studio · {}", studio.name))
        .id(egui::Id::new("rainee-brush-studio"))
        .open(&mut open)
        .collapsible(false)
        .frame(egui::Frame::window(&ctx.global_style()).inner_margin(18))
        .default_width(1080.0)
        .min_width(760.0)
        .default_pos(ctx.content_rect().center() - vec2(540.0, 290.0))
        .show(ctx, |ui| {
            Tokens::style_rainee(ui);
            ui.horizontal(|ui| {
                ui.label("Editable vector brush");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    apply = widgets::primary_button(ui, "Done").clicked();
                    cancel = ui.button("Cancel").clicked();
                    if ui.button("Reset draft").on_hover_text("Restore the definition from when Studio was opened").clicked() {
                        studio.draft = studio.original.clone();
                    }
                });
            });
            ui.separator();
            let width = ui.available_width();
            let nav_width = (width * 0.21).clamp(158.0, 220.0);
            let settings_width = width * 0.31;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(nav_width, 430.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                for (i, name) in studio_controls::PAGES.iter().enumerate() {
                    if ui.add(egui::Button::new(*name).selected(studio.page == i).min_size(vec2(nav_width, 36.0))).clicked() {
                        studio.page = i;
                    }
                }
                ui.add_space(16.0);
                ui.small("Settings depend on the selected brush engine.");
                });
                ui.separator();
                ui.allocate_ui_with_layout(vec2(settings_width, 430.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().max_height(430.0).show(ui, |ui| {
                    studio_controls::show(ui, &mut studio);
                });
                });
                ui.separator();
                ui.vertical(|ui| {
                ui.set_width((width - nav_width - settings_width - 40.0).max(220.0));
                ui.label("Rendered stroke preview");
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 90.0), Sense::hover());
                if let Some(tex) = preview_tone(ui, &studio.draft, r.size(), true) {
                    ui.painter().image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                }
                drawing_pad(ui, &mut studio);
                });
            });
        });
    if apply {
        match apply_studio(app, &studio) {
            Ok(()) => open = false,
            Err(e) => app.status(e),
        }
    }
    set_pstate(ctx, "rainee-brush-editor", if open && !cancel { Some(studio) } else { None });
}

pub fn menu(app: &mut VectorcraftApp, ui: &mut Ui) {
    let sel = selected_brush(app);
    let (_, current) = brushes(app);
    let target = sel.clone().or(current);
    let has_sel = app.session.active().is_some_and(|d| !d.selection.is_empty());
    for (label, ty) in [(tl!("New Calligraphic Brush"), "calligraphic"), (tl!("New Bristle Brush"), "bristle")] {
        if menu_item(ui, label, true, false) {
            app.run("brush.new", json!({"type": ty})).ok();
        }
    }
    if menu_item(ui, "New Natural Brush", true, false) {
        app.run("brush.new", json!({"type":"natural"})).ok();
    }
    for (label, ty) in [
        (tl!("New Art Brush from Selection"), "art"),
        (tl!("New Scatter Brush from Selection"), "scatter"),
        (tl!("New Pattern Brush from Selection"), "pattern"),
    ] {
        if menu_item(ui, label, has_sel, false) {
            app.run("brush.new", json!({"type": ty})).ok();
        }
    }
    if menu_item(ui, tl!("Duplicate Brush"), target.is_some(), false)
        && let Some(n) = &target
    {
        app.run("brush.duplicate", json!({"name": n})).ok();
    }
    if menu_item(ui, tl!("Delete Brush"), target.is_some(), false)
        && let Some(n) = &target
    {
        app.run("brush.delete", json!({"name": n})).ok();
    }
    if menu_item(ui, tl!("Remove Brush Stroke"), sel.is_some(), false) {
        app.run("brush.remove", json!({})).ok();
    }
    if menu_item(ui, tl!("Expand Brush Strokes"), sel.is_some(), false) {
        app.run("object.expandBrush", json!({})).ok();
    }
    ui.separator();
    let mut hidden: Vec<String> = pstate(ui.ctx(), "br-hidden");
    for (ty, label) in KINDS {
        let shown = !hidden.iter().any(|h| h == ty);
        if menu_item(ui, &crate::i18n::fmt(tl!("Show {kind} Brushes"), &[("kind", tl!(label))]), true, shown) {
            if shown {
                hidden.push(ty.to_string());
            } else {
                hidden.retain(|h| h != ty);
            }
            set_pstate(ui.ctx(), "br-hidden", hidden.clone());
        }
    }
    ui.separator();
    let list: bool = pstate(ui.ctx(), "br-list");
    if menu_item(ui, tl!("Thumbnail View"), true, !list) {
        set_pstate(ui.ctx(), "br-list", false);
    }
    if menu_item(ui, tl!("List View"), true, list) {
        set_pstate(ui.ctx(), "br-list", true);
    }
}

#[cfg(test)]
mod tests {
    use egui::{Pos2, Rect, pos2};
    use vectorcraft_doc::NodeId;
    use vectorcraft_engine::Session;

    use super::*;

    #[test]
    fn studio_pad_is_disposable_and_edits_are_undoable() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({})).unwrap();
        let ctx = egui::Context::default();
        let name = brushes(&mut app).0.first().unwrap().0.clone();
        let original = app.run("brush.get", json!({"name":name})).unwrap();
        let before = app.session.doc().unwrap().doc.clone();
        open_studio(&app, &ctx, &name, &original);
        let mut studio = pstate::<Option<Studio>>(&ctx, "rainee-brush-editor").unwrap();
        studio.strokes = vec![vec![[0.1, 0.2], [0.4, 0.8], [0.9, 0.3]]];
        let pad = pad_document(&studio.draft, &studio.strokes, 300.0, 200.0).unwrap();
        assert_eq!(pad.layers.first().unwrap().children().unwrap().len(), 1);
        assert_eq!(app.session.doc().unwrap().doc, before);
        studio.draft["size"] = json!(8.0);
        apply_studio(&mut app, &studio).unwrap();
        assert_eq!(app.run("brush.get", json!({"name":name})).unwrap()["size"], json!(8.0));
        app.run("edit.undo", json!({})).unwrap();
        assert_eq!(app.run("brush.get", json!({"name":name})).unwrap(), original);
        app.run("brush.options", json!({"name":name,"params":{"size":12.0}})).unwrap();
        assert!(apply_studio(&mut app, &studio).is_err(), "stale external edit must not be overwritten");
        app.run("file.new", json!({})).unwrap();
        assert!(!studio_valid(&app, &studio));
        assert!(apply_studio(&mut app, &studio).is_err());
    }

    #[test]
    fn brush_library_preferences_survive_json_roundtrip() {
        let mut ui = crate::state::UiState::default();
        ui.brush_library_pinned = vec!["Studio Ink".into()];
        ui.brush_library_recent = vec!["Dry Ink".into()];
        ui.brush_library_set = "pinned".into();
        let restored: crate::state::UiState = serde_json::from_value(serde_json::to_value(&ui).unwrap()).unwrap();
        assert_eq!(restored.brush_library_pinned, ui.brush_library_pinned);
        assert_eq!(restored.brush_library_recent, ui.brush_library_recent);
        assert_eq!(restored.brush_library_set, "pinned");
    }

    /// One headless frame of the panel, 236 pt wide as in the dock.
    fn frame(ctx: &egui::Context, app: &mut VectorcraftApp, events: Vec<egui::Event>) {
        let raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(236.0, 600.0))), events, ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| show(app, ui));
        out.textures_delta.clear();
    }

    fn button(at: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
    }

    #[test]
    fn art_dropped_on_the_panel_becomes_an_art_brush_and_a_brush_drags_out() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 200, "height": 100})).unwrap();
        let id = app.run("shape.star", json!({"cx": 50, "cy": 50, "radius1": 20, "radius2": 9, "points": 5})).unwrap()["id"].as_u64().unwrap();
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, vec![]);
        let (before, _) = brushes(&mut app);
        // Art dragged off the canvas and released over the list.
        let over = pos2(100.0, 60.0);
        egui::DragAndDrop::set_payload(&ctx, PanelDrag::Art(vec![NodeId(id)]));
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(over), button(over, false)]);
        let (after, _) = brushes(&mut app);
        assert_eq!(after.len(), before.len() + 1);
        let new: Vec<_> = after.iter().filter(|b| !before.contains(b)).collect();
        assert!(matches!(new.as_slice(), [(_, ty)] if ty == "art"), "{new:?}");
        // Dragging the first tile out of the panel carries that brush.
        frame(&ctx, &mut app, vec![]);
        let tile = pos2(36.0, 16.0);
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(tile), button(tile, true)]);
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(tile + vec2(40.0, 40.0))]);
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(tile + vec2(80.0, 80.0))]);
        let drag = egui::DragAndDrop::payload::<PanelDrag>(&ctx);
        assert!(matches!(drag.as_deref(), Some(PanelDrag::Brush { name, def }) if *name == after[0].0 && def["name"] == json!(name)), "{drag:?}");
    }
}
