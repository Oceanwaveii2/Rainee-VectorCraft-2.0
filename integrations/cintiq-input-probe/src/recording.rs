//! Deterministic native-packet fixtures. No model training, guessed intent or document edits.
use crate::{
    Record,
    pen_input::{CaptureStats, PenCapture, PenSample, Phase, RawPenPacket},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORDS: usize = 200_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    #[default]
    Unlabelled,
    SlowCurves,
    FastStrokes,
    Corners,
    Reversals,
    PressureChanges,
    Buttons,
}
impl Scenario {
    pub fn function_key(key: u64) -> Option<Self> {
        match key {
            0x70 => Some(Self::SlowCurves),
            0x71 => Some(Self::FastStrokes),
            0x72 => Some(Self::Corners),
            0x73 => Some(Self::Reversals),
            0x74 => Some(Self::PressureChanges),
            0x75 => Some(Self::Buttons),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub sequence: usize,
    pub scenario: Scenario,
    pub raw: RawPenPacket,
    pub normalized: PenSample,
}
#[derive(Debug, Serialize)]
pub struct Stroke {
    pub id: usize,
    pub scenario: Scenario,
    pub sample_indices: Vec<usize>,
    pub complete: bool,
    pub starts_with_down: bool,
    pub mixed_labels: bool,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub source_sha256: String,
    pub samples: usize,
    pub complete_strokes: usize,
    pub incomplete_strokes: usize,
    pub phases: BTreeMap<String, usize>,
    pub scenarios: BTreeMap<String, usize>,
    pub pressure_range: Option<[f32; 2]>,
    pub tilt_ranges: [Option<[i32; 2]>; 2],
    pub timestamped_samples: usize,
    pub eraser_samples: usize,
    pub barrel_button_samples: usize,
    pub auxiliary_messages: usize,
    pub virtual_key_downs: BTreeMap<u64, usize>,
    pub recorded_dpi: BTreeMap<u32, usize>,
    pub native_stats: CaptureStats,
    pub warnings: Vec<String>,
    pub approved_for_training: bool,
}
pub struct Recording {
    pub samples: Vec<Sample>,
    pub strokes: Vec<Stroke>,
    source_sha256: String,
    stats: CaptureStats,
    warnings: Vec<String>,
    auxiliary: Vec<Record>,
}

impl Recording {
    pub fn read(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Recording exceeds 64 MiB; split the recording".into());
        }
        let hash = format!("{:x}", Sha256::digest(&bytes));
        Self::parse(&bytes, hash)
    }
    fn parse(bytes: &[u8], source_sha256: String) -> Result<Self, String> {
        let mut result = Self {
            samples: Vec::new(),
            strokes: Vec::new(),
            source_sha256,
            stats: CaptureStats::default(),
            warnings: Vec::new(),
            auxiliary: Vec::new(),
        };
        let mut reader = BufReader::new(bytes);
        let mut line = String::new();
        let mut header = false;
        let mut closed = false;
        let mut capture = PenCapture::default();
        let mut times: HashMap<(u64, u32), u64> = HashMap::new();
        let mut active: HashMap<(u64, u32), usize> = HashMap::new();
        let mut scenario = Scenario::default();
        let mut records = 0;
        loop {
            line.clear();
            let size = (&mut reader).take(65_537).read_line(&mut line).map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            records += 1;
            if size > 65_536 || records > MAX_RECORDS {
                return Err("Recording record/count limit exceeded".into());
            }
            if !line.ends_with('\n') {
                return Err("Truncated recording record".into());
            }
            let record: Record = serde_json::from_str(&line).map_err(|e| format!("Invalid record {records}: {e}"))?;
            if closed {
                return Err("Records follow the closing summary".into());
            }
            if !header && !matches!(record, Record::Header { .. }) {
                return Err("Recording header is missing".into());
            }
            match record {
                Record::Header { schema: 1 | 2, rotation_confirmed: false, .. } if !header => header = true,
                Record::Header { .. } => return Err("Unsupported or repeated recording header".into()),
                Record::Sample { raw, normalized } => {
                    if raw.normalize(false)? != normalized {
                        return Err("Raw and normalized replay data disagree".into());
                    }
                    if capture.history(vec![raw.clone()], 1).len() != 1 {
                        return Err("Duplicate or rejected authoritative sample".into());
                    }
                    let key = (raw.device, raw.pointer_id);
                    if raw.performance_count != 0 && raw.performance_frequency != 0 {
                        if times.get(&key).is_some_and(|previous| raw.performance_count < *previous) {
                            return Err("Per-pointer clock regressed".into());
                        }
                        times.insert(key, raw.performance_count);
                    }
                    let index = result.samples.len();
                    if matches!(normalized.phase, Phase::Down | Phase::Drag | Phase::Up | Phase::Cancel) {
                        if normalized.phase == Phase::Down || !active.contains_key(&key) {
                            let id = result.strokes.len();
                            result.strokes.push(Stroke {
                                id,
                                scenario,
                                sample_indices: Vec::new(),
                                complete: false,
                                starts_with_down: normalized.phase == Phase::Down,
                                mixed_labels: false,
                            });
                            active.insert(key, id);
                        }
                        if let Some(stroke) = active.get(&key).and_then(|id| result.strokes.get_mut(*id)) {
                            stroke.sample_indices.push(index);
                            stroke.mixed_labels |= stroke.scenario != scenario;
                            stroke.complete = normalized.phase == Phase::Up && stroke.starts_with_down;
                        }
                        if matches!(normalized.phase, Phase::Up | Phase::Cancel) {
                            active.remove(&key);
                        }
                    }
                    result.samples.push(Sample { sequence: index, scenario, raw, normalized });
                }
                Record::Scenario { scenario: label } => scenario = label,
                Record::Auxiliary { .. } => result.auxiliary.push(record),
                Record::Diagnostic { message } => result.warnings.push(message),
                Record::Lifecycle { pointer_id, kind } => {
                    if kind == "capture_lost" {
                        active.retain(|(_, id), _| *id != pointer_id);
                    }
                }
                Record::Summary { stats, native_errors, log_queue_dropped, .. } => {
                    if stats.accepted != result.samples.len() as u64 {
                        return Err("Summary sample count disagrees with recording".into());
                    }
                    if stats.invalid != 0 || stats.truncated_history != 0 || native_errors != 0 || log_queue_dropped != 0 {
                        result.warnings.push(format!(
                            "Capture quality failure: invalid={}, truncations={}, API errors={}, log drops={}",
                            stats.invalid, stats.truncated_history, native_errors, log_queue_dropped
                        ));
                    }
                    result.stats = stats;
                    closed = true;
                }
            }
        }
        if !header || result.samples.is_empty() {
            return Err("No native pen samples: hardware acceptance is still pending".into());
        }
        if !closed {
            return Err("Recording has no closing summary; close the test window first".into());
        }
        Ok(result)
    }
    pub fn report(&self) -> Report {
        let mut report = Report {
            source_sha256: self.source_sha256.clone(),
            samples: self.samples.len(),
            complete_strokes: self.strokes.iter().filter(|s| s.complete).count(),
            incomplete_strokes: self.strokes.iter().filter(|s| !s.complete).count(),
            phases: BTreeMap::new(),
            scenarios: BTreeMap::new(),
            pressure_range: None,
            tilt_ranges: [None; 2],
            timestamped_samples: 0,
            eraser_samples: 0,
            barrel_button_samples: 0,
            auxiliary_messages: self.auxiliary.len(),
            virtual_key_downs: BTreeMap::new(),
            recorded_dpi: BTreeMap::new(),
            native_stats: self.stats.clone(),
            warnings: self.warnings.clone(),
            approved_for_training: false,
        };
        for sample in &self.samples {
            *report.phases.entry(format!("{:?}", sample.normalized.phase)).or_default() += 1;
            *report.scenarios.entry(format!("{:?}", sample.scenario)).or_default() += 1;
            *report.recorded_dpi.entry(sample.raw.window_dpi).or_default() += 1;
            report.timestamped_samples += usize::from(sample.normalized.time_seconds.is_some());
            report.eraser_samples += usize::from(sample.normalized.eraser);
            report.barrel_button_samples += usize::from(sample.normalized.barrel_button);
            if let Some(p) = sample.normalized.pressure {
                report.pressure_range = Some(report.pressure_range.map_or([p, p], |r| [r[0].min(p), r[1].max(p)]));
            }
            for (axis, value) in sample.normalized.tilt_degrees.iter().enumerate() {
                if let (Some(value), Some(range)) = (value, report.tilt_ranges.get_mut(axis)) {
                    *range = Some(range.map_or([*value, *value], |r| [r[0].min(*value), r[1].max(*value)]));
                }
            }
        }
        for auxiliary in &self.auxiliary {
            if let Record::Auxiliary { message: 0x100 | 0x104, code, .. } = auxiliary {
                *report.virtual_key_downs.entry(*code).or_default() += 1;
            }
        }
        report
    }
    /// Virtual clock playback, no sleeping or input injection. Actual packets are never resampled.
    pub(crate) fn timed_batches(&self, batch_size: usize, mut consume: impl FnMut(u64, &[Sample])) -> Result<(), String> {
        if batch_size == 0 || batch_size > 4096 {
            return Err("Replay batch size must be 1..4096".into());
        }
        let first = self.samples.first().ok_or("Empty recording")?;
        let origin = first.raw.performance_count;
        let frequency = first.raw.performance_frequency;
        if origin == 0 || frequency == 0 {
            return Err("Replay timing needs native QPC timestamps".into());
        }
        let mut previous = origin;
        for sample in &self.samples {
            if sample.raw.performance_frequency != frequency || sample.raw.performance_count < previous {
                return Err("Replay timing has inconsistent clocks".into());
            }
            previous = sample.raw.performance_count;
        }
        for chunk in self.samples.chunks(batch_size) {
            let last = chunk.last().ok_or("Empty replay batch")?;
            let elapsed = last.raw.performance_count.checked_sub(origin).ok_or("Replay time regressed")?;
            let us = (u128::from(elapsed) * 1_000_000 / u128::from(frequency)).min(u128::from(u64::MAX)) as u64;
            consume(us, chunk);
        }
        Ok(())
    }
    pub fn export(&self, output: &Path) -> Result<(), String> {
        if !output.is_absolute() {
            return Err("Fixture output must be an absolute path".into());
        }
        if !self.warnings.is_empty() {
            return Err("Capture has quality warnings; review before fixture export".into());
        }
        // Prove playback is causal and frame-independent before creating the derived artifact.
        self.timed_batches(1, |_, _| {})?;
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = std::fs::OpenOptions::new().create_new(true).write(true).open(output).map_err(|e| e.to_string())?;
        let mut writer = std::io::BufWriter::new(file);
        #[derive(Serialize)]
        struct Fixture<'a> {
            schema: u32,
            coordinate_space: &'static str,
            approved_for_training: bool,
            report: Report,
            samples: &'a [Sample],
            strokes: &'a [Stroke],
            auxiliary: &'a [Record],
        }
        serde_json::to_writer(
            &mut writer,
            &Fixture {
                schema: 1,
                coordinate_space: "physical_screen_and_window_client_pixels_not_document_units",
                approved_for_training: false,
                report: self.report(),
                samples: &self.samples,
                strokes: &self.strokes,
                auxiliary: &self.auxiliary,
            },
        )
        .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        println!(
            "Exported {} unchanged actual samples and {} stroke spans to {} (not approved for training)",
            self.samples.len(),
            self.strokes.len(),
            output.display()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raw(i: u32, phase: Phase) -> RawPenPacket {
        RawPenPacket {
            pointer_id: 1,
            frame_id: i,
            device: 7,
            screen_px: [i as i32, 8],
            client_px: Some([i as i32, 8]),
            window_dpi: 192,
            performance_count: 1000 + u64::from(i) * 10,
            performance_frequency: 1000,
            system_tick_ms: i * 10,
            pointer_flags: match phase {
                Phase::Down => 0x10000,
                Phase::Up => 0x40000,
                _ => 4,
            },
            pen_flags: 0,
            channel_mask: 13,
            pressure: 100 + i,
            tilt_x: -5,
            tilt_y: 8,
            rotation: 0,
            button_change: 0,
            key_states: 0,
        }
    }
    fn records(labelled: bool) -> Vec<Record> {
        let mut result =
            vec![Record::Header { schema: 2, backend: "synthetic_test".into(), owner_tablet: "synthetic".into(), rotation_confirmed: false }];
        if labelled {
            result.push(Record::Scenario { scenario: Scenario::Corners });
        }
        for (i, phase) in [Phase::Down, Phase::Drag, Phase::Up].into_iter().enumerate() {
            let raw = raw(i as u32, phase);
            result.push(Record::Sample { normalized: raw.normalize(false).unwrap(), raw });
        }
        result.push(Record::Summary {
            stats: CaptureStats { received: 3, accepted: 3, ..Default::default() },
            native_errors: 0,
            promoted_mouse_messages: 0,
            log_queue_dropped: 0,
        });
        result
    }
    fn parse(records: &[Record]) -> Result<Recording, String> {
        let data = records.iter().map(|r| serde_json::to_string(r).unwrap() + "\n").collect::<String>();
        Recording::parse(data.as_bytes(), "synthetic".into())
    }
    #[test]
    fn labelled_strokes_preserve_pressure_and_never_auto_approve_training() {
        let recording = parse(&records(true)).unwrap();
        assert_eq!(recording.report().complete_strokes, 1);
        assert_eq!(recording.strokes[0].scenario, Scenario::Corners);
        assert_eq!(recording.samples[1].raw.pressure, 101);
        assert!(!recording.report().approved_for_training);
    }
    #[test]
    fn unlabeled_data_does_not_invent_slow_fast_or_corner_intent() {
        let recording = parse(&records(false)).unwrap();
        assert!(recording.samples.iter().all(|s| s.scenario == Scenario::Unlabelled));
    }
    #[test]
    fn playback_batching_cannot_change_authoritative_packets() {
        let recording = parse(&records(true)).unwrap();
        let mut one = Vec::new();
        let mut batched = Vec::new();
        recording.timed_batches(1, |_, p| one.extend(p.iter().map(|s| s.raw.clone()))).unwrap();
        recording.timed_batches(64, |_, p| batched.extend(p.iter().map(|s| s.raw.clone()))).unwrap();
        assert_eq!(one, batched);
        assert!(recording.timed_batches(0, |_, _| {}).is_err());
    }
    #[test]
    fn incomplete_capture_summary_and_tampered_samples_are_rejected() {
        let mut r = records(false);
        r.pop();
        assert!(parse(&r).is_err());
        let mut r = records(false);
        if let Record::Sample { normalized, .. } = &mut r[1] {
            normalized.pressure = Some(0.9);
        }
        assert!(parse(&r).is_err());
        let mut r = records(false);
        if let Record::Summary { stats, .. } = &mut r[4] {
            stats.accepted = 99;
        }
        assert!(parse(&r).is_err());
    }
    #[test]
    fn duplicate_and_regressing_samples_are_rejected() {
        let mut r = records(false);
        r.insert(2, r[1].clone());
        assert!(parse(&r).is_err());
        let mut r = records(false);
        if let Record::Sample { raw, normalized } = &mut r[2] {
            raw.performance_count = 1;
            *normalized = raw.normalize(false).unwrap();
        }
        assert!(parse(&r).is_err());
    }
    #[test]
    fn canceled_strokes_are_incomplete_and_auxiliary_keys_never_create_ink() {
        let mut r = records(false);
        if let Record::Sample { raw, normalized } = &mut r[3] {
            raw.pointer_flags = 0x8000;
            *normalized = raw.normalize(false).unwrap();
        }
        r.insert(1, Record::Auxiliary { message: 0x100, code: 0xdd, detail: 0, tick_ms: 5 });
        let recording = parse(&r).unwrap();
        assert_eq!(recording.report().complete_strokes, 0);
        assert_eq!(recording.report().virtual_key_downs.get(&0xdd), Some(&1));
        assert_eq!(recording.report().samples, 3);
    }
    #[test]
    fn oversized_truncated_and_post_summary_records_fail() {
        assert!(Recording::parse(&vec![b'x'; 65_537], "synthetic".into()).is_err());
        assert!(Recording::parse(b"{}", "synthetic".into()).is_err());
        let mut r = records(false);
        r.push(Record::Scenario { scenario: Scenario::Buttons });
        assert!(parse(&r).is_err());
    }

    #[test]
    fn slow_curves_fast_strokes_corners_reversals_and_stationary_pressure_replay_exactly() {
        // Original synthetic trajectories, not mislabeled human recordings or training evidence.
        for scenario in [Scenario::SlowCurves, Scenario::FastStrokes, Scenario::Corners, Scenario::Reversals, Scenario::PressureChanges] {
            let mut r = vec![
                Record::Header { schema: 2, backend: "synthetic_test".into(), owner_tablet: "synthetic".into(), rotation_confirmed: false },
                Record::Scenario { scenario },
            ];
            for i in 0..100u32 {
                let phase = if i == 0 {
                    Phase::Down
                } else if i == 99 {
                    Phase::Up
                } else {
                    Phase::Drag
                };
                let mut packet = raw(i, phase);
                packet.performance_count = 1000 + u64::from(i) * if scenario == Scenario::FastStrokes { 2 } else { 10 };
                packet.screen_px = match scenario {
                    Scenario::SlowCurves => [i as i32, (20.0 * (f64::from(i) / 20.0).sin()) as i32],
                    Scenario::FastStrokes => [i as i32 * 12, 0],
                    Scenario::Corners => {
                        if i < 50 {
                            [i as i32, 0]
                        } else {
                            [49, i as i32 - 49]
                        }
                    }
                    Scenario::Reversals => [if i < 50 { i as i32 } else { 99 - i as i32 }, 0],
                    Scenario::PressureChanges => {
                        packet.pressure = 100 + i * 8;
                        [40, 40]
                    }
                    _ => unreachable!(),
                };
                packet.client_px = Some(packet.screen_px);
                r.push(Record::Sample { normalized: packet.normalize(false).unwrap(), raw: packet });
            }
            r.push(Record::Summary {
                stats: CaptureStats { received: 100, accepted: 100, ..Default::default() },
                native_errors: 0,
                promoted_mouse_messages: 0,
                log_queue_dropped: 0,
            });
            let recording = parse(&r).unwrap();
            assert_eq!(recording.report().complete_strokes, 1);
            assert_eq!(recording.strokes[0].scenario, scenario);
            for batch in [1, 8, 100] {
                let mut replay = Vec::new();
                recording.timed_batches(batch, |_, p| replay.extend(p.iter().map(|s| s.raw.clone()))).unwrap();
                assert_eq!(replay, recording.samples.iter().map(|s| s.raw.clone()).collect::<Vec<_>>());
            }
        }
    }

    #[test]
    fn scenario_keys_do_not_treat_remapped_brackets_as_physical_pen_buttons() {
        assert_eq!(Scenario::function_key(0x70), Some(Scenario::SlowCurves));
        assert_eq!(Scenario::function_key(0x75), Some(Scenario::Buttons));
        assert_eq!(Scenario::function_key(0xdb), None);
        assert_eq!(Scenario::function_key(0xdd), None);
    }

    #[test]
    fn fixture_export_never_overwrites_and_quality_failure_blocks_export() {
        let path = std::env::temp_dir().join(format!("cintiq-fixture-protected-{}.json", std::process::id()));
        let recording = parse(&records(true)).unwrap();
        recording.export(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(recording.export(&path).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
        let mut r = records(true);
        if let Some(Record::Summary { native_errors, .. }) = r.last_mut() {
            *native_errors = 1;
        }
        assert!(parse(&r).unwrap().export(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
