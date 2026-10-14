//! Edit → Preferences (⌘K): Illustrator-style category list + fields, rendered from the engine's
//! preference table ([`PREF_SPECS`]). The dialog edits a working copy in `Dialog::fields`
//! (one field per preference key, so agents can `ui.dialog.set` any of them) and OK applies it
//! through `prefs.set`. Also: persistence of engine prefs inside `UiState` and the per-frame
//! application of the UI-side preferences (brightness, canvas colour, UI scaling, render threads).

use serde_json::{Map, Value, json};
use vectorcraft_engine::Prefs;
use vectorcraft_engine::cmd::prefscmds::{GPU_PREFERENCES, PREF_CATEGORIES, PREF_GROUPS, PREF_SPECS, PrefKind};

use crate::state::Dialog;
use crate::theme::{self, Brightness, Tokens};
use crate::{VectorcraftApp, widgets};

/// UI-only fields shown in the dialog (backed by `UiState`, not engine prefs).
pub const UI_FIELDS: &[(&str, &str, &str)] = &[
    ("__smartGuides", "Smart Guides", "Smart Guides (View → Smart Guides)"),
    ("__snapToGrid", "Guides & Grid", "Snap to Grid"),
    ("__taskBar", "General", "Enable Contextual Task Bar"),
];

pub fn open(app: &mut VectorcraftApp, category: Option<&str>) {
    // One field per preference key or group (the flattener presets have commands of their own).
    let mut fields: Map<String, Value> = app.session.prefs.to_json().as_object().cloned().unwrap_or_default();
    fields.retain(|k, _| vectorcraft_engine::cmd::prefscmds::spec(k).is_some() || PREF_GROUPS.contains(&k.as_str()));
    // Units ▸ General shows (and OK sets) the open document's units.
    if let Some(st) = app.session.active() {
        fields.insert("unitsGeneral".into(), json!(st.doc.units.key()));
    }
    let cat = category.and_then(|c| PREF_CATEGORIES.iter().find(|x| x.eq_ignore_ascii_case(c))).copied().unwrap_or(PREF_CATEGORIES[0]);
    fields.insert("__category".into(), json!(cat));
    fields.insert("__smartGuides".into(), json!(app.ui.view.smart_guides));
    fields.insert("__snapToGrid".into(), json!(app.ui.view.snap_to_grid));
    fields.insert("__taskBar".into(), json!(app.ui.task_bar));
    fields.insert("__scratchDrives".into(), json!(scratch_drives()));
    fields.insert("__splashImagePath".into(), json!(app.ui.splash_image_path.clone().unwrap_or_default()));
    app.ui.dialog = Some(Dialog { kind: "preferences".into(), fields });
}

/// Enumerate drives when Preferences opens, not every UI frame. An unavailable
/// previously selected drive remains visible in the dropdown until changed.
fn scratch_drives() -> Vec<String> {
    #[cfg(windows)]
    {
        (b'A'..=b'Z').map(|letter| format!("{}:\\", letter as char)).filter(|drive| std::path::Path::new(drive).is_dir()).collect()
    }
    #[cfg(not(windows))]
    {
        vec!["/".into()]
    }
}

/// OK: validate and apply every preference in the dialog (nothing changes on error).
pub fn confirm(app: &mut VectorcraftApp) -> Result<Value, String> {
    let Some(d) = app.ui.dialog.clone() else { return Err("no dialog open".into()) };
    let values: Map<String, Value> = d.fields.iter().filter(|(k, _)| !k.starts_with("__")).map(|(k, v)| (k.clone(), v.clone())).collect();
    app.run("prefs.set", json!({ "values": values }))?;
    app.ui.view.smart_guides = d.fields.get("__smartGuides").and_then(Value::as_bool).unwrap_or(app.ui.view.smart_guides);
    app.ui.view.snap_to_grid = d.fields.get("__snapToGrid").and_then(Value::as_bool).unwrap_or(app.ui.view.snap_to_grid);
    app.ui.task_bar = d.fields.get("__taskBar").and_then(Value::as_bool).unwrap_or(app.ui.task_bar);
    if let Some(p) = d.fields.get("__splashImagePath").and_then(Value::as_str) {
        let p = p.trim().to_string();
        app.ui.splash_image_path = if p.is_empty() { None } else { Some(p) };
    }
    app.ui.dialog = None;
    app.ui.engine_prefs = app.session.prefs.to_json();
    Ok(Value::Null)
}

/// Copy the engine prefs into `UiState` so the host persists them with the UI state.
pub fn snapshot(app: &mut VectorcraftApp) {
    app.ui.engine_prefs = app.session.prefs.to_json();
}

/// Restore engine prefs from `UiState` after the host loaded saved UI state.
pub fn restore(app: &mut VectorcraftApp) {
    let mut p: Prefs = serde_json::from_value(app.ui.engine_prefs.clone()).unwrap_or_default();
    if app.ui.engine_prefs.is_null() {
        // Older preference files: keep the saved brightness.
        p.ui_brightness = app.ui.brightness.id().into();
    }
    // Older preference files kept the interface language with the UI state; English was the
    // default there, so only another language is carried over (onto an unset preference).
    if let Some(code) = app.ui.legacy_language.take()
        && p.interface_language == "auto"
        && code != "en"
        && crate::i18n::Lang::from_code(&code).is_some()
    {
        p.interface_language = code;
    }
    // 0.5.0 saved its default, `powerSaving`, which Automatic replaced (#502): any value this
    // version doesn't offer reads as the default, as the desktop app reads it at startup.
    if !GPU_PREFERENCES.iter().any(|(v, _)| *v == p.gpu_preference) {
        p.gpu_preference = Prefs::default().gpu_preference;
    }
    app.session.apply_prefs(p);
}

#[derive(Clone, Copy, PartialEq)]
struct Applied {
    brightness: Brightness,
    rainee: bool,
    darkness: f64,
    glass: f64,
    opaque: bool,
    /// User Interface › Canvas Color, unless it matches the interface brightness.
    canvas: Option<egui::Color32>,
    threads: i32,
    tool_tips: bool,
    scrub: bool,
    bare_points: bool,
}

/// User Interface › Canvas Color: the canvas around the artboards, or `None` to match the
/// interface brightness. The greys keep the white page's edge in view (#663).
fn canvas_color(key: &str) -> Option<egui::Color32> {
    match key {
        "white" => Some(egui::Color32::WHITE),
        "lightGray" => Some(egui::Color32::from_gray(0xc8)),
        "mediumGray" => Some(egui::Color32::from_gray(0x96)),
        "darkGray" => Some(egui::Color32::from_gray(0x5a)),
        _ => None,
    }
}

/// Per frame: push UI-side preferences into egui / the renderer when they change.
pub fn apply_runtime(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let p = &app.session.prefs;
    if let Some(b) = Brightness::parse(&p.ui_brightness)
        && b != app.ui.brightness
    {
        app.ui.brightness = b;
    }
    let mut want = Applied {
        brightness: app.ui.brightness,
        rainee: app.ui.procreate_panels,
        darkness: p.rainee_darkness,
        glass: p.ui_glass_intensity,
        opaque: p.ui_reduce_transparency,
        canvas: canvas_color(&p.canvas_color),
        threads: p.render_threads,
        tool_tips: p.show_tool_tips,
        scrub: p.scrub_numeric_fields,
        bare_points: p.numbers_without_units_are_points,
    };
    // Preview appearance without persisting the draft. Cancel restores the saved values
    // on the next frame, including when another workspace is selected.
    if let Some(draft) = app.ui.dialog.as_ref().filter(|d| d.kind == "preferences") {
        want.darkness = draft.fields.get("raineeDarkness").and_then(Value::as_f64).unwrap_or(want.darkness);
        want.glass = draft.fields.get("uiGlassIntensity").and_then(Value::as_f64).unwrap_or(want.glass);
        want.opaque = draft.fields.get("uiReduceTransparency").and_then(Value::as_bool).unwrap_or(want.opaque);
    }
    let id = egui::Id::new("dc-applied-prefs");
    let prev: Option<Applied> = ctx.data(|d| d.get_temp::<Option<Applied>>(id)).flatten();
    if prev != Some(want) {
        theme::apply_workspace(ctx, want.brightness, want.rainee, want.darkness, want.glass, want.opaque);
        // General › Show Tool Tips: off, no button or field shows its tool tip (they never come
        // due), whichever widget asks for one.
        let delay = if want.tool_tips { egui::style::Interaction::default().tooltip_delay } else { f32::INFINITY };
        ctx.global_style_mut(|s| s.interaction.tooltip_delay = delay);
        crate::scrub::set_enabled(ctx, want.scrub);
        crate::widgets::set_bare_numbers_are_points(ctx, want.bare_points);
        if let Some(c) = want.canvas {
            let mut t = Tokens::get(ctx);
            t.pasteboard = c;
            ctx.data_mut(|d| d.insert_temp(egui::Id::NULL, t));
        }
        if prev.is_some_and(|pr| pr.threads != want.threads) {
            vectorcraft_render::set_default_threads(u16::try_from(want.threads).ok());
            app.canvas.renderer.threads = vectorcraft_render::default_threads();
            app.canvas.worker = None;
            app.canvas.worker_started = false;
        }
        app.canvas.key = None;
        ctx.data_mut(|d| d.insert_temp(id, Some(want)));
    }
    let z = app.session.prefs.ui_scaling.clamp(0.75, 2.0) as f32;
    if (ctx.zoom_factor() - z).abs() > 1e-3 {
        ctx.set_zoom_factor(z);
    }
}

pub fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let Some(mut d) = app.ui.dialog.clone() else { return };
    detect_pen_button(&mut d, ctx);
    let t = Tokens::get(ctx);
    let (mut ok, mut cancel, mut reset) = (false, false, false);
    let cat = d.str("__category");
    let cat = PREF_CATEGORIES.iter().find(|c| **c == cat).copied().unwrap_or(PREF_CATEGORIES[0]);
    crate::dialogs::modal::show(ctx, tl!("Preferences"), egui::Id::new("dialog-preferences"), -20.0, 20, |ui| {
        ui.set_width(760.0);
        crate::dialogs::modal::heading(ui, tl!("Preferences"));
        ui.add_space(12.0);
        ui.horizontal_top(|ui| {
            // Category list.
            egui::Frame::NONE.fill(t.panel_darker).corner_radius(egui::CornerRadius::same(4)).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                ui.set_width(196.0);
                ui.set_min_height(430.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for c in PREF_CATEGORIES {
                        let sel = *c == cat;
                        let text = egui::RichText::new(tl!(*c)).size(12.5).color(if sel { t.text_strong } else { t.text });
                        let b = egui::Button::selectable(sel, text).frame_when_inactive(false).min_size(egui::vec2(184.0, 24.0));
                        if ui.add(b).clicked() {
                            d.fields.insert("__category".into(), json!(c));
                        }
                    }
                });
            });
            ui.add_space(14.0);
            // Fields.
            ui.vertical(|ui| {
                ui.set_width(530.0);
                ui.label(egui::RichText::new(tl!(cat)).font(theme::semibold(14.0)).color(t.text_strong));
                ui.add_space(8.0);
                egui::ScrollArea::vertical().id_salt(("prefs", cat)).max_height(400.0).auto_shrink([false, false]).show(ui, |ui| {
                    category_fields(ui, &mut d, cat);
                });
            });
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if widgets::secondary_button(ui, tl!("Reset Preferences"))
                .on_hover_text(tl!("Restore every preference to its default (applied on OK)"))
                .clicked()
            {
                reset = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::primary_button(ui, tl!("OK")).clicked() {
                    ok = true;
                }
                ui.add_space(8.0);
                if widgets::secondary_button(ui, tl!("Cancel")).clicked() {
                    cancel = true;
                }
                ui.add_space(16.0);
                let i = PREF_CATEGORIES.iter().position(|c| *c == cat).unwrap_or(0);
                if ui.add_enabled(i + 1 < PREF_CATEGORIES.len(), egui::Button::new(tl!("Next"))).clicked() {
                    d.fields.insert("__category".into(), json!(PREF_CATEGORIES[i + 1]));
                }
                if ui.add_enabled(i > 0, egui::Button::new(tl!("Previous"))).clicked() {
                    d.fields.insert("__category".into(), json!(PREF_CATEGORIES[i - 1]));
                }
            });
        });
    });
    if reset {
        for (k, v) in Prefs::default().to_json().as_object().cloned().unwrap_or_default() {
            d.fields.insert(k, v);
        }
    }
    app.ui.dialog = Some(d);
    if cancel {
        app.ui.dialog = None;
    } else if ok && let Err(e) = confirm(app) {
        app.status(e);
    }
}

fn category_fields(ui: &mut egui::Ui, d: &mut Dialog, cat: &str) {
    let t = Tokens::get(ui.ctx());
    if cat == "Pen & Wacom Buttons" {
        ui.label(tl!("Detect each pen button, choose its tool, then click OK."));
        ui.horizontal(|ui| {
            for (button, label) in [(1, tl!("Detect button 1 key")), (2, tl!("Detect button 2 key"))] {
                if widgets::secondary_button(ui, label).clicked() {
                    d.fields.insert("__penLearn".into(), json!(button));
                    d.fields.insert("__penLearnStarted".into(), json!(ui.input(|i| i.time)));
                    d.fields.insert("__penDetected".into(), json!("Press that pen button now (10 seconds)."));
                }
            }
        });
        if let Some(message) = d.fields.get("__penDetected").and_then(Value::as_str) {
            ui.label(message);
        }
        if d.fields.contains_key("__penLearn") {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        ui.collapsing(tl!("Driver setup and safety notes"), |ui| {
            ui.label("These settings use driver-sent keys; they do not reprogram the Wacom driver. Your capture contained [ and ]. F13 / F14 application-specific mappings avoid bracket shortcut conflicts.");
            ui.label("Keyboard presses of the same keys also trigger assignments. Dialogs and text editing are excluded. Hold mode needs key-down and key-up; use Press to switch for click-only mappings.");
            ui.label("Tool changes wait until a stroke finishes. If both buttons use the same key, button 1 wins. Physical eraser-tip mapping requires native Windows Ink packets.");
        });
        ui.add_space(8.0);
    }
    let mut section = "";
    let mut first = true;
    let specs: Vec<_> = PREF_SPECS.iter().filter(|s| s.category == cat).collect();
    for (key, c, label) in UI_FIELDS {
        if *c == cat && cat != "Smart Guides" {
            bool_row(ui, d, key, label);
        }
    }
    if cat == "General" {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(tl!("Launch Screen Artwork")).font(theme::semibold(12.5)).color(t.text_strong));
        ui.add_space(2.0);
        let cur_path = d.fields.get("__splashImagePath").and_then(Value::as_str).unwrap_or("").to_string();
        labeled(ui, tl!("Custom Image Path"), |ui| {
            let mut s = cur_path.clone();
            if ui.add(egui::TextEdit::singleline(&mut s).hint_text("Paste image path (PNG/JPEG) or drop file here...").desired_width(260.0)).changed()
            {
                d.fields.insert("__splashImagePath".into(), json!(s));
            }
        });
        ui.add_space(8.0);
    }
    if cat == "Smart Guides" {
        bool_row(ui, d, "__smartGuides", UI_FIELDS[0].2);
        ui.add_space(4.0);
    }
    for sp in specs {
        if sp.section != section {
            section = sp.section;
            if !section.is_empty() {
                if !first {
                    ui.add_space(8.0);
                }
                ui.label(egui::RichText::new(tl!(section)).font(theme::semibold(12.5)).color(t.text_strong));
                ui.add_space(2.0);
            }
        }
        first = false;
        let v = d.fields.get(sp.key).cloned().unwrap_or(Value::Null);
        match sp.kind {
            // Units › Numbers Without Units Are Points only tells points from picas: dimmed
            // unless a unit is Picas.
            PrefKind::Bool if sp.key == "numbersWithoutUnitsArePoints" => {
                ui.add_enabled_ui(picas_in_use(d), |ui| bool_row(ui, d, sp.key, sp.label));
            }
            // Performance › Animated Zoom needs GPU Performance: dimmed while it is off.
            PrefKind::Bool if sp.key == "animatedZoom" => {
                ui.add_enabled_ui(d.bool("gpuPerformance"), |ui| bool_row(ui, d, sp.key, sp.label));
            }
            PrefKind::Bool => bool_row(ui, d, sp.key, sp.label),
            PrefKind::Num { min, max, unit } => {
                labeled(ui, sp.label, |ui| {
                    let mut x = v.as_f64().unwrap_or(min);
                    let new = if sp.key == "uiScaling" {
                        ui.add(egui::Slider::new(&mut x, min..=max).step_by(0.05).text(tl!("Smaller ↔ Larger"))).changed().then_some(x)
                    } else if matches!(sp.key, "raineeDarkness" | "uiGlassIntensity") {
                        ui.add(egui::Slider::new(&mut x, min..=max).suffix("%").step_by(1.0)).changed().then_some(x)
                    } else {
                        widgets::range_field(ui, sp.key, x, min..=max, &format!(" {unit}"), 3, 110.0)
                    };
                    if let Some(x) = new {
                        d.fields.insert(sp.key.into(), json!(x));
                    }
                });
            }
            PrefKind::Length { min, max, measure } => {
                // In the dialog's own Units choice, so a change there shows right away.
                let unit = vectorcraft_doc::Unit::named(&d.str(measure.pref_key())).unwrap_or_default();
                let x = d.f64(sp.key, min);
                labeled(ui, sp.label, |ui| {
                    if let Some(x) = widgets::num_field(ui, sp.key, Some(x), unit, 110.0) {
                        d.fields.insert(sp.key.into(), json!(x.clamp(min, max)));
                    }
                });
            }
            PrefKind::Int { min, max } => {
                labeled(ui, sp.label, |ui| {
                    let mut x = v.as_i64().unwrap_or(min);
                    let new = if sp.key == "anchorSize" {
                        ui.add(egui::Slider::new(&mut x, min..=max).show_value(false).text(tl!("Size"))).changed().then_some(x)
                    } else {
                        widgets::range_field(ui, sp.key, x as f64, min as f64..=max as f64, "", 0, 110.0).map(|x| x as i64)
                    };
                    if let Some(x) = new {
                        d.fields.insert(sp.key.into(), json!(x));
                    }
                });
            }
            PrefKind::Choice(opts) => {
                labeled(ui, sp.label, |ui| {
                    let cur = v.as_str().unwrap_or("");
                    let cur_label = opts.iter().find(|o| o.0 == cur).map(|o| o.1).unwrap_or(cur);
                    let labels: Vec<&str> = opts.iter().map(|o| o.1).collect();
                    if let Some(i) = widgets::dropdown(ui, sp.key, cur_label, &labels, 260.0) {
                        d.fields.insert(sp.key.into(), json!(opts[i].0));
                    }
                });
                if sp.key == "gpuPreference" {
                    // The window's graphics device is chosen once, when the app starts.
                    ui.label(egui::RichText::new(tl!("Applies the next time VectorCraft starts.")).color(t.text_dim).size(11.0));
                }
            }
            PrefKind::Color => {
                labeled(ui, sp.label, |ui| {
                    let hex = v.as_str().unwrap_or("#000000");
                    let c = vectorcraft_color::Color::from_hex(hex).map(|c| c.to_rgba8(1.0)).unwrap_or([0, 0, 0, 255]);
                    let mut rgb = [c[0], c[1], c[2]];
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        d.fields.insert(sp.key.into(), json!(format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])));
                    }
                    ui.label(egui::RichText::new(hex).color(t.text_dim).size(11.0));
                });
            }
            PrefKind::Text if matches!(sp.key, "penButton1Tool" | "penButton2Tool" | "penEraserTool") => {
                let tools: Vec<_> = vectorcraft_tools::catalog::all_tools().collect();
                let mut labels = vec!["No assignment"];
                labels.extend(tools.iter().map(|t| t.label));
                let cur = v.as_str().unwrap_or("none");
                let label = tools.iter().find(|t| t.id == cur).map_or("No assignment", |t| t.label);
                labeled(ui, sp.label, |ui| {
                    if let Some(i) = widgets::dropdown(ui, sp.key, label, &labels, 260.0) {
                        let id = i.checked_sub(1).and_then(|j| tools.get(j)).map_or("none", |t| t.id);
                        d.fields.insert(sp.key.into(), json!(id));
                    }
                });
            }
            PrefKind::Text if sp.key == "scratchPrimary" || sp.key == "scratchSecondary" => {
                labeled(ui, sp.label, |ui| {
                    let current = v.as_str().unwrap_or("");
                    let mut choices = vec![if sp.key == "scratchPrimary" { "Startup".to_string() } else { "None".to_string() }];
                    if let Some(drives) = d.fields.get("__scratchDrives").and_then(Value::as_array) {
                        choices.extend(drives.iter().filter_map(Value::as_str).map(str::to_owned));
                    }
                    if !choices.iter().any(|drive| drive.eq_ignore_ascii_case(current)) && !current.is_empty() {
                        choices.push(current.to_string());
                    }
                    let labels: Vec<&str> = choices.iter().map(String::as_str).collect();
                    if let Some(i) = widgets::dropdown(ui, sp.key, current, &labels, 260.0) {
                        d.fields.insert(sp.key.into(), json!(choices[i]));
                    }
                });
            }
            PrefKind::Text if sp.key == "interfaceLanguage" => {
                // `auto` plus every registered language, by its own name.
                labeled(ui, sp.label, |ui| {
                    let cur = v.as_str().unwrap_or("auto");
                    let codes: Vec<&str> = std::iter::once("auto").chain(crate::i18n::Lang::all().map(|l| l.code())).collect();
                    let labels: Vec<&str> = std::iter::once(tl!("Auto")).chain(crate::i18n::Lang::all().map(|l| l.name())).collect();
                    let cur_label = codes.iter().position(|c| c.eq_ignore_ascii_case(cur)).and_then(|i| labels.get(i).copied()).unwrap_or(cur);
                    if let Some(i) = widgets::dropdown(ui, sp.key, cur_label, &labels, 260.0)
                        && let Some(code) = codes.get(i)
                    {
                        d.fields.insert(sp.key.into(), json!(code));
                    }
                });
            }
            PrefKind::Text => {
                labeled(ui, sp.label, |ui| {
                    let mut s = v.as_str().unwrap_or("").to_string();
                    if ui.add(egui::TextEdit::singleline(&mut s).desired_width(260.0)).changed() {
                        d.fields.insert(sp.key.into(), json!(s));
                    }
                });
            }
        }
    }
}

/// Detection observes driver-sent keys only; it never pretends to reprogram physical hardware.
fn detect_pen_button(d: &mut Dialog, ctx: &egui::Context) {
    let Some(button) = d.fields.get("__penLearn").and_then(Value::as_u64).filter(|b| *b == 1 || *b == 2) else { return };
    let start = d.fields.get("__penLearnStarted").and_then(Value::as_f64).unwrap_or(0.0);
    let timed_out = ctx.input(|i| i.time - start > 10.0);
    let key = ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } if modifiers.is_none() => Some(*key),
            _ => None,
        })
    });
    if let Some(key) = key {
        let found = vectorcraft_engine::cmd::prefscmds::PEN_BUTTON_KEYS
            .iter()
            .find(|(name, _)| crate::shortcuts::parse(name).is_some_and(|shortcut| shortcut.logical_key == key));
        let message = if let Some((name, _)) = found {
            d.fields.insert(format!("penButton{button}Key"), json!(name));
            format!("Button {button}: received {name}. Choose its tool and behavior below, then click OK.")
        } else {
            format!("Received {}. Use an unmodified [, ], or F13–F16 driver mapping for this panel.", key.name())
        };
        d.fields.insert("__penDetected".into(), json!(message));
        d.fields.remove("__penLearn");
        ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { key:k, .. } if *k == key)));
    } else if timed_out {
        d.fields.remove("__penLearn");
        d.fields.insert(
            "__penDetected".into(),
            json!("No supported key received. Map the pen button to [, ], or F13/F14 in your Wacom driver and retry."),
        );
    }
}

/// Is one of the dialog's Units (General, Stroke, Type, East Asian Type) Picas?
fn picas_in_use(d: &Dialog) -> bool {
    ["unitsGeneral", "unitsStroke", "unitsType", "unitsAsianType"].iter().any(|k| d.str(k) == vectorcraft_doc::Unit::Picas.key())
}

fn bool_row(ui: &mut egui::Ui, d: &mut Dialog, key: &str, label: &str) {
    let mut b = d.bool(key);
    let response =
        if crate::theme::is_rainee(ui.ctx()) { crate::widgets::toggle_switch(ui, tl!(label), &mut b) } else { ui.checkbox(&mut b, tl!(label)) };
    if response.changed() {
        d.fields.insert(key.into(), json!(b));
    }
}

fn labeled(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(210.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(210.0);
            ui.add(egui::Label::new(egui::RichText::new(format!("{}{}", tl!(label), tl!(":"))).color(t.text)).truncate());
        });
        add(ui);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Services;
    use vectorcraft_engine::Session;

    #[test]
    fn scratch_choices_are_available_and_persisted() {
        let mut app = VectorcraftApp::new(Session::new(), Services::default());
        open(&mut app, Some("Performance & Storage"));
        let drives = app.ui.dialog.as_ref().unwrap().fields["__scratchDrives"].as_array().unwrap();
        #[cfg(windows)]
        assert!(drives.iter().any(|d| d.as_str().is_some_and(|s| s.ends_with(":\\"))));
        let choice = drives.first().and_then(Value::as_str).unwrap_or("Startup").to_string();
        app.ui.dialog.as_mut().unwrap().fields.insert("scratchPrimary".into(), json!(choice));
        confirm(&mut app).unwrap();
        assert_eq!(app.session.prefs.scratch_primary, choice);
    }

    /// User Interface › Canvas Color (#663): White and the three greys paint the canvas around the
    /// artboards; Match User Interface Brightness keeps the theme's.
    #[test]
    fn canvas_color_choices_paint_the_pasteboard() {
        let mut app = VectorcraftApp::new(Session::new(), Services::default());
        let ctx = egui::Context::default();
        apply_runtime(&mut app, &ctx);
        let themed = Tokens::get(&ctx).pasteboard;
        for (key, want) in [
            ("white", egui::Color32::WHITE),
            ("lightGray", egui::Color32::from_gray(0xc8)),
            ("mediumGray", egui::Color32::from_gray(0x96)),
            ("darkGray", egui::Color32::from_gray(0x5a)),
            ("matchUi", themed),
        ] {
            app.run("prefs.set", serde_json::json!({"key": "canvasColor", "value": key})).unwrap();
            apply_runtime(&mut app, &ctx);
            assert_eq!(Tokens::get(&ctx).pasteboard, want, "{key}");
        }
        assert!(app.run("prefs.set", serde_json::json!({"key": "canvasColor", "value": "pink"})).is_err());
    }

    fn app() -> VectorcraftApp {
        VectorcraftApp::new(Session::new(), Services::default())
    }

    #[test]
    fn dialog_ok_applies_prefs_and_ui_fields() {
        let mut a = app();
        open(&mut a, Some("general"));
        let d = a.ui.dialog.as_mut().unwrap();
        assert_eq!(d.str("__category"), "General");
        d.fields.insert("keyboardIncrement".into(), json!(4));
        d.fields.insert("uiBrightness".into(), json!("light"));
        d.fields.insert("__smartGuides".into(), json!(false));
        confirm(&mut a).unwrap();
        assert!(a.ui.dialog.is_none());
        assert_eq!(a.session.prefs.keyboard_increment, 4.0);
        assert_eq!(a.session.prefs.ui_brightness, "light");
        assert!(!a.ui.view.smart_guides);
        assert_eq!(a.ui.engine_prefs["keyboardIncrement"], json!(4.0));
    }

    /// General › Show Tool Tips: off, a button held under the pointer shows no tool tip; back on,
    /// it does again (#394).
    #[test]
    fn show_tool_tips_off_hides_tool_tips() {
        fn texts(s: &egui::Shape, out: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
                _ => {}
            }
        }
        let mut a = app();
        let ctx = egui::Context::default();
        let mut time = 0.0;
        // Come from elsewhere and hover the button for two seconds: did its tool tip show?
        let mut hover = |a: &mut VectorcraftApp| {
            let mut shown = vec![];
            for i in 0..20 {
                time += 0.1;
                let events = match i {
                    0 => vec![egui::Event::PointerGone],
                    1 => vec![egui::Event::PointerMoved(egui::pos2(20.0, 12.0))],
                    _ => vec![],
                };
                let raw = egui::RawInput {
                    events,
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    apply_runtime(a, ui.ctx());
                    ui.button("Button").on_hover_text("The tool tip");
                });
                out.textures_delta.clear();
                out.shapes.iter().for_each(|c| texts(&c.shape, &mut shown));
            }
            shown.iter().any(|t| t == "The tool tip")
        };
        assert!(hover(&mut a), "on by default");
        a.run("prefs.set", json!({"key": "showToolTips", "value": false})).unwrap();
        assert!(!hover(&mut a), "off: no tool tip");
        a.run("prefs.set", json!({"key": "showToolTips", "value": true})).unwrap();
        assert!(hover(&mut a), "on again");
    }

    #[test]
    fn dialog_ok_with_invalid_value_keeps_dialog_and_prefs() {
        let mut a = app();
        open(&mut a, None);
        a.ui.dialog.as_mut().unwrap().fields.insert("anchorSize".into(), json!(42));
        assert!(confirm(&mut a).is_err());
        assert!(a.ui.dialog.is_some());
        assert_eq!(a.session.prefs, Prefs::default());
    }

    #[test]
    fn prefs_persist_through_ui_state() {
        let mut a = app();
        a.run("prefs.set", json!({"key": "gridlineEvery", "value": 50})).unwrap();
        snapshot(&mut a);
        let saved = serde_json::to_vec(&a.ui).unwrap();
        let mut b = app();
        b.ui = serde_json::from_slice::<crate::UiState>(&saved).unwrap().sanitized();
        restore(&mut b);
        assert_eq!(b.session.prefs.gridline_every, 50.0);
    }

    #[test]
    fn old_pref_files_keep_their_brightness() {
        let mut a = app();
        a.ui.brightness = Brightness::Light;
        a.ui.engine_prefs = Value::Null;
        restore(&mut a);
        assert_eq!(a.session.prefs.ui_brightness, "light");
    }

    #[test]
    fn rainee_appearance_preview_cancel_and_persistence() {
        let mut a = app();
        a.run("window.workspace", json!({"name":"Rainee 2.0"})).unwrap();
        let ctx = egui::Context::default();
        apply_runtime(&mut a, &ctx);
        let before = Tokens::get(&ctx).panel;
        open(&mut a, Some("User Interface"));
        a.ui.dialog.as_mut().unwrap().fields.insert("raineeDarkness".into(), json!(100.0));
        apply_runtime(&mut a, &ctx);
        assert_ne!(Tokens::get(&ctx).panel, before);
        assert_eq!(a.session.prefs.rainee_darkness, 65.0);
        a.ui.dialog = None;
        apply_runtime(&mut a, &ctx);
        assert_eq!(Tokens::get(&ctx).panel, before);
        a.run("prefs.set", json!({"values":{"raineeDarkness":80.0,"uiGlassIntensity":75.0,"uiReduceTransparency":true}})).unwrap();
        snapshot(&mut a);
        let mut b = app();
        b.ui.engine_prefs = a.ui.engine_prefs.clone();
        restore(&mut b);
        assert_eq!(b.session.prefs.rainee_darkness, 80.0);
        assert_eq!(b.session.prefs.ui_glass_intensity, 75.0);
        assert!(b.session.prefs.ui_reduce_transparency);
    }

    /// Performance › Graphics Processor (#306, #502): shown with its restart note, applied by OK
    /// and saved with the UI state under the key the desktop app reads before the window opens.
    #[test]
    fn graphics_processor_preference_shows_and_persists() {
        fn texts(s: &egui::Shape, out: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
                _ => {}
            }
        }
        let mut a = app();
        open(&mut a, Some("Performance"));
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        // The window sizes itself on the first frame and draws on the second.
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| show(&mut a, ui.ctx()));
        out.textures_delta.clear();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| show(&mut a, ui.ctx()));
        out.textures_delta.clear();
        let mut shown = vec![];
        out.shapes.iter().for_each(|c| texts(&c.shape, &mut shown));
        assert!(shown.iter().any(|t| t.starts_with("Graphics Processor")), "{shown:?}");
        assert!(shown.iter().any(|t| t == "Automatic"), "{shown:?}");
        assert!(shown.iter().any(|t| t == "Applies the next time VectorCraft starts."), "{shown:?}");
        a.ui.dialog.as_mut().unwrap().fields.insert("gpuPreference".into(), json!("highPerformance"));
        confirm(&mut a).unwrap();
        assert_eq!(a.session.prefs.gpu_preference, "highPerformance");
        let saved: Value = serde_json::from_slice(&serde_json::to_vec(&a.ui).unwrap()).unwrap();
        assert_eq!(saved["engine_prefs"]["gpuPreference"], json!("highPerformance"));
    }

    /// Units › Numbers Without Units Are Points (#394): the preference reaches the fields when it
    /// changes, and its checkbox is dimmed unless a unit is Picas.
    #[test]
    fn numbers_without_units_are_points_reaches_the_fields_and_dims_without_picas() {
        let mut a = app();
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let frame = |a: &mut VectorcraftApp| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                apply_runtime(a, ui.ctx());
                show(a, ui.ctx());
            });
            out.textures_delta.clear();
        };
        let read = |ctx: &egui::Context| widgets::typed_unit(ctx, vectorcraft_doc::Unit::Picas);
        frame(&mut a);
        assert_eq!(read(&ctx), vectorcraft_doc::Unit::Points, "on by default");
        a.run("prefs.set", json!({"key": "numbersWithoutUnitsArePoints", "value": false})).unwrap();
        frame(&mut a);
        assert_eq!(read(&ctx), vectorcraft_doc::Unit::Picas, "off");
        // The checkbox: dimmed with every unit in points, enabled once a unit is Picas.
        open(&mut a, Some("Units"));
        let d = a.ui.dialog.as_mut().unwrap();
        assert!(!picas_in_use(d), "dimmed in points");
        d.fields.insert("unitsStroke".into(), json!("picas"));
        assert!(picas_in_use(d), "enabled with a unit in picas");
    }

    /// 0.5.0 saved `powerSaving` for everyone (its default): it reads as Automatic, while the
    /// choices this version offers are kept (#502).
    #[test]
    fn a_graphics_processor_this_version_does_not_offer_reads_as_automatic() {
        for (saved, read) in [("powerSaving", "automatic"), ("turbo", "automatic"), ("lowPower", "lowPower"), ("highPerformance", "highPerformance")]
        {
            let mut a = app();
            a.ui.engine_prefs = json!({"gpuPreference": saved});
            restore(&mut a);
            assert_eq!(a.session.prefs.gpu_preference, read, "{saved}");
        }
    }

    #[test]
    fn every_category_renders_fields() {
        for c in PREF_CATEGORIES {
            assert!(PREF_SPECS.iter().any(|s| s.category == *c));
        }
    }

    #[test]
    fn pen_button_key_detection_is_dialog_local_and_times_out() {
        let ctx = egui::Context::default();
        let mut a = app();
        open(&mut a, Some("Pen & Wacom Buttons"));
        let d = a.ui.dialog.as_mut().unwrap();
        d.fields.insert("__penLearn".into(), json!(1));
        d.fields.insert("__penLearnStarted".into(), json!(0.0));
        let event = egui::Event::Key { key: egui::Key::F13, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
        ctx.run_ui(egui::RawInput { time: Some(1.0), events: vec![event], ..Default::default() }, |ui| {
            detect_pen_button(a.ui.dialog.as_mut().unwrap(), ui.ctx());
        })
        .textures_delta
        .clear();
        let d = a.ui.dialog.as_mut().unwrap();
        assert_eq!(d.str("penButton1Key"), "F13");
        assert!(!d.fields.contains_key("__penLearn"));
        assert_eq!(a.session.prefs.pen_button1_key, "[", "changes are applied only on OK");
        let d = a.ui.dialog.as_mut().unwrap();
        d.fields.insert("__penLearn".into(), json!(2));
        d.fields.insert("__penLearnStarted".into(), json!(1.0));
        ctx.run_ui(egui::RawInput { time: Some(12.0), ..Default::default() }, |ui| {
            detect_pen_button(a.ui.dialog.as_mut().unwrap(), ui.ctx());
        })
        .textures_delta
        .clear();
        assert!(!a.ui.dialog.as_ref().unwrap().fields.contains_key("__penLearn"));
        assert!(a.ui.dialog.as_ref().unwrap().str("__penDetected").contains("No supported key"));
    }

    #[test]
    fn pen_button_category_renders_tool_choices_and_applies_on_ok() {
        fn texts(s: &egui::Shape, out: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut a = app();
        open(&mut a, Some("Pen & Wacom Buttons"));
        let mut shown = vec![];
        for _ in 0..3 {
            let input =
                egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400., 1000.))), ..Default::default() };
            let mut output = ctx.run_ui(input, |ui| show(&mut a, ui.ctx()));
            output.shapes.iter().for_each(|s| texts(&s.shape, &mut shown));
            output.textures_delta.clear();
        }
        assert!(shown.iter().any(|t| t == "Detect button 1 key"), "{shown:?}");
        assert!(shown.iter().any(|t| t == "Eraser Tool"), "{shown:?}");
        let d = a.ui.dialog.as_mut().unwrap();
        d.fields.insert("penButtonsEnabled".into(), json!(true));
        d.fields.insert("penButton1Mode".into(), json!("switch"));
        confirm(&mut a).unwrap();
        assert!(a.session.prefs.pen_buttons_enabled);
        assert_eq!(a.ui.engine_prefs["penButton1Mode"], json!("switch"));
    }
}
