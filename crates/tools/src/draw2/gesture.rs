//! Freehand gesture tools: Pencil (N), Paintbrush (B), Blob Brush (Shift+B), Eraser (Shift+E),
//! Knife, Smooth, Path Eraser and Join.
//!
//! Pencil and Paintbrush preview the fitted path live (`path.freehand` re-applied on the snapshot)
//! and commit on release; Alt on release (or ending near the start) closes the path, and starting
//! near an end of a selected open path continues it. The Pencil's new path starts and ends on
//! Smart Guides ([`DrawSnap`]: an anchor, a path, in line with the art), hovering too; the points
//! between follow the hand. The other tools collect the drag polyline (shown as an overlay) and run
//! one command on release.

use serde_json::{Value, json};
use vectorcraft_doc::NodeId;
use vectorcraft_geom::Point;

use super::{FEEDBACK, points_json, polyline};
use crate::guides::DrawSnap;
use crate::{Action, Cursor, Mods, Overlay, PointerEvent, PointerKind, Tool, ToolContext, ToolKey};

pub struct GestureTool {
    id: &'static str,
    points: Vec<Point>,
    pressures: Vec<f32>,
    active: bool,
    began: bool,
    /// Pencil/Paintbrush continuing a selected open path: (path, continue from its start).
    extend: Option<(NodeId, bool)>,
    /// Fit tolerance in points (Pencil/Paintbrush/Smooth "Fidelity").
    pub fidelity: f64,
    /// Brush / eraser diameter in points.
    pub size: f64,
    erase_all: bool,
    /// Fill new pencil/brush strokes.
    pub fill: bool,
    /// "Edit selected paths within N pixels".
    pub edit_within: f64,
    /// "Close paths when ends are within N pixels".
    pub close_within: f64,
    /// Smart Guides for the Pencil's ends.
    snap: DrawSnap,
    tracker: crate::stylus::Tracker,
    learned: crate::learned_prediction::Learned,
    stabilize: bool,
    min_cutoff: f64,
    beta: f64,
    prediction_ms: f64,
    magnetic: bool,
    snap_strength: f64,
    angle_lock: f64,
    prediction_allowed: bool,
    pen_samples: Vec<Value>,
    pressure_width: bool,
    pressure_gamma: f64,
    pressure_min: f64,
    tilt_nib: bool,
    hover_preview: bool,
    nib_roundness: f64,
    nib_angle: f64,
    expressive_bypass: bool,
    defer_preview: bool,
}

impl GestureTool {
    pub fn new(id: &str) -> Self {
        let id: &'static str = match id {
            "paintbrush" => "paintbrush",
            "blobBrush" => "blobBrush",
            "eraser" => "eraser",
            "knife" => "knife",
            "smooth" => "smooth",
            "pathEraser" => "pathEraser",
            "join" => "join",
            _ => "pencil",
        };
        let fidelity = if id == "smooth" { 2.5 } else { 1.5 };
        Self {
            id,
            points: vec![],
            pressures: vec![],
            active: false,
            began: false,
            extend: None,
            fidelity,
            size: 10.0,
            erase_all: true,
            fill: false,
            edit_within: 12.0,
            close_within: 15.0,
            snap: DrawSnap::default(),
            tracker: crate::stylus::Tracker::default(),
            learned: crate::learned_prediction::Learned::default(),
            stabilize: false,
            min_cutoff: 1.0,
            beta: 0.01,
            prediction_ms: 0.0,
            magnetic: false,
            snap_strength: 0.75,
            angle_lock: 0.0,
            prediction_allowed: true,
            pen_samples: vec![],
            pressure_width: false,
            pressure_gamma: 1.0,
            pressure_min: 0.1,
            tilt_nib: false,
            hover_preview: false,
            nib_roundness: 0.2,
            nib_angle: 0.0,
            expressive_bypass: false,
            defer_preview: false,
        }
    }

    /// Do the ends of its strokes snap (the Pencil)?
    fn snaps(&self) -> bool {
        self.id == "pencil"
    }

    fn draws_path(&self) -> bool {
        matches!(self.id, "pencil" | "paintbrush")
    }

    fn stroke_json(&self) -> Value {
        if self.pressures.len() == self.points.len() && self.pressures.iter().any(|p| *p < 1.0) {
            Value::Array(self.points.iter().zip(&self.pressures).map(|(p, f)| json!([p.x, p.y, (f64::from(*f) * 1000.0).round() / 1000.0])).collect())
        } else {
            points_json(&self.points)
        }
    }

    fn label(&self) -> &'static str {
        match self.id {
            "paintbrush" => "Paintbrush",
            "blobBrush" => "Blob Brush",
            "eraser" => "Eraser",
            "knife" => "Knife",
            "smooth" => "Smooth",
            "pathEraser" => "Path Eraser",
            "join" => "Join",
            _ => "Pencil",
        }
    }

    /// The selected open path whose end is near `p`.
    fn find_extend(&self, cx: &ToolContext, p: Point) -> Option<(NodeId, bool)> {
        let tol = cx.tol(self.edit_within);
        for id in &cx.selection.objects {
            let Some(pd) = cx.doc.node(*id).and_then(|n| n.path_data()) else { continue };
            if !cx.doc.is_editable(*id) {
                continue;
            }
            let Some(sp) = pd.subpaths.iter().rev().find(|s| !s.closed && !s.anchors.is_empty()) else { continue };
            let (first, last) = (sp.anchors[0].p, sp.anchors[sp.anchors.len() - 1].p);
            if p.distance(last) <= tol {
                return Some((*id, false));
            }
            if p.distance(first) <= tol {
                return Some((*id, true));
            }
        }
        None
    }

    /// Parameters of `path.freehand` for the current stroke.
    pub fn freehand_params(&self, closed: bool) -> Value {
        let mut v = json!({
            "points": self.stroke_json(),
            "fidelity": self.fidelity,
            "closed": closed,
            "style": if self.id == "paintbrush" { "brush" } else { "pencil" },
            "fill": self.fill,
        });
        if let Some((id, start)) = self.extend {
            v["extend"] = json!({"id": id.0, "end": if start { "start" } else { "end" }});
        }
        if (self.pressure_width || self.tilt_nib) && !self.expressive_bypass {
            v["penSamples"] = json!(self.pen_samples);
            v["expressive"] = json!({"pressureWidth":self.pressure_width,"pressureGamma":self.pressure_gamma,
                "pressureMin":self.pressure_min,"tiltNib":self.tilt_nib,"nibRoundness":self.nib_roundness,"nibAngle":self.nib_angle});
        }
        v
    }

    fn finish(&mut self, cx: &ToolContext, m: Mods) -> Vec<Action> {
        let pts = &self.points;
        let has_sel = !cx.selection.is_empty();
        let tol = cx.tol(self.edit_within);
        match self.id {
            "pencil" | "paintbrush" => {
                if !self.began {
                    return vec![];
                }
                let near = pts.len() > 2 && pts[0].distance(pts[pts.len() - 1]) <= cx.tol(self.close_within);
                let closed = m.alt || (self.extend.is_none() && near);
                vec![Action::Preview("path.freehand".into(), self.freehand_params(closed)), Action::Commit]
            }
            "blobBrush" => vec![Action::Exec("path.blob".into(), json!({"points": points_json(pts), "size": self.size}))],
            "eraser" => {
                vec![Action::Exec("path.eraseRegion".into(), json!({"points": points_json(pts), "size": self.size, "eraseAll": self.erase_all}))]
            }
            "knife" if pts.len() >= 2 => vec![Action::Exec("path.knife".into(), json!({"points": points_json(pts)}))],
            "smooth" if has_sel && pts.len() >= 2 => {
                vec![Action::Exec("path.smoothRegion".into(), json!({"points": points_json(pts), "radius": tol, "fidelity": self.fidelity}))]
            }
            "pathEraser" if has_sel => vec![Action::Exec("path.eraseSegments".into(), json!({"points": points_json(pts), "width": cx.tol(8.0)}))],
            "join" if pts.len() >= 2 => vec![Action::Exec("path.joinScrub".into(), json!({"points": points_json(pts), "tolerance": tol}))],
            _ => vec![],
        }
    }
}

impl Tool for GestureTool {
    fn id(&self) -> &'static str {
        self.id
    }
    fn busy(&self) -> bool {
        self.active
    }
    fn pointer(&mut self, cx: &ToolContext, ev: &PointerEvent) -> Vec<Action> {
        // Do not extrapolate across a deliberate modifier bypass or a constraint boundary.
        self.prediction_allowed = !ev.mods.ctrl && !ev.mods.cmd && !self.magnetic && !(self.angle_lock > 0.0 && ev.mods.shift);
        if ev.kind == PointerKind::Down {
            self.tracker.reset();
            self.learned.reset();
        }
        if self.draws_path() && matches!(ev.kind, PointerKind::Down | PointerKind::Drag) {
            self.learned.sample(ev.prediction_input, self.prediction_ms, !self.defer_preview);
        }
        let mut p = if self.draws_path() && matches!(ev.kind, PointerKind::Down | PointerKind::Drag | PointerKind::Up) {
            self.tracker.sample(ev.pos, ev.time, self.stabilize && !ev.mods.ctrl, self.min_cutoff, self.beta)
        } else {
            ev.pos
        };
        if self.draws_path()
            && self.active
            && !ev.mods.ctrl
            && !ev.mods.cmd
            && (self.magnetic || (self.angle_lock > 0.0 && ev.mods.shift))
            && let Some(origin) = self.points.first().copied()
        {
            let g = crate::distort::perspective::PerspectiveGrid::current(cx.doc);
            p = crate::stylus::magnet(
                p,
                origin,
                &g,
                cx.tol(12.0),
                self.snap_strength,
                self.magnetic,
                self.angle_lock,
                ev.mods.shift,
                cx.constrain_angle,
            );
        }
        match ev.kind {
            PointerKind::Move if self.snaps() => {
                self.snap.hover(cx, p, &[], None);
                vec![]
            }
            PointerKind::Down => {
                self.active = true;
                self.began = false;
                self.expressive_bypass = ev.mods.ctrl;
                // Do not overwrite an existing path's appearance with a new pen capture. Expressive
                // strokes start as separate editable paths; ordinary continuation is unchanged.
                self.extend = if self.draws_path() && (!(self.pressure_width || self.tilt_nib) || self.expressive_bypass) {
                    self.find_extend(cx, p)
                } else {
                    None
                };
                // A path continued starts at its end; a new one where it snaps.
                let start = if self.snaps() && self.extend.is_none() { self.snap.press(cx, p, &[], None) } else { p };
                self.points = vec![start];
                self.pressures = if self.id == "paintbrush" { vec![ev.pressure] } else { vec![] };
                self.pen_samples = vec![pen_sample(ev)];
                vec![]
            }
            PointerKind::Drag => {
                if !self.active {
                    return vec![];
                }
                if self.snaps() {
                    // Where the stroke would end: shown, taken on release.
                    self.snap.drag(cx, p, None);
                }
                if self.points.last().is_some_and(|l| l.distance(p) < cx.tol(1.0)) {
                    // A pressure/tilt change at a stationary tip must not become a duplicate
                    // centreline point, nor silently vanish behind the geometry distance filter.
                    if (self.pressure_width || self.tilt_nib) && !self.expressive_bypass {
                        if let Some(last) = self.pen_samples.last_mut() {
                            *last = pen_sample(ev);
                        }
                        if self.began && !self.defer_preview {
                            return vec![Action::Preview("path.freehand".into(), self.freehand_params(false))];
                        }
                    }
                    return vec![];
                }
                self.points.push(p);
                if self.id == "paintbrush" {
                    self.pressures.push(ev.pressure);
                }
                self.pen_samples.push(pen_sample(ev));
                if !self.draws_path() {
                    return vec![];
                }
                let mut out = vec![];
                if !self.began {
                    self.began = true;
                    out.push(Action::Begin(self.label().into()));
                }
                if !self.defer_preview {
                    out.push(Action::Preview("path.freehand".into(), self.freehand_params(false)));
                }
                out
            }
            PointerKind::Up => {
                if !self.active {
                    return vec![];
                }
                let end = if self.snaps() { self.snap.drag(cx, p, None) } else { p };
                self.snap.clear();
                if self.points.last().is_some_and(|l| l.distance(p) >= cx.tol(1.0)) {
                    self.points.push(end);
                    if self.id == "paintbrush" {
                        self.pressures.push(ev.pressure);
                    }
                    self.pen_samples.push(pen_sample(ev));
                } else if self.snaps()
                    && self.points.len() > 1
                    && let Some(last) = self.points.last_mut()
                {
                    // The last step was where the button went up: it moves where that snaps.
                    *last = end;
                }
                let out = self.finish(cx, ev.mods);
                self.active = false;
                self.began = false;
                self.points.clear();
                self.pressures.clear();
                self.pen_samples.clear();
                self.tracker.reset();
                self.learned.reset();
                self.extend = None;
                out
            }
            _ => vec![],
        }
    }
    fn pointer_batch(&mut self, cx: &ToolContext, events: &[PointerEvent]) -> Vec<Action> {
        let mut actions = vec![];
        for (i, event) in events.iter().enumerate() {
            self.defer_preview = i + 1 < events.len();
            actions.extend(self.pointer(cx, event));
        }
        self.defer_preview = false;
        // If the last packet is a hover, a preceding drag still needs its final measured preview.
        if self.began && !matches!(events.last().map(|e| e.kind), Some(PointerKind::Drag | PointerKind::Up)) {
            actions.push(Action::Preview("path.freehand".into(), self.freehand_params(false)));
        }
        actions
    }
    fn key(&mut self, _cx: &ToolContext, key: ToolKey, _m: Mods) -> Vec<Action> {
        match key {
            ToolKey::Escape if self.active => {
                let began = self.began;
                self.snap.clear();
                self.active = false;
                self.began = false;
                self.points.clear();
                self.pressures.clear();
                self.pen_samples.clear();
                self.tracker.reset();
                self.learned.reset();
                if began { vec![Action::Cancel] } else { vec![] }
            }
            ToolKey::BracketLeft if matches!(self.id, "blobBrush" | "eraser") => {
                self.size = (self.size - 1.0).max(1.0);
                vec![]
            }
            ToolKey::BracketRight if matches!(self.id, "blobBrush" | "eraser") => {
                self.size = (self.size + 1.0).min(1000.0);
                vec![]
            }
            _ => vec![],
        }
    }
    fn claims_key(&self, _cx: &ToolContext, key: ToolKey) -> bool {
        matches!(key, ToolKey::BracketLeft | ToolKey::BracketRight) && matches!(self.id, "blobBrush" | "eraser")
    }
    fn deactivate(&mut self, _cx: &ToolContext) -> Vec<Action> {
        let began = self.began;
        self.snap.clear();
        self.active = false;
        self.began = false;
        self.points.clear();
        self.pressures.clear();
        self.pen_samples.clear();
        self.tracker.reset();
        self.learned.reset();
        if began { vec![Action::Commit] } else { vec![] }
    }
    fn overlays(&self, cx: &ToolContext) -> Vec<Overlay> {
        let mut o = self.snap.guides().to_vec();
        if self.active
            && self.draws_path()
            && self.prediction_allowed
            && let Some((a, b)) = self.tracker.prediction(self.prediction_ms, cx.zoom)
            && let Some(head) = self.points.last().copied()
        {
            let delta = if self.learned.enabled() { self.learned.prediction(cx.zoom) } else { Some(b - a) };
            if let Some(delta) = delta {
                o.push(Overlay::Line { a: head, b: head + delta, color: FEEDBACK, dashed: false });
            }
        }
        if !self.active || self.points.len() < 2 || self.began {
            return o;
        }
        let width = match self.id {
            "blobBrush" | "eraser" => (self.size * cx.zoom) as f32,
            _ => 1.0,
        };
        let color = if self.id == "eraser" { [0x80, 0x80, 0x80] } else { FEEDBACK };
        o.push(Overlay::Path { path: polyline(&self.points), color, width, dashed: self.id == "knife" });
        o
    }
    fn cursor(&self, _cx: &ToolContext, _p: Point, _m: Mods) -> Cursor {
        Cursor::Crosshair
    }
    fn options(&self) -> Value {
        match self.id {
            "pencil" | "paintbrush" => {
                json!({"fidelity": self.fidelity, "fill": self.fill, "editWithin": self.edit_within, "closeWithin": self.close_within,
                    "stabilize":self.stabilize,"minCutoff":self.min_cutoff,"beta":self.beta,
                    "predictionMs":self.prediction_ms,"learnedPrediction":self.learned.enabled(),
                    "learnedStatus":self.learned.status(),"learnedInferences":self.learned.inferences,"learnedSkipped":self.learned.skipped,
                    "learnedLastUs":self.learned.last_us,"learnedMaxUs":self.learned.max_us,"learnedStaleResults":self.learned.stale_results,
                    "magnetic":self.magnetic,"snapStrength":self.snap_strength,"angleLock":self.angle_lock,
                    "pressureWidth":self.pressure_width,"pressureGamma":self.pressure_gamma,"pressureMin":self.pressure_min,
                    "tiltNib":self.tilt_nib,"nibRoundness":self.nib_roundness,"nibAngle":self.nib_angle,"hoverPreview":self.hover_preview})
            }
            "smooth" => json!({"fidelity": self.fidelity}),
            "blobBrush" => json!({"size": self.size}),
            "eraser" => json!({"size": self.size, "eraseAll": self.erase_all}),
            _ => Value::Null,
        }
    }
    fn set_option(&mut self, key: &str, v: &Value) {
        if key == "eraseAll" && self.id == "eraser" {
            if let Some(value) = v.as_bool() {
                self.erase_all = value;
            }
            return;
        }
        match key {
            "fidelity" => self.fidelity = v.as_f64().unwrap_or(self.fidelity).clamp(0.05, 100.0),
            "size" => self.size = v.as_f64().unwrap_or(self.size).clamp(0.1, 1000.0),
            "fill" => self.fill = v.as_bool().unwrap_or(self.fill),
            "editWithin" => self.edit_within = v.as_f64().unwrap_or(self.edit_within).clamp(0.0, 100.0),
            "closeWithin" => self.close_within = v.as_f64().unwrap_or(self.close_within).clamp(0.0, 100.0),
            "stabilize" => self.stabilize = v.as_bool().unwrap_or(self.stabilize),
            "pressureWidth" => self.pressure_width = v.as_bool().unwrap_or(self.pressure_width),
            "tiltNib" => self.tilt_nib = v.as_bool().unwrap_or(self.tilt_nib),
            "hoverPreview" => self.hover_preview = v.as_bool().unwrap_or(self.hover_preview),
            "pressureGamma" => self.pressure_gamma = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.pressure_gamma).clamp(0.1, 4.0),
            "pressureMin" => self.pressure_min = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.pressure_min).clamp(0.01, 1.0),
            "nibRoundness" => self.nib_roundness = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.nib_roundness).clamp(0.01, 1.0),
            "nibAngle" => self.nib_angle = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.nib_angle).clamp(-180.0, 180.0),
            "minCutoff" => self.min_cutoff = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.min_cutoff).clamp(0.1, 60.0),
            "beta" => self.beta = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.beta).clamp(0.0, 2.0),
            "predictionMs" => self.prediction_ms = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.prediction_ms).clamp(0.0, 16.0),
            "learnedPrediction" => {
                if let Some(enabled) = v.as_bool() {
                    self.learned.enable(enabled)
                }
            }
            "magnetic" => self.magnetic = v.as_bool().unwrap_or(self.magnetic),
            "snapStrength" => self.snap_strength = v.as_f64().filter(|v| v.is_finite()).unwrap_or(self.snap_strength).clamp(0.0, 1.0),
            "angleLock" => {
                if let Some(a) = v.as_f64().filter(|a| [0.0, 15.0, 30.0, 45.0].contains(a)) {
                    self.angle_lock = a;
                }
            }
            _ => {}
        }
    }
    fn wants_ticks(&self) -> bool {
        self.active && self.prediction_ms > 0.0
    }
    fn tick(&mut self, _cx: &ToolContext, dt: f64) -> Vec<Action> {
        self.tracker.tick(dt);
        self.learned.tick(dt);
        vec![]
    }
}

fn pen_sample(ev: &PointerEvent) -> Value {
    json!({"pressure": if ev.pressure.is_finite() { ev.pressure.clamp(0.0, 1.0) } else { 1.0 },
        "tilt": ev.tilt})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use vectorcraft_doc::Selection;

    fn drag(t: &mut GestureTool, cx: &ToolContext, pts: &[(f64, f64)], m: Mods) -> Vec<Vec<Action>> {
        let mut v = vec![t.pointer(cx, &PointerEvent::new(PointerKind::Down, pts[0].0, pts[0].1))];
        for &(x, y) in &pts[1..] {
            v.push(t.pointer(cx, &PointerEvent::new(PointerKind::Drag, x, y)));
        }
        let l = pts[pts.len() - 1];
        v.push(t.pointer(cx, &PointerEvent::new(PointerKind::Up, l.0, l.1).with_mods(m)));
        v
    }

    /// The Pencil's new path starts and ends on Smart Guides (#506), the points between where
    /// the hand went.
    #[test]
    fn pencil_ends_snap_to_smart_guides() {
        let (d, _) = doc_with_rect();
        let s = Selection::default();
        let p = paint();
        let cx = cx(&d, &s, &p);
        let mut t = GestureTool::new("pencil");
        let acts = drag(&mut t, &cx, &[(102.0, 98.0), (151.0, 300.0), (250.0, 320.0), (198.0, 203.0)], Mods::default());
        let Some(Action::Preview(_, v)) = acts.last().and_then(|a| a.first()) else { panic!("{acts:?}") };
        assert_eq!(v["points"], json!([[100.0, 100.0], [151.0, 300.0], [250.0, 320.0], [200.0, 200.0]]));
        // The Paintbrush follows the hand all the way.
        let mut t = GestureTool::new("paintbrush");
        let acts = drag(&mut t, &cx, &[(102.0, 98.0), (151.0, 300.0)], Mods::default());
        let Some(Action::Preview(_, v)) = acts.last().and_then(|a| a.first()) else { panic!("{acts:?}") };
        assert_eq!(v["points"][0], json!([102.0, 98.0]));
    }

    /// #852: the Paintbrush sends each point's pen pressure; a mouse (always fully pressed) and
    /// the Pencil send plain points.
    #[test]
    fn paintbrush_sends_pen_pressure() {
        let (d, _) = doc_with_rect();
        let s = Selection::default();
        let p = paint();
        let cx = cx(&d, &s, &p);
        let stroke = |id: &str, pressures: [f32; 3]| {
            let mut t = GestureTool::new(id);
            let kinds = [PointerKind::Down, PointerKind::Drag, PointerKind::Up];
            let mut last = vec![];
            for ((k, x), f) in kinds.into_iter().zip([0.0, 40.0, 80.0]).zip(pressures) {
                last = t.pointer(&cx, &PointerEvent { pressure: f, ..PointerEvent::new(k, x, 0.0) });
            }
            let Some(Action::Preview(_, v)) = last.first() else { panic!("{last:?}") };
            v["points"].clone()
        };
        assert_eq!(stroke("paintbrush", [0.25, 0.5, 1.0]), json!([[0.0, 0.0, 0.25], [40.0, 0.0, 0.5], [80.0, 0.0, 1.0]]));
        assert_eq!(stroke("paintbrush", [1.0; 3]), json!([[0.0, 0.0], [40.0, 0.0], [80.0, 0.0]]));
        assert_eq!(stroke("pencil", [0.25, 0.5, 1.0]), json!([[0.0, 0.0], [40.0, 0.0], [80.0, 0.0]]));
    }

    #[test]
    fn pencil_previews_and_commits() {
        let (d, _) = doc_with_rect();
        let s = Selection::default();
        let p = paint();
        let cx = cx(&d, &s, &p);
        let mut t = GestureTool::new("pencil");
        let acts = drag(&mut t, &cx, &[(0.0, 0.0), (10.0, 5.0), (20.0, 0.0)], Mods::default());
        assert!(acts[0].is_empty());
        assert_eq!(acts[1][0], Action::Begin("Pencil".into()));
        assert!(matches!(&acts[1][1], Action::Preview(c, v) if c == "path.freehand" && v["points"].as_array().unwrap().len() == 2));
        let last = acts.last().unwrap();
        assert!(matches!(&last[0], Action::Preview(_, v) if v["closed"] == false && v["style"] == "pencil"));
        assert_eq!(last[1], Action::Commit);
        assert!(!t.busy());
    }

    #[test]
    fn pencil_alt_closes_and_continues_selected_path() {
        let (d, id) = doc_with_rect();
        let s = Selection::default();
        let p = paint();
        let cx = cx(&d, &s, &p);
        let mut t = GestureTool::new("paintbrush");
        let acts = drag(&mut t, &cx, &[(0.0, 0.0), (10.0, 5.0), (20.0, 0.0)], Mods { alt: true, ..Default::default() });
        assert!(matches!(&acts.last().unwrap()[0], Action::Preview(_, v) if v["closed"] == true && v["style"] == "brush"));
        // A selected closed rect is not continued; an open path would be (see engine tests).
        let mut s2 = Selection::default();
        s2.set([id]);
        let cx2 = crate::testutil::cx(&d, &s2, &p);
        let acts = drag(&mut t, &cx2, &[(100.0, 100.0), (90.0, 90.0)], Mods::default());
        assert!(matches!(&acts[1][1], Action::Preview(_, v) if v.get("extend").is_none()));
    }

    #[test]
    fn eraser_and_knife_exec_on_release() {
        let (d, _) = doc_with_rect();
        let s = Selection::default();
        let p = paint();
        let cx = cx(&d, &s, &p);
        let mut t = GestureTool::new("eraser");
        assert_eq!(t.options()["eraseAll"], true, "all editable paths is the new default");
        let acts = drag(&mut t, &cx, &[(50.0, 150.0), (250.0, 150.0)], Mods::default());
        assert!(acts[1].is_empty());
        assert!(matches!(&acts[2][0], Action::Exec(c, v) if c == "path.eraseRegion" && v["size"] == 10.0));
        t.set_option("eraseAll", &json!(true));
        assert_eq!(t.options()["eraseAll"], true);
        let all_acts = drag(&mut t, &cx, &[(50.0, 150.0), (250.0, 150.0)], Mods::default());
        assert!(matches!(&all_acts[2][0], Action::Exec(c, v) if c == "path.eraseRegion" && v["eraseAll"] == true));
        let mut k = GestureTool::new("knife");
        k.pointer(&cx, &PointerEvent::new(PointerKind::Down, 50.0, 150.0));
        k.pointer(&cx, &PointerEvent::new(PointerKind::Drag, 150.0, 150.0));
        assert_eq!(k.overlays(&cx).len(), 1);
        let a = k.pointer(&cx, &PointerEvent::new(PointerKind::Up, 250.0, 150.0));
        assert!(matches!(&a[0], Action::Exec(c, v) if c == "path.knife" && v["points"].as_array().unwrap().len() == 3));
    }

    #[test]
    fn smooth_needs_selection_and_brackets_resize() {
        let (d, _) = doc_with_rect();
        let s = Selection::default();
        let p = paint();
        let cx = cx(&d, &s, &p);
        let mut t = GestureTool::new("smooth");
        let acts = drag(&mut t, &cx, &[(50.0, 150.0), (250.0, 150.0)], Mods::default());
        assert!(acts.iter().all(|a| a.is_empty()));
        let mut b = GestureTool::new("blobBrush");
        b.key(&cx, ToolKey::BracketRight, Mods::default());
        assert_eq!(b.options()["size"], 11.0);
    }
}
