# Spatial canvas and stylus controls

October 11, 2026. Implemented in the local VectorCraft Gemini Edition checkout for Rainee workflow components 4 and 5. This is 2D canvas work. No 3D Stage, animation, document-format migration or new spatial index is introduced.

## Infinite board and navigation

The existing infinite board and independent backdrop/artboard colors are preserved. Artboards still define finite PDF and raster export frames. A regression fixture saves and reloads both compressed and plain native documents containing negative-coordinate art, artwork crossing an artboard edge and artwork far outside it. It checks the retained colors/bounds, a finite PNG and a finite single-page PDF.

Finite and infinite board grids share nested powers-of-two spacing. Major lines stay approximately 24–48 logical screen points apart; minor lines and dots fade across zoom transitions. Lines use one physical pixel; dots align to physical pixels and scale with display DPI. The document's stored grid spacing and existing snapping settings are unchanged. Pixel Preview keeps its separate one-document-pixel measurement grid above the existing 600% threshold.

Screen bounds now include all four transformed rectangle corners. Coordinate fixtures include pan, negative coordinates, five view rotations and stress zooms from 0.001 to 1000, beyond the normal UI zoom range. Existing renderer bounds/culling are reused. A fixture checks that an offscreen object's shadow reaching the viewport remains visible while a distant object is culled, including a cached second render.

## Stylus controls

Double-click Pencil or Paintbrush to open its Tool Options. Existing Fidelity, fill and endpoint settings remain. Each tool saves its own additional controls:

| Control | Default | Behavior |
| --- | --- | --- |
| Speed-adaptive stabilizer | Off | Optional 1 Euro input filter; Ctrl bypasses it |
| Low-speed cutoff | 1 Hz | Range 0.1–60 Hz; higher values follow slow input more closely |
| Speed response | 0.01 | Range 0–2; increases cutoff as input speeds up |
| Preview prediction | 0 ms, off | Configurable up to 16 ms; try 12–16 ms |
| Magnetic perspective rays | Off | Attracts to visible planar rays/vertical axes or sampled curved reference lines |
| Attraction strength | 0.75 | Range 0–1; attraction fades to zero at the 12-screen-point tolerance boundary |
| Angle lock | Off | Hold Shift for optional 15°, 30° or 45° locks |

Ctrl/Command bypasses new magnetic and angle constraints. Pencil's existing endpoint Smart Guides retain their priority. Nearby attractions blend rather than switching abruptly between equally close rays. Intentional angle locking is separate from magnetic attraction.

Prediction is a temporary stroke-head overlay anchored to accepted input. It never enters the committed freehand sample list or saved document. It is bounded to 24 logical screen points and disappears after a pause, reversal, sharp raw-input turn, cancellation or release. Prediction is suppressed during modifier bypass or magnetic/angle constraints. Turning it off leaves committed paths unchanged. Manual editing and the existing Begin/Preview/Commit undo flow remain.

Native events use the UI's monotonic frame clock. Automation can supply monotonic sample timestamps in seconds using `pointer_gesture.events[].time`; omitted timestamps use a deterministic 120 Hz fallback. These are not OS-level high-frequency pen timestamps or end-to-end latency measurements.

The filter implements the equations described by [Casiez, Roussel and Vogel's 1 Euro Filter reference](https://gery.casiez.net/1euro/) and [CHI 2012 paper](https://dl.acm.org/doi/10.1145/2207676.2208639). The implementation is original; it does not copy another application's source.

## Perspective modes

Use View → Perspective Grid → Define Grid to select the mode. Existing presets and tools remain in use. Grid definitions save with the document; user presets save in preferences.

| Mode | Projection and controls | Artwork behavior |
| --- | --- | --- |
| 1/2/3-point | Existing planar homographies and vanishing-point controls | Existing plane attachment, selection and editing remain |
| 4-point cylindrical | 360° longitude, forward/left/right/rear seam reference points; center, horizon, viewing distance and density | Reference grid only; draw ordinary editable 2D paths |
| 5-point hemispherical | Equidistant 180° hemisphere, forward/left/right/up/down reference points; center, horizon, viewing distance and density | Reference grid only; draw ordinary editable 2D paths |

Curved grids are not homographies. Planar attachment/drawing/transform commands reject them rather than applying an incorrect warp. Their planar widget and digit-plane shortcuts are inactive. Horizon and origin handles remain available through the Perspective Grid tool, with distance and density set in Define Grid. Existing attached artwork is not reprojected simply by selecting a curved reference grid. Nonlinear artwork attachment is not implemented.

## Automation

The new controls use existing commands and option discovery:

```json
{"command":"tool.setOption","params":{"tool":"pencil","values":{"stabilize":true,"minCutoff":1,"beta":0.01,"predictionMs":14,"magnetic":true,"snapStrength":0.75,"angleLock":30}}}
```

```json
{"command":"perspective.grid.preset","params":{"kind":5}}
```

`tool.setOption` with only a tool ID reads its options. `perspective.grid.get` reads the grid/definition/station; `perspective.presets.list` includes both curved presets. The existing command catalogue describes the new mode range and unavailable planar attachment. No separate AI-only drawing implementation is added.

## Verification and remaining acceptance

The new integration fixtures verify finite exports and native persistence, all five perspective modes, undo/redo, per-tool preference persistence, Ctrl bypass and byte-identical committed documents with prediction on/off. Synthetic math fixtures exercise jitter, fast motion, pauses, clock discontinuity, corners, reversals, attraction boundaries, intersecting rays and angle choices. Headless egui frames check the actual option dialogs and perspective selector.

At 120 Hz, the synthetic alternating ±1-point jitter fixture averages 0.0270 points after filtering. The 1200-points/second straight-line fixture averages 14.696 points of trailing distance, equivalent to 12.247 ms at that speed. These figures describe the supplied synthetic trajectories, not hardware latency, general tremor removal or an optimum setting.

Eighteen generated grid fixtures cover finite/infinite boards at 1×/2× DPI and stress zooms, plus dot style and both curved projections. They rasterize the actual grid painter shapes using VectorCraft's renderer; they are not screenshots of a mounted desktop window. Fixture output and test logs are under `E:\CodexBuild\VectorCraft-Spatial-2026-10-10`.

Real pen testing remains required for end-to-end latency, lag/overshoot perception and corner retention across devices and zoom levels. Prediction therefore remains off by default. No hardware speedup or complete milestone acceptance is claimed.

### Regression results

The native engine suite passed 1500 tests, tools passed 280, and renderer passed 168. The UI suite passed 825 tests with seven known failures from the wider run excluded. The affected areas are OS cursor fixtures (two), existing splash/Codex/catalog coverage (three), simulated-paper color and titlebar overlap. Those failures are not hidden by changing their assertions. All new labels have entries in the eight complete-language catalogs; the existing untranslated splash labels still prevent the global translation gate from passing.

Native application and CLI compilation passed. Engine/tools/UI also compile for `wasm32-unknown-unknown`. A live headless stdio MCP smoke test initialized the server, discovered the timestamp schema, set the new Paintbrush controls, created an editable path with timestamped events, selected the hemispherical grid and confirmed that attachment discovery reports an explicit disabled reason. This was not an embedded model-panel or mounted desktop handshake.

Strict Clippy remains blocked by existing nested conditionals in `chrome.rs` and `Iterator::last` in `dialogs/splash.rs`. No blanket lint suppression was added. Whitespace checks pass; the whole checkout is not being described as release-clean.

### Build instructions

The [development executable](../../dist/VectorCraft-Spatial-Stylus-2026-10-11/vectorcraft.exe) and [CLI](../../dist/VectorCraft-Spatial-Stylus-2026-10-11/vectorcraft-cli.exe) are saved together with licenses and a [build readme/checksums](../../dist/VectorCraft-Spatial-Stylus-2026-10-11/README.md). They retain base version 1.2.0; the dated package name identifies this local build. Final focused grid, Pixel Preview, engine integration and stylus tests were rerun after the lint fixes, followed by native compilation and the wasm check.

Large compiler output was placed on E: because D: has limited space. The runnable package is a development build, with debug information and optimization disabled for this verification cycle. It is not a performance-qualified release or a portable Codex bridge package. The existing Gemini executable is not overwritten.

```powershell
$env:CARGO_TARGET_DIR='E:\CodexBuild\VectorCraft-Spatial-2026-10-10'
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_PROFILE_DEV_OPT_LEVEL='0'
$env:CARGO_PROFILE_TEST_OPT_LEVEL='0'
cargo build -p vectorcraft -p vectorcraft-cli --config 'profile.dev.package."*".opt-level=0'
cargo test -p vectorcraft-engine -p vectorcraft-tools -p vectorcraft-render -p vectorcraft-ui-egui --lib --no-run --config 'profile.dev.package."*".opt-level=0'
```

Set `VECTORCRAFT_SPATIAL_FIXTURES` to an output directory and run the UI test binary filtered by `tests_spatial` to regenerate the eighteen pictures. Original artwork is not required or bundled.
