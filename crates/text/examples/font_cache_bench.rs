//! Discovery-only benchmark, not whole-app launch or an OS cold-cache benchmark.
//! Use a new output directory: no existing cache is removed or replaced.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    use vectorcraft_text::{FontDb, system_font_dirs};
    let root = std::env::args_os().nth(1).map(PathBuf::from).ok_or("provide a fresh benchmark output directory")?;
    std::fs::create_dir(&root)?;
    let paths = system_font_dirs();
    for trial in 0..3 {
        for mode in ["baseline", "cold", "warm"] {
            let db = FontDb::with_font_dirs(paths.clone());
            if mode != "baseline" {
                db.set_discovery_cache(Some(root.join(format!("fonts-{trial}.bin"))));
            }
            db.load_system_fonts();
            if let Some(report) = db.discovery_report() {
                println!(
                    "{}",
                    serde_json::json!({
                        "trial": trial, "mode": mode, "faces": report.faces,
                        "cacheHit": report.cache_hit, "cacheWritten": report.cache_written,
                        "inventoryMs": report.inventory_ms, "totalMs": report.total_ms,
                    })
                );
            }
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
