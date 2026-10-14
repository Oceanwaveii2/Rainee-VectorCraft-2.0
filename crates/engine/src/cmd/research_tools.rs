use super::*;
use crate::research_tools::Proposal;
use serde_json::json;
use vectorcraft_doc::NodeKind;
pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "assist.junction.preview","Preview learned junction suggestions",[],None,"{ids?:selected stable IDs, gap?:0.1..32 pt (8), confidence?:0..1 (.5)} Optional SketchConnectivity worker. Plain open paths; preview only, no global rewiring.",has_doc,junction),
        cmd!(query "assist.brush.preview","Preview neural natural brush",[],None,"{ids?:selected stable IDs, seed?:0..4294967295 (594), color?:[r,g,b] in 0..1} Optional NeuBE CUDA worker. Plain open paths, bounded raster appearance, never replaces source vectors.",has_doc,brush),
        cmd!(query "assist.status","Research tool status",[],None,"{} Worker capabilities, job/document, errors, proposals and model timing; refreshes stale-state checks",always,status),
        cmd!(query "assist.telemetry","Research tool telemetry",[],None,"{} Last 32 local job lifecycle records and retained backend timings; contains no raw pen samples or artwork",always,telemetry),
        cmd!(query "assist.cancel","Cancel research tool",[],None,"{} Cancel bounded worker; no document changes",always,cancel),
        cmd!(query "assist.reject","Reject research preview",[],None,"{} Discard proposal; original art unchanged",always,reject),
        cmd!(
            "assist.accept",
            "Accept research preview",
            [],
            None,
            "{job?: current token, indices?:junction suggestion indices, proposal?:resolved journal proposal} Revalidate captured sources; approved edits in one undo step. No automatic acceptance.",
            has_doc,
            accept
        ),
        cmd!(
            "assist.brush.release",
            "Release neural raster appearance",
            [],
            None,
            "{id?:selected neural appearance group} Restore original editable vectors, remove only generated raster, one undo step; then regenerate after source edits.",
            has_doc,
            release
        ),
    ]
}
fn number(p: &Value, key: &str, default: f64, min: f64, max: f64) -> Result<f64> {
    p.get(key).map_or(Ok(default), |v| {
        v.as_f64().filter(|v| v.is_finite() && (*v >= min && *v <= max)).ok_or_else(|| bad("assist.preview", format!("{key} must be {min}..{max}")))
    })
}
fn preview(s: &mut Session, p: &Value, kind: &str) -> Result<Value> {
    let ids = if p.get("ids").is_some() { checked_ids_param(s, p, "ids", "assist.preview")? } else { s.doc()?.selection.objects.clone() };
    let gap = number(p, "gap", 8.0, 0.1, 32.0)?;
    let threshold = number(p, "confidence", 0.5, 0.0, 1.0)?;
    let seed = match p.get("seed") {
        None => 594,
        Some(v) => v.as_u64().and_then(|v| u32::try_from(v).ok()).ok_or_else(|| bad("assist.brush.preview", "seed must be a u32"))?,
    };
    let color = match p.get("color") {
        None => [0.1, 0.25, 0.65],
        Some(v) => {
            let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| bad("assist.brush.preview", "color requires three channels"))?;
            let mut c = [0.0; 3];
            for (i, v) in a.iter().enumerate() {
                c[i] = v
                    .as_f64()
                    .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
                    .ok_or_else(|| bad("assist.brush.preview", "color channels must be 0..1"))?;
            }
            c
        }
    };
    let r = s.start_research_tool(kind, &ids, gap, threshold, seed, color);
    if let Err(e) = &r
        && !s.research_tools.running()
    {
        s.research_tools.clear("failed");
        s.research_tools.message = e.to_string();
    }
    r
}
fn junction(s: &mut Session, p: &Value) -> Result<Value> {
    preview(s, p, "junction")
}
fn brush(s: &mut Session, p: &Value) -> Result<Value> {
    preview(s, p, "brush")
}
fn status(s: &mut Session, _: &Value) -> Result<Value> {
    s.poll_research_tools();
    Ok(s.research_tools.status())
}
fn telemetry(s: &mut Session, _: &Value) -> Result<Value> {
    s.poll_research_tools();
    Ok(s.research_tools.telemetry())
}
fn cancel(s: &mut Session, _: &Value) -> Result<Value> {
    s.research_tools.clear("cancelled");
    Ok(s.research_tools.status())
}
fn reject(s: &mut Session, _: &Value) -> Result<Value> {
    s.research_tools.clear("rejected");
    Ok(s.research_tools.status())
}
fn accept(s: &mut Session, p: &Value) -> Result<Value> {
    let job = p.get("job").map(|v| v.as_u64().ok_or_else(|| bad("assist.accept", "job must be an integer"))).transpose()?;
    let indices = p
        .get("indices")
        .map(|v| {
            v.as_array().ok_or_else(|| bad("assist.accept", "indices must be an array")).and_then(|a| {
                a.iter()
                    .map(|v| v.as_u64().and_then(|v| usize::try_from(v).ok()).ok_or_else(|| bad("assist.accept", "invalid index")))
                    .collect::<Result<Vec<_>>>()
            })
        })
        .transpose()?;
    let resolved =
        p.get("proposal").map(|v| serde_json::from_value::<Proposal>(v.clone()).map_err(|e| bad("assist.accept", e.to_string()))).transpose()?;
    let mut proposal = resolved.clone().or_else(|| s.research_tools.proposal.clone()).ok_or_else(|| bad("assist.accept", "No ready proposal"))?;
    if let Some(ref indices) = indices {
        proposal.junctions = indices
            .iter()
            .map(|i| proposal.junctions.get(*i).cloned().ok_or_else(|| bad("assist.accept", "invalid suggestion index")))
            .collect::<Result<Vec<_>>>()?;
    }
    let result = s.apply_research_tool(job, indices, resolved)?;
    // Store resolved, approved geometry for offline journal replay, not an inference rerun.
    s.note_journal("proposal", serde_json::to_value(&proposal).map_err(|e| bad("assist.accept", e.to_string()))?);
    s.note_journal("indices", json!((0..proposal.junctions.len()).collect::<Vec<_>>()));
    s.research_tools.clear("accepted");
    Ok(result)
}
fn release(s: &mut Session, p: &Value) -> Result<Value> {
    let id = p
        .get("id")
        .and_then(Value::as_u64)
        .map(NodeId)
        .or_else(|| s.active()?.selection.objects.first().copied())
        .ok_or_else(|| bad("assist.brush.release", "Select one neural appearance group"))?;
    if !s.doc()?.doc.is_editable(id) {
        return Err(bad("assist.brush.release", "Group is not editable"));
    }
    let group = s.doc()?.doc.node(id).ok_or_else(|| bad("assist.brush.release", "Missing group"))?;
    let attrs = group.extras().attrs.as_ref().ok_or_else(|| bad("assist.brush.release", "Not a neural appearance group"))?;
    if !attrs.data.iter().any(|(k, v)| k == "rainee-neube" && v == "1") {
        return Err(bad("assist.brush.release", "Not a neural appearance group"));
    }
    let raster = attrs
        .data
        .iter()
        .find(|(k, _)| k == "raster-id")
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .map(NodeId)
        .ok_or_else(|| bad("assist.brush.release", "Missing raster marker"))?;
    let NodeKind::Group { children, clip: false } = &group.kind else {
        return Err(bad("assist.brush.release", "Invalid group"));
    };
    if !children.iter().any(|n| n.id == raster && matches!(n.kind, NodeKind::Image(_))) {
        return Err(bad("assist.brush.release", "Generated raster missing"));
    }
    let sources: Vec<_> = children.iter().filter(|n| n.id != raster).cloned().collect();
    if sources.is_empty() {
        return Err(bad("assist.brush.release", "No source vectors"));
    }
    let (parent, at, _) = s.doc()?.doc.position(id).ok_or_else(|| bad("assist.brush.release", "Missing group position"))?;
    s.edit("Release neural appearance", |d, sel| {
        d.remove(id)?;
        let mut ids = Vec::new();
        for (i, n) in sources.iter().enumerate() {
            let mut source = (**n).clone();
            source.visible = true;
            ids.push(d.insert(parent, at + i, source)?);
        }
        sel.set(ids.clone());
        Ok(json!({"ids":ids,"appearanceRemoved":true}))
    })
}
