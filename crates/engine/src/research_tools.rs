//! Optional local research assistants. Preview workers cannot mutate the document.
use crate::{Result, Session, cleanup::error};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use vectorcraft_doc::{Document, ImageBlob, ImageObject, Node, NodeId, NodeKind};
use vectorcraft_geom::{Affine, PathEl, Point, Rect};
mod worker;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub id: NodeId,
    pub points: Vec<Point>,
    pub width: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Junction {
    pub a: NodeId,
    pub tail: bool,
    pub b: NodeId,
    pub point: Point,
    pub confidence: f64,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub max_gap: f64,
    pub kind: String,
    pub originals: Vec<Node>,
    pub junctions: Vec<Junction>,
    pub png: Vec<u8>,
    pub region: Rect,
    pub size: [u32; 2],
    pub telemetry: Value,
}
#[derive(Clone)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(super) struct Input {
    pub kind: String,
    pub strokes: Vec<Stroke>,
    pub proposal: Proposal,
    pub gap: f64,
    pub threshold: f64,
    pub seed: u32,
    pub color: [f64; 3],
}
#[derive(Default)]
pub struct ResearchTools {
    uid: u64,
    snapshot: Option<Arc<Document>>,
    receiver: Option<mpsc::Receiver<std::result::Result<Proposal, String>>>,
    cancel: Arc<AtomicBool>,
    pub proposal: Option<Proposal>,
    pub state: String,
    pub message: String,
    pub job: u64,
    pub kind: String,
    last_telemetry: Value,
    events: std::collections::VecDeque<Value>,
}
impl Drop for ResearchTools {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl ResearchTools {
    pub fn running(&self) -> bool {
        self.receiver.is_some()
    }
    pub fn clear(&mut self, state: &str) {
        if let Some(p) = &self.proposal {
            self.last_telemetry =
                if p.telemetry.to_string().len() <= 8192 { p.telemetry.clone() } else { json!({"details":"oversized backend telemetry omitted"}) };
        }
        self.cancel.store(true, Ordering::Relaxed);
        self.receiver = None;
        self.snapshot = None;
        self.proposal = None;
        self.state = state.into();
        self.message.clear();
        self.message = match state {
            "accepted" => "Approved preview applied",
            "cancelled" => "Cancelled; artwork unchanged",
            "rejected" => "Preview rejected; artwork unchanged",
            "stale" => "Artwork changed or document switched; generate a new preview",
            _ => "",
        }
        .into();
        if state != "reset" {
            self.record_event();
        }
    }
    pub fn record_event(&mut self) {
        if self.events.len() >= 32 {
            self.events.pop_front();
        }
        let details = if matches!(self.state.as_str(), "ready" | "accepted") { self.last_telemetry.clone() } else { Value::Null };
        self.events
            .push_back(json!({"job":self.job,"document":self.uid,"kind":self.kind,"state":self.state,"message":self.message,"telemetry":details}));
    }
    pub fn telemetry(&self) -> Value {
        json!({"history":self.events,"lastCompleted":self.last_telemetry,"limit":32,"scope":"local job metadata; no raw pen samples or artwork"})
    }
    pub fn status(&self) -> Value {
        json!({"state":if self.state.is_empty(){"idle"}else{&self.state},"kind":self.kind,"job":self.job,"document":self.uid,"message":self.message,
        "capabilities":worker::capabilities(),"junctions":self.proposal.as_ref().map(|p|&p.junctions),"rasterSize":self.proposal.as_ref().filter(|p|p.kind=="brush").map(|p|p.size),
        "telemetry":self.proposal.as_ref().map(|p|&p.telemetry).or_else(||(self.state=="accepted").then_some(&self.last_telemetry)),
        "lastCompletedTelemetry":self.last_telemetry,"experimental":true})
    }
}
impl Session {
    pub fn poll_research_tools(&mut self) {
        if self.research_tools.snapshot.is_some()
            && !self
                .active()
                .is_some_and(|d| d.uid == self.research_tools.uid && self.research_tools.snapshot.as_ref().is_some_and(|s| Arc::ptr_eq(s, &d.doc)))
        {
            self.research_tools.clear("stale");
            self.research_tools.message = "Artwork changed or document switched; generate a new preview".into();
            return;
        }
        if let Some(rx) = &self.research_tools.receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.research_tools.receiver = None;
                    match result {
                        Ok(p) => {
                            self.research_tools.state = "ready".into();
                            self.research_tools.message = if p.kind == "junction" {
                                format!("{} confidence-ranked junction suggestions. Nothing applied.", p.junctions.len())
                            } else {
                                "Neural raster appearance ready. Original vector strokes are unchanged.".into()
                            };
                            self.research_tools.proposal = Some(p);
                        }
                        Err(e) => {
                            self.research_tools.state = "failed".into();
                            self.research_tools.message = e;
                        }
                    }
                    if let Some(p) = &self.research_tools.proposal {
                        self.research_tools.last_telemetry = p.telemetry.clone();
                    }
                    self.research_tools.record_event();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.research_tools.receiver = None;
                    self.research_tools.state = "failed".into();
                    self.research_tools.message = "Research worker disconnected".into();
                    self.research_tools.record_event();
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    pub(crate) fn start_research_tool(&mut self, kind: &str, ids: &[NodeId], gap: f64, threshold: f64, seed: u32, color: [f64; 3]) -> Result<Value> {
        self.poll_research_tools();
        if self.research_tools.running() {
            return Err(error("Cancel the current research job first"));
        }
        if self.tool_busy() || self.doc()?.interaction.is_some() {
            return Err(error("Finish the drawing gesture first"));
        }
        let minimum = if kind == "brush" { 1 } else { 2 };
        if !(minimum..=64).contains(&ids.len()) {
            return Err(error(format!("Select {minimum}–64 ordinary open vector strokes")));
        }
        let doc = &self.doc()?.doc;
        let mut originals = Vec::new();
        let mut strokes = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut total = 0;
        for id in ids {
            let n = doc.node(*id).ok_or_else(|| error("Missing source stroke"))?;
            if !seen.insert(*id) || !doc.is_editable(*id) || !crate::cleanup::plain(n) {
                return Err(error(
                    "Requires distinct, visible ordinary open strokes. Pressure/nib/brush effects, masks and fills are not supported by this first adapter.",
                ));
            }
            if doc.ancestry(*id).is_some_and(|a| {
                a.iter().any(|id| {
                    doc.node(*id).is_some_and(|n| {
                        n.mask.is_some()
                            || !n.appearance.effects.is_empty()
                            || matches!(n.kind, NodeKind::Group { clip: true, .. } | NodeKind::Layer { clip: true, .. })
                    })
                })
            }) {
                return Err(error("Masked or effect containers are not supported"));
            }
            let path = n.path_data().ok_or_else(|| error("Not a path"))?;
            let mut length = 0.0;
            for sp in &path.subpaths {
                for a in &sp.anchors {
                    for p in [a.p, a.h_in, a.h_out] {
                        if !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 1e7 || p.y.abs() > 1e7 {
                            return Err(error("Invalid source geometry"));
                        }
                    }
                    length += a.p.distance(a.h_in) + a.p.distance(a.h_out);
                }
            }
            if length > 10000.0 {
                return Err(error("Stroke sampling budget exceeded"));
            }
            let mut points = Vec::new();
            kurbo::flatten(path.to_bezpath().iter(), 0.25, |el| {
                if let PathEl::MoveTo(p) | PathEl::LineTo(p) = el
                    && points.len() <= 4096
                {
                    points.push(p);
                }
            });
            total += points.len();
            if !(2..=4096).contains(&points.len()) || total > 16384 {
                return Err(error("Select a smaller stroke set"));
            }
            originals.push(n.clone());
            strokes.push(Stroke { id: *id, points, width: n.appearance.stroke_width() });
        }
        let mut region = Rect::ZERO;
        let mut first = true;
        for s in &strokes {
            for p in &s.points {
                let r = Rect::new(p.x - s.width, p.y - s.width, p.x + s.width, p.y + s.width);
                region = if first { r } else { region.union(r) };
                first = false;
            }
        }
        region = region.inflate(8.0, 8.0);
        if kind == "brush" {
            let mut positions = Vec::new();
            let mut parent = None;
            for (i, n) in originals.iter().enumerate() {
                let (p, at, _) = doc.position(n.id).ok_or_else(|| error("Missing parent"))?;
                if i > 0 && p != parent {
                    return Err(error("Neural appearance currently needs strokes in one parent"));
                }
                parent = p;
                positions.push(at);
            }
            positions.sort_unstable();
            if positions.windows(2).any(|w| w[1] != w[0] + 1) {
                return Err(error("Select consecutive sibling strokes for neural appearance"));
            }
        }
        let scale = (480.0 / region.width().max(region.height()).max(1.0)).min(4.0);
        let size = [(region.width() * scale).ceil().clamp(32.0, 512.0) as u32, (region.height() * scale).ceil().clamp(32.0, 512.0) as u32];
        // Extend the region when the minimum raster dimension adds blank space.
        region = Rect::new(region.x0, region.y0, region.x0 + size[0] as f64 / scale, region.y0 + size[1] as f64 / scale);
        let proposal = Proposal { max_gap: gap, kind: kind.into(), originals, junctions: vec![], png: vec![], region, size, telemetry: Value::Null };
        let input = Input { kind: kind.into(), strokes, proposal, gap, threshold, seed, color };
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        worker::spawn(input, cancel.clone(), tx).map_err(error)?;
        self.research_tools.clear("reset");
        self.research_tools.state = "running".into();
        self.research_tools.uid = self.doc()?.uid;
        self.research_tools.snapshot = Some(self.doc()?.doc.clone());
        self.research_tools.cancel = cancel;
        self.research_tools.receiver = Some(rx);
        self.research_tools.kind = kind.into();
        self.research_tools.job += 1;
        self.research_tools.record_event();
        Ok(self.research_tools.status())
    }
    pub(crate) fn apply_research_tool(&mut self, job: Option<u64>, chosen: Option<Vec<usize>>, resolved: Option<Proposal>) -> Result<Value> {
        self.poll_research_tools();
        let p = if let Some(p) = resolved {
            p
        } else {
            if job.is_some_and(|j| j != self.research_tools.job) || self.research_tools.state != "ready" {
                return Err(error("No current ready proposal"));
            }
            self.research_tools.proposal.clone().ok_or_else(|| error("No preview"))?
        };
        if p.originals.len() > 64 || p.originals.is_empty() || p.png.len() > 4_000_000 || p.junctions.len() > 4096 {
            return Err(error("Proposal limits exceeded"));
        }
        for n in &p.originals {
            if self.doc()?.doc.node(n.id) != Some(n) || !self.doc()?.doc.is_editable(n.id) {
                return Err(error("Source strokes changed"));
            }
        }
        if p.kind == "junction" {
            if !p.max_gap.is_finite() || !(0.1..=32.0).contains(&p.max_gap) {
                return Err(error("Invalid authorized gap"));
            }
            let chosen = chosen.unwrap_or_else(|| (0..p.junctions.len()).collect());
            if chosen.is_empty() {
                return Err(error("No junction suggestions to accept"));
            }
            let mut edits = Vec::new();
            let mut endpoints = std::collections::BTreeSet::new();
            for i in chosen {
                let j = p.junctions.get(i).ok_or_else(|| error("Invalid suggestion index"))?;
                if !endpoints.insert((j.a, j.tail)) {
                    return Err(error("Conflicting suggestions move the same endpoint; accept individually"));
                }
                let a = p.originals.iter().find(|n| n.id == j.a).and_then(Node::path_data).ok_or_else(|| error("Invalid junction source"))?;
                let b = p.originals.iter().find(|n| n.id == j.b).and_then(Node::path_data).ok_or_else(|| error("Invalid junction target"))?;
                let sp = a.subpaths.first().ok_or_else(|| error("Empty path"))?;
                let anchor = if j.tail { sp.anchors.last() } else { sp.anchors.first() }.ok_or_else(|| error("Empty source"))?;
                let q = b.nearest(j.point).ok_or_else(|| error("Empty junction target"))?.3;
                if !j.confidence.is_finite()
                    || !(0.0..=1.0).contains(&j.confidence)
                    || anchor.p.distance(q) > p.max_gap + 0.001
                    || q.distance(j.point) > 0.5
                {
                    return Err(error("Invalid junction displacement"));
                }
                edits.push((j.a, j.tail, q));
            }
            self.edit("Accept junction suggestions", |d, _| {
                for (id, tail, q) in &edits {
                    let n = d.node_mut(*id).ok_or_else(|| error("Missing source"))?;
                    let path = n.path_data_mut().ok_or_else(|| error("Source is not a path"))?;
                    let sp = path.subpaths.first_mut().ok_or_else(|| error("Empty path"))?;
                    let a = if *tail { sp.anchors.last_mut() } else { sp.anchors.first_mut() }.ok_or_else(|| error("Empty path"))?;
                    let delta = *q - a.p;
                    a.p = *q;
                    a.h_in += delta;
                    a.h_out += delta;
                }
                Ok(json!({"junctionsApplied":edits.len()}))
            })
        } else if p.kind == "brush" {
            let dimensions = image::ImageReader::with_format(std::io::Cursor::new(&p.png), image::ImageFormat::Png)
                .into_dimensions()
                .map_err(|e| error(e.to_string()))?;
            if dimensions != (p.size[0], p.size[1])
                || p.size.iter().any(|v| *v < 32 || *v > 512)
                || [p.region.x0, p.region.y0, p.region.x1, p.region.y1].iter().any(|v| !v.is_finite() || v.abs() > 1e7)
                || p.region.width() <= 0.0
                || p.region.height() <= 0.0
            {
                return Err(error("Invalid raster appearance dimensions"));
            }
            // No external image links: accepted appearance survives native save/reopen.
            self.edit("Accept neural brush appearance", |d, sel| {
                let leader = p.originals.first().ok_or_else(|| error("No source"))?;
                let (parent, at, _) = d.position(leader.id).ok_or_else(|| error("Missing parent"))?;
                let mut children = Vec::new();
                for original in &p.originals {
                    let mut n = (*d.remove(original.id)?).clone();
                    n.visible = false;
                    children.push(Arc::new(n));
                }
                let id = d.alloc_id();
                let key = format!("neube-{}", id.0);
                d.images.insert(key.clone(), ImageBlob::new("image/png", p.png.clone()));
                let xf = Affine::translate(p.region.origin().to_vec2())
                    * Affine::scale_non_uniform(p.region.width() / p.size[0] as f64, p.region.height() / p.size[1] as f64);
                let mut raster = Node::new(
                    id,
                    NodeKind::Image(ImageObject { key, width: p.size[0], height: p.size[1], xf, link: None, placement: Default::default() }),
                );
                raster.name = Some("NeuBE raster appearance (regenerate after source edits)".into());
                children.push(Arc::new(raster));
                let mut group = Node::group(d.alloc_id(), children);
                group.name = Some("Neural brush + editable vector sources".into());
                group.attrs = Some(Box::new(vectorcraft_doc::ObjectAttributes {
                    note:
                        "NeuBE raster appearance. Use Release Neural Appearance to restore visible editable vectors, then regenerate after editing."
                            .into(),
                    data: vec![
                        ("rainee-neube".into(), "1".into()),
                        ("raster-id".into(), id.0.to_string()),
                        ("telemetry".into(), p.telemetry.to_string()),
                    ],
                    ..Default::default()
                }));
                let gid = group.id;
                d.insert(parent, at, group)?;
                sel.set([gid]);
                Ok(json!({"group":gid,"raster":id,"sourceIds":p.originals.iter().map(|n|n.id).collect::<Vec<_>>(),"rasterAppearance":true}))
            })
        } else {
            Err(error("Unknown research proposal kind"))
        }
    }
}
