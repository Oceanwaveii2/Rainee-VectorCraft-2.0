//! No arbitrary executable/path parameters. Research installation is pinned and opt-in.
#[cfg(target_os = "windows")]
use super::{Group, preserve_unsafe_groups};
use super::{Input, Plan};
#[cfg(target_os = "windows")]
use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::AtomicBool, mpsc::Sender};
#[cfg(any(target_os = "windows", test))]
use vectorcraft_geom::Point;

const EXE: &str = "D:/DevData/Rainee/build/stripmaker-scip/author/Release/clustering-solve.exe";
#[cfg(target_os = "windows")]
const HASH: &str = "f3029840132ffe65d73ab2d800947ba6f6c2227246dbfe8966323d569bf8c053";
pub(super) fn available() -> std::result::Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Err("Research worker available on Windows only".into());
    }
    if !std::path::Path::new(EXE).is_file() {
        return Err("Optional StripMaker SCIP/OSQP research worker is not installed".into());
    }
    Ok(())
}
pub(super) fn availability() -> serde_json::Value {
    match available() {
        Ok(()) => serde_json::json!({"installed":true,"verification":"SHA-256 checked before execution","productionPortable":false}),
        Err(e) => serde_json::json!({"installed":false,"reason":e}),
    }
}
pub(super) fn spawn(input: Input, cancel: Arc<AtomicBool>, tx: Sender<std::result::Result<Plan, String>>) -> std::result::Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::thread::Builder::new()
            .name("sketch-cleanup".into())
            .spawn(move || {
                let result = run(input, &cancel);
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (input, cancel, tx);
        Err("Windows worker unavailable".into())
    }
}

#[cfg(target_os = "windows")]
fn run(input: Input, cancel: &AtomicBool) -> std::result::Result<Plan, String> {
    use sha2::{Digest, Sha256};
    use std::os::windows::process::CommandExt;
    use std::{
        fs,
        io::Write,
        process::{Command, Stdio},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    available()?;
    let exe = fs::read(EXE).map_err(|e| e.to_string())?;
    if format!("{:x}", Sha256::digest(&exe)) != HASH {
        return Err("Worker binary changed: re-audit before use".into());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled".into());
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let dir = std::path::PathBuf::from(format!("D:/DevData/Rainee/scratch/cleanup/job-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Keep bounded job diagnostics on D for artwork review; they are disposable research cache.
    let minx = input.strokes.iter().flatten().map(|p| p.x).fold(f64::INFINITY, f64::min) - 8.0;
    let miny = input.strokes.iter().flatten().map(|p| p.y).fold(f64::INFINITY, f64::min) - 8.0;
    let maxx = input.strokes.iter().flatten().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max) - minx + 8.0;
    let maxy = input.strokes.iter().flatten().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max) - miny + 8.0;
    let mut file = fs::File::create(dir.join("input.scap")).map_err(|e| e.to_string())?;
    writeln!(file, "#{}\t{}\n@{}", maxx.ceil() as u64, maxy.ceil() as u64, input.width).map_err(|e| e.to_string())?;
    for (i, stroke) in input.strokes.iter().enumerate() {
        writeln!(file, "{{\n#{i}\t{i}").map_err(|e| e.to_string())?;
        for (j, p) in stroke.iter().enumerate() {
            writeln!(file, "{}\t{}\t{}", p.x - minx, p.y - miny, j * 10).map_err(|e| e.to_string())?;
        }
        writeln!(file, "}}").map_err(|e| e.to_string())?;
    }
    drop(file);
    let log = fs::File::create(dir.join("worker.log")).map_err(|e| e.to_string())?;
    let mut command = Command::new(EXE);
    command.args(["-i"]).arg(dir.join("input.scap")).arg("-o").arg(&dir).arg("--interm").arg(&dir)
        .env("PATH",format!("D:/DevData/Rainee/tools/scip-10.1.0/scipoptsuite-10.1.0-win-x64/bin;D:/DevData/Rainee/build/stripmaker-scip/msvc_19.41_cxx20_64_md_release;{}",std::env::var("PATH").unwrap_or_default()))
        .env_remove("RAINEE_SCIP_DEBUG").current_dir(&dir).creation_flags(0x08000000)
        .stdin(Stdio::null()).stdout(Stdio::from(log.try_clone().map_err(|e|e.to_string())?)).stderr(Stdio::from(log));
    let mut child = Child(command.spawn().map_err(|e| format!("Worker launch failed: {e}"))?);
    let start = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        if start.elapsed().as_secs_f64() > input.timeout {
            return Err("Cleanup timed out; no art changed".into());
        }
        if let Some(status) = child.0.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                return Err(format!("Worker failed ({status}); diagnostics: {}", dir.display()));
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let read = |name: &str| -> std::result::Result<String, String> {
        let path = dir.join(name);
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 8_000_000 {
            return Err("Worker output too large".into());
        }
        fs::read_to_string(path).map_err(|e| e.to_string())
    };
    let grouped = parse(&read("input_out.scap")?)?;
    let fitted = parse(&read("final_prediction_fit.scap")?)?;
    let mut mapping = std::collections::BTreeMap::<usize, Vec<usize>>::new();
    let mut seen = std::collections::BTreeSet::new();
    for stroke in grouped {
        let original = input.strokes.get(stroke.id).ok_or("Worker returned invalid ordinal")?;
        if !seen.insert(stroke.id) || original.len() != stroke.points.len() {
            return Err("Worker changed source samples".into());
        }
        for (a, b) in original.iter().zip(&stroke.points) {
            let restored = Point::new(b.x + minx, b.y + miny);
            if a.distance(restored) > 0.001 + (b.x.abs() + b.y.abs()) * 0.00001 {
                return Err("Worker changed source geometry".into());
            }
        }
        mapping.entry(stroke.group).or_default().push(stroke.id);
    }
    if seen.len() != input.strokes.len() {
        return Err("Worker omitted strokes".into());
    }
    let mut plan = input.plan.clone();
    for stroke in fitted {
        let mut members = mapping.remove(&stroke.group).ok_or("Duplicate or unknown fitted group")?;
        members.sort_unstable();
        plan.groups.push(Group {
            originals: members.iter().map(|i| input.plan.originals[*i].id).collect(),
            points: stroke.points.iter().map(|p| Point::new(p.x + minx, p.y + miny)).collect(),
        });
    }
    if !mapping.is_empty() {
        return Err("Missing fitted group".into());
    }
    preserve_unsafe_groups(&input, &mut plan)?;
    Ok(plan)
}
#[cfg(target_os = "windows")]
struct Child(std::process::Child);
#[cfg(target_os = "windows")]
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(any(target_os = "windows", test))]
struct Stroke {
    id: usize,
    group: usize,
    points: Vec<Point>,
}
#[cfg(any(target_os = "windows", test))]
fn parse(text: &str) -> std::result::Result<Vec<Stroke>, String> {
    let mut output = Vec::new();
    let mut current: Option<Stroke> = None;
    let mut header = false;
    let mut total = 0;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line == "{" {
            if current.is_some() {
                return Err("Nested SCAP stroke".into());
            }
            current = Some(Stroke { id: usize::MAX, group: usize::MAX, points: Vec::new() });
        } else if line == "}" {
            let stroke = current.take().ok_or("Unexpected SCAP close")?;
            if stroke.id == usize::MAX || stroke.group == usize::MAX || stroke.points.len() < 2 || output.len() >= 64 {
                return Err("Invalid SCAP stroke".into());
            }
            output.push(stroke);
        } else if let Some(stroke) = current.as_mut() {
            let parts: Vec<_> = line.split_whitespace().collect();
            if line.starts_with('#') {
                if parts.len() != 2 || stroke.id != usize::MAX {
                    return Err("Invalid SCAP stroke header".into());
                }
                stroke.id = parts[0].trim_start_matches('#').parse().map_err(|_| "Invalid SCAP ID")?;
                stroke.group = parts[1].parse().map_err(|_| "Invalid SCAP group")?;
            } else {
                if parts.len() != 3 || stroke.points.len() >= 4096 || total >= super::MAX_POINTS {
                    return Err("Invalid SCAP point count".into());
                }
                let number = |v: &str| v.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or("Nonfinite SCAP number");
                let p = Point::new(number(parts[0])?, number(parts[1])?);
                let _ = number(parts[2])?;
                stroke.points.push(p);
                total += 1;
            }
        } else if line.starts_with('#') && !header {
            header = true;
        } else if line.starts_with('@') && header {
        } else {
            return Err("Unexpected SCAP data".into());
        }
    }
    if current.is_some() || !header || output.is_empty() {
        return Err("Truncated SCAP output".into());
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_truncated_nonfinite_duplicate_headers() {
        for s in ["#10 10\n@2\n{\n#0 0\n1 2 0", "#10 10\n@2\n{\n#0 0\nNaN 2 0\n1 2 0\n}", "#10 10\n{\n#0 0\n#0 0\n1 2 0\n1 2 0\n}"] {
            assert!(parse(s).is_err());
        }
    }
    #[test]
    fn parses_finite_output() {
        let p = parse("#10 10\n@2\n{\n#0 4\n1 2 0\n3 4 10\n}").unwrap();
        assert_eq!(p[0].group, 4);
    }
}
