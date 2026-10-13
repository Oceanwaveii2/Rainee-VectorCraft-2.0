#[cfg(target_os = "windows")]
use super::Junction;
use super::{Input, Proposal};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool, mpsc::Sender};
const JUNCTION_EXE: &str = "D:/DevData/Rainee/build/stripmaker-scip/Release/junction-worker.exe";
#[cfg(target_os = "windows")]
const JUNCTION_HASH: &str = "fbe8f994eaa868e9bc59f50c7cfd42734438d04a08c3d325fd5300fb9ec37807";
const PYTHON: &str = "D:/DevData/Rainee/venvs/neube/Scripts/python.exe";
const SCRIPT: &str = "D:/0/Rainee drawing app/ArtCraft-Foundation/VectorCraft - Gemini Edition/integrations/neube-lab/worker.py";
pub(super) fn capabilities() -> Value {
    json!({"junction":{"installed":cfg!(target_os="windows")&&std::path::Path::new(JUNCTION_EXE).is_file(),"backend":"StripMaker-bundled SketchConnectivity features + pretrained forests","scope":"local endpoint/T suggestions; no global topology solver"},
    "brush":{"installed":cfg!(target_os="windows")&&std::path::Path::new(PYTHON).is_file()&&std::path::Path::new(SCRIPT).is_file(),"backend":"NeuBE style1; isolated CUDA PyTorch research runtime","scope":"bounded raster appearance; editable source vectors retained","productionPortable":false}})
}
pub(super) fn spawn(input: Input, cancel: Arc<AtomicBool>, tx: Sender<std::result::Result<Proposal, String>>) -> std::result::Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::thread::Builder::new()
            .name("vector-research-assistant".into())
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
        Err("Research assistants currently require the local Windows installations".into())
    }
}
#[cfg(target_os = "windows")]
fn run(mut input: Input, cancel: &AtomicBool) -> std::result::Result<Proposal, String> {
    use sha2::{Digest, Sha256};
    use std::os::windows::process::CommandExt;
    use std::{
        fs,
        process::{Command, Stdio},
        sync::atomic::Ordering,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let dir = std::path::PathBuf::from(format!("D:/DevData/Rainee/scratch/research-tools/job-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let strokes: Vec<_> =
        input.strokes.iter().map(|s| json!({"points":s.points.iter().map(|p|[p.x,p.y]).collect::<Vec<_>>(),"width":s.width})).collect();
    let scale = input.proposal.size[0] as f64 / input.proposal.region.width();
    let job = json!({"strokes":strokes,"gap":input.gap,"size":input.proposal.size,"mapping":[input.proposal.region.x0,input.proposal.region.y0,scale],"seed":input.seed,"color":input.color});
    let infile = dir.join("input.json");
    fs::write(&infile, serde_json::to_vec(&job).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let output = dir.join(if input.kind == "junction" { "output.json" } else { "appearance.png" });
    let mut cmd = if input.kind == "junction" {
        let bytes = fs::read(JUNCTION_EXE).map_err(|e| format!("Junction worker not installed: {e}"))?;
        if format!("{:x}", Sha256::digest(bytes)) != JUNCTION_HASH {
            return Err("Junction worker changed; rebuild/re-audit required".into());
        }
        Command::new(JUNCTION_EXE)
    } else {
        let mut c = Command::new(PYTHON);
        c.arg("-I").arg(SCRIPT);
        c
    };
    let log = fs::File::create(dir.join("worker.log")).map_err(|e| e.to_string())?;
    cmd.arg(&infile).arg(&output).current_dir(&dir).creation_flags(0x08000000).stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone().map_err(|e|e.to_string())?)).stderr(Stdio::from(log))
        .env("TEMP","D:/DevData/Rainee/temp").env("TMP","D:/DevData/Rainee/temp")
        .env("PATH",format!("D:/DevData/Rainee/tools/scip-10.1.0/scipoptsuite-10.1.0-win-x64/bin;D:/DevData/Rainee/build/stripmaker-scip/msvc_19.41_cxx20_64_md_release;{}",std::env::var("PATH").unwrap_or_default()));
    let mut child = Child(cmd.spawn().map_err(|e| format!("Research worker launch failed: {e}"))?);
    let begin = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled; no artwork changed".into());
        }
        if begin.elapsed().as_secs() > 120 {
            return Err(format!("Research job timed out; no artwork changed. Diagnostics: {}", dir.display()));
        }
        if let Some(status) = child.0.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                return Err(format!("Research worker failed ({status}); no art changed. Diagnostics: {}", dir.display()));
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if fs::metadata(&output).map_err(|e| e.to_string())?.len() > 4_000_000 {
        return Err("Research output exceeds preview budget".into());
    }
    if input.kind == "junction" {
        let data: Value = serde_json::from_slice(&fs::read(&output).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let suggestions = data["suggestions"].as_array().filter(|a| a.len() <= 4096).ok_or("Invalid worker suggestions")?;
        for s in suggestions {
            let idx = |key: &str| -> std::result::Result<usize, String> {
                s[key].as_u64().and_then(|i| usize::try_from(i).ok()).ok_or_else(|| "Invalid stroke ordinal".into())
            };
            let a = input.strokes.get(idx("a")?).ok_or("Invalid source")?;
            let b = input.strokes.get(idx("b")?).ok_or("Invalid target")?;
            let confidence = s["confidence"].as_f64().filter(|p| p.is_finite() && (0.0..=1.0).contains(p)).ok_or("Invalid confidence")?;
            if confidence < input.threshold {
                continue;
            }
            let x = s["point"][0].as_f64().filter(|x| x.is_finite()).ok_or("Invalid candidate point")?;
            let y = s["point"][1].as_f64().filter(|x| x.is_finite()).ok_or("Invalid candidate point")?;
            let tail = s["tail"].as_bool().ok_or("Invalid endpoint")?;
            let source = if tail { a.points.last() } else { a.points.first() }.ok_or("Empty source")?;
            let point = vectorcraft_geom::Point::new(x, y);
            if a.id == b.id || source.distance(point) > input.gap + 0.001 {
                return Err("Worker exceeded authorized gap distance".into());
            }
            input.proposal.junctions.push(Junction {
                a: a.id,
                tail,
                b: b.id,
                point,
                confidence,
                kind: s["kind"].as_str().unwrap_or("junction").into(),
            });
        }
        input.proposal.junctions.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
        input.proposal.telemetry =
            json!({"evaluations":data["evaluations"],"backend":"SketchConnectivity bundled author forests","seconds":begin.elapsed().as_secs_f64()});
    } else {
        input.proposal.png = fs::read(&output).map_err(|e| e.to_string())?;
        let image = image::load_from_memory(&input.proposal.png).map_err(|e| e.to_string())?;
        if image.width() != input.proposal.size[0] || image.height() != input.proposal.size[1] {
            return Err("Neural raster dimensions differ from captured region".into());
        }
        let log = fs::read_to_string(dir.join("worker.log")).unwrap_or_default();
        input.proposal.telemetry =
            log.lines().rev().find_map(|l| serde_json::from_str::<Value>(l).ok()).unwrap_or_else(|| json!({"seconds":begin.elapsed().as_secs_f64()}));
    }
    Ok(input.proposal)
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
