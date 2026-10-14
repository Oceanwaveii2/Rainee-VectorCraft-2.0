//! Explicit, read-only Codex requests from a floating in-app panel.
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};

use serde_json::{Value, json};

use crate::VectorcraftApp;

#[derive(Default)]
pub struct CodexPanel {
    pub open: bool,
    pub prompt: String,
    pub model: String,
    pub reply: String,
    pub status: String,
    pending: Option<Receiver<Result<Value, String>>>,
}

impl CodexPanel {
    fn poll(&mut self) {
        if let Some(rx) = &self.pending
            && let Ok(result) = rx.try_recv()
        {
            match result {
                Ok(v) if v["ok"] == true => {
                    self.reply = v["text"].as_str().unwrap_or_default().to_owned();
                    self.status = format!("Answered by {}", v["model"].as_str().unwrap_or("Codex"));
                }
                Ok(v) => self.status = v["error"].as_str().unwrap_or("Bridge error").to_owned(),
                Err(e) => self.status = e,
            }
            self.pending = None;
        }
    }

    fn send(&mut self, context: Value) {
        if self.pending.is_some() || self.prompt.trim().is_empty() {
            return;
        }
        let (tx, rx) = channel();
        self.pending = Some(rx);
        self.status = "Waiting for Codex…".into();
        let request = json!({"prompt": self.prompt, "model": self.model, "context": context});
        std::thread::spawn(move || {
            let result = run_bridge(&request);
            let _ = tx.send(result);
        });
    }
}

fn run_bridge(request: &Value) -> Result<Value, String> {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/codex_bridge.py");
    let python = std::env::var("RAINEE_CODEX_PYTHON").unwrap_or_else(|_| "python".into());
    let mut command = Command::new(python);
    command.arg(script);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: a prompt must not pop up a console.
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start Codex bridge: {e}"))?;
    if let Some(mut input) = child.stdin.take() {
        writeln!(input, "{request}").map_err(|e| format!("Could not send prompt: {e}"))?;
    }
    let output = child.wait_with_output().map_err(|e| format!("Codex bridge failed: {e}"))?;
    if output.stdout.len() > 1_000_000 {
        return Err("Codex reply exceeded the display limit".into());
    }
    serde_json::from_slice(&output.stdout).map_err(|_| "Codex bridge returned no valid reply".into())
}

pub fn show(app: &mut VectorcraftApp, ctx: &egui::Context) {
    app.codex.poll();
    if !app.codex.open {
        return;
    }
    let mut open = app.codex.open;
    let mut send = false;
    egui::Window::new("MCP UI Designer · Vector Work Edition")
        .drag_area(egui::WindowDrag::TitleBar)
        .open(&mut open)
        .default_pos(egui::pos2(480.0, 90.0))
        .default_width(420.0)
        .default_height(430.0)
        .show(ctx, |ui| {
            ui.label("Ask about the active document. Sending a prompt shares its bounded snapshot with Codex.");
            ui.horizontal(|ui| {
                ui.label("Model (optional)");
                ui.text_edit_singleline(&mut app.codex.model);
            });
            ui.add(egui::TextEdit::multiline(&mut app.codex.prompt).hint_text("What would you like to draw or change?").desired_rows(4));
            if ui.add_enabled(app.codex.pending.is_none(), egui::Button::new("Ask Codex")).clicked() {
                send = true;
            }
            ui.label(&app.codex.status);
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(&app.codex.reply);
            });
            ui.small("Read-only preview: Codex cannot edit the canvas through this panel yet.");
        });
    app.codex.open = open;
    if send {
        let context = app.run("document.inspect", json!({})).unwrap_or_else(|e| json!({"error": e}));
        app.codex.send(context);
    }
}
