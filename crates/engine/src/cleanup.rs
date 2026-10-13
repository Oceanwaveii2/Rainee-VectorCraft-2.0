//! Experimental selected-stroke cleanup. The optional research worker is isolated from the
//! Rust editor. Preview never mutates art; acceptance retains the originals and is journalled.
use crate::{EngineError, Result, Session};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use vectorcraft_doc::{Document, Node, NodeId, NodeKind};
use vectorcraft_geom::{PathData, PathEl, Point, SubPath};

mod profiles;
mod worker;
pub const MAX_POINTS: usize = 16384;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub originals: Vec<NodeId>,
    pub points: Vec<Point>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub originals: Vec<Node>,
    pub groups: Vec<Group>,
    pub max_deviation: f64,
}
#[derive(Clone)]
pub(crate) struct Input {
    pub plan: Plan,
    pub strokes: Vec<Vec<Point>>,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub width: f64,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub timeout: f64,
}
#[derive(Default)]
pub struct Cleanup {
    pub(crate) uid: u64,
    pub(crate) snapshot: Option<Arc<Document>>,
    pub(crate) receiver: Option<mpsc::Receiver<std::result::Result<Plan, String>>>,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) plan: Option<Plan>,
    pub(crate) state: String,
    pub(crate) message: String,
    pub(crate) serial: u64,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl Cleanup {
    pub fn running(&self) -> bool {
        self.receiver.is_some()
    }
    pub fn status(&self) -> Value {
        let mut status = self.summary();
        status["proposal"] = serde_json::to_value(&self.plan).unwrap_or(Value::Null);
        status
    }
    /// Small UI status: do not serialize full artwork on every paint frame.
    pub fn summary(&self) -> Value {
        json!({"state": if self.state.is_empty() {"idle"} else {&self.state}, "message":self.message,
            "job":self.serial, "document":self.uid, "experimental":true,
            "backend":worker::availability(), "groupCount":self.plan.as_ref().map(|p|p.groups.len()),
            "bundleCount":self.plan.as_ref().map(|p|p.groups.iter().filter(|g|g.originals.len()>1).count()),
            "profilePolicy":"Leader pressure/tilt knots projected to the approved centerline; no averaging; all original samples retained",
            "unsupported":"Filled, closed, compound, masked, live and mixed base-appearance paths; scatter/pattern/bristle effects; ambiguous profile projection; non-Windows worker"})
    }
    pub fn stop(&mut self, state: &str) {
        self.cancel.store(true, Ordering::Relaxed);
        self.receiver = None;
        self.plan = None;
        self.snapshot = None;
        self.state = state.into();
        self.message.clear();
    }
}

pub(crate) fn error(s: impl Into<String>) -> EngineError {
    EngineError::Other(s.into())
}

pub(crate) fn plain(node: &Node) -> bool {
    let NodeKind::Path { path, live: None, clipping: false, guide: false, .. } = &node.kind else { return false };
    let Some(st) = node.appearance.stroke() else { return false };
    path.subpaths.len() == 1
        && !path.is_closed()
        && path.anchor_count() <= 4096
        && node.visible
        && !node.locked
        && node.opacity == 1.0
        && node.blend == vectorcraft_color::BlendMode::Normal
        && node.mask.is_none()
        && node.perspective.is_none()
        && node.trace.is_none()
        && node.wrap.is_none()
        && node.graph.is_none()
        && node.shaper.is_none()
        && node.slice.is_none()
        && node.appearance.is_basic()
        && node.appearance.fill_paint().is_none()
        && !st.paint.is_none()
        && st.width.is_finite()
        && st.width > 0.0
        && profiles::valid(st)
        && st.dash.as_ref().is_none_or(|d| !d.is_dashed())
        && st.start_arrow.is_none()
        && st.end_arrow.is_none()
        && st.align == vectorcraft_doc::StrokeAlign::Center
}

pub(crate) fn capture(s: &Session, ids: &[NodeId], tolerance: f64, deviation: f64, timeout: f64) -> Result<Input> {
    let doc = &s.doc()?.doc;
    if s.tool_busy() || s.doc()?.interaction.is_some() {
        return Err(error("Finish the active gesture first"));
    }
    if !(2..=64).contains(&ids.len()) {
        return Err(error("Select 2–64 ordinary open strokes"));
    }
    let mut originals = Vec::new();
    let mut positions = Vec::new();
    let mut parent = None;
    let mut strokes = Vec::new();
    let mut total = 0;
    for id in ids {
        let n = doc.node(*id).ok_or_else(|| error("Missing cleanup target"))?;
        if !doc.is_editable(*id) || !plain(n) {
            return Err(error("Cleanup requires bounded open strokes; fills, invalid pressure/tilt and effects remain unchanged"));
        }
        if let Some(name) = n.appearance.stroke().and_then(|stroke| stroke.brush.as_deref()) {
            let definition = vectorcraft_brush::find(doc, name);
            if !definition.is_some_and(|brush| match &brush.kind {
                vectorcraft_brush::BrushKind::Art(_) | vectorcraft_brush::BrushKind::Natural(_) => true,
                vectorcraft_brush::BrushKind::Calligraphic(c) => c.variation == [0.0; 3],
                _ => false,
            }) {
                return Err(error("Cleanup supports Art/Natural and non-random Calligraphic centerlines; this brush effect is preserved unchanged"));
            }
        }
        let (p, i, _) = doc.position(*id).ok_or_else(|| error("Missing stroke position"))?;
        if !originals.is_empty() && (parent != p || originals.first().is_some_and(|first: &Node| !profiles::same_base(first, n))) {
            return Err(error("Cleanup needs contiguous siblings with identical base appearance (pressure/tilt may differ)"));
        }
        parent = p;
        positions.push(i);
        if let Some(ancestors) = doc.ancestry(*id) {
            for a in ancestors.into_iter().filter(|a| a != id) {
                if let Some(n) = doc.node(a)
                    && (n.mask.is_some()
                        || !n.appearance.effects.is_empty()
                        || matches!(n.kind, NodeKind::Group { clip: true, .. } | NodeKind::Layer { clip: true, .. }))
                {
                    return Err(error("Expand masked/effect containers before cleanup"));
                }
            }
        }
        let path = n.path_data().ok_or_else(|| error("Not a path"))?;
        let controls: Vec<_> = path.subpaths.iter().flat_map(|sp| sp.anchors.iter().flat_map(|a| [a.h_in, a.p, a.h_out])).collect();
        if controls.iter().any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 1e7 || p.y.abs() > 1e7)
            || controls.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() > 10000.0
        {
            return Err(error("Stroke control polygon exceeds cleanup sampling limits"));
        }
        let mut points = Vec::new();
        kurbo::flatten(path.to_bezpath().iter(), tolerance, |el| {
            if let PathEl::MoveTo(p) | PathEl::LineTo(p) = el
                && points.len() <= 4096
            {
                points.push(p);
            }
        });
        total += points.len();
        if !(2..=4096).contains(&points.len())
            || total > MAX_POINTS
            || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 1e7 || p.y.abs() > 1e7)
        {
            return Err(error("Cleanup input exceeds bounded geometry limits"));
        }
        originals.push(n.clone());
        strokes.push(points);
    }
    positions.sort_unstable();
    if positions.windows(2).any(|w| w[1] != w[0] + 1) {
        return Err(error("Select consecutive sibling paths without duplicates"));
    }
    let width = originals.first().map(|n| n.appearance.stroke_width()).unwrap_or(1.0);
    Ok(Input { plan: Plan { originals, groups: Vec::new(), max_deviation: deviation }, strokes, width, timeout })
}

impl Session {
    pub fn poll_cleanup(&mut self) {
        let valid = self
            .active()
            .is_some_and(|d| d.uid == self.cleanup.uid && self.cleanup.snapshot.as_ref().is_some_and(|snapshot| Arc::ptr_eq(snapshot, &d.doc)));
        if self.cleanup.snapshot.is_some() && !valid {
            self.cleanup.stop("stale");
            self.cleanup.message = "Document content changed or document switched; rerun cleanup".into();
            return;
        }
        if let Some(rx) = &self.cleanup.receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.cleanup.receiver = None;
                    match result {
                        Ok(plan) => {
                            let bundles = plan.groups.iter().filter(|g| g.originals.len() > 1).count();
                            self.cleanup.message = if bundles == 0 {
                                "No safe stroke bundles found at this tolerance. Originals unchanged; no cleanup to accept.".into()
                            } else {
                                format!(
                                    "{bundles} safe bundles ready. Leader pressure/tilt will transfer; originals and unmerged strokes remain unchanged."
                                )
                            };
                            self.cleanup.plan = Some(plan);
                            self.cleanup.state = "ready".into();
                        }
                        Err(message) => {
                            self.cleanup.message = message;
                            self.cleanup.state = "failed".into();
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.cleanup.receiver = None;
                    self.cleanup.state = "failed".into();
                    self.cleanup.message = "Worker disconnected".into();
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    pub(crate) fn start_cleanup(&mut self, input: Input) -> Result<Value> {
        worker::available().map_err(error)?;
        self.cleanup.stop("running");
        self.cleanup.uid = self.doc()?.uid;
        self.cleanup.snapshot = Some(self.doc()?.doc.clone());
        self.cleanup.serial += 1;
        self.cleanup.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cleanup.cancel.clone();
        let (tx, rx) = mpsc::channel();
        worker::spawn(input, cancel, tx).map_err(error)?;
        self.cleanup.receiver = Some(rx);
        Ok(self.cleanup.status())
    }
    pub(crate) fn apply_cleanup(&mut self, plan: Plan) -> Result<Value> {
        let ids: Vec<_> = plan.originals.iter().map(|n| n.id).collect();
        let input = capture(self, &ids, 0.25, plan.max_deviation, 30.0)?;
        if input.plan.originals != plan.originals {
            return Err(error("Cleanup targets changed"));
        }
        validate(&input, &plan).map_err(error)?;
        if !plan.groups.iter().any(|g| g.originals.len() > 1) {
            return Err(error("No stroke bundles found; originals remain unchanged"));
        }
        self.edit("Sketch Cleanup", |doc, selection| {
            let mut selected = Vec::new();
            // Process topmost bundles first. Singletons retain their exact original geometry.
            let mut groups: Vec<_> = plan.groups.iter().collect();
            groups.sort_by_key(|g| std::cmp::Reverse(g.originals.iter().filter_map(|id| doc.position(*id).map(|p| p.1)).min().unwrap_or(0)));
            for group in groups {
                if group.originals.len() == 1 {
                    selected.extend(&group.originals);
                    continue;
                }
                let leader = group.originals.first().and_then(|id| doc.node(*id)).cloned().ok_or_else(|| error("Missing leader"))?;
                let (parent, _, _) = doc.position(leader.id).ok_or_else(|| error("Missing parent"))?;
                let at = group.originals.iter().filter_map(|id| doc.position(*id).map(|p| p.1)).min().ok_or_else(|| error("Missing position"))?;
                let mut originals = Vec::new();
                for id in &group.originals {
                    originals.push(doc.remove(*id)?);
                }
                let mut backup = Node::group(doc.alloc_id(), originals);
                backup.name = Some("Cleanup Originals (hidden)".into());
                backup.visible = false;
                doc.insert(parent, at, backup)?;
                let mut fit = leader;
                profiles::transfer(&mut fit, &group.points)?;
                fit.id = doc.alloc_id();
                fit.name = Some("Cleaned Stroke".into());
                fit.kind = Node::path(fit.id, PathData::single(SubPath::polyline(&group.points, false)), fit.appearance.clone()).kind;
                let id = doc.insert(parent, at + 1, fit)?;
                selected.push(id);
            }
            selection.set(selected.clone());
            Ok(json!({"ids":selected,"originalsRetained":true}))
        })
    }
}

/// Reject unsafe research fits individually, never weakening acceptance validation.
/// Malformed mappings still fail the entire job. Preserved singletons keep exact nodes on accept.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn preserve_unsafe_groups(input: &Input, plan: &mut Plan) -> std::result::Result<(), String> {
    let mut groups = Vec::new();
    for group in &plan.groups {
        let mut subset = input.clone();
        subset.plan.originals.clear();
        subset.strokes.clear();
        for id in &group.originals {
            let i = input.plan.originals.iter().position(|n| n.id == *id).ok_or("Unknown proposal target")?;
            subset.plan.originals.push(input.plan.originals[i].clone());
            subset.strokes.push(input.strokes[i].clone());
        }
        let candidate = Plan { originals: subset.plan.originals.clone(), groups: vec![group.clone()], max_deviation: plan.max_deviation };
        match validate(&subset, &candidate) {
            Ok(()) if group.originals.len() > 1 => groups.push(group.clone()),
            Ok(()) => {
                for (node, points) in subset.plan.originals.iter().zip(&subset.strokes) {
                    groups.push(Group { originals: vec![node.id], points: points.clone() });
                }
            }
            Err(message) if message == "Fitted stroke exceeds permitted deviation" || message == "Fit discards part of an original stroke" => {
                for (node, points) in subset.plan.originals.iter().zip(&subset.strokes) {
                    groups.push(Group { originals: vec![node.id], points: points.clone() });
                }
            }
            Err(message) => return Err(message),
        }
    }
    let candidate = Plan { originals: plan.originals.clone(), groups, max_deviation: plan.max_deviation };
    validate(input, &candidate)?;
    *plan = candidate;
    Ok(())
}

pub(crate) fn validate(input: &Input, plan: &Plan) -> std::result::Result<(), String> {
    if !plan.max_deviation.is_finite()
        || !(0.25..=32.0).contains(&plan.max_deviation)
        || plan.groups.is_empty()
        || plan.groups.len() > input.strokes.len()
    {
        return Err("Invalid cleanup proposal limits".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0;
    let mut budget = 0usize;
    for group in &plan.groups {
        total += group.points.len();
        if group.originals.is_empty() || !(2..=4096).contains(&group.points.len()) || total > MAX_POINTS {
            return Err("Invalid proposal size".into());
        }
        let mut source = Vec::new();
        for id in &group.originals {
            if !seen.insert(*id) {
                return Err("Duplicate proposal target".into());
            }
            let i = input.plan.originals.iter().position(|n| n.id == *id).ok_or("Unknown proposal target")?;
            source.push(&input.strokes[i]);
        }
        let source_points: usize = source.iter().map(|s| s.len()).sum();
        let fit_samples = samples(&group.points, plan.max_deviation / 2.0)?;
        let source_samples: Vec<_> = source.iter().map(|s| samples(s, plan.max_deviation / 2.0)).collect::<std::result::Result<_, _>>()?;
        let source_sample_count: usize = source_samples.iter().map(|s| s.len()).sum();
        budget = budget
            .saturating_add(source_points.saturating_mul(fit_samples.len()))
            .saturating_add(source_sample_count.saturating_mul(group.points.len()));
        if budget > 2_000_000 {
            return Err("Proposal exceeds interactive validation budget; clean a smaller selection".into());
        }
        // Distance to a polyline is 1-Lipschitz. Sampling at most deviation/2 apart,
        // with a deviation/4 margin, also bounds the unsampled segment interiors.
        for p in &fit_samples {
            if !p.x.is_finite()
                || !p.y.is_finite()
                || source.iter().map(|s| distance(*p, s)).fold(f64::INFINITY, f64::min) > plan.max_deviation * 0.75
            {
                return Err("Fitted stroke exceeds permitted deviation".into());
            }
        }
        for stroke in source_samples {
            if stroke.iter().any(|p| distance(*p, &group.points) > plan.max_deviation * 0.75) {
                return Err("Fit discards part of an original stroke".into());
            }
        }
    }
    if seen.len() != input.plan.originals.len() {
        return Err("Proposal omits original strokes".into());
    }
    Ok(())
}
fn samples(line: &[Point], step: f64) -> std::result::Result<Vec<Point>, String> {
    let mut samples = Vec::new();
    for pair in line.windows(2) {
        let length = pair[0].distance(pair[1]);
        if !length.is_finite() || length / step > MAX_POINTS as f64 {
            return Err("Invalid or oversized proposal segment".into());
        }
        let count = (length / step).ceil().max(1.0) as usize;
        if samples.len() + count > MAX_POINTS {
            return Err("Proposal sampling exceeds limits".into());
        }
        for i in 0..count {
            samples.push(pair[0] + (pair[1] - pair[0]) * (i as f64 / count as f64));
        }
    }
    if let Some(p) = line.last() {
        samples.push(*p);
    }
    Ok(samples)
}
fn distance(p: Point, line: &[Point]) -> f64 {
    line.windows(2)
        .map(|w| {
            let v = w[1] - w[0];
            let len = v.hypot2();
            let t = if len > 1e-20 { ((p - w[0]).dot(v) / len).clamp(0.0, 1.0) } else { 0.0 };
            p.distance(w[0] + v * t)
        })
        .fold(f64::INFINITY, f64::min)
}
