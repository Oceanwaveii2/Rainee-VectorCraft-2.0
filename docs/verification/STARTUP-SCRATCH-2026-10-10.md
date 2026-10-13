# Startup and Scratch Verification

2026-10-10. This first implementation covers component 3.1, Font Discovery Binary Cache, and the scratch routing needed by that consumer. It does not complete the other startup, navigation or stylus components.

## Font Discovery Cache

The [font database](../../crates/text/src/fontdb.rs) still discovers installed fonts and reads their original bytes only when needed. Its [binary cache](../../crates/text/src/fontdb/cache.rs) stores the family/style catalogue, classifications, aliases, PostScript-name mappings and a filesystem inventory. It contains no copied font files or glyph data.

`fonts.bin` has a schema-version header and a checksummed gzip payload. Reads are limited to 8 MiB encoded and 32 MiB decoded. Cache reuse requires the current scan roots, directory timestamps and each candidate font file's size/timestamp to match. Font discovery and interpretation changes require a schema-version bump. A corrupt, incompatible, oversized or unavailable cache falls back to live discovery. Writes use exclusively-created temporary files and atomic replacement; failed writes preserve the previous cache. File changes which deliberately preserve both size and timestamp are outside this metadata invalidation contract.

Installed-font checks now include individual file metadata, so a replacement in place can trigger a rescan without its parent directory changing. The existing font-menu refresh and rescan path remains. The saved Additional Fonts Folder is applied before the first desktop background scan.

## Scratch Routing

[Desktop startup](../../apps/vectorcraft/src/main.rs) configures the cache before the font scan. The [scratch resolver](../../apps/vectorcraft/src/scratch.rs) tries the primary preference, the secondary preference, then the default cache directory. Under each chosen root, only `VectorCraft Scratch/font-discovery-v1/fonts.bin` belongs to this consumer. `Startup` means the default cache under the app preferences directory; `None` disables the secondary candidate. Missing, linked, blocked or unwritable reserved subdirectories are skipped. A cache write failure during discovery falls back to live discovery rather than blocking startup.

The existing drive selectors remain. Absolute directory values are also supported by the resolver; the folder-picker UI, capacity gauges and Purge Cache button are still component 7.3 work. Preference changes apply to font-cache routing on the next launch. Runs with `VECTORCRAFT_NO_PREFS` do not use this disk cache. The log reports the resolved cache path, cache outcome, face count and discovery timings.

Recovery copies, documents, original fonts, ICC profiles, undo history and assets are not relocated or purged. No broad cleanup command was added.

## Checks

| Check | Result |
| --- | --- |
| Text crate unit tests | 197 passed |
| Fresh-process installed-font integration test | 1 passed |
| Isolated scratch-routing tests | 3 passed |
| Native desktop compile check | Passed |
| Text library WebAssembly compile check | Passed |
| Text and desktop Clippy with `--no-deps` and `-D warnings` | Passed |
| Formatting and whitespace checks for changed source | Passed |

The three new [font-cache tests](../../crates/text/src/tests_sysfonts.rs) cover metadata-only round trips, lazy loading, aliases/styles/traits, installation/removal, timestamp/size changes, different discovery roots, old/truncated/corrupt caches, an unwritable cache destination and regeneration after removing the disposable entry. The three [scratch-routing checks](../../apps/vectorcraft/tests/scratch_routing.rs) cover `Startup`/`None`, unavailable-primary fallback and preservation of existing files and recovery data. The isolated routing harness was compiled directly with `rustc --test` and the built `log` dependency, avoiding a graphics-application link.

An all-dependency strict Clippy invocation stopped on four existing `collapsible_if` findings in `crates/doc/src/setup.rs`. That unrelated canvas-color code was not changed. This is not a full workspace CI pass.

## Local Discovery Measurements

The [benchmark](../../crates/text/examples/font_cache_bench.rs) uses a fresh database for each mode on the same Windows machine. Each trial scans 1,760 faces. Baseline disables the new disk cache; cold creates a new metadata cache; warm reuses it. These are development-profile discovery measurements, not OS-cold-cache or complete desktop startup measurements. The benchmark uses Windows font directories and registry paths; it does not install the desktop DirectWrite callback for service-managed fonts.

| Trial | Cache disabled | New cache | Warm cache |
| --- | --- | --- | --- |
| 1 | 345.9141 ms | 344.8794 ms | 168.9875 ms |
| 2 | 414.7366 ms | 361.2059 ms | 137.9477 ms |
| 3 | 314.7311 ms | 338.7499 ms | 135.7031 ms |
| Median | 345.9141 ms | 344.8794 ms | 137.9477 ms |

All warm trials reported a cache hit; all cold trials wrote their metadata cache. Median warm discovery was about 60% faster than cache-cold discovery in this small local sample. This does not establish the draft's 80-90% target or a whole-app startup speedup.

Reproduce from the project root, using an output directory which does not already exist:

```powershell
cargo test -p vectorcraft-text
cargo clippy -p vectorcraft-text --no-deps --all-targets -- -D warnings
cargo clippy -p vectorcraft --no-deps --bin vectorcraft -- -D warnings
cargo check -p vectorcraft
cargo check -p vectorcraft-text --lib --target wasm32-unknown-unknown
cargo run -p vectorcraft-text --example font_cache_bench -- C:\Temp\vc-font-cache-new-run
```

## Remaining Components

- **3.1:** package a runnable updated build and measure full cold/warm desktop launch, including service fonts and manual live installation checks. The existing `vectorcraft-gemini.exe` is unchanged.
- **3.2:** retain recovery copies. The default autosave interval is two minutes (`Prefs`); an incremental journal needs an agreed loss-window requirement and fault-injection plan before implementation. No journal or native-format migration was added.
- **3.3:** keep the existing node-identity and device-pixel thumbnail cache in memory. Disk reuse needs stable content/render/DPI keys and edit-invalidation fixtures before implementation.
- **3.4:** `IccProfile` already caches direct transforms in memory. Evaluate disk reuse and transform costs without changing ICC behavior or duplicating that cache; color-error tolerances remain to be agreed.
- **4.1:** leave the owner-reported infinite canvas and independent colors intact. Finite export/save/reopen acceptance remains separate.
- **4.2 and 4.3:** compare the current 2D grid and culling with zoom/DPI/effect-bound fixtures. No new spatial index or 3D work was added.
- **5.1 and 5.2:** recorded stylus fixtures and latency measurement precede optional stabilization and display-only prediction. Neither committed paths nor existing smoothing behavior changed.

3D Stage and animation remain outside this implementation.
