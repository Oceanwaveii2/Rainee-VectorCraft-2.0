use super::*;
use crate::cleanup::{self, Plan};
use serde_json::json;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(query "sketch.cleanup.inspect", "Inspect Cleanup Selection", [], None,
            "{ids?: selected stable object IDs, tolerance?, maxDeviation?, timeout? as preview} → eligible, selected, reason. Read-only bounded preflight using the same admission checks as preview; does not run ML or alter art.", has_doc, inspect),
        cmd!(query "sketch.cleanup.start", "Preview Sketch Cleanup", [], None,
            "{ids?: selected stable object IDs, tolerance?:0.1..2 pt (default .25), maxDeviation?:.25..32 pt (default 4), timeout?:1..90 seconds (default 30)} Experimental Windows worker; identical base appearance open Art/Natural/non-random Calligraphic strokes. Leader pressure/tilt knots projected to approved centerline, not averaged; ambiguous projection refuses acceptance. Originals retained. Preview does not modify art.", has_doc, start),
        cmd!(query "sketch.cleanup.status", "Sketch Cleanup Status", [], None,
            "{} → job, document, state, backend availability, failure reason, proposal geometry; polls background work", always, status),
        cmd!(query "sketch.cleanup.cancel", "Cancel Sketch Cleanup", [], None, "{} Cancel background job; unchanged art", always, cancel),
        cmd!(query "sketch.cleanup.reject", "Reject Sketch Cleanup", [], None, "{} Discard preview; unchanged art", always, reject),
        cmd!(
            "sketch.cleanup.accept",
            "Accept Sketch Cleanup",
            [],
            None,
            "{job?: current job token; proposal?: resolved journal plan} One undo step; stable originals retained in hidden groups. Reject stale content/document; journal replay requires no worker.",
            has_doc,
            accept
        ),
    ]
}
fn inspect(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = if p.get("ids").is_some() { checked_ids_param(s, p, "ids", "sketch.cleanup.inspect")? } else { s.doc()?.selection.objects.clone() };
    let result = cleanup::capture(
        s,
        &ids,
        number(p, "tolerance", 0.25, 0.1, 2.0)?,
        number(p, "maxDeviation", 4.0, 0.25, 32.0)?,
        number(p, "timeout", 30.0, 1.0, 90.0)?,
    );
    Ok(json!({"eligible":result.is_ok(),"selected":ids.len(),
        "reason":result.err().map(|e|e.to_string()),"originalsUnchanged":true,
        "profilePolicy":"leader-projected; no averaging; originals retained"}))
}
fn number(p: &Value, key: &str, default: f64, min: f64, max: f64) -> Result<f64> {
    match p.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_f64()
            .filter(|v| v.is_finite() && *v >= min && *v <= max)
            .ok_or_else(|| bad("sketch.cleanup.start", format!("{key} must be {min}..{max}"))),
    }
}
fn start(s: &mut Session, p: &Value) -> Result<Value> {
    s.poll_cleanup();
    if s.cleanup.running() {
        return Err(bad("sketch.cleanup.start", "Cancel current job first"));
    }
    let ids = if p.get("ids").is_some() { checked_ids_param(s, p, "ids", "sketch.cleanup.start")? } else { s.doc()?.selection.objects.clone() };
    let input = cleanup::capture(
        s,
        &ids,
        number(p, "tolerance", 0.25, 0.1, 2.0)?,
        number(p, "maxDeviation", 4.0, 0.25, 32.0)?,
        number(p, "timeout", 30.0, 1.0, 90.0)?,
    );
    let result = input.and_then(|input| s.start_cleanup(input));
    if let Err(e) = &result {
        s.cleanup.stop("failed");
        s.cleanup.message = e.to_string();
    }
    result
}
fn status(s: &mut Session, _: &Value) -> Result<Value> {
    s.poll_cleanup();
    Ok(s.cleanup.status())
}
fn cancel(s: &mut Session, _: &Value) -> Result<Value> {
    s.cleanup.stop("cancelled");
    Ok(s.cleanup.status())
}
fn reject(s: &mut Session, _: &Value) -> Result<Value> {
    s.cleanup.stop("rejected");
    Ok(s.cleanup.status())
}
fn accept(s: &mut Session, p: &Value) -> Result<Value> {
    s.poll_cleanup();
    if p.get("proposal").is_none() && p.get("job").is_some_and(|v| v.as_u64() != Some(s.cleanup.serial)) {
        return Err(bad("sketch.cleanup.accept", "Wrong job token"));
    }
    let plan: Plan = if let Some(value) = p.get("proposal") {
        if value.to_string().len() > 4_000_000 {
            return Err(bad("sketch.cleanup.accept", "Proposal too large"));
        }
        serde_json::from_value(value.clone()).map_err(|e| bad("sketch.cleanup.accept", e.to_string()))?
    } else {
        s.cleanup.plan.clone().ok_or_else(|| bad("sketch.cleanup.accept", "No current ready preview"))?
    };
    let serialized = serde_json::to_value(&plan).map_err(|e| cleanup::error(e.to_string()))?;
    let result = s.apply_cleanup(plan)?;
    s.note_journal("proposal", serialized);
    s.cleanup.stop("accepted");
    Ok(result)
}
