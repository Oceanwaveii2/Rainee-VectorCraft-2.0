//! Packet-by-packet drawing: retain native channels, avoid the frame-level pen/mouse duplicate.
use super::{VectorcraftApp, Xf};
use std::collections::VecDeque;
use std::time::Duration;
use vectorcraft_tools::{
    Mods, PointerEvent, PointerKind, ToolKey,
    pen_input::{PenSample, Phase, RawPenPacket},
};
use web_time::Instant;

const LIMIT: usize = 8192;
#[derive(Default)]
pub struct Input {
    queue: VecDeque<(RawPenPacket, PenSample, Instant)>,
    pub last: Option<(RawPenPacket, PenSample)>,
    pub received: u64,
    pub dropped: u64,
    /// Native adapter diagnostics, not synthetic pointer events or persisted document data.
    pub capture_status: Option<serde_json::Value>,
    active: Option<(u32, u64)>,
    interrupted: bool,
    restore_tool: Option<(String, String)>,
    hover: Option<(vectorcraft_geom::Point, Option<[f64; 2]>, Instant)>,
    hover_document: Option<u64>,
    frames: u64,
    timed_packets: u64,
    queue_ms_sum: f64,
    queue_ms_max: f64,
    dispatch_ms: f64,
}
impl Input {
    pub fn push(&mut self, raw: RawPenPacket, sample: PenSample) {
        self.received += 1;
        if self.queue.len() >= LIMIT {
            self.dropped += self.queue.len() as u64;
            self.queue.clear();
            self.interrupted = true;
        }
        self.queue.push_back((raw, sample, Instant::now()));
    }
    pub fn interrupt(&mut self) {
        self.interrupted = true;
        self.hover = None;
    }
    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({"received":self.received,"dropped":self.dropped,"pending":self.queue.len(),
            "active":self.active.is_some(),"last":self.last,"capture":self.capture_status,
            "barrelRotation":"unavailable unless actual pen capability is confirmed",
            "hoverAvailable":self.hover.is_some_and(|(_,_,at)| at.elapsed()<Duration::from_millis(100)),
            "timing":{"scope":"UI queue residence and dispatch CPU time, not input-to-photon",
                "frames":self.frames,"packets":self.timed_packets,"queueMeanMs":self.queue_ms_sum/self.timed_packets.max(1) as f64,
                "queueMaxMs":self.queue_ms_max,"lastDispatchMs":self.dispatch_ms}})
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_frame(
    app: &mut VectorcraftApp,
    ui: &egui::Ui,
    resp: &egui::Response,
    rect: egui::Rect,
    xf: Xf,
    view: vectorcraft_engine::ViewInfo,
    modifiers: Mods,
) -> bool {
    let began = Instant::now();
    let was_active = app.native_pen.active.is_some();
    let mut packets = std::mem::take(&mut app.native_pen.queue);
    let interrupted = std::mem::take(&mut app.native_pen.interrupted);
    let uid = app.session.active().map(|d| d.uid);
    if interrupted || app.native_pen.active.is_some_and(|(_, id)| Some(id) != uid) {
        app.native_pen.hover = None;
        packets.clear();
        if app.native_pen.active.is_some_and(|(_, id)| Some(id) == uid) {
            let r = app.session.tool_key(ToolKey::Escape, modifiers, view);
            super::apply_requests(app, r);
        }
        app.native_pen.active = None;
        restore(app);
    }
    let mut handled = was_active;
    let mut events = Vec::new();
    for (raw, sample, queued) in packets {
        let wait = queued.elapsed().as_secs_f64() * 1000.0;
        app.native_pen.timed_packets = app.native_pen.timed_packets.saturating_add(1);
        app.native_pen.queue_ms_sum += wait;
        app.native_pen.queue_ms_max = app.native_pen.queue_ms_max.max(wait);
        app.native_pen.last = Some((raw.clone(), sample.clone()));
        let Some([x, y]) = raw.client_px else { continue };
        let ppp = ui.ctx().pixels_per_point();
        if !ppp.is_finite() || ppp <= 0.0 {
            continue;
        }
        let screen = egui::pos2(x as f32 / ppp, y as f32 / ppp);
        if sample.phase == Phase::Hover && app.native_pen.active.is_none() {
            app.native_pen.hover = if rect.contains(screen) && ui.ctx().layer_id_at(screen) == Some(ui.layer_id()) {
                let tilt = match sample.tilt_degrees {
                    [Some(x), Some(y)] => Some(document_tilt([x as f64, y as f64], xf.rot)),
                    _ => None,
                };
                let pos = xf.to_doc(screen);
                app.hover_doc = Some(pos);
                app.native_pen.hover_document = uid;
                Some((pos, tilt, Instant::now()))
            } else {
                None
            };
            ui.ctx().request_repaint_after(Duration::from_millis(100));
            continue;
        }
        app.native_pen.hover = None;
        let active = app.native_pen.active.is_some_and(|(id, _)| id == raw.pointer_id);
        if sample.phase == Phase::Cancel {
            if active {
                events.clear();
                let r = app.session.tool_key(ToolKey::Escape, modifiers, view);
                super::apply_requests(app, r);
                app.native_pen.active = None;
                restore(app);
                handled = true;
            }
            continue;
        }
        // Navigation/selection keep the established UI path; high-rate drawing owns its native
        // contact exclusively. A second pointer must not finish or append to the first stroke.
        if sample.phase == Phase::Down
            && app.native_pen.active.is_none()
            && rect.contains(screen)
            && (resp.hovered() || ui.ctx().layer_id_at(screen) == Some(ui.layer_id()))
            && !modifiers.space
            && (matches!(app.session.tool_id(), "pencil" | "paintbrush" | "blobBrush" | "eraser" | "smooth" | "pathEraser" | "knife" | "join")
                || ((sample.eraser || sample.inverted) && app.session.prefs.pen_eraser_tool != "none"))
        {
            if (sample.eraser || sample.inverted) && app.session.prefs.pen_eraser_tool != "none" {
                let old = app.session.tool_id().to_string();
                let tool = app.session.prefs.pen_eraser_tool.clone();
                if app.run("tool.select", serde_json::json!({"tool":tool})).is_ok() {
                    app.native_pen.restore_tool = Some((old, tool));
                }
            }
            app.native_pen.active = uid.map(|id| (raw.pointer_id, id));
            ui.ctx().memory_mut(|m| m.stop_text_input());
        } else if !active {
            continue;
        }
        if app.native_pen.active.is_none() {
            continue;
        }
        let kind = match sample.phase {
            Phase::Down => PointerKind::Down,
            Phase::Drag => PointerKind::Drag,
            Phase::Up => PointerKind::Up,
            _ => continue,
        };
        let ev = PointerEvent {
            kind,
            pos: xf.to_doc(screen),
            mods: modifiers,
            pressure: sample.pressure.unwrap_or(1.0),
            time: sample.time_seconds,
            prediction_input: if !sample.eraser && !sample.inverted {
                sample.time_seconds.zip(sample.pressure).and_then(|(time, pressure)| {
                    let [Some(tx), Some(ty)] = sample.tilt_degrees else { return None };
                    Some(vectorcraft_tools::learned_prediction::Sample {
                        time,
                        pressure,
                        tilt: [tx as f64, ty as f64],
                        position: vectorcraft_geom::Point::new(raw.screen_px[0] as f64, raw.screen_px[1] as f64),
                        basis: [xf.delta_to_doc(egui::vec2(1.0 / ppp, 0.0)), xf.delta_to_doc(egui::vec2(0.0, 1.0 / ppp))],
                    })
                })
            } else {
                None
            },
            tilt: match sample.tilt_degrees {
                [Some(x), Some(y)] => Some(document_tilt([x as f64, y as f64], xf.rot)),
                _ => None,
            },
        };
        app.hover_doc = Some(ev.pos);
        if matches!(app.session.tool_id(), "pencil" | "paintbrush") {
            events.push(ev);
        } else {
            super::dispatch(app, &ev, view);
        }
        handled = true;
        if sample.phase == Phase::Up {
            flush(app, &mut events, view);
            app.native_pen.active = None;
            restore(app);
        }
    }
    flush(app, &mut events, view);
    if handled {
        if app.session.tool_wants_ticks() {
            let r = app.session.tool_tick(f64::from(ui.input(|i| i.unstable_dt).min(0.25)), view);
            super::apply_requests(app, r);
        }
        ui.ctx().request_repaint();
        // The frame-level pointer is a promoted duplicate. Do not let it create a second drag.
        ui.data_mut(|d| {
            d.remove::<super::Drag>(super::drag_id());
        });
    }
    app.native_pen.frames = app.native_pen.frames.saturating_add(1);
    app.native_pen.dispatch_ms = began.elapsed().as_secs_f64() * 1000.0;
    handled
}

pub(super) fn paint_hover(app: &VectorcraftApp, painter: &egui::Painter, xf: &Xf) {
    if app.native_pen.hover_document != app.session.active().map(|d| d.uid) {
        return;
    }
    if !matches!(app.session.tool_id(), "pencil" | "paintbrush") {
        return;
    }
    let options = app.session.tool_options();
    if options["hoverPreview"].as_bool() != Some(true) {
        return;
    }
    let Some((center, tilt, at)) = app.native_pen.hover else {
        return;
    };
    if at.elapsed() >= Duration::from_millis(100) {
        return;
    }
    let radius = app.session.paint.stroke_width.clamp(0.1, 4096.0) * 0.5;
    if !radius.is_finite() {
        return;
    }
    let roundness = options["nibRoundness"].as_f64().filter(|v| v.is_finite()).unwrap_or(0.2).clamp(0.01, 1.0);
    let (mut angle, ratio) = if options["tiltNib"].as_bool() == Some(true) {
        tilt.map_or((0.0, 1.0), |[x, y]| {
            let x = x.to_radians().tan();
            let y = y.to_radians().tan();
            (y.atan2(x), 1.0 - (1.0 - roundness) * (x.hypot(y).atan() / std::f64::consts::FRAC_PI_2))
        })
    } else {
        (0.0, 1.0)
    };
    angle += options["nibAngle"].as_f64().filter(|v| v.is_finite()).unwrap_or(0.0).to_radians();
    let points = (0..32)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / 32.0;
            let (x, y) = (radius * t.cos(), radius * ratio * t.sin());
            xf.to_screen(center + vectorcraft_geom::Vec2::new(x * angle.cos() - y * angle.sin(), x * angle.sin() + y * angle.cos()))
        })
        .collect();
    painter.add(egui::Shape::closed_line(points, egui::Stroke::new(1.0, crate::theme::Tokens::get(painter.ctx()).accent)));
}
fn flush(app: &mut VectorcraftApp, events: &mut Vec<PointerEvent>, view: vectorcraft_engine::ViewInfo) {
    if !events.is_empty() {
        let r = app.session.pointer_batch(events, view);
        super::apply_requests(app, r);
        events.clear();
    }
}
fn restore(app: &mut VectorcraftApp) {
    if let Some((tool, assigned)) = app.native_pen.restore_tool.take()
        && app.session.tool_id() == assigned
        && let Err(e) = app.run("tool.select", serde_json::json!({"tool":tool}))
    {
        app.status(e);
    }
}

/// Tilt is angular: rotate the projected shaft, not the angles themselves.
fn document_tilt(tilt: [f64; 2], view_rotation: f64) -> [f64; 2] {
    let [x, y] = tilt.map(|a| a.clamp(-89.9, 89.9).to_radians().tan());
    let (sn, cs) = view_rotation.sin_cos();
    [(x * cs + y * sn).atan().to_degrees(), (-x * sn + y * cs).atan().to_degrees()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vectorcraft_engine::{Session, ViewInfo};
    fn packet(flags: u32, x: i32, pressure: u32) -> RawPenPacket {
        RawPenPacket {
            pointer_id: 1,
            frame_id: x as u32,
            device: 1,
            screen_px: [x, 200],
            client_px: Some([x, 200]),
            window_dpi: 96,
            performance_count: x as u64,
            performance_frequency: 1000,
            system_tick_ms: x as u32,
            pointer_flags: flags,
            pen_flags: 0,
            channel_mask: 13,
            pressure,
            tilt_x: 60,
            tilt_y: 0,
            rotation: 0,
            button_change: 0,
            key_states: 0,
        }
    }
    fn frame(app: &mut VectorcraftApp, ctx: &egui::Context) {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() };
        ctx.run_ui(raw, |ui| {
            let rect = ui.max_rect();
            let response = ui.allocate_rect(rect, egui::Sense::drag());
            let xf = Xf { rect, zoom: 1.0, center: vectorcraft_geom::Point::new(400.0, 300.0), rot: 0.0 };
            dispatch_frame(app, ui, &response, rect, xf, ViewInfo::default(), Mods::default());
        })
        .textures_delta
        .clear();
    }
    fn app() -> VectorcraftApp {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.run("file.new", json!({"width":800,"height":600})).unwrap();
        app.select_tool("paintbrush");
        app.run("tool.setOption", json!({"values":{"pressureWidth":true,"tiltNib":true,"closeWithin":0}})).unwrap();
        app
    }
    #[test]
    fn native_pen_burst_without_egui_hover_draws_once_with_real_channels() {
        let mut app = app();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx);
        for (flags, x, p) in [(0x10004, 100, 256), (4, 150, 768), (4, 200, 1024), (0x40000, 200, 0)] {
            let raw = packet(flags, x, p);
            app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        }
        frame(&mut app, &ctx);
        let d = app.session.active().unwrap();
        assert_eq!(d.history.undo.len(), 1);
        let stroke = d.doc.node(d.selection.objects[0]).unwrap().appearance.stroke().unwrap();
        assert!((stroke.profile.as_ref().unwrap().at(0.0).0 - 0.325).abs() < 1e-6);
        assert!((stroke.nib.as_ref().unwrap().at(0.0).1 - 0.5).abs() < 1e-6);
        assert!(!app.native_pen.status()["active"].as_bool().unwrap());
    }

    #[test]
    fn hover_has_real_tilt_without_document_edits_and_expires() {
        let mut app = app();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx);
        let before = vectorcraft_format::save(&app.session.active().unwrap().doc, false);
        let raw = packet(0, 120, 0);
        app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        frame(&mut app, &ctx);
        let (pos, tilt, _) = app.native_pen.hover.unwrap();
        assert!((pos.x - 120.0).abs() < 1e-6);
        let tilt = tilt.unwrap();
        assert!((tilt[0] - 60.0).abs() < 1e-8 && tilt[1].abs() < 1e-8);
        assert_eq!(before, vectorcraft_format::save(&app.session.active().unwrap().doc, false));
        assert!(app.session.active().unwrap().history.undo.is_empty());
        assert_eq!(app.native_pen.status()["hoverAvailable"], true);
        app.native_pen.hover.as_mut().unwrap().2 = Instant::now() - Duration::from_millis(110);
        assert_eq!(app.native_pen.status()["hoverAvailable"], false);
        app.native_pen.interrupt();
        assert!(app.native_pen.hover.is_none());
        assert_eq!(app.native_pen.timed_packets, 1);
        assert!(app.native_pen.queue_ms_sum.is_finite() && app.native_pen.dispatch_ms.is_finite());
    }

    #[test]
    fn hover_option_is_independent_and_persists_in_existing_settings() {
        let mut app = app();
        app.select_tool("pencil");
        app.run("tool.setOption", serde_json::json!({"tool":"pencil","values":{"hoverPreview":true}})).unwrap();
        assert_eq!(app.session.tool_options()["hoverPreview"], true);
        app.select_tool("paintbrush");
        assert_eq!(app.session.tool_options()["hoverPreview"], false);
        app.select_tool("pencil");
        assert_eq!(app.session.tool_options()["hoverPreview"], true);
    }

    #[test]
    fn hover_footprint_is_painted_only_when_enabled_and_bound_to_document() {
        let mut app = app();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx);
        let raw = packet(0, 120, 0);
        app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        frame(&mut app, &ctx);
        let draw = |app: &VectorcraftApp| {
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let xf = Xf { rect: ui.max_rect(), zoom: 1.0, center: vectorcraft_geom::Point::ZERO, rot: 0.0 };
                paint_hover(app, ui.painter(), &xf);
            });
            output.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Path(p) if p.closed && p.points.len()==32))
        };
        assert!(!draw(&app));
        app.run("tool.setOption", json!({"values":{"hoverPreview":true}})).unwrap();
        assert!(draw(&app));
        app.run("file.new", json!({"width":800,"height":600})).unwrap();
        assert!(!draw(&app));
    }
    #[test]
    fn focus_loss_cancels_preview_and_drops_queued_samples() {
        let mut app = app();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx);
        for (flags, x) in [(0x10004, 100), (4, 200)] {
            let raw = packet(flags, x, 512);
            app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        }
        frame(&mut app, &ctx);
        assert!(app.session.active().unwrap().doc.art_bounds().is_some());
        let raw = packet(0x40000, 250, 0);
        app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        app.native_pen.interrupt();
        frame(&mut app, &ctx);
        assert!(app.session.active().unwrap().doc.art_bounds().is_none());
        assert!(app.session.active().unwrap().history.undo.is_empty());
    }
    #[test]
    fn tilt_azimuth_follows_view_rotation_without_inventing_roll() {
        let t = document_tilt([60.0, 0.0], std::f64::consts::FRAC_PI_2);
        assert!(t[0].abs() < 1e-8 && (t[1] + 60.0).abs() < 1e-8);
    }
    #[test]
    fn physical_eraser_uses_assigned_tool_and_restores_latched_original() {
        let mut app = app();
        let ctx = egui::Context::default();
        app.select_tool("selection");
        app.session.prefs.pen_eraser_tool = "pencil".into();
        frame(&mut app, &ctx);
        let mut raw = packet(0x10004, 100, 512);
        raw.pen_flags = 4;
        app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        frame(&mut app, &ctx);
        assert_eq!(app.session.tool_id(), "pencil");
        app.session.prefs.pen_eraser_tool = "eraser".into();
        for (flags, x) in [(4, 180), (0x40000, 180)] {
            let mut raw = packet(flags, x, 512);
            raw.pen_flags = 4;
            app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        }
        frame(&mut app, &ctx);
        assert_eq!(app.session.tool_id(), "selection");
        assert_eq!(app.session.active().unwrap().history.undo.len(), 1);
    }

    #[test]
    fn front_rear_eraser_front_changes_art_and_restores_paintbrush() {
        let mut app = app();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx);
        for (flags, x) in [(0x10004, 100), (4, 180), (4, 260), (0x40000, 260)] {
            let raw = packet(flags, x, 512);
            app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        }
        frame(&mut app, &ctx);
        assert_eq!(app.session.active().unwrap().history.undo.len(), 1);
        let before = serde_json::to_value(&app.session.active().unwrap().doc).unwrap();

        for (flags, y) in [(0x10004, 160), (4, 200), (0x40000, 240)] {
            let mut raw = packet(flags, 180, 512);
            raw.client_px = Some([180, y]);
            raw.screen_px = [180, y];
            raw.pen_flags = 6; // Real inverted + eraser flags, not a mapped keyboard shortcut.
            app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        }
        frame(&mut app, &ctx);
        assert_eq!(app.session.tool_id(), "paintbrush");
        assert_eq!(app.session.active().unwrap().history.undo.len(), 2);
        assert_ne!(serde_json::to_value(&app.session.active().unwrap().doc).unwrap(), before);

        for (flags, x) in [(0x10004, 300), (4, 350), (0x40000, 400)] {
            let raw = packet(flags, x, 512);
            app.native_pen.push(raw.clone(), raw.normalize(false).unwrap());
        }
        frame(&mut app, &ctx);
        assert_eq!(app.session.tool_id(), "paintbrush");
        assert_eq!(app.session.active().unwrap().history.undo.len(), 3);
        assert!(!app.native_pen.status()["active"].as_bool().unwrap());
    }

    #[test]
    fn adapter_diagnostics_do_not_count_as_pen_samples() {
        let mut input = Input::default();
        input.capture_status = Some(json!({"bound":true,"focused":false,"pointerMessages":0}));
        let status = input.status();
        assert_eq!(status["received"], 0);
        assert_eq!(status["capture"]["focused"], false);
        assert_eq!(status["capture"]["bound"], true);
    }

    /// Local owner recordings are not distributed fixtures or a training dataset. Replay exercises
    /// the native canvas consumer, not Windows driver delivery or physical presentation latency.
    #[test]
    #[ignore = "requires VECTORCRAFT_CINTIQ_FIXTURES pointing to the owner's local recording fixtures"]
    fn recorded_cintiq_canvas_replay_is_batch_invariant() {
        let root = std::path::PathBuf::from(std::env::var("VECTORCRAFT_CINTIQ_FIXTURES").unwrap());
        for session in [1, 2] {
            let file = root.join(format!("cintiq-2026-10-11-session-{session}.json"));
            let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
            assert_eq!(fixture["approved_for_training"], false);
            let samples = fixture["samples"].as_array().unwrap();
            let packets: Vec<RawPenPacket> = samples
                .iter()
                .map(|s| {
                    let raw: RawPenPacket = serde_json::from_value(s["raw"].clone()).unwrap();
                    let recorded: PenSample = serde_json::from_value(s["normalized"].clone()).unwrap();
                    assert_eq!(raw.normalize(false).unwrap(), recorded);
                    raw
                })
                .collect();
            assert_eq!(packets.len(), fixture["report"]["samples"].as_u64().unwrap() as usize);
            let coords: Vec<_> = packets.iter().map(|p| p.client_px.unwrap()).collect();
            let lo = [coords.iter().map(|p| p[0]).min().unwrap(), coords.iter().map(|p| p[1]).min().unwrap()];
            let hi = [coords.iter().map(|p| p[0]).max().unwrap(), coords.iter().map(|p| p[1]).max().unwrap()];
            // One uniform affine placement preserves angles and relative shape. Recorded pressure,
            // tilt, QPC, flags and device identity remain unchanged. No scenario labels are inferred.
            let scale = (700.0 / f64::from((hi[0] - lo[0]).max(1))).min(500.0 / f64::from((hi[1] - lo[1]).max(1)));
            for brush in ["Rainee Pencil", "Rainee Dry Ink"] {
                let mut baseline = None;
                for batch in [32, 512] {
                    let mut app = app();
                    app.run("brush.setCurrent", json!({"name":brush})).unwrap();
                    app.run("tool.setOption", json!({"tool":"eraser","values":{"eraseAll":true}})).unwrap();
                    let ctx = egui::Context::default();
                    frame(&mut app, &ctx);
                    for chunk in packets.chunks(batch) {
                        for original in chunk {
                            let mut raw = original.clone();
                            let p = original.client_px.unwrap();
                            let placed =
                                [50 + (f64::from(p[0] - lo[0]) * scale).round() as i32, 50 + (f64::from(p[1] - lo[1]) * scale).round() as i32];
                            raw.client_px = Some(placed);
                            raw.screen_px = placed;
                            let sample = raw.normalize(false).unwrap();
                            app.native_pen.push(raw, sample);
                        }
                        frame(&mut app, &ctx);
                    }
                    assert_eq!(app.native_pen.received, packets.len() as u64);
                    assert_eq!(app.native_pen.dropped, 0);
                    assert!(app.native_pen.active.is_none());
                    assert_eq!(app.session.tool_id(), "paintbrush");
                    let doc = &app.session.active().unwrap().doc;
                    assert!(doc.art_bounds().is_some());
                    let bytes = vectorcraft_format::save(doc, false);
                    let reopened = vectorcraft_format::load(&bytes).unwrap();
                    assert_eq!(vectorcraft_format::save(&reopened, false), bytes);
                    if batch == 32 {
                        if let Ok(output) = std::env::var("VECTORCRAFT_CINTIQ_REPLAY_OUTPUT") {
                            use std::io::Write;
                            let file = std::path::PathBuf::from(output).join(format!("session-{session}-{}.vectorcraft", brush.replace(' ', "-")));
                            std::fs::OpenOptions::new().write(true).create_new(true).open(file).unwrap().write_all(&bytes).unwrap();
                        }
                    }
                    let art = serde_json::to_value(&doc.layers).unwrap();
                    if let Some(expected) = &baseline {
                        assert_eq!(&art, expected, "session {session}, {brush}, batch {batch}");
                    } else {
                        baseline = Some(art.clone());
                    }
                    app.run("edit.undo", json!({})).unwrap();
                    app.run("edit.redo", json!({})).unwrap();
                    assert_eq!(serde_json::to_value(&app.session.active().unwrap().doc.layers).unwrap(), art);
                    println!(
                        "CINTIQ_REPLAY session={session} brush={brush} batch={batch} packets={} bytes={} undo_steps={}",
                        packets.len(),
                        bytes.len(),
                        app.session.active().unwrap().history.undo.len()
                    );
                }
            }
        }
    }
    #[test]
    fn queue_is_bounded_and_overflow_cancels_instead_of_connecting_missing_samples() {
        let raw = RawPenPacket {
            pointer_id: 1,
            frame_id: 1,
            device: 1,
            screen_px: [0, 0],
            client_px: Some([0, 0]),
            window_dpi: 96,
            performance_count: 1,
            performance_frequency: 1000,
            system_tick_ms: 1,
            pointer_flags: 4,
            pen_flags: 0,
            channel_mask: 0,
            pressure: 0,
            tilt_x: 0,
            tilt_y: 0,
            rotation: 0,
            button_change: 0,
            key_states: 0,
        };
        let mut input = Input::default();
        for _ in 0..LIMIT + 1 {
            input.push(raw.clone(), raw.normalize(false).unwrap());
        }
        assert_eq!(input.queue.len(), 1);
        assert_eq!(input.dropped, LIMIT as u64);
        assert!(input.interrupted);
    }
}
