//! Brushes panel: the document's brush library with rendered stroke previews. Clicking a brush
//! applies it to the selected paths (and makes it the Paintbrush's current brush); double-clicking
//! one opens its Brush Options.

use std::collections::HashMap;

use egui::{Sense, Ui, vec2};
use serde_json::{Value, json};
use vectorcraft_doc::{Appearance, Document, Node, PressureProfile, color::Color, color::Paint};

use super::{first_selected, pstate, set_pstate};
use crate::VectorcraftApp;
use crate::dialogs::brush_options;
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
/// JSON. The stroke goes from light to heavy pen pressure and back, so pressure-sensitive brushes
/// show how they vary.
fn preview(ui: &Ui, def: &Value, size: egui::Vec2) -> Option<egui::TextureHandle> {
    preview_tone(ui, def, size, false)
}

fn preview_tone(ui: &Ui, def: &Value, size: egui::Vec2, light: bool) -> Option<egui::TextureHandle> {
    let build = |w: f64, h: f64| {
        let mut doc = Document::new(w, h);
        doc.unknown.insert("brushes".into(), json!([def]));
        let name = def["name"].as_str()?.to_string();
        let mut ap = Appearance::basic(Paint::None, Paint::solid(Color::BLACK), 1.0);
        let st = ap.stroke_mut()?;
        st.brush = Some(name);
        st.pressure = Some(PressureProfile::sample());
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
    let mut options: Option<String> = None;
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
            // A click applies the brush; the second click of a double-click opens its options.
            let mut pick = |resp: egui::Response, name: &String| {
                if resp.double_clicked() {
                    options = Some(name.clone());
                } else if resp.clicked() {
                    clicked = Some(name.clone());
                }
            };
            if list_view {
                for (name, _) in group {
                    pick(row(ui, name, vec2(ui.available_width(), 26.0), true), name);
                }
            } else {
                ui.horizontal_wrapped(|ui| {
                    for (name, _) in group {
                        pick(row(ui, name, vec2(64.0, 28.0), false), name);
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
    if let Some(name) = options
        && let Err(e) = app.run("ui.brushOptions", json!({ "name": name }))
    {
        app.status(e);
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
    let (mut list, current) = brushes(app);
    let mut query: String = pstate(ui.ctx(), "rainee-brush-search");
    ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search brushes").desired_width(ui.available_width()));
    set_pstate(ui.ctx(), "rainee-brush-search", query.clone());
    let mut set = app.ui.brush_library_set.clone();
    let mut pinned = app.ui.brush_library_pinned.clone();
    let mut recent = app.ui.brush_library_recent.clone();
    if set == "recent" {
        list.sort_by_key(|(name, _)| recent.iter().position(|n| n == name).unwrap_or(usize::MAX));
    }
    let mut picked = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(76.0);
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
                egui::Frame::NONE.fill(if selected { skin.row_selected } else { skin.input }).corner_radius(7).inner_margin(6).show(ui, |ui| {
                    let name_hit =
                        ui.add(egui::Label::new(egui::RichText::new(name).size(12.5).strong()).truncate().sense(Sense::click())).on_hover_text(name);
                    if name_hit.clicked() {
                        picked = Some(name.clone());
                    }
                    if name_hit.double_clicked() {
                        open_studio(app, ui.ctx(), name, &def);
                    }
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
mod studio_layout;
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
    move_to: Option<egui::Pos2>,
}

fn open_studio(app: &VectorcraftApp, ctx: &egui::Context, name: &str, def: &Value) {
    let Some(doc) = app.session.active() else { return };
    set_pstate(
        ctx,
        "rainee-brush-editor",
        Some(Studio { document: doc.uid, name: name.into(), original: def.clone(), draft: def.clone(), ..Default::default() }),
    );
}

pub(crate) fn studio_open(app: &mut VectorcraftApp, ctx: &egui::Context, name: &str) -> Result<Value, String> {
    let def = app.run("brush.get", json!({"name":name}))?;
    open_studio(app, ctx, name, &def);
    ctx.request_repaint();
    Ok(studio_inspect(ctx))
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
    let new_name = studio.draft.get("name").and_then(Value::as_str).map(str::trim).filter(|n| *n != studio.name);
    if new_name == Some("") {
        return Err("Give this brush a name before saving".into());
    }
    app.run("brush.options", json!({"name":studio.name,"params":studio.draft,"newName":new_name}))?;
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
        let undo = ui.button("Undo").on_hover_text("Remove the last preview stroke");
        set_pstate(ui.ctx(), "studio-undo-preview-rect", Some([undo.rect.left(), undo.rect.top(), undo.rect.width(), undo.rect.height()]));
        if undo.clicked() {
            studio.strokes.pop();
            studio.drawing = false;
        }
        let clear = ui.button("Clear");
        set_pstate(ui.ctx(), "studio-clear-preview-rect", Some([clear.rect.left(), clear.rect.top(), clear.rect.width(), clear.rect.height()]));
        if clear.clicked() {
            studio.strokes.clear();
            studio.drawing = false;
        }
    });
    let size = vec2(ui.available_width().max(40.0), (ui.available_height() - 28.0).clamp(100.0, 520.0));
    let (r, response) = ui.allocate_exact_size(size, Sense::drag());
    set_pstate(ui.ctx(), "studio-pad-rect", Some([r.left(), r.top(), r.width(), r.height()]));
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
    ui.small("Preview strokes stay off your document.");
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
    let mut cancel = app.ui.dialog.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let room = ctx.content_rect().shrink(16.0);
    let last_rect = ctx.memory(|m| m.area_rect(egui::Id::new("rainee-brush-studio")));
    let mut window = egui::Window::new(format!("Brush Studio · {}", studio.name))
        .id(egui::Id::new("rainee-brush-studio"))
        .open(&mut open)
        .collapsible(false)
        .frame(egui::Frame::window(&ctx.global_style()).inner_margin(16))
        .title_bar(false)
        .movable(false)
        .drag_area(egui::WindowDrag::Off)
        .resizable(true)
        .default_size(vec2(1080.0, 680.0).min(room.size()))
        .min_size(vec2(740.0, 500.0).min(room.size()))
        .max_size(room.size())
        .default_pos(room.center() - vec2(540.0, 340.0).min(room.size() * 0.5));
    if let Some(position) = studio.move_to.take() {
        window = window.current_pos(position);
    }
    window.show(ctx, |ui| {
        Tokens::style_rainee(ui);
        let (header, _) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::hover());
        let title_rect = egui::Rect::from_min_max(header.min, header.right_bottom() - vec2(270.0, 0.0));
        set_pstate(ctx, "studio-header-rect", Some([title_rect.left(), title_rect.top(), title_rect.width(), title_rect.height()]));
        let grip = ui.interact(title_rect, ui.id().with("studio-header-drag"), Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
        if grip.dragged()
            && !grip.drag_started()
            && let Some(rect) = last_rect
        {
            let p = rect.min + grip.drag_delta();
            studio.move_to = Some(egui::pos2(
                p.x.max(room.left()).min((room.right() - rect.width()).max(room.left())),
                p.y.max(room.top()).min((room.bottom() - rect.height()).max(room.top())),
            ));
            ctx.request_repaint();
        }
        let mut title = ui.new_child(egui::UiBuilder::new().max_rect(title_rect));
        title.vertical(|ui| {
            ui.add(egui::Label::new(egui::RichText::new("Brush Studio").size(24.0).strong()).selectable(false).sense(Sense::hover()));
            ui.add(
                egui::Label::new(egui::RichText::new(&studio.name).color(Tokens::get(ctx).text_dim))
                    .truncate()
                    .selectable(false)
                    .sense(Sense::hover()),
            )
            .on_hover_text(&studio.name);
        });
        let mut actions = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(header.right_top() - vec2(262.0, 0.0), header.max))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        actions.scope(|ui| {
            let done = ui.add(
                egui::Button::new(egui::RichText::new("Done").color(egui::Color32::BLACK).strong())
                    .fill(egui::Color32::from_rgb(255, 159, 10))
                    .min_size(vec2(66.0, 30.0)),
            );
            apply = done.clicked();
            set_pstate(ctx, "studio-done-rect", Some([done.rect.left(), done.rect.top(), done.rect.width(), done.rect.height()]));
            let cancel_button = ui.button("Cancel");
            cancel |= cancel_button.clicked();
            set_pstate(
                ctx,
                "studio-cancel-rect",
                Some([cancel_button.rect.left(), cancel_button.rect.top(), cancel_button.rect.width(), cancel_button.rect.height()]),
            );
            if ui.button("Reset draft").on_hover_text("Restore the definition from when Studio was opened").clicked() {
                studio.draft = studio.original.clone();
            }
        });
        ui.separator();
        studio_layout::show(app, ui, &mut studio);
    });
    if apply {
        match apply_studio(app, &studio) {
            Ok(()) => open = false,
            Err(e) => app.status(e),
        }
    }
    set_pstate(ctx, "rainee-brush-editor", if open && !cancel { Some(studio) } else { None });
}

pub(crate) fn studio_inspect(ctx: &egui::Context) -> Value {
    let studio = pstate::<Option<Studio>>(ctx, "rainee-brush-editor");
    json!({"open":studio.is_some(),"name":studio.as_ref().map(|s|&s.name),
        "section":studio.as_ref().and_then(|s|studio_controls::PAGES.get(s.page)),
        "previewStrokes":studio.as_ref().map(|s|s.strokes.len()),
        "sections":pstate::<Vec<(String,[f32;4])>>(ctx,"studio-section-rects"),
        "header":pstate::<Option<[f32;4]>>(ctx,"studio-header-rect"),
        "pad":pstate::<Option<[f32;4]>>(ctx,"studio-pad-rect"),
        "done":pstate::<Option<[f32;4]>>(ctx,"studio-done-rect"),
        "cancel":pstate::<Option<[f32;4]>>(ctx,"studio-cancel-rect"),
        "undoPreview":pstate::<Option<[f32;4]>>(ctx,"studio-undo-preview-rect"),
        "clearPreview":pstate::<Option<[f32;4]>>(ctx,"studio-clear-preview-rect"),
        "controls":pstate::<std::collections::BTreeMap<String,[f32;4]>>(ctx,"studio-control-rects"),
        "rect":ctx.memory(|m|m.area_rect(egui::Id::new("rainee-brush-studio"))).map(|r|[r.left(),r.top(),r.width(),r.height()])})
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
    // On the selected path's brush, else the current one, as the other items.
    if menu_item(ui, tl!("Brush Options…"), brush_options::available(app), false)
        && let Err(e) = app.run("ui.brushOptions", json!({}))
    {
        app.status(e);
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
    fn studio_sections_and_idle_frames_do_not_resize_the_window_or_edit_art() {
        for (size, dpi) in [(vec2(1440.0, 900.0), 1.0), (vec2(1024.0, 768.0), 2.0)] {
            let mut app = VectorcraftApp::new(Session::new(), Default::default());
            app.run("file.new", json!({})).unwrap();
            let before = app.session.doc().unwrap().doc.clone();
            let ctx = egui::Context::default();
            crate::theme::install_fonts(&ctx);
            crate::theme::apply_workspace(&ctx, Default::default(), true, 65.0, 55.0, false);
            let name = brushes(&mut app).0.first().unwrap().0.clone();
            let original = app.run("brush.get", json!({"name":name})).unwrap();
            open_studio(&app, &ctx, &name, &original);
            let mut time = 0.0;
            let mut frame = |app: &mut VectorcraftApp, events: Vec<egui::Event>| {
                time += 0.05;
                let mut raw =
                    egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), time: Some(time), events, ..Default::default() };
                raw.viewports.entry(egui::ViewportId::ROOT).or_default().native_pixels_per_point = Some(dpi);
                ctx.run_ui(raw, |ui| show_studio(app, ui.ctx())).textures_delta.clear();
                ctx.memory(|m| m.area_rect(egui::Id::new("rainee-brush-studio"))).unwrap()
            };
            for _ in 0..8 {
                frame(&mut app, vec![]);
            }
            let start = frame(&mut app, vec![]);
            for page in (0..studio_controls::PAGES.len()).cycle().take(27) {
                let mut studio = pstate::<Option<Studio>>(&ctx, "rainee-brush-editor").unwrap();
                studio.page = page;
                set_pstate(&ctx, "rainee-brush-editor", Some(studio));
                for _ in 0..4 {
                    let now = frame(&mut app, vec![]);
                    assert!((now.size() - start.size()).length() < 1.0, "page {page}: {start:?} -> {now:?}");
                    assert!((now.min - start.min).length() < 1.0, "subsection changed position: {start:?} -> {now:?}");
                    assert!(now.width() <= size.x && now.height() <= size.y, "{now:?}");
                }
            }
            let sections = pstate::<Vec<(String, [f32; 4])>>(&ctx, "studio-section-rects");
            for (label, r) in sections {
                let at = pos2(r[0] + r[2] * 0.5, r[1] + r[3] * 0.5);
                let moved = at + vec2(1.5, 1.0);
                let touch =
                    |phase, pos| egui::Event::Touch { device_id: egui::TouchDeviceId(42), id: egui::TouchId(1), phase, pos, force: Some(0.6) };
                let button =
                    |pos, pressed| egui::Event::PointerButton { pos, pressed, button: egui::PointerButton::Primary, modifiers: Default::default() };
                frame(&mut app, vec![touch(egui::TouchPhase::Start, at), egui::Event::PointerMoved(at), button(at, true)]);
                frame(&mut app, vec![touch(egui::TouchPhase::Move, moved), egui::Event::PointerMoved(moved)]);
                let now = frame(&mut app, vec![touch(egui::TouchPhase::End, moved), button(moved, false)]);
                assert!(ctx.input(|i| i.has_touch_screen()), "exercise the tablet/touch drag default");
                assert!((now.min - start.min).length() < 1.0 && (now.size() - start.size()).length() < 1.0, "touch on {label}: {start:?} -> {now:?}");
                assert_eq!(
                    pstate::<Option<Studio>>(&ctx, "rainee-brush-editor").unwrap().page,
                    studio_controls::PAGES.iter().position(|p| *p == label).unwrap()
                );
            }
            let header = pstate::<Option<[f32; 4]>>(&ctx, "studio-header-rect").unwrap();
            let at = pos2(header[0] + 80.0, header[1] + 20.0);
            let button =
                |pos, pressed| egui::Event::PointerButton { pos, pressed, button: egui::PointerButton::Primary, modifiers: Default::default() };
            frame(&mut app, vec![egui::Event::PointerMoved(at), button(at, true)]);
            frame(&mut app, vec![egui::Event::PointerMoved(at + vec2(12.0, 8.0))]);
            frame(&mut app, vec![button(at + vec2(12.0, 8.0), false)]);
            let moved = frame(&mut app, vec![]);
            assert!((moved.min - start.min).length() > 5.0, "header must still drag deliberately");
            assert_eq!(app.session.doc().unwrap().doc, before);
        }
    }

    #[test]
    fn studio_renaming_updates_stroke_references_and_undo_restores_them() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({})).unwrap();
        let name = brushes(&mut app).0.first().unwrap().0.clone();
        app.run("shape.rectangle", json!({"x":10,"y":10,"width":100,"height":100})).unwrap();
        app.run("brush.apply", json!({"name":name})).unwrap();
        let before = app.session.doc().unwrap().doc.clone();
        let original = app.run("brush.get", json!({"name":name})).unwrap();
        let mut draft = original.clone();
        draft["name"] = json!("Studio renamed brush");
        let studio = Studio { document: app.session.active().unwrap().uid, name: name.clone(), original, draft, ..Default::default() };
        apply_studio(&mut app, &studio).unwrap();
        assert!(app.run("brush.get", json!({"name":"Studio renamed brush"})).is_ok());
        assert_eq!(selected_brush(&app).as_deref(), Some("Studio renamed brush"));
        app.run("edit.undo", json!({})).unwrap();
        assert_eq!(app.session.doc().unwrap().doc, before);
    }

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

    /// #852: double-clicking a brush opens its Brush Options; OK changes the brush (and strokes
    /// painted with it). A single click only applies the brush.
    #[test]
    fn double_clicking_a_brush_opens_its_options() {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width": 200, "height": 100})).unwrap();
        let ctx = egui::Context::default();
        let clicks = |at: Pos2, n: usize| -> Vec<egui::Event> {
            std::iter::once(egui::Event::PointerMoved(at)).chain((0..n).flat_map(|_| [button(at, true), button(at, false)])).collect()
        };
        let at_time = |app: &mut VectorcraftApp, time: f64, events: Vec<egui::Event>| {
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(236.0, 600.0))),
                time: Some(time),
                events,
                ..Default::default()
            };
            ctx.run_ui(raw, |ui| show(app, ui)).textures_delta.clear();
        };
        at_time(&mut app, 0.0, vec![]);
        let (list, _) = brushes(&mut app);
        let first = list[0].0.clone();
        assert_eq!(list[0].1, "calligraphic");
        // The first tile, clicked once: it becomes current, no dialog.
        let tile = pos2(36.0, 16.0);
        at_time(&mut app, 1.0, clicks(tile, 1));
        assert!(app.ui.dialog.is_none());
        assert_eq!(brushes(&mut app).1.as_deref(), Some(first.as_str()));
        // Double-clicked: Calligraphic Brush Options on that brush.
        at_time(&mut app, 3.0, clicks(tile, 2));
        let d = app.ui.dialog.as_mut().expect("Brush Options opened");
        assert_eq!((d.kind.as_str(), d.str("name"), d.str("sizeMode")), (crate::dialogs::brush_options::KIND, first.clone(), "fixed".into()));
        d.fields.insert("sizeMode".into(), json!("pressure"));
        d.fields.insert("sizeVariation".into(), json!(2));
        d.fields.insert("name".into(), json!("Pressure Round"));
        crate::dialogs::confirm(&mut app).unwrap();
        assert!(app.ui.dialog.is_none());
        let def = app.run("brush.get", json!({"name": "Pressure Round"})).unwrap();
        assert_eq!((def["modes"].clone(), def["variation"][2].clone()), (json!(["fixed", "fixed", "pressure"]), json!(2.0)));
        assert_eq!(brushes(&mut app).1.as_deref(), Some("Pressure Round"), "renamed, still current");
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
