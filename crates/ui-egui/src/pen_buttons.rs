//! Driver-key pen assignments. A physical button and a keyboard key are indistinguishable
//! here: mappings are explicitly opt-in and never alter the Wacom driver's settings.
use crate::VectorcraftApp;
use egui::{Event, Key};
use serde_json::json;

#[derive(Default)]
pub(crate) struct State {
    held: Vec<(Key, String)>,
    base: Option<(u64, String)>,
    applied: Option<String>,
    pending_switch: Option<(u64, String)>,
}

pub(crate) fn handle(app: &mut VectorcraftApp, ctx: &egui::Context) {
    let mut state = std::mem::take(&mut app.pen_buttons);
    let uid = app.session.active().map(|d| d.uid);
    if state.base.as_ref().is_some_and(|(id, _)| Some(*id) != uid) || state.applied.as_ref().is_some_and(|tool| tool != app.session.tool_id()) {
        // Manual selection and document switching take precedence over temporary tools.
        state = State::default();
    }
    let lost_focus = ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::WindowFocused(false))));
    let allowed = app.session.prefs.pen_buttons_enabled
        && !lost_focus
        && app.ui.dialog.is_none()
        && !app.ui.palette_open
        && !ctx.egui_wants_keyboard_input()
        && !app.session.tool_wants_text();
    if !allowed {
        state.held.clear();
        state.pending_switch = None;
        if lost_focus && state.base.is_some() && app.session.tool_busy() {
            let result = app.session.tool_key(vectorcraft_tools::ToolKey::Escape, Default::default(), app.view_info());
            crate::canvas::apply_requests(app, result);
        }
    }
    let p = &app.session.prefs;
    let bindings = [
        (p.pen_button1_key.clone(), p.pen_button1_tool.clone(), p.pen_button1_mode.clone()),
        (p.pen_button2_key.clone(), p.pen_button2_tool.clone(), p.pen_button2_mode.clone()),
    ];
    let mut consumed = Vec::new();
    for event in ctx.input(|i| i.events.clone()) {
        let Event::Key { key, pressed, repeat, modifiers, .. } = event else { continue };
        if !pressed {
            if state.held.iter().any(|(k, _)| *k == key) {
                state.held.retain(|(k, _)| *k != key);
                consumed.push(key);
            }
            continue;
        }
        if !allowed || !modifiers.is_none() {
            continue;
        }
        let Some((_, target, mode)) =
            bindings.iter().find(|(name, tool, _)| tool != "none" && crate::shortcuts::parse(name).is_some_and(|s| s.logical_key == key))
        else {
            continue;
        };
        consumed.push(key);
        if repeat || state.held.iter().any(|(k, _)| *k == key) {
            continue;
        }
        let Some(id) = uid else { continue };
        if mode == "hold" {
            if state.base.is_none() {
                state.base = Some((id, app.session.tool_id().into()));
            }
            state.held.push((key, target.clone()));
        } else {
            state.held.clear();
            state.base = None;
            state.applied = None;
            state.pending_switch = Some((id, target.clone()));
        }
    }
    ctx.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Key { key, .. } if consumed.contains(key))));
    // Selecting a tool deactivates its gesture: never do it in the middle of a stroke.
    if !app.session.tool_busy() {
        if let Some((id, target)) = state.pending_switch.take() {
            if Some(id) == uid {
                select(app, &target);
            }
        } else if let Some((_, target)) = state.held.last() {
            if select(app, target) {
                state.applied = Some(target.clone());
            }
        } else if let Some((id, previous)) = state.base.take() {
            if Some(id) == uid && state.applied.as_ref().is_some_and(|t| t == app.session.tool_id()) {
                select(app, &previous);
            }
            state.applied = None;
        }
    }
    app.pen_buttons = state;
}

fn select(app: &mut VectorcraftApp, target: &str) -> bool {
    if app.session.tool_id() == target {
        return true;
    }
    match app.run("tool.select", json!({"tool":target})) {
        Ok(_) => true,
        Err(e) => {
            app.status(e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> VectorcraftApp {
        let mut a = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        a.run("file.new", json!({"width":400,"height":400})).unwrap();
        a.select_tool("paintbrush");
        a.run("prefs.set", json!({"key":"penButtonsEnabled","value":true})).unwrap();
        a
    }
    fn key(key: Key, pressed: bool) -> Event {
        Event::Key { key, pressed, repeat: false, physical_key: None, modifiers: Default::default() }
    }
    fn frame(a: &mut VectorcraftApp, c: &egui::Context, events: Vec<Event>) {
        c.run_ui(egui::RawInput { events, ..Default::default() }, |ui| crate::shortcuts::handle(a, ui.ctx())).textures_delta.clear();
    }
    #[test]
    fn hold_eraser_restores_and_consumes_brush_size_shortcut() {
        let mut a = app();
        let c = egui::Context::default();
        frame(&mut a, &c, vec![key(Key::OpenBracket, true)]);
        assert_eq!(a.session.tool_id(), "eraser");
        frame(&mut a, &c, vec![key(Key::OpenBracket, false)]);
        assert_eq!(a.session.tool_id(), "paintbrush");
        assert!(a.session.active().unwrap().history.undo.is_empty());
    }

    #[test]
    #[ignore = "requires VECTORCRAFT_CINTIQ_FIXTURES pointing to local owner recordings"]
    fn recorded_cintiq_button_keys_replay_without_art_changes() {
        let root = std::path::PathBuf::from(std::env::var("VECTORCRAFT_CINTIQ_FIXTURES").unwrap());
        let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("cintiq-2026-10-11-session-2.json")).unwrap()).unwrap();
        let mut a = app();
        a.session.prefs.pen_button1_key = "[".into();
        a.session.prefs.pen_button1_tool = "eraser".into();
        a.session.prefs.pen_button1_mode = "hold".into();
        a.session.prefs.pen_button2_key = "]".into();
        a.session.prefs.pen_button2_tool = "pencil".into();
        a.session.prefs.pen_button2_mode = "hold".into();
        let before = vectorcraft_format::save(&a.session.active().unwrap().doc, false);
        let c = egui::Context::default();
        let mut held = Vec::new();
        let mut downs = [0, 0];
        for auxiliary in fixture["auxiliary"].as_array().unwrap() {
            let (k, target, index) = match auxiliary["code"].as_u64() {
                Some(219) => (Key::OpenBracket, "eraser", 0),
                Some(221) => (Key::CloseBracket, "pencil", 1),
                _ => continue,
            };
            let pressed = match auxiliary["message"].as_u64() {
                Some(256) => true,
                Some(257) => false,
                _ => continue,
            };
            if pressed {
                downs[index] += 1;
                held.push((k, target));
            } else {
                held.retain(|(key, _)| *key != k);
            }
            frame(&mut a, &c, vec![key(k, pressed)]);
            assert_eq!(a.session.tool_id(), held.last().map_or("paintbrush", |(_, t)| *t));
        }
        assert_eq!(downs, [6, 16]);
        assert!(held.is_empty());
        assert_eq!(vectorcraft_format::save(&a.session.active().unwrap().doc, false), before);
        println!("CINTIQ_REPLAY buttons bracket_downs={} close_bracket_downs={} artwork_unchanged=true", downs[0], downs[1]);
    }
    #[test]
    fn switch_persists_on_release_and_roundtrips_preferences() {
        let mut a = app();
        let c = egui::Context::default();
        a.run("prefs.set", json!({"values":{"penButton2Tool":"pencil","penButton2Key":"F14"}})).unwrap();
        frame(&mut a, &c, vec![key(Key::F14, true)]);
        frame(&mut a, &c, vec![key(Key::F14, false)]);
        assert_eq!(a.session.tool_id(), "pencil");
        crate::prefs_dialog::snapshot(&mut a);
        let ui = serde_json::from_slice(&serde_json::to_vec(&a.ui).unwrap()).unwrap();
        let mut b = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        b.ui = ui;
        crate::prefs_dialog::restore(&mut b);
        assert_eq!(b.session.prefs.pen_button2_tool, "pencil");
    }
    #[test]
    fn disabled_dialog_and_typing_do_not_switch_tools() {
        let mut a = app();
        let c = egui::Context::default();
        a.session.prefs.pen_buttons_enabled = false;
        frame(&mut a, &c, vec![key(Key::OpenBracket, true)]);
        assert_eq!(a.session.tool_id(), "paintbrush");
        a.session.prefs.pen_buttons_enabled = true;
        crate::prefs_dialog::open(&mut a, Some("Pen & Wacom Buttons"));
        frame(&mut a, &c, vec![key(Key::OpenBracket, true)]);
        assert_eq!(a.session.tool_id(), "paintbrush");
        a.ui.dialog = None;
        a.select_tool("type");
        frame(&mut a, &c, vec![key(Key::OpenBracket, true)]);
        assert_eq!(a.session.tool_id(), "type");
    }
    #[test]
    fn focus_loss_and_manual_tool_choice_cannot_leave_stuck_eraser() {
        let mut a = app();
        let c = egui::Context::default();
        frame(&mut a, &c, vec![key(Key::OpenBracket, true)]);
        frame(&mut a, &c, vec![Event::WindowFocused(false)]);
        assert_eq!(a.session.tool_id(), "paintbrush");
        frame(&mut a, &c, vec![Event::WindowFocused(true), key(Key::OpenBracket, true)]);
        a.select_tool("pencil");
        frame(&mut a, &c, vec![key(Key::OpenBracket, false)]);
        assert_eq!(a.session.tool_id(), "pencil");
    }
    #[test]
    fn release_during_stroke_waits_for_commit_before_restoring() {
        let mut a = app();
        let c = egui::Context::default();
        a.session.prefs.pen_button1_tool = "pencil".into();
        frame(&mut a, &c, vec![key(Key::OpenBracket, true)]);
        let event = |kind, x| vectorcraft_tools::PointerEvent {
            kind,
            pos: vectorcraft_geom::Point::new(x, 100.),
            pressure: 1.,
            time: None,
            prediction_input: None,
            tilt: None,
            mods: Default::default(),
        };
        a.session.pointer(&event(vectorcraft_tools::PointerKind::Down, 100.), Default::default()).unwrap();
        frame(&mut a, &c, vec![key(Key::OpenBracket, false)]);
        assert_eq!(a.session.tool_id(), "pencil");
        a.session.pointer(&event(vectorcraft_tools::PointerKind::Drag, 180.), Default::default()).unwrap();
        a.session.pointer(&event(vectorcraft_tools::PointerKind::Up, 180.), Default::default()).unwrap();
        frame(&mut a, &c, vec![]);
        assert_eq!(a.session.tool_id(), "paintbrush");
        assert_eq!(a.session.active().unwrap().history.undo.len(), 1);
    }
}
