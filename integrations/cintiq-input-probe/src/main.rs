//! Opt-in, window-local native pen diagnostic. Does not modify the editor or drawings.
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};

// Share the exact portable contract tested and available to the editor tools, without UI dependencies there.
#[path = "../../../crates/tools/src/motion_model.rs"]
mod model;
#[cfg(windows)]
mod native;
#[path = "../../../crates/tools/src/pen_input.rs"]
mod pen_input;
mod prediction;
mod recording;
#[path = "../../../crates/tools/src/stylus_tracker.rs"]
mod stylus_tracker;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case")]
enum Record {
    Header { schema: u32, backend: String, owner_tablet: String, rotation_confirmed: bool },
    Sample { raw: pen_input::RawPenPacket, normalized: pen_input::PenSample },
    Diagnostic { message: String },
    Lifecycle { pointer_id: u32, kind: String },
    Scenario { scenario: recording::Scenario },
    Auxiliary { message: u32, code: u64, detail: i64, tick_ms: u32 },
    Summary { stats: pen_input::CaptureStats, native_errors: u64, promoted_mouse_messages: u64, log_queue_dropped: u64 },
}

#[derive(Clone)]
struct Log {
    send: mpsc::SyncSender<Record>,
    dropped: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
}

impl Log {
    fn record(&self, record: Record) {
        if let Err(error) = self.send.try_send(record) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            if matches!(error, mpsc::TrySendError::Disconnected(_)) {
                self.failed.store(true, Ordering::Relaxed);
            }
        }
    }
}

type WriterThread = std::thread::JoinHandle<Result<(), String>>;
fn writer(path: &Path) -> Result<(Log, WriterThread), String> {
    if !path.is_absolute() {
        return Err("Output must be an absolute path".into());
    }
    // Explicit destination, never replace an existing recording.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = std::fs::OpenOptions::new().create_new(true).write(true).open(path).map_err(|e| format!("Cannot create {}: {e}", path.display()))?;
    let (send, recv) = mpsc::sync_channel(8192);
    let failed = Arc::new(AtomicBool::new(false));
    let worker_failed = failed.clone();
    let worker = std::thread::spawn(move || {
        let result = (|| {
            let mut file = BufWriter::new(file);
            let mut flushed = std::time::Instant::now();
            loop {
                match recv.recv_timeout(std::time::Duration::from_millis(250)) {
                    Ok(record) => {
                        serde_json::to_writer(&mut file, &record).map_err(|e| e.to_string())?;
                        file.write_all(b"\n").map_err(|e| e.to_string())?;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => file.flush().map_err(|e| e.to_string())?,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
                if flushed.elapsed() >= std::time::Duration::from_millis(250) {
                    file.flush().map_err(|e| e.to_string())?;
                    flushed = std::time::Instant::now();
                }
            }
            file.flush().map_err(|e| e.to_string())
        })();
        if result.is_err() {
            worker_failed.store(true, Ordering::Relaxed);
        }
        result
    });
    Ok((Log { send, failed, dropped: Arc::new(AtomicU64::new(0)) }, worker))
}

fn replay(path: &Path) -> Result<(), String> {
    let recording = recording::Recording::read(path)?;
    let report = recording.report();
    let mut replayed = 0;
    let mut batches = 0;
    let mut elapsed_us = 0;
    recording.timed_batches(1, |time_us, samples| {
        replayed += samples.len();
        batches += 1;
        elapsed_us = time_us;
    })?;
    println!(
        "Virtual-clock replay: {replayed} unchanged native samples in {batches} batches over {elapsed_us} us; {} paired strokes, {} incomplete. No canvas input injected.",
        report.complete_strokes, report.incomplete_strokes
    );
    for warning in &report.warnings {
        eprintln!("Capture warning: {warning}");
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--evaluate-prediction")
        && args.len() == 6
        && args.get(2).is_some_and(|a| a == "--model")
        && args.get(4).is_some_and(|a| a == "--output")
    {
        let capture = recording::Recording::read(Path::new(&args[1]))?;
        let mut model = model::Model::load(Path::new(&args[3]))?;
        println!("{}", model.describe());
        let report = prediction::evaluate(&capture, &mut model)?;
        prediction::save(&report, Path::new(&args[5]))?;
        println!("Causal prediction evaluation saved: {}. No document edits or model training.", args[5]);
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--replay") && args.len() == 2 {
        return replay(Path::new(&args[1]));
    }
    if args.first().is_some_and(|a| a == "--review") && args.len() == 2 {
        let capture = recording::Recording::read(Path::new(&args[1]))?;
        println!("{}", serde_json::to_string_pretty(&capture.report()).map_err(|e| e.to_string())?);
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--export-fixtures") && args.len() == 4 && args.get(2).is_some_and(|a| a == "--output") {
        let capture = recording::Recording::read(Path::new(&args[1]))?;
        return capture.export(Path::new(&args[3]));
    }
    if args.first().is_some_and(|a| a == "--help") {
        println!(
            "cintiq-input-probe [--output ABSOLUTE.jsonl]\ncintiq-input-probe --replay recording.jsonl\ncintiq-input-probe --review recording.jsonl\ncintiq-input-probe --export-fixtures recording.jsonl --output ABSOLUTE.json\ncintiq-input-probe --evaluate-prediction recording.jsonl --model model.tflite --output ABSOLUTE.json\nF1 slow curves, F2 fast strokes, F3 corners, F4 reversals, F5 pressure, F6 buttons.\nNo output argument: creates a new capture beside the executable. Close the test window to finish."
        );
        return Ok(());
    }
    let output = if args.is_empty() {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let parent = exe.parent().ok_or("Executable has no parent directory")?;
        let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis();
        parent.join("captures").join(format!("cintiq-{time}-{}.jsonl", std::process::id()))
    } else if args.len() == 2 && args.first().is_some_and(|a| a == "--output") {
        PathBuf::from(&args[1])
    } else {
        return Err("Use --help for valid arguments".into());
    };
    #[cfg(not(windows))]
    {
        let _ = output;
        Err("Native capture currently supports Windows 10/11; portable normalization tests and replay remain available".into())
    }
    #[cfg(windows)]
    {
        println!("Recording only pen events targeted at the test window: {}", output.display());
        println!(
            "Hover, draw light-to-heavy strokes, tilt left/right and toward/away, flip to eraser, press each side button. Move the window to the Cintiq if necessary. No real canvas is edited."
        );
        let (log, worker) = writer(&output)?;
        log.record(Record::Header {
            schema: 2,
            backend: "windows_ink_pointer_history".into(),
            owner_tablet: "DTK-2420 (owner reported; pen model not verified)".into(),
            rotation_confirmed: false,
        });
        let result = native::run(log.clone());
        drop(log);
        let written = worker.join().map_err(|_| "Capture writer failed to finish")?;
        written?;
        result?;
        println!("Capture closed. Validate it with --replay \"{}\"", output.display());
        Ok(())
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Cintiq test: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_backpressure_is_counted_not_silently_lost() {
        let (send, _recv) = mpsc::sync_channel(1);
        let log = Log { send, dropped: Arc::new(AtomicU64::new(0)), failed: Arc::new(AtomicBool::new(false)) };
        log.record(Record::Diagnostic { message: "first".into() });
        log.record(Record::Diagnostic { message: "second".into() });
        assert_eq!(log.dropped.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn empty_capture_is_not_hardware_success() {
        let path = std::env::temp_dir().join(format!("cintiq-empty-{}.jsonl", std::process::id()));
        let (log, worker) = writer(&path).unwrap();
        log.record(Record::Header { schema: 1, backend: "synthetic".into(), owner_tablet: "test".into(), rotation_confirmed: false });
        drop(log);
        worker.join().unwrap().unwrap();
        assert!(replay(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn writer_and_replay_preserve_realistic_clock_and_sensor_values() {
        let path = std::env::temp_dir().join(format!("cintiq-replay-{}.jsonl", std::process::id()));
        let (log, worker) = writer(&path).unwrap();
        log.record(Record::Header { schema: 1, backend: "synthetic_test".into(), owner_tablet: "test".into(), rotation_confirmed: false });
        let raw = pen_input::RawPenPacket {
            pointer_id: 12,
            frame_id: 700,
            device: 10,
            screen_px: [-2000, 500],
            client_px: Some([100, 200]),
            window_dpi: 192,
            performance_count: 1_234_567_891_234,
            performance_frequency: 10_000_000,
            system_tick_ms: u32::MAX,
            pointer_flags: 4,
            pen_flags: 1,
            channel_mask: 13,
            pressure: 367,
            tilt_x: -30,
            tilt_y: 15,
            rotation: 0,
            button_change: 0,
            key_states: 0,
        };
        log.record(Record::Sample { normalized: raw.normalize(false).unwrap(), raw });
        log.record(Record::Summary {
            stats: pen_input::CaptureStats { received: 1, accepted: 1, ..Default::default() },
            native_errors: 0,
            promoted_mouse_messages: 0,
            log_queue_dropped: 0,
        });
        drop(log);
        worker.join().unwrap().unwrap();
        assert!(replay(&path).is_ok());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn recordings_never_replace_existing_files() {
        let path = std::env::temp_dir().join(format!("cintiq-protected-{}.jsonl", std::process::id()));
        let (log, worker) = writer(&path).unwrap();
        assert!(writer(&path).is_err());
        drop(log);
        worker.join().unwrap().unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
