//! Document-bound embedded editing. The host owns authorization, never the model.
use crate::VectorcraftApp;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};
struct Tool {
    created: std::time::Instant,
    id: u64,
    name: String,
    args: Value,
    reply: Sender<Value>,
}
enum Event {
    Data(Value),
    Tool(Tool),
}
#[derive(Default)]
pub struct CodexPanel {
    pub open: bool,
    pub prompt: String,
    pub model: String,
    pub reply: String,
    pub status: String,
    pending: Option<Receiver<Event>>,
    proposal: Option<Tool>,
    cancel: Arc<AtomicBool>,
    uid: Option<u64>,
    revision: u64,
    trusted: bool,
    history: VecDeque<Value>,
    operations: u64,
}
impl Drop for CodexPanel {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl CodexPanel {
    pub fn telemetry(&self) -> Value {
        json!({"state":self.status,"running":self.pending.is_some(),"document":self.uid,"revision":self.revision,"trusted":self.trusted,"completedOperations":self.operations,"reply":self.reply,"proposal":self.proposal.as_ref().map(|p|json!({"id":p.id,"arguments":p.args})),"history":self.history,"historyLimit":64})
    }
    fn record(&mut self, state: &str, details: Value) {
        if self.history.len() >= 64 {
            self.history.pop_front();
        }
        self.history.push_back(json!({"state":state,"document":self.uid,"details":details}));
    }
    fn send(&mut self, context: Value, uid: u64, revision: u64) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("A model request is running".into());
        }
        if self.prompt.trim().is_empty() || self.prompt.len() > 16000 {
            return Err("Enter a prompt up to 16000 bytes".into());
        }
        self.uid = Some(uid);
        self.revision = revision;
        self.reply.clear();
        self.proposal = None;
        self.cancel = Arc::new(AtomicBool::new(false));
        self.operations = 0;
        let (tx, rx) = mpsc::channel();
        let request = json!({"prompt":self.prompt,"model":self.model,"context":context});
        let cancel = self.cancel.clone();
        std::thread::Builder::new()
            .name("embedded-codex".into())
            .spawn(move || {
                if let Err(e) = bridge(request, cancel, tx.clone()) {
                    let _ = tx.send(Event::Data(json!({"event":"done","ok":false,"error":e})));
                }
            })
            .map_err(|e| e.to_string())?;
        self.pending = Some(rx);
        self.status = "Connecting to signed-in Codex…".into();
        self.record("requested", json!({"trusted":self.trusted}));
        Ok(())
    }
    fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(p) = self.proposal.take() {
            let _ = p.reply.send(json!({"id":p.id,"ok":false,"error":"Owner cancelled"}));
        }
        self.status = "Cancelling; completed edits remain undoable".into();
        self.record("cancel_requested", Value::Null);
    }
}
fn bridge(request: Value, cancel: Arc<AtomicBool>, tx: Sender<Event>) -> Result<(), String> {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let packaged = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("integrations/codex_bridge.py")));
    let script = packaged
        .filter(|p| p.is_file())
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/codex_bridge.py"));
    let mut cmd = Command::new(std::env::var("RAINEE_CODEX_PYTHON").unwrap_or_else(|_| "python".into()));
    cmd.arg(script);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| format!("Bridge launch: {e}"))?;
    let result = (|| {
        let mut input = child.stdin.take().ok_or("Missing bridge stdin")?;
        let output = child.stdout.take().ok_or("Missing bridge stdout")?;
        writeln!(input, "{request}").map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        let (lines, line_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut bytes = Vec::new();
                match Read::by_ref(&mut reader).take(2_000_001).read_until(b'\n', &mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if bytes.len() > 2_000_000 {
                            let _ = lines.send(Err("Bridge event exceeded budget".to_owned()));
                            break;
                        }
                        if lines.send(serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string())).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let began = Instant::now();
        let mut cancel_at = None;
        loop {
            if cancel.load(Ordering::Relaxed) && cancel_at.is_none() {
                writeln!(input, "{{\"cancel\":true}}").map_err(|e| e.to_string())?;
                input.flush().map_err(|e| e.to_string())?;
                cancel_at = Some(Instant::now());
            }
            if cancel_at.is_some_and(|t: Instant| t.elapsed() > Duration::from_secs(8)) {
                return Err("Cancelled; completed edits remain undoable".into());
            }
            if began.elapsed() > Duration::from_secs(600) {
                return Err("Model job deadline exceeded".into());
            }
            let v = match line_rx.recv_timeout(Duration::from_millis(40)) {
                Ok(v) => v?,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => return Err("Bridge disconnected".into()),
            };
            if v["event"] == "tool" {
                let id = v["id"].as_u64().ok_or("Invalid tool id")?;
                let (reply, response) = mpsc::channel();
                tx.send(Event::Tool(Tool {
                    created: Instant::now(),
                    id,
                    name: v["name"].as_str().unwrap_or_default().into(),
                    args: v["arguments"].clone(),
                    reply,
                }))
                .map_err(|e| e.to_string())?;
                let response = loop {
                    match response.recv_timeout(Duration::from_millis(40)) {
                        Ok(v) => break v,
                        Err(mpsc::RecvTimeoutError::Timeout) if !cancel.load(Ordering::Relaxed) && began.elapsed() < Duration::from_secs(600) => {
                            continue;
                        }
                        _ => break json!({"id":id,"ok":false,"error":"Cancelled or expired"}),
                    }
                };
                writeln!(input, "{response}").map_err(|e| e.to_string())?;
                input.flush().map_err(|e| e.to_string())?;
            } else {
                let done = v["event"] == "done";
                tx.send(Event::Data(v)).map_err(|e| e.to_string())?;
                if done {
                    return Ok(());
                }
            }
        }
    })();
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}
fn bound(app: &VectorcraftApp) -> Result<(), String> {
    let d = app.session.active().ok_or("No active document")?;
    if Some(d.uid) != app.codex.uid || d.revision != app.codex.revision {
        return Err("Stale document or selection; inspect again before editing".into());
    }
    if app.session.tool_busy() || d.interaction.is_some() {
        return Err("Finish the manual gesture first".into());
    }
    Ok(())
}
fn forbidden(v: &Value) -> bool {
    match v {
        Value::Object(m) => m.iter().any(|(k, v)| {
            matches!(k.to_lowercase().as_str(), "path" | "filename" | "directory" | "url" | "uri" | "file" | "commandline") || forbidden(v)
        }),
        Value::Array(a) => a.iter().any(forbidden),
        _ => false,
    }
}
fn policy(id: &str, p: &Value) -> Result<(), String> {
    if forbidden(p) {
        return Err("Filesystem/URL parameters are outside canvas permission".into());
    }
    if ["file.", "app.", "prefs.", "plugin.", "plugins.", "window.", "codex.", "library.", "print.", "link.", "links.", "template."]
        .iter()
        .any(|s| id.starts_with(s))
        || ["import", "export", "save", "load", "clipboard", "paste", "copy", "cut", "package"].iter().any(|s| id.to_lowercase().contains(s))
    {
        return Err("Files, clipboard and global settings need a separate owner workflow".into());
    }
    if vectorcraft_engine::find_command(id).is_none() && !matches!(id, "tool.select" | "tool.setOption" | "tool.options" | "tool.gesture") {
        return Err("Non-canvas UI action is not authorized".into());
    }
    Ok(())
}
fn needs_approval(id: &str) -> bool {
    id.contains("delete")
        || id.contains("remove")
        || id.contains("clear")
        || id.to_lowercase().contains("erase")
        || id == "path.knife"
        || id == "edit.undo"
        || id == "edit.redo"
}
fn inspect(app: &mut VectorcraftApp) -> Result<Value, String> {
    let uid = app.session.active().ok_or("No document")?.uid;
    if Some(uid) != app.codex.uid {
        return Err("Requested document is no longer active".into());
    }
    let data = app.run("document.inspect", json!({}))?;
    let d = app.session.active().ok_or("No document")?;
    app.codex.revision = d.revision;
    Ok(
        json!({"document":data,"uid":uid,"revision":d.revision,"tool":app.session.tool_id(),"options":app.session.tool_options(),"pen":app.native_pen.status(),"research":app.session.research_tools.status()}),
    )
}
fn execute(app: &mut VectorcraftApp, p: Tool, approve: bool) {
    let started = std::time::Instant::now();
    let before = app.session.active().map(|d| d.revision);
    let id = p.args["command"].as_str().unwrap_or_default().to_owned();
    let params = p.args.get("params").cloned().unwrap_or_else(|| json!({}));
    let result = if p.created.elapsed() > std::time::Duration::from_secs(110) || app.codex.cancel.load(Ordering::Relaxed) {
        Err("Operation expired or request cancelled".into())
    } else if !approve {
        Err("Owner rejected operation".into())
    } else {
        bound(app).and_then(|_| policy(&id, &params)).and_then(|_| app.run(&id, params))
    };
    if result.is_ok()
        && let Some(d) = app.session.active()
    {
        app.codex.revision = d.revision;
    }
    match &result {
        Ok(_) => {
            app.codex.operations += 1;
            app.codex.status = format!("Executed {id}");
        }
        Err(e) => app.codex.status = e.clone(),
    }
    app.codex.record(if result.is_ok(){"executed"}else{"rejected_or_failed"},json!({"command":id,"error":result.as_ref().err(),"elapsedMs":started.elapsed().as_secs_f64()*1000.0,"beforeRevision":before,"afterRevision":app.session.active().map(|d|d.revision)}));
    let response = match result {
        Ok(v) => json!({"id":p.id,"ok":true,"result":v}),
        Err(e) => json!({"id":p.id,"ok":false,"error":e}),
    };
    let _ = p.reply.send(response);
}
pub fn command(app: &mut VectorcraftApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    Some(match id {
        "window.codexEditor" => {
            app.codex.open = p["open"].as_bool().unwrap_or(!app.codex.open);
            Ok(json!({"open":app.codex.open}))
        }
        "tool.gesture" => gesture(app, p),
        "codex.status" => Ok(app.codex.telemetry()),
        "codex.cancel" => {
            app.codex.cancel();
            Ok(app.codex.telemetry())
        }
        "codex.approve" | "codex.reject" => {
            if let Some(r) = app.codex.proposal.take() {
                execute(app, r, id == "codex.approve");
                Ok(app.codex.telemetry())
            } else {
                Err("No pending operation".into())
            }
        }
        "codex.request" => {
            if let Some(text) = p["prompt"].as_str() {
                app.codex.prompt = text.into();
            }
            if let Some(model) = p["model"].as_str() {
                app.codex.model = model.into();
            }
            app.codex.trusted = p["trusted"].as_bool().unwrap_or(false);
            start(app).map(|_| app.codex.telemetry())
        }
        _ => return None,
    })
}
fn gesture(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    use vectorcraft_tools::{PointerEvent, PointerKind};
    let raw = p["events"].as_array().filter(|a| a.len() >= 2 && a.len() <= 4096).ok_or("Provide 2–4096 document-space samples")?;
    let mut events = Vec::with_capacity(raw.len());
    for (i, e) in raw.iter().enumerate() {
        let kind = match e["kind"].as_str() {
            Some("down") if i == 0 => PointerKind::Down,
            Some("up") if i + 1 == raw.len() => PointerKind::Up,
            Some("drag") if i > 0 && i + 1 < raw.len() => PointerKind::Drag,
            _ => return Err("Gesture must be down, optional drags, up".into()),
        };
        let x = e["x"].as_f64().filter(|v| v.is_finite() && v.abs() < 1e7).ok_or("Invalid x")?;
        let y = e["y"].as_f64().filter(|v| v.is_finite() && v.abs() < 1e7).ok_or("Invalid y")?;
        let mut event = PointerEvent::new(kind, x, y);
        event.pressure = PointerEvent::json_pressure(e);
        event.time = PointerEvent::json_time(e);
        event.tilt = PointerEvent::json_tilt(e);
        events.push(event);
    }
    if app.session.tool_busy() {
        return Err("Finish the active gesture first".into());
    }
    let view = app.view_info();
    let before = app.session.active().cloned().ok_or("No document")?;
    let journal = app.session.journal.len();
    for event in events {
        match app.session.pointer(&event, view) {
            Ok(requests) => crate::canvas::apply_requests(app, Ok(requests)),
            Err(e) => {
                let _ = app.session.tool_key(vectorcraft_tools::ToolKey::Escape, Default::default(), view);
                *app.session.doc_mut().map_err(|e| e.to_string())? = before;
                app.session.journal.truncate(journal);
                return Err(e.to_string());
            }
        }
    }
    Ok(
        json!({"completed":true,"tool":app.session.tool_id(),"document":app.session.active().map(|d|d.uid),"revision":app.session.active().map(|d|d.revision)}),
    )
}
fn start(app: &mut VectorcraftApp) -> Result<(), String> {
    let d = app.session.active().ok_or("Open a document before asking Codex")?;
    let (uid, revision) = (d.uid, d.revision);
    let context = app.run("document.inspect", json!({}))?;
    app.codex.send(context, uid, revision)
}
fn poll(app: &mut VectorcraftApp) {
    if app.codex.proposal.as_ref().is_some_and(|p| p.created.elapsed() > std::time::Duration::from_secs(110))
        && let Some(p) = app.codex.proposal.take()
    {
        execute(app, p, false);
    }
    for _ in 0..32 {
        let event = match app.codex.pending.as_ref().map(Receiver::try_recv) {
            Some(Ok(e)) => e,
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                app.codex.pending = None;
                app.codex.status = "Bridge disconnected".into();
                break;
            }
            _ => break,
        };
        match event {
            Event::Data(v) => match v["event"].as_str() {
                Some("delta") => {
                    if app.codex.reply.len() < 128000 {
                        app.codex.reply.push_str(v["text"].as_str().unwrap_or_default());
                    }
                }
                Some("status") => app.codex.status = v["text"].as_str().unwrap_or_default().into(),
                Some("done") => {
                    if let Some(text) = v["text"].as_str() {
                        app.codex.reply = text.chars().take(128000).collect();
                    }
                    app.codex.status = if v["ok"] == true {
                        "Model finished; inspect executed operations and canvas".into()
                    } else {
                        v["error"].as_str().unwrap_or("Model failed").into()
                    };
                    app.codex.record("model_finished", json!({"ok":v["ok"],"error":v["error"]}));
                    app.codex.pending = None;
                    app.codex.proposal = None;
                }
                _ => {}
            },
            Event::Tool(p) => {
                if app.codex.cancel.load(Ordering::Relaxed) {
                    let _ = p.reply.send(json!({"id":p.id,"ok":false,"error":"Request cancelled"}));
                    continue;
                }
                if p.name == "run_editor_command" {
                    let id = p.args["command"].as_str().unwrap_or_default();
                    let destructive_gesture = id == "tool.gesture" && matches!(app.session.tool_id(), "eraser" | "pathEraser" | "knife");
                    if app.codex.trusted && !needs_approval(id) && !destructive_gesture {
                        execute(app, p, true);
                    } else {
                        app.codex.status = "Awaiting owner approval".into();
                        app.codex.record("proposed", json!({"command":id}));
                        app.codex.proposal = Some(p);
                    }
                } else {
                    let result = match p.name.as_str() {
                        "inspect_editor" => inspect(app),
                        "discover_commands" => {
                            let mut data = crate::control::all_commands(app);
                            if let Some(rows) = data.as_array_mut() {
                                if let Some(filter) = p.args["filter"].as_str() {
                                    rows.retain(|row| row["id"].as_str().is_some_and(|id| id.contains(filter)));
                                }
                                for row in rows {
                                    let id = row["id"].as_str().unwrap_or_default().to_owned();
                                    row["aiPolicy"] = json!({"available":policy(&id,&json!({})).is_ok(),"requiresApproval":needs_approval(&id),"reason":policy(&id,&json!({})).err()});
                                }
                            }
                            Ok(data)
                        }
                        "inspect_canvas" => bound(app).and_then(|_| {
                            let d = app.session.active().ok_or("No document")?;
                            let rect = d.doc.artboards.get(app.view().map_or(0, |v| v.artboard)).ok_or("No active artboard")?.rect;
                            let scale = (512.0 / rect.width().max(rect.height()).max(1.0)).min(1.0);
                            let img = app.canvas.renderer.render_region(&d.doc, rect, scale, true);
                            let png = img.to_png().map_err(|e| e.to_string())?;
                            Ok(json!({"width":img.width,"height":img.height,"pngBase64":vectorcraft_format::base64_encode(&png)}))
                        }),
                        _ => Err("Unknown model tool".into()),
                    };
                    let response = match result {
                        Ok(v) => json!({"id":p.id,"ok":true,"result":v}),
                        Err(e) => json!({"id":p.id,"ok":false,"error":e}),
                    };
                    let _ = p.reply.send(response);
                }
            }
        }
    }
}
pub fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    poll(app);
    if app.codex.pending.is_none() && app.session.active().map(|d| d.uid) != app.codex.uid {
        app.codex.trusted = false;
    }
    if app.codex.pending.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(40));
    }
    if !app.codex.open {
        return;
    }
    let mut open = true;
    let (mut send, mut approve, mut reject, mut cancel) = (false, false, false, false);
    egui::Window::new("MCP UI Designer · Vector Work Edition")
        .drag_area(egui::WindowDrag::TitleBar)
        .open(&mut open)
        .default_width(510.0)
        .default_height(580.0)
        .show(ctx, |ui| {
            ui.label("Requests share the active document with signed-in Codex. Files and remote actions remain blocked.");
            ui.horizontal(|ui| {
                ui.label("Model (optional)");
                ui.text_edit_singleline(&mut app.codex.model);
            });
            ui.add(egui::TextEdit::multiline(&mut app.codex.prompt).desired_rows(3));
            ui.add_enabled_ui(app.codex.pending.is_none(), |ui| {
                ui.checkbox(&mut app.codex.trusted, "Trust ordinary canvas edits for this request/document");
            });
            ui.horizontal(|ui| {
                send = ui.add_enabled(app.codex.pending.is_none(), egui::Button::new("Send editing request")).clicked();
                cancel = ui.add_enabled(app.codex.pending.is_some(), egui::Button::new("Cancel")).clicked();
            });
            ui.label(&app.codex.status);
            if let Some(p) = &app.codex.proposal {
                ui.separator();
                ui.label(format!("Proposed: {}", p.args["command"]));
                ui.horizontal(|ui| {
                    approve = ui.button("Approve operation").clicked();
                    reject = ui.button("Reject operation").clicked();
                });
                ui.small(format!("Approval expires in {} seconds", 110_u64.saturating_sub(p.created.elapsed().as_secs())));
                egui::ScrollArea::vertical().id_salt("codex-proposal").max_height(100.0).show(ui, |ui| {
                    ui.label(p.args["params"].to_string().chars().take(4000).collect::<String>());
                });
            }
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                ui.label(&app.codex.reply);
                ui.separator();
                for row in app.codex.history.iter().rev().take(12) {
                    ui.small(row.to_string());
                }
            });
            ui.small("Cancellation stops future work; completed edits remain undoable where supported.");
        });
    app.codex.open = open;
    if send && let Err(e) = start(app) {
        app.codex.status = e;
    }
    if cancel {
        app.codex.cancel();
    }
    if (approve || reject)
        && let Some(p) = app.codex.proposal.take()
    {
        execute(app, p, approve);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permissions() {
        assert!(policy("file.save", &json!({})).is_err());
        assert!(policy("image.set", &json!({"path":"C:/secret"})).is_err());
        assert!(policy("path.create", &json!({"d":"M0 0 L10 10"})).is_ok());
        assert!(needs_approval("object.delete"));
    }
    #[test]
    fn bounded_history() {
        let mut p = CodexPanel::default();
        for _ in 0..100 {
            p.record("fixture", Value::Null);
        }
        assert_eq!(p.history.len(), 64);
    }
    fn app() -> VectorcraftApp {
        let mut s = vectorcraft_engine::Session::new();
        s.execute("file.new", &json!({"width":240,"height":160})).unwrap();
        let mut a = VectorcraftApp::new(s, Default::default());
        let d = a.session.active().unwrap();
        a.codex.uid = Some(d.uid);
        a.codex.revision = d.revision;
        a
    }
    fn operation() -> (Tool, Receiver<Value>) {
        let (tx, rx) = mpsc::channel();
        (
            Tool {
                created: std::time::Instant::now(),
                id: 1,
                name: "run_editor_command".into(),
                args: json!({"command":"path.create","params":{"d":"M20 20 L80 80"}}),
                reply: tx,
            },
            rx,
        )
    }
    #[test]
    fn approved_path_is_visible_and_undoable() {
        let mut a = app();
        let before = a.session.active().unwrap().doc.clone();
        let (p, rx) = operation();
        execute(&mut a, p, true);
        assert_eq!(rx.recv().unwrap()["ok"], true);
        assert!(!Arc::ptr_eq(&before, &a.session.active().unwrap().doc));
        a.run("edit.undo", json!({})).unwrap();
        assert!(Arc::ptr_eq(&before, &a.session.active().unwrap().doc));
    }
    #[test]
    fn rejection_and_stale_operation_do_not_edit() {
        let mut a = app();
        let before = a.session.active().unwrap().doc.clone();
        let (p, rx) = operation();
        execute(&mut a, p, false);
        assert_eq!(rx.recv().unwrap()["ok"], false);
        assert!(Arc::ptr_eq(&before, &a.session.active().unwrap().doc));
        a.run("path.create", json!({"d":"M1 1 L2 2"})).unwrap();
        let before = a.session.active().unwrap().doc.clone();
        let expected = a.codex.revision;
        let (p, rx) = operation();
        execute(&mut a, p, true);
        assert_eq!(rx.recv().unwrap()["ok"], false);
        assert_eq!(a.codex.revision, expected);
        assert!(Arc::ptr_eq(&before, &a.session.active().unwrap().doc));
    }
    #[test]
    fn cancelled_queued_tool_cannot_execute() {
        let mut a = app();
        let before = a.session.active().unwrap().doc.clone();
        let (tx, rx) = mpsc::channel();
        a.codex.pending = Some(rx);
        a.codex.trusted = true;
        a.codex.cancel.store(true, Ordering::Relaxed);
        let (p, reply) = operation();
        tx.send(Event::Tool(p)).unwrap();
        poll(&mut a);
        assert_eq!(reply.recv().unwrap()["ok"], false);
        assert!(Arc::ptr_eq(&before, &a.session.active().unwrap().doc));
    }
}
