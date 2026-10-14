//! Opt-in AOSP inference. Bounded asynchronous jobs; physical pixels in, disposable delta out.
#[cfg(not(target_arch = "wasm32"))]
use crate::motion_model::Model;
use crate::stylus::OneEuro1D;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, SyncSender};
use vectorcraft_geom::{Point, Vec2};

const STEP: f64 = 0.004_166_666;
pub const MODEL_PATH: &str = "D:/0/Rainee drawing app/ArtCraft-Foundation/VectorCraft - Gemini Edition/integrations/cintiq-input-probe/models/aosp/motion_predictor_model.tflite";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub time: f64,
    pub position: Point,
    pub pressure: f32,
    pub tilt: [f64; 2],
    /// Document displacement of one physical screen pixel on each axis.
    pub basis: [Vec2; 2],
}

/// 1 Euro angular filter for 2-DOF stylus tilt (`tilt_x`, `tilt_y` in degrees) to eliminate
/// high-frequency transducer jitter during Cosserat bristle and calligraphic nib evaluation.
#[derive(Clone, Debug, Default)]
pub struct StabilizedTilt {
    tx: OneEuro1D,
    ty: OneEuro1D,
    last: Option<[f64; 2]>,
}

impl StabilizedTilt {
    pub fn reset(&mut self) {
        self.tx.reset();
        self.ty.reset();
        self.last = None;
    }

    /// Filters a `[tilt_x, tilt_y]` pair (degrees in `[-90, 90]`) at timestamp `t` (seconds).
    pub fn sample(&mut self, tilt: [f64; 2], t: f64) -> [f64; 2] {
        if !t.is_finite() || !tilt.iter().all(|a| a.is_finite() && (-90.0..=90.0).contains(a)) {
            return self.last.unwrap_or([0.0, 0.0]);
        }
        let fx = self.tx.filter(tilt[0], t, 1.5, 0.02).clamp(-90.0, 90.0);
        let fy = self.ty.filter(tilt[1], t, 1.5, 0.02).clamp(-90.0, 90.0);
        let out = [fx, fy];
        self.last = Some(out);
        out
    }

    #[must_use]
    pub fn current(&self) -> Option<[f64; 2]> {
        self.last
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct Work {
    epoch: u64,
    sample_revision: u64,
    sample: Sample,
    features: [Vec<f32>; 5],
    from: Point,
    to: Point,
    horizon: f64,
}
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
enum Reply {
    Loaded,
    Error(String),
    Predicted(u64, u64, Vec2, f64),
    PredictedWithPressure(u64, u64, Vec2, f32, f64),
}
struct Worker {
    tx: SyncSender<Work>,
    rx: Receiver<Reply>,
}
#[derive(Default)]
pub struct Learned {
    enabled: bool,
    worker: Option<Worker>,
    history: Vec<Sample>,
    epoch: u64,
    /// Advances on actual input, including across stroke/history resets.
    sample_revision: u64,
    age: f64,
    delta: Option<Vec2>,
    predicted_pressure: Option<f32>,
    tilt_filter: StabilizedTilt,
    status: String,
    pub inferences: u64,
    pub skipped: u64,
    pub last_us: f64,
    pub max_us: f64,
    pub stale_results: u64,
}
impl Learned {
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn status(&self) -> &str {
        if !self.enabled {
            "off"
        } else if self.status.is_empty() {
            "waiting for measured pen input"
        } else {
            &self.status
        }
    }
    pub fn reset(&mut self) {
        self.history.clear();
        self.delta = None;
        self.predicted_pressure = None;
        self.tilt_filter.reset();
        self.epoch = self.epoch.wrapping_add(1);
        self.age = 1.0;
    }
    pub fn enable(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.reset();
        self.enabled = enabled;
        if !enabled {
            self.worker = None;
            self.status.clear();
            return;
        }
        self.status.clear();
    }
    fn start_worker(&mut self) {
        if self.worker.is_some() {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.status = "unavailable: native model worker only".into();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, requests) = std::sync::mpsc::sync_channel::<Work>(1);
            let (responses, rx) = std::sync::mpsc::sync_channel(16);
            let spawned = std::thread::Builder::new().name("aosp-stroke-head".into()).spawn(move || {
                let mut model = match Model::load(std::path::Path::new(MODEL_PATH)) {
                    Ok(model) => model,
                    Err(e) => {
                        let _ = responses.try_send(Reply::Error(e));
                        return;
                    }
                };
                let _ = responses.try_send(Reply::Loaded);
                while let Ok(w) = requests.recv() {
                    let began = std::time::Instant::now();
                    match model
                        .infer(&w.features)
                        .and_then(|out| {
                            decode_with_pressure(&out, w.from, w.to, w.sample.pressure, w.horizon)
                                .ok_or_else(|| "Model abstained at a stop or short horizon".into())
                        })
                    {
                        Ok((p, pred_p)) => {
                            let d = p - w.to;
                            let doc = w.sample.basis[0] * d.x + w.sample.basis[1] * d.y;
                            let _ = responses.try_send(Reply::PredictedWithPressure(
                                w.epoch,
                                w.sample_revision,
                                doc,
                                pred_p,
                                began.elapsed().as_secs_f64() * 1e6,
                            ));
                        }
                        Err(_) => {
                            let _ = responses.try_send(Reply::Predicted(w.epoch, w.sample_revision, Vec2::ZERO, began.elapsed().as_secs_f64() * 1e6));
                        }
                    }
                }
            });
            match spawned {
                Ok(_) => {
                    self.worker = Some(Worker { tx, rx });
                    self.status = "loading".into();
                }
                Err(e) => self.status = format!("unavailable: {e}"),
            }
        }
    }
    pub fn sample(&mut self, sample: Option<Sample>, horizon: f64, submit: bool) {
        if !self.enabled {
            return;
        }
        let Some(s) = sample.filter(|s| {
            s.time.is_finite()
                && s.time >= 0.0
                && s.position.x.is_finite()
                && s.position.y.is_finite()
                && s.pressure.is_finite()
                && (0.0..=1.0).contains(&s.pressure)
                && s.tilt.iter().all(|a| a.is_finite() && (-90.0..=90.0).contains(a))
                && s.basis.iter().all(|a| a.x.is_finite() && a.y.is_finite() && a.hypot() > 0.0 && a.hypot() < 10000.0)
        }) else {
            self.reset();
            self.skipped += 1;
            self.poll();
            return;
        };
        // Actual input replaces speculative ink immediately, even if no new job is submitted.
        self.delta = None;
        self.predicted_pressure = None;
        if let Some(revision) = self.sample_revision.checked_add(1) {
            self.sample_revision = revision;
        } else {
            // Keep request identities distinct even at counter exhaustion.
            self.reset();
            self.sample_revision = 1;
        }
        if self.worker.is_none() && self.status.is_empty() {
            self.start_worker();
        }
        if self.history.last().is_some_and(|last| s.time <= last.time || s.time - last.time > 0.020 || s.basis != last.basis) {
            self.reset();
        }
        let _ = self.tilt_filter.sample(s.tilt, s.time);
        self.history.push(s);
        if self.history.len() > 64 {
            self.history.remove(0);
        }
        self.age = 0.0;
        // Drain after advancing input identity: queued old replies are diagnostics only.
        self.poll();
        // Every sample is retained causally, but at most one inference is queued per batch.
        if submit
            && horizon > 0.0
            && let Some((features, from, to)) = features(&self.history)
            && let Some(worker) = &self.worker
            && worker
                .tx
                .try_send(Work {
                    epoch: self.epoch,
                    sample_revision: self.sample_revision,
                    sample: s,
                    features,
                    from,
                    to,
                    horizon: horizon.clamp(1.0, 16.0),
                })
                .is_err()
        {
            self.skipped += 1;
        }
    }
    pub fn tick(&mut self, dt: f64) {
        if dt.is_finite() && dt > 0.0 {
            self.age += dt;
        }
        self.poll();
    }
    fn poll(&mut self) {
        if let Some(worker) = &self.worker {
            while let Ok(reply) = worker.rx.try_recv() {
                match reply {
                    Reply::Loaded => self.status = "ready: experimental AOSP model".into(),
                    Reply::Error(e) => self.status = format!("unavailable: {e}"),
                    Reply::Predicted(epoch, sample_revision, d, us) => {
                        self.inferences += 1;
                        self.last_us = us;
                        self.max_us = self.max_us.max(us);
                        if epoch == self.epoch && sample_revision == self.sample_revision && !self.history.is_empty() {
                            self.delta = Some(d);
                        } else {
                            self.stale_results += 1;
                        }
                    }
                    Reply::PredictedWithPressure(epoch, sample_revision, d, p, us) => {
                        self.inferences += 1;
                        self.last_us = us;
                        self.max_us = self.max_us.max(us);
                        if epoch == self.epoch && sample_revision == self.sample_revision && !self.history.is_empty() {
                            self.delta = Some(d);
                            self.predicted_pressure = Some(p.clamp(0.0, 1.0));
                        } else {
                            self.stale_results += 1;
                        }
                    }
                }
            }
        }
    }
    pub fn prediction(&self, zoom: f64) -> Option<Vec2> {
        if !self.enabled || self.age > 0.025 {
            return None;
        }
        let d = self.delta?;
        let max = 24.0 / zoom.max(1e-9);
        Some(d * (max / d.hypot().max(max)))
    }
    /// Returns the 5-DOF MLP predicted pressure (`out[2]`) at the current horizon when valid.
    #[must_use]
    pub fn predicted_pressure(&self) -> Option<f32> {
        if !self.enabled || self.age > 0.025 {
            return None;
        }
        self.predicted_pressure
    }
    /// Returns the 1 Euro stabilized stylus tilt `[tilt_x, tilt_y]` in degrees.
    #[must_use]
    pub fn stabilized_tilt(&self) -> Option<[f64; 2]> {
        if !self.enabled {
            return None;
        }
        self.tilt_filter.current()
    }
}
fn at(history: &[Sample], time: f64) -> Option<Sample> {
    let right = history.partition_point(|s| s.time <= time);
    let a = *history.get(right.checked_sub(1)?)?;
    if (a.time - time).abs() < 1e-9 {
        return Some(a);
    }
    let b = *history.get(right)?;
    if b.time - a.time > 0.020 || a.basis != b.basis {
        return None;
    }
    let t = (time - a.time) / (b.time - a.time);
    Some(Sample {
        time,
        position: a.position.lerp(b.position, t),
        pressure: a.pressure + (b.pressure - a.pressure) * t as f32,
        tilt: std::array::from_fn(|i| a.tilt[i] + (b.tilt[i] - a.tilt[i]) * t),
        basis: a.basis,
    })
}
fn features(history: &[Sample]) -> Option<([Vec<f32>; 5], Point, Point)> {
    let now = history.last()?.time;
    let points = (0..7).map(|i| at(history, now - (6 - i) as f64 * STEP)).collect::<Option<Vec<_>>>()?;
    let mut channels: [Vec<f32>; 5] = std::array::from_fn(|_| Vec::new());
    let mut from: Option<Point> = None;
    let mut to = points.first()?.position;
    for s in points.iter().skip(1) {
        let v = s.position - to;
        let axis = from.map_or(v, |a| to - a);
        let phi = if from.is_some() { v.y.atan2(v.x) - axis.y.atan2(axis.x) } else { 0.0 };
        let [tx, ty] = s.tilt.map(f64::to_radians);
        let tilt = (tx.cos() * ty.cos()).clamp(-1.0, 1.0).acos();
        let orientation = std::f64::consts::FRAC_PI_2 - (-tx.sin()).atan2(ty.sin()) - axis.y.atan2(axis.x);
        for (c, val) in channels.iter_mut().zip([v.hypot() as f32, phi as f32, s.pressure, tilt as f32, orientation as f32]) {
            c.push(val);
        }
        if v.hypot() > 0.0 {
            from = Some(to);
            to = s.position;
        }
    }
    for c in &mut channels {
        c.remove(0);
    }
    Some((channels, from?, to))
}
#[cfg(not(target_arch = "wasm32"))]
fn decode_with_pressure(out: &[Vec<f32>; 3], from: Point, to: Point, base_pressure: f32, ms: f64) -> Option<(Point, f32)> {
    let mut a = from;
    let mut b = to;
    let mut path = vec![to];
    let mut pressures = vec![base_pressure.clamp(0.0, 1.0)];
    for ((&r, &phi), &p) in out[0].iter().zip(&out[1]).zip(&out[2]) {
        if r < 0.2 || !r.is_finite() || !phi.is_finite() {
            break;
        }
        let angle = (b - a).y.atan2((b - a).x) + phi as f64;
        let next = b + Vec2::new(angle.cos(), angle.sin()) * r as f64;
        a = b;
        b = next;
        path.push(next);
        let clean_p = if p.is_finite() { p.clamp(0.0, 1.0) } else { base_pressure.clamp(0.0, 1.0) };
        pressures.push(clean_p);
    }
    let t = ms / 1000.0 / STEP;
    let i = t.floor() as usize;
    let fract = t.fract();
    let pos = path.get(i)?.lerp(*path.get(i + 1)?, fract);
    let p0 = *pressures.get(i)?;
    let p1 = *pressures.get(i + 1)?;
    let pred_p = (p0 + (p1 - p0) * (fract as f32)).clamp(0.0, 1.0);
    Some((pos, pred_p))
}
#[cfg(test)]
mod tests {
    use super::*;

    // No thread or model is started: tests own both ends of the worker channels.
    fn injected_worker() -> (Learned, Receiver<Work>, SyncSender<Reply>) {
        let (tx, requests) = std::sync::mpsc::sync_channel(1);
        let (responses, rx) = std::sync::mpsc::sync_channel(16);
        let l = Learned { enabled: true, worker: Some(Worker { tx, rx }), ..Default::default() };
        (l, requests, responses)
    }

    fn measured(i: u32, x: f64) -> Sample {
        Sample {
            time: f64::from(i) * STEP,
            position: Point::new(x, 0.0),
            pressure: 0.5,
            tilt: [10.0, 15.0],
            basis: [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
        }
    }

    fn submit_head(l: &mut Learned, requests: &Receiver<Work>) -> Work {
        for i in 0..8 {
            l.sample(Some(measured(i, f64::from(i) * 4.0)), 14.0, i == 7);
        }
        requests.try_recv().unwrap()
    }

    #[test]
    fn actual_sample_immediately_replaces_provisional_prediction() {
        let (mut l, requests, responses) = injected_worker();
        let work = submit_head(&mut l, &requests);
        responses.try_send(Reply::Predicted(work.epoch, work.sample_revision, Vec2::new(4.0, 0.0), 10.0)).unwrap();
        l.tick(0.0);
        assert_eq!(l.prediction(1.0), Some(Vec2::new(4.0, 0.0)));
        l.sample(Some(measured(8, 32.0)), 14.0, false);
        assert!(l.delta.is_none());
        assert!(l.prediction(1.0).is_none());
        assert_eq!(l.sample_revision, work.sample_revision + 1);
        assert_eq!(l.inferences, 1);
    }

    #[test]
    fn delayed_reply_is_diagnostic_only_and_current_revision_is_accepted() {
        let (mut l, requests, responses) = injected_worker();
        let old = submit_head(&mut l, &requests);
        // Queue before actual input too: sample() must drain it as stale, not accept it first.
        responses.try_send(Reply::Predicted(old.epoch, old.sample_revision, Vec2::new(20.0, 0.0), 30.0)).unwrap();
        l.sample(Some(measured(8, 32.0)), 14.0, true);
        let current = requests.try_recv().unwrap();
        assert!(current.sample.time - old.sample.time < 0.020);
        assert_eq!(l.stale_results, 1);
        assert!(l.delta.is_none());
        responses.try_send(Reply::Predicted(current.epoch, current.sample_revision, Vec2::new(2.0, 0.0), 12.0)).unwrap();
        l.tick(0.0);
        responses.try_send(Reply::Predicted(old.epoch, old.sample_revision, Vec2::new(20.0, 0.0), 40.0)).unwrap();
        l.tick(0.0);
        assert_eq!(l.prediction(1.0), Some(Vec2::new(2.0, 0.0)));
        assert_eq!((l.inferences, l.stale_results, l.skipped), (3, 2, 0));
        assert_eq!((l.last_us, l.max_us), (40.0, 40.0));
    }

    #[test]
    fn continued_actual_input_and_reversal_never_revive_old_prediction() {
        let (mut l, requests, responses) = injected_worker();
        let old = submit_head(&mut l, &requests);
        responses.try_send(Reply::Predicted(old.epoch, old.sample_revision, Vec2::new(4.0, 0.0), 10.0)).unwrap();
        l.tick(0.0);
        let mut tracker = crate::stylus::Tracker::default();
        for i in 0..8 {
            let s = measured(i, f64::from(i) * 4.0);
            tracker.sample(s.position, Some(s.time), false, 1.0, 0.01);
        }
        for i in 8..40 {
            let s = measured(i, 28.0 - f64::from(i - 7) * 4.0);
            l.sample(Some(s), 14.0, false);
            tracker.sample(s.position, Some(s.time), false, 1.0, 0.01);
            if i == 8 {
                assert!(tracker.prediction(14.0, 1.0).is_none());
                responses.try_send(Reply::Predicted(old.epoch, old.sample_revision, Vec2::new(4.0, 0.0), 11.0)).unwrap();
            } else {
                assert!(tracker.prediction(14.0, 1.0).is_some());
            }
            l.tick(STEP);
            assert!(l.prediction(1.0).is_none());
            assert_eq!(l.sample_revision, u64::from(i) + 1);
        }
        assert_eq!(l.stale_results, 1);
    }

    #[test]
    fn expiry_and_reset_preserve_authoritative_samples_and_reject_old_epoch() {
        let (mut l, requests, responses) = injected_worker();
        let work = submit_head(&mut l, &requests);
        let history = l.history.clone();
        let inputs = features(&history).unwrap();
        responses.try_send(Reply::Predicted(work.epoch, work.sample_revision, Vec2::new(4.0, 0.0), 10.0)).unwrap();
        l.tick(0.0);
        assert!(l.prediction(1.0).is_some());
        l.tick(0.026);
        assert!(l.prediction(1.0).is_none());
        assert_eq!(l.history, history);
        assert_eq!(features(&l.history).unwrap(), inputs);
        l.reset();
        assert!(l.history.is_empty());
        assert!(l.delta.is_none());
        assert_eq!(l.sample_revision, work.sample_revision);
        l.sample(Some(measured(9, 36.0)), 14.0, false);
        assert_eq!(l.sample_revision, work.sample_revision + 1);
        // Matching revision alone cannot cross a stroke/reset boundary.
        responses.try_send(Reply::Predicted(work.epoch, l.sample_revision, Vec2::new(4.0, 0.0), 15.0)).unwrap();
        l.tick(0.0);
        assert!(l.prediction(1.0).is_none());
        assert_eq!(l.stale_results, 1);
        assert_eq!(l.history, vec![measured(9, 36.0)]);
    }
    #[test]
    #[ignore = "Requires pinned local AOSP weights; synthetic inputs only, not Cintiq replay"]
    fn installed_model_runs_in_bounded_background_worker() {
        let mut l = Learned::default();
        l.enable(true);
        for i in 0..32 {
            l.sample(
                Some(Sample {
                    time: i as f64 * STEP,
                    position: Point::new(i as f64 * 4.0, 0.0),
                    pressure: 0.5,
                    tilt: [10.0, 15.0],
                    basis: [Vec2::new(0.5, 0.0), Vec2::new(0.0, 0.5)],
                }),
                14.0,
                true,
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
            l.tick(0.0);
        }
        let start = std::time::Instant::now();
        while l.inferences == 0 && start.elapsed().as_secs() < 3 {
            std::thread::sleep(std::time::Duration::from_millis(2));
            l.tick(0.0);
        }
        assert!(l.status().starts_with("ready"), "{}", l.status());
        assert!(l.inferences > 0);
        assert!(l.prediction(1.0).is_some());
        l.reset();
        assert!(l.prediction(1.0).is_none());
        l.enable(false);
        assert_eq!(l.status(), "off");
    }
    #[test]
    fn missing_channels_clear_prediction_and_causal_history() {
        let (mut l, _requests, _responses) = injected_worker();
        for i in 0..10 {
            l.sample(
                Some(Sample {
                    time: i as f64 * STEP,
                    position: Point::new(i as f64 * 4.0, 0.0),
                    pressure: 0.5,
                    tilt: [0.0, 0.0],
                    basis: [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
                }),
                14.0,
                false,
            );
        }
        assert_eq!(features(&l.history).unwrap().0[0].len(), 5);
        l.delta = Some(Vec2::new(4.0, 0.0));
        assert!(l.prediction(1.0).is_some());
        l.tick(0.04);
        assert!(l.prediction(1.0).is_none());
        l.sample(None, 14.0, false);
        assert!(l.history.is_empty());
        assert!(l.prediction(1.0).is_none());
    }
    #[test]
    fn predicted_pressure_and_stabilized_tilt_track_5dof_state() {
        let (mut l, requests, responses) = injected_worker();
        let work = submit_head(&mut l, &requests);
        responses
            .try_send(Reply::PredictedWithPressure(work.epoch, work.sample_revision, Vec2::new(3.0, 1.0), 0.78, 9.0))
            .unwrap();
        l.tick(0.0);
        assert_eq!(l.prediction(1.0), Some(Vec2::new(3.0, 1.0)));
        assert_eq!(l.predicted_pressure(), Some(0.78));
        assert!(l.stabilized_tilt().is_some());
    }
}
