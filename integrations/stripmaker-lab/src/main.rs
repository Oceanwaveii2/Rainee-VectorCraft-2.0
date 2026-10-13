use serde_json::json;
use std::{
    io::{self, BufRead},
    path::Path,
};
use stripmaker_classifier_lab::{Forest, Model, REVISION};

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = args.first().ok_or("usage: stripmaker-classifier-lab SOURCE_ROOT [local|contextual] [--probes]")?;
    let model = match args.get(1).map(String::as_str).unwrap_or("local") {
        "local" => Model::Local,
        "contextual" => Model::Contextual,
        _ => return Err("unknown model".into()),
    };
    let path = Path::new(root).join("src/classifier").join(model.file());
    if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 4_000_000 {
        return Err("model file too large".into());
    }
    let forest = Forest::from_bytes(model, &std::fs::read(path).map_err(|e| e.to_string())?)?;
    eprintln!(
        "{}",
        json!({"revision":REVISION,"model":model.file(),"sha256CanonicalLF":model.sha256(),"dimensions":forest.dimensions(),"trees":150,"nodes":forest.nodes(),"scope":"exact-feature inference only; no grouping or fitting"})
    );
    if args.get(2).is_some_and(|a| a == "--probes") {
        for features in forest.probes() {
            println!("{}", json!({"probability":forest.predict(&features)?,"features":features}));
        }
        return Ok(());
    }
    // Bounded JSON-lines input. Rejected rows produce explicit errors, not fake scores.
    let mut reader = io::stdin().lock();
    loop {
        let mut bytes = Vec::new();
        loop {
            let buf = reader.fill_buf().map_err(|e| e.to_string())?;
            if buf.is_empty() {
                break;
            }
            let n = buf.iter().position(|c| *c == b'\n').map_or(buf.len(), |i| i + 1);
            if bytes.len() + n > 65536 {
                return Err("feature row exceeds 64 KiB".into());
            }
            let done = buf.get(n.saturating_sub(1)) == Some(&b'\n');
            bytes.extend_from_slice(buf.get(..n).ok_or("invalid buffer length")?);
            reader.consume(n);
            if done {
                break;
            }
        }
        if bytes.is_empty() {
            break;
        }
        let result = serde_json::from_slice::<Vec<f64>>(&bytes).map_err(|e| e.to_string()).and_then(|features| forest.predict(&features));
        println!(
            "{}",
            match result {
                Ok(p) => json!({"probability":p}),
                Err(e) => json!({"error":e}),
            }
        );
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
