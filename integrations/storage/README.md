# Rainee development storage on D

The Rainee project, documentation, packaged recorder and human pen captures live under `D:\0\Rainee drawing app`. New development storage belongs on D rather than the nearly full C drive.

The migration script copies explicit Rust and probe-build directories, verifies every file by SHA-256, then creates compatibility junctions at the previous paths. Only after verification and a successful junction read does it remove the original duplicate directory. It refuses existing destinations or unexpected links instead of overwriting them. Migration logs and file manifests live in `D:\DevData\MigrationLogs\Rainee-2026-10-11`. A failed copy leaves the source intact; a failed junction setup retains the source at the recorded backup path.

| Data | D location | Compatibility path |
| --- | --- | --- |
| Cargo dependencies and tools | `D:\DevData\Rust\cargo` | `C:\Users\User\.cargo` |
| Rust toolchains | `D:\DevData\Rust\rustup` | `C:\Users\User\.rustup` |
| Cintiq probe build | `D:\DevData\Rainee\build\Cintiq-Input-Probe-2026-10-11` | `E:\CodexBuild\Cintiq-Input-Probe-2026-10-11` |
| Existing editor build cache | `D:\DevData\Rainee\build\editor` | `C:\Users\User\AppData\Local\Temp\Rainee-ArtCraft-target` |
| Preferences, logs and recovery folder | `D:\DevData\Rainee\appdata\VectorCraft` | `C:\Users\User\AppData\Roaming\VectorCraft` |
| Embedded bridge data | `D:\DevData\Rainee\appdata\RaineeCodexBridge` | `C:\Users\User\AppData\Local\RaineeCodexBridge` |
| Earlier research package | `D:\DevData\Rainee\reference-inbox` | Original Downloads folder is a junction |
| Installer downloads | `D:\DevData\Rainee\installers` | Original files moved from Downloads |
| Obsolete generated test outputs | Windows Recycle Bin on D | 23 migrated temporary folders recycled; restore through the bin |
| Rainee process temporary files | `D:\DevData\Rainee\temp` | No Windows-wide TEMP change |

Use this setup in the project shell before building:

```powershell
. ./integrations/storage/Use-DDrive.ps1 -Target Probe
cargo test --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml
cargo clippy --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml --all-targets -- -D warnings
cargo build --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml
```

For an editor build, select `-Target Editor`; outputs then reuse `D:\DevData\Rainee\build\editor`. The setup sets Cargo/Rustup locations, target directory and TEMP/TMP for the current shell and child processes only. Compatibility junctions let existing Rust and application-data paths continue working. Windows, Codex's own live data, unrelated downloads and unrelated application settings are not moved by these scripts.

The migration is local machine administration, not a new app scratch-disk feature. Recovery files and application preferences retain their original logical paths through a junction; their physical storage is on D. Explicit Preferences-selected scratch directories are not rewritten. Keep recovery records and owner documents out of cache-purge actions. Hash manifests identify archived files and allow restoring them from D if needed.

The old `C:\Users\User\.cargo.rainee-migration-2026-10-11` backup remains because an active `uv.exe` blocked its cleanup, and a subsequent cleanup command was rejected by the execution policy. Cargo's active path is already a junction to its fully verified D copy. Do not stop unrelated services to remove the backup. It is a retained duplicate, not missing migrated data; inspect it after the locking processes close before reclaiming its remaining space.

## Verified migration results

On October 11, 2026, source and destination SHA-256 checks passed for 71,797 Cargo files, 65,963 Rustup files, 2,906 Cintiq build files and 48,458 editor-cache files. The editor cache contains 38.572 GiB of logical file data; logical sizes are not a measure of space reclaimed. Preferences, logs, the recovery folder, bridge data, six earlier research files and two installer downloads are also stored on D. Old application-data and Rust paths are compatibility junctions. The retained Cargo backup is approximately 2 GiB; the other verified original duplicates were removed only after their replacements were readable.

The D-drive setup passed an offline, lockfile-preserving recorder rebuild with Cargo/rustc 1.98.0: all 25 tests passed, clippy with warnings denied passed, and the native debug build succeeded. The rebuilt executable replayed both closed recordings without changing their 24,855 authoritative samples or 57 complete strokes. Its SHA-256 is `334D81EFB506823E0F10C166B8406BEA761DFE3F1A8D1D5D120BE2A88A556E8F`. The existing portable v0.2 package and both fixture hashes remain unchanged. This checks the standalone recorder, not full editor CI or ML inference.

C had 69.72 GiB free and D had 102.28 GiB free at the final measurement. Other work on this machine also changed disk usage during the migration; do not attribute the entire free-space change to this cleanup.

## Recoverable cleanup and protected projects

After the owner requested trashing unused outputs, 23 verified temporary test directories containing 52 generated files (284,051 bytes) were sent to the Windows Recycle Bin. All 23 bin entries were confirmed; the intermediate `temp-archive` is now empty. Their original paths and migration manifests are recorded in `D:\DevData\MigrationLogs\Rainee-2026-10-11\recycled-test-outputs.json`. The bin was not emptied. Recycle Bin storage remains allocated until the owner empties it.

`recycle-obsolete-test-data.ps1` previews only the explicit migrated test-output whitelist by default. Its `-Apply` mode validates the manifest, hashes, resolved paths and process references before recycling; it does not scan or remove other projects. Human recovery records, pen captures, research sources, preferences and active caches are excluded. Age alone does not prove a folder is disposable. Preserve functional projects, including Nanobanana Suite, even if they have not been edited recently. No broad Downloads, Windows, Codex or other application cleanup was performed.
