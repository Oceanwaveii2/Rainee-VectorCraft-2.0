//! Startup routing for the first disposable scratch consumer: font discovery metadata.
//! Recovery, artwork, original fonts and profiles never live in these cache directories.

use std::path::{Path, PathBuf};

const OWNED: &str = "VectorCraft Scratch";
const FONT_TIER: &str = "font-discovery-v1";

/// Existing preferences accept drive roots or directories. Startup uses the default cache;
/// None disables that secondary candidate. Settings changes take effect at the next launch.
pub(super) fn candidates(primary: &str, secondary: &str, default: &Path) -> Vec<PathBuf> {
    let root = |value: &str| match value {
        "None" | "" => None,
        "Startup" => Some(default.to_path_buf()),
        _ => {
            let p = PathBuf::from(value);
            p.is_absolute().then_some(p)
        }
    };
    let mut out = Vec::new();
    for p in [root(primary), root(secondary), Some(default.to_path_buf())].into_iter().flatten() {
        let path = p.join(OWNED).join(FONT_TIER);
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// Validate only the reserved app-owned subdirectories, not the selected user's folder.
/// Failed/offline/read-only candidates fall back; no broad folder deletion or font copying.
pub(super) fn prepare(paths: &[PathBuf]) -> Option<PathBuf> {
    for directory in paths {
        let Some(owned) = directory.parent() else { continue };
        let Some(root) = owned.parent() else { continue };
        if !root.is_dir() {
            log::warn!("scratch root unavailable: {}", root.display());
            continue;
        }
        let result = (|| -> std::io::Result<PathBuf> {
            for p in [owned, directory.as_path()] {
                match std::fs::symlink_metadata(p) {
                    Ok(m) if m.is_dir() && !m.file_type().is_symlink() => (),
                    Ok(_) => return Err(std::io::Error::other("scratch subdirectory is linked or not a directory")),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(p)?,
                    Err(e) => return Err(e),
                }
            }
            // Test writability with a uniquely owned probe, never replace an existing file.
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let probe = directory.join(format!(".probe-{}-{n}", std::process::id()));
            let f = std::fs::OpenOptions::new().write(true).create_new(true).open(&probe)?;
            drop(f);
            std::fs::remove_file(probe)?;
            Ok(directory.join("fonts.bin"))
        })();
        match result {
            Ok(path) => {
                log::info!("font discovery scratch: {}", path.display());
                return Some(path);
            }
            Err(e) => log::warn!("scratch unavailable at {}: {e}", directory.display()),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_and_none_are_not_literal_folder_names() {
        let root = std::env::temp_dir();
        let paths = candidates("Startup", "None", &root);
        assert_eq!(paths, [root.join(OWNED).join(FONT_TIER)]);
        assert_eq!(candidates("relative", "None", &root), paths);
    }

    #[test]
    fn unavailable_primary_uses_secondary_and_keeps_recovery_data() {
        let root = std::env::temp_dir().join(format!("vc-scratch-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let recovery = root.join("recovery.vectorcraft");
        std::fs::write(&recovery, b"protected").unwrap();
        let missing = root.join("offline");
        let paths = candidates(&missing.to_string_lossy(), &root.to_string_lossy(), &root);
        let cache = prepare(&paths).unwrap();
        assert_eq!(cache, root.join(OWNED).join(FONT_TIER).join("fonts.bin"));
        assert_eq!(std::fs::read(&recovery).unwrap(), b"protected");
        assert!(!missing.exists());
    }

    #[test]
    fn a_reserved_name_that_is_a_file_is_never_replaced() {
        let root = std::env::temp_dir().join(format!("vc-scratch-blocked-{}", std::process::id()));
        let primary = root.join("primary");
        let secondary = root.join("secondary");
        std::fs::create_dir_all(&primary).unwrap();
        std::fs::create_dir_all(&secondary).unwrap();
        let reserved = primary.join(OWNED);
        std::fs::write(&reserved, b"existing user file").unwrap();
        let paths = candidates(&primary.to_string_lossy(), &secondary.to_string_lossy(), &secondary);
        assert_eq!(prepare(&paths).unwrap(), secondary.join(OWNED).join(FONT_TIER).join("fonts.bin"));
        assert_eq!(std::fs::read(reserved).unwrap(), b"existing user file");
    }
}
