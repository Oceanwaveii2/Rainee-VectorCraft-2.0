//! Causal replay laboratory. Future points are targets only, never inference inputs.
//! Experimental Android adaptation: fixed 240 Hz history, physical screen pixels.
//! Polar and tilt conventions adapted from AOSP (Copyright 2023 The Android Open Source Project).
//! SPDX-License-Identifier: Apache-2.0
use crate::{
    model::{Model, SHA256},
    pen_input::Phase,
    recording::Recording,
    stylus_tracker::Tracker,
};
use serde::Serialize;
use std::{collections::BTreeMap, io::Write, path::Path, time::Instant};
use vectorcraft_geom::Point;

const STEP: f64 = 0.004_166_666;
const HORIZONS: [f64; 3] = [12.0, 14.0, 16.0];
const NAMES: [&str; 6] = ["hold", "last_velocity", "existing_predictor", "existing_stabilized", "aosp_raw", "aosp_guarded"];

#[derive(Clone, Copy, Debug)]
struct Frame {
    time: f64,
    p: Point,
    pressure: Option<f32>,
    tilt: [Option<i32>; 2],
    dpi: u32,
}
#[derive(Default)]
struct Errors {
    errors: Vec<f64>,
    available: usize,
    better_than_hold: usize,
}
#[derive(Serialize)]
pub struct Metrics {
    targets: usize,
    predictions: usize,
    coverage: f64,
    mean_error_px: Option<f64>,
    p95_error_px: Option<f64>,
    p99_error_px: Option<f64>,
    max_error_px: Option<f64>,
    fraction_better_than_hold: Option<f64>,
}
impl Errors {
    fn add(&mut self, predicted: Option<Point>, current: Point, target: Point) {
        self.available += usize::from(predicted.is_some());
        let error = predicted.unwrap_or(current).distance(target);
        self.better_than_hold += usize::from(error < current.distance(target));
        self.errors.push(error);
    }
    fn finish(mut self) -> Metrics {
        let n = self.errors.len();
        self.errors.sort_by(f64::total_cmp);
        Metrics {
            targets: n,
            predictions: self.available,
            coverage: self.available as f64 / n.max(1) as f64,
            mean_error_px: (n > 0).then(|| self.errors.iter().sum::<f64>() / n as f64),
            p95_error_px: quantile(&self.errors, 0.95),
            p99_error_px: quantile(&self.errors, 0.99),
            max_error_px: self.errors.last().copied(),
            fraction_better_than_hold: (n > 0).then(|| self.better_than_hold as f64 / n as f64),
        }
    }
}
fn quantile(sorted: &[f64], q: f64) -> Option<f64> {
    sorted.get(((sorted.len().saturating_sub(1)) as f64 * q).ceil() as usize).copied()
}
#[derive(Serialize)]
pub struct Evaluation {
    schema: u32,
    evaluator_version: String,
    capture_sha256: String,
    model_sha256: String,
    model_description: String,
    model_executed: bool,
    model_inferences: usize,
    inference_mean_us: Option<f64>,
    inference_p95_us: Option<f64>,
    inference_max_us: Option<f64>,
    evaluated_strokes: usize,
    excluded_strokes: BTreeMap<String, usize>,
    model_history_unavailable: usize,
    model_noise_floor_abstentions: usize,
    training_performed: bool,
    document_edits: usize,
    promoted_to_editor: bool,
    limitations: Vec<String>,
    horizons: BTreeMap<String, BTreeMap<String, Metrics>>,
    per_stroke: Vec<StrokeEvaluation>,
}
#[derive(Serialize)]
struct StrokeEvaluation {
    id: usize,
    scenario: String,
    horizons: BTreeMap<String, BTreeMap<String, Metrics>>,
}

// Only interpolation between already-arrived observations is allowed for model history.
// The same helper is used separately for scoring future targets, never passed to the model.
fn at_time(frames: &[Frame], time: f64) -> Option<Frame> {
    let right = frames.partition_point(|f| f.time <= time);
    let a = *frames.get(right.checked_sub(1)?)?;
    if (a.time - time).abs() < 1e-9 {
        return Some(a);
    }
    let b = *frames.get(right)?;
    let dt = b.time - a.time;
    if dt <= 0.0 || dt > 0.020 || a.dpi != b.dpi {
        return None;
    }
    let t = (time - a.time) / dt;
    Some(Frame {
        time,
        p: a.p.lerp(b.p, t),
        pressure: a.pressure.zip(b.pressure).map(|(a, b)| a + (b - a) * t as f32),
        // Interpolating tilt slopes is a trial adaptation, not verified Wacom/Android sensor parity.
        tilt: std::array::from_fn(|i| a.tilt[i].zip(b.tilt[i]).map(|(a, b)| (a as f64 + (b - a) as f64 * t).round() as i32)),
        dpi: a.dpi,
    })
}

fn features(history: &[Frame], count: usize) -> Option<([Vec<f32>; 5], Point, Point)> {
    let now = history.last()?.time;
    let points = (0..count + 2).map(|i| at_time(history, now - (count + 1 - i) as f64 * STEP)).collect::<Option<Vec<_>>>()?;
    let mut input: [Vec<f32>; 5] = std::array::from_fn(|_| Vec::new());
    let mut axis_from: Option<Point> = None;
    let mut axis_to = points.first()?.p;
    for f in points.iter().skip(1) {
        let v = f.p - axis_to;
        let r = v.hypot();
        let axis = axis_from.map_or(v, |a| axis_to - a);
        let axis_phi = axis.y.atan2(axis.x);
        let phi = if axis_from.is_some() { v.y.atan2(v.x) - axis_phi } else { 0.0 };
        let tx = (f.tilt[0]? as f64).to_radians();
        let ty = (f.tilt[1]? as f64).to_radians();
        // Android InputReader tilt convention, not barrel rotation.
        let tilt = (tx.cos() * ty.cos()).clamp(-1.0, 1.0).acos();
        let orientation = (-tx.sin()).atan2(ty.sin());
        let orientation = if axis.hypot() > 0.0 { std::f64::consts::FRAC_PI_2 - orientation - axis_phi } else { 0.0 };
        for (channel, value) in input.iter_mut().zip([r as f32, phi as f32, f.pressure?, tilt as f32, orientation as f32]) {
            channel.push(value);
        }
        if r > 0.0 {
            axis_from = Some(axis_to);
            axis_to = f.p;
        }
    }
    // Extra prior segment supplies the reference direction for the first retained feature.
    for channel in &mut input {
        if !channel.is_empty() {
            channel.remove(0);
        }
    }
    Some((input, axis_from?, axis_to))
}
fn endpoints(output: &[Vec<f32>; 3], from: Point, to: Point) -> Option<[Point; 3]> {
    let mut axis_from = from;
    let mut axis_to = to;
    let mut path = vec![to];
    for (&r, &phi) in output.first()?.iter().zip(output.get(1)?) {
        if r < 0.2 || !r.is_finite() || !phi.is_finite() {
            break;
        }
        let a = axis_to - axis_from;
        let angle = a.y.atan2(a.x) + phi as f64;
        let next = axis_to + vectorcraft_geom::Vec2::new(angle.cos(), angle.sin()) * r as f64;
        axis_from = axis_to;
        axis_to = next;
        path.push(next);
    }
    let mut result = [to; 3];
    for (dest, ms) in result.iter_mut().zip(HORIZONS) {
        let step = ms / 1000.0 / STEP;
        let lo = step.floor() as usize;
        let a = *path.get(lo)?;
        let b = *path.get(lo + 1)?;
        *dest = a.lerp(b, step.fract());
    }
    Some(result)
}
fn bounded(current: Point, p: Point, dpi: u32) -> Point {
    let delta = p - current;
    let max = 24.0 * dpi as f64 / 96.0; // Same physical bound as baseline at zoom 1.
    current + delta * (max / delta.hypot().max(max))
}
fn empty_errors() -> BTreeMap<String, BTreeMap<String, Errors>> {
    HORIZONS.into_iter().map(|h| (h.to_string(), NAMES.into_iter().map(|name| (name.into(), Errors::default())).collect())).collect()
}
fn finish(errors: BTreeMap<String, BTreeMap<String, Errors>>) -> BTreeMap<String, BTreeMap<String, Metrics>> {
    errors.into_iter().map(|(h, group)| (h, group.into_iter().map(|(name, errors)| (name, errors.finish())).collect())).collect()
}

pub fn evaluate(capture: &Recording, model: &mut Model) -> Result<Evaluation, String> {
    let review = capture.report();
    if !review.warnings.is_empty() {
        return Err("Capture quality warnings must be resolved before prediction evaluation".into());
    }
    let mut report = Evaluation {
        schema: 1,
        evaluator_version: env!("CARGO_PKG_VERSION").into(),
        capture_sha256: review.source_sha256,
        model_sha256: SHA256.into(),
        model_description: model.describe(),
        model_executed: false,
        model_inferences: 0,
        inference_mean_us: None,
        inference_p95_us: None,
        inference_max_us: None,
        evaluated_strokes: 0,
        excluded_strokes: BTreeMap::new(),
        model_history_unavailable: 0,
        model_noise_floor_abstentions: 0,
        training_performed: false,
        document_edits: 0,
        promoted_to_editor: false,
        horizons: BTreeMap::new(),
        per_stroke: Vec::new(),
        limitations: vec![
            "Unlabelled sessions cannot establish corner/reversal/pressure-specific acceptance".into(),
            "Offline endpoint error is not measured pen-to-photon latency".into(),
            "Rust kernels have not been cross-checked against the TensorFlow Lite runtime".into(),
            "Android fixed-rate and tilt adaptation require hardware validation; pretrained training-data provenance is not established".into(),
            "All methods scored on identical future targets; unavailable prediction falls back to current measured position".into(),
            "Existing predictors run in 96-DPI logical units at zoom 1; ML uses physical screen pixels; no heuristic tuning on these captures".into(),
        ],
    };
    let mut all = empty_errors();
    let mut timings = Vec::new();
    for stroke in &capture.strokes {
        let samples = stroke.sample_indices.iter().map(|i| capture.samples.get(*i).ok_or("Bad stroke index")).collect::<Result<Vec<_>, _>>()?;
        let excluded = if !stroke.complete {
            Some("incomplete")
        } else if samples.iter().any(|s| s.normalized.eraser || s.normalized.inverted) {
            Some("eraser_or_inverted")
        } else {
            None
        };
        if let Some(reason) = excluded {
            *report.excluded_strokes.entry(reason.into()).or_default() += 1;
            continue;
        }
        let first = samples.first().ok_or("Empty stroke")?;
        let origin = first.raw.performance_count;
        let freq = first.raw.performance_frequency;
        if origin == 0 || freq == 0 || first.raw.window_dpi == 0 {
            return Err("Missing clock/DPI".into());
        }
        let mut frames = Vec::new();
        for s in &samples {
            if s.raw.device != first.raw.device || s.raw.window_dpi != first.raw.window_dpi || s.raw.performance_frequency != freq {
                return Err("Device/DPI/clock changed within stroke".into());
            }
            let ticks = s.raw.performance_count.checked_sub(origin).ok_or("Reversed clock")?;
            let f = Frame {
                time: ticks as f64 / freq as f64,
                p: Point::new(s.raw.screen_px[0] as f64, s.raw.screen_px[1] as f64),
                pressure: s.normalized.pressure,
                tilt: s.normalized.tilt_degrees,
                dpi: s.raw.window_dpi,
            };
            if frames.last().is_some_and(|p: &Frame| f.time <= p.time) {
                return Err("Repeated/reversed timestamp within stroke".into());
            }
            frames.push(f);
        }
        report.evaluated_strokes += 1;
        let mut local = empty_errors();
        let mut raw = Tracker::default();
        let mut stabilized = Tracker::default();
        for (index, f) in frames.iter().enumerate() {
            // Endpoints are computed from this prefix only. Future target lookup happens below.
            let history = frames.get(..=index).ok_or("Missing prefix")?;
            let scale = f.dpi as f64 / 96.0;
            raw.sample(Point::new(f.p.x / scale, f.p.y / scale), Some(f.time), false, 1.0, 0.01);
            stabilized.sample(Point::new(f.p.x / scale, f.p.y / scale), Some(f.time), true, 1.0, 0.01);
            // Never infer a new overlay at pen-up; existing samples still supply scoring targets.
            if capture.samples.get(*stroke.sample_indices.get(index).ok_or("Missing phase")?).is_some_and(|s| s.normalized.phase == Phase::Up) {
                break;
            }
            let learned = if let Some((input, from, to)) = features(history, model.input_length) {
                let clock = Instant::now();
                let output = model.infer(&input)?;
                timings.push(clock.elapsed().as_secs_f64() * 1e6);
                report.model_inferences += 1;
                report.model_executed = true;
                let points = endpoints(&output, from, to);
                if points.is_none() {
                    report.model_noise_floor_abstentions += 1;
                }
                points
            } else {
                report.model_history_unavailable += 1;
                None
            };
            for (h, ms) in HORIZONS.iter().enumerate() {
                let map_back = |p: Point| Point::new(p.x * scale, p.y * scale);
                let baseline = raw.prediction(*ms, 1.0).map(|(_, p)| map_back(p));
                let filtered = stabilized.prediction(*ms, 1.0).map(|(_, p)| map_back(p));
                let linear = index.checked_sub(1).and_then(|i| history.get(i)).and_then(|prev| {
                    let dt = f.time - prev.time;
                    (dt > 0.0 && dt <= 0.020).then(|| f.p + (f.p - prev.p) * (*ms / 1000.0 / dt))
                });
                let ml = learned.and_then(|p| p.get(h).copied());
                let guarded = baseline.and(ml).map(|p| bounded(f.p, p, f.dpi));
                let predictions = [Some(f.p), linear, baseline, filtered, ml, guarded];
                // Target data is never routed back into predictors or the resampled history.
                let Some(target) = at_time(&frames, f.time + ms / 1000.0) else {
                    continue;
                };
                let key = ms.to_string();
                for (name, predicted) in NAMES.iter().zip(predictions) {
                    all.get_mut(&key).and_then(|group| group.get_mut(*name)).ok_or("Missing aggregate")?.add(predicted, f.p, target.p);
                    local.get_mut(&key).and_then(|group| group.get_mut(*name)).ok_or("Missing stroke metric")?.add(predicted, f.p, target.p);
                }
            }
        }
        report.per_stroke.push(StrokeEvaluation { id: stroke.id, scenario: format!("{:?}", stroke.scenario), horizons: finish(local) });
        // Exercise the same pause-expiry API used by the canvas without changing saved geometry.
        raw.tick(0.040);
        if raw.prediction(14.0, 1.0).is_some() {
            return Err("Baseline failed to expire at pause".into());
        }
        raw.reset();
        stabilized.reset();
    }
    if timings.is_empty() {
        return Err("No deployable model inference: insufficient supported history".into());
    }
    report.inference_mean_us = Some(timings.iter().sum::<f64>() / timings.len() as f64);
    timings.sort_by(f64::total_cmp);
    report.inference_p95_us = quantile(&timings, 0.95);
    report.inference_max_us = timings.last().copied();
    report.horizons = finish(all);
    Ok(report)
}
pub fn save(report: &Evaluation, path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Report destination must be absolute".into());
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(path).map_err(|e| e.to_string())?;
    file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line() -> Vec<Frame> {
        (0..30)
            .map(|i| Frame { time: i as f64 * STEP, p: Point::new(i as f64 * 4.0, 0.0), pressure: Some(0.5), tilt: [Some(0), Some(0)], dpi: 192 })
            .collect()
    }
    #[test]
    fn causal_features_cannot_read_future_targets() {
        let mut frames = line();
        let original = features(&frames[..12], 5).unwrap();
        for f in &mut frames[12..] {
            f.p = Point::new(-9999.0, 7777.0);
            f.pressure = None;
        }
        assert_eq!(features(&frames[..12], 5).unwrap(), original);
        assert!(at_time(&frames[..12], frames[11].time + STEP).is_none());
    }
    #[test]
    fn regular_resampling_missing_channels_and_pause() {
        let mut frames = line();
        let f = features(&frames, 5).unwrap().0;
        assert!(f[0].iter().all(|r| (*r - 4.0).abs() < 1e-4));
        assert!(f[1].iter().all(|phi| phi.abs() < 1e-6));
        assert!(f[3].iter().all(|tilt| tilt.abs() < 1e-6));
        frames[29].tilt[0] = None;
        assert!(features(&frames, 5).is_none());
        let mut frames = line();
        frames[29].time += 0.1;
        assert!(features(&frames, 5).is_none());
    }
    #[test]
    fn exact_horizon_and_physical_bound() {
        let output = [vec![4.0; 5], vec![0.0; 5], vec![0.5; 5]];
        let p = endpoints(&output, Point::ZERO, Point::new(4.0, 0.0)).unwrap();
        for (p, h) in p.iter().zip(HORIZONS) {
            assert!((p.x - (4.0 + h / 1000.0 / STEP * 4.0)).abs() < 1e-5);
        }
        assert_eq!(bounded(Point::ZERO, Point::new(1000.0, 0.0), 192), Point::new(48.0, 0.0));
        let stationary = [vec![0.0; 5], vec![0.0; 5], vec![0.5; 5]];
        assert!(endpoints(&stationary, Point::ZERO, Point::new(1.0, 0.0)).is_none());
    }
    #[test]
    fn stationary_and_short_history_do_not_infer() {
        let mut f = line();
        assert!(features(&f[..3], 5).is_none());
        for p in &mut f {
            p.p = Point::ZERO;
        }
        assert!(features(&f, 5).is_none());
    }
    #[test]
    fn all_methods_score_abstentions_on_the_same_targets() {
        let mut errors = Errors::default();
        errors.add(None, Point::ZERO, Point::new(3.0, 4.0));
        errors.add(Some(Point::new(3.0, 4.0)), Point::ZERO, Point::new(3.0, 4.0));
        let metrics = errors.finish();
        assert_eq!(metrics.targets, 2);
        assert_eq!(metrics.coverage, 0.5);
        assert_eq!(metrics.mean_error_px, Some(2.5));
    }
    #[test]
    fn real_capture_evaluation_preserves_authoritative_data_and_report_destination() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model_path = base.join("models/aosp/motion_predictor_model.tflite");
        let capture_path = base.join("captures/cintiq-1791713023544-99848.jsonl");
        if !model_path.exists() || !capture_path.exists() {
            eprintln!("Owner recording/model absent; real evaluation must be run separately");
            return;
        }
        let capture = Recording::read(&capture_path).unwrap();
        let before = serde_json::to_vec(&(&capture.samples, &capture.strokes)).unwrap();
        let mut model = Model::load(&model_path).unwrap();
        let report = evaluate(&capture, &mut model).unwrap();
        assert!(report.model_executed);
        assert_eq!(report.document_edits, 0);
        assert!(!report.training_performed && !report.promoted_to_editor);
        for group in report.horizons.values() {
            let n = group.get("hold").unwrap().targets;
            assert!(group.values().all(|m| m.targets == n));
        }
        assert_eq!(before, serde_json::to_vec(&(&capture.samples, &capture.strokes)).unwrap());
        let dest = std::env::temp_dir().join(format!("cintiq-prediction-{}.json", std::process::id()));
        save(&report, &dest).unwrap();
        assert!(save(&report, &dest).is_err());
        assert!(save(&report, Path::new("relative.json")).is_err());
        std::fs::remove_file(dest).unwrap();
    }
}
