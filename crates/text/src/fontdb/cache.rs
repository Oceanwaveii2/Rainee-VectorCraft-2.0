//! Versioned, bounded discovery metadata. Never contains or redistributes font bytes.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{Alias, Catalog, PostScriptNames, ScanStamp};

// Bump when the catalogue, aliases, classification or discovery semantics change.
const MAGIC: &[u8] = b"VCFONT\0\x01";
const MAX_ENCODED: u64 = 8 * 1024 * 1024;
const MAX_DECODED: u64 = 32 * 1024 * 1024;
const MAX_FACES: usize = 100_000;

#[derive(Clone, Debug)]
pub struct FontDiscoveryReport {
    pub cache_enabled: bool,
    pub cache_hit: bool,
    pub cache_written: bool,
    pub faces: usize,
    pub inventory_ms: f64,
    pub total_ms: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Snapshot {
    stamp: ScanStamp,
    catalog: Catalog,
    aliases: HashMap<String, Vec<Alias>>,
    postscript: PostScriptNames,
}

#[derive(serde::Serialize)]
struct SnapshotRef<'a> {
    stamp: &'a ScanStamp,
    catalog: &'a Catalog,
    aliases: &'a HashMap<String, Vec<Alias>>,
    postscript: &'a PostScriptNames,
}

type CachedCatalog = (Catalog, HashMap<String, Vec<Alias>>, PostScriptNames);

pub(super) fn read(path: &Path, stamp: &ScanStamp) -> Option<CachedCatalog> {
    let m = std::fs::symlink_metadata(path).ok()?;
    if !m.is_file() || m.file_type().is_symlink() {
        return None;
    }
    let mut bytes = Vec::new();
    File::open(path).ok()?.take(MAX_ENCODED + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > MAX_ENCODED || !bytes.starts_with(MAGIC) {
        return None;
    }
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(bytes.get(MAGIC.len()..)?).take(MAX_DECODED + 1).read_to_end(&mut decoded).ok()?;
    if decoded.len() as u64 > MAX_DECODED {
        return None;
    }
    let s: Snapshot = serde_json::from_slice(&decoded).ok()?;
    if s.stamp != *stamp || s.catalog.values().map(|f| f.faces.len()).sum::<usize>() > MAX_FACES {
        return None;
    }
    // Cached paths must belong to the live inventory, never arbitrary files from cached input.
    let paths: std::collections::HashSet<_> = stamp.modified.iter().map(|(p, _)| p).collect();
    if s.catalog.iter().any(|(key, family)| {
        *key != family.name.to_ascii_lowercase() || family.faces.iter().any(|f| !paths.contains(&f.path) || !f.weight.is_finite())
    }) {
        return None;
    }
    Some((s.catalog, s.aliases, s.postscript))
}

pub(super) fn write(
    path: &Path,
    stamp: &ScanStamp,
    catalog: &Catalog,
    aliases: &HashMap<String, Vec<Alias>>,
    postscript: &PostScriptNames,
) -> std::io::Result<()> {
    // A cache path must be an ordinary file. Never follow a linked cache into user data.
    if let Ok(m) = std::fs::symlink_metadata(path)
        && (!m.is_file() || m.file_type().is_symlink())
    {
        return Err(std::io::Error::other("font cache is not an ordinary file"));
    }
    let json = serde_json::to_vec(&SnapshotRef { stamp, catalog, aliases, postscript })?;
    if json.len() as u64 > MAX_DECODED {
        return Err(std::io::Error::other("font discovery metadata exceeds cache limit"));
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&json)?;
    let compressed = gz.finish()?;
    if (compressed.len() + MAGIC.len()) as u64 > MAX_ENCODED {
        return Err(std::io::Error::other("font discovery cache exceeds size limit"));
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_file_name(format!(".fonts-{}-{n}.tmp", std::process::id()));
    let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = f.write_all(MAGIC).and_then(|()| f.write_all(&compressed)).and_then(|()| f.sync_all());
    drop(f);
    result.and_then(|()| std::fs::rename(&tmp, path)).inspect_err(|_| {
        // Only our exclusively-created temporary cache file is disposable.
        let _ = std::fs::remove_file(&tmp);
    })
}
