# VectorCraft Gemini v1.2 - capability audit

Audit date: 2026-10-10, America/Los_Angeles. Purpose: correct the workload against the actual application, especially the mistaken implication that storage had to be built or replaced. This is an implementation assessment, not a new owner architecture lock.

## Inspected artifacts and evidence boundary

- Source package: `D:/0/Rainee drawing app/ArtCraft-Foundation/VectorCraft-Gemini-v1.2-Workflow-NoGPU.zip` (15,096,853 bytes; 1,500 entries; no `.exe` entry).
- Source checkout: `D:/0/Rainee drawing app/ArtCraft-Foundation/VectorCraft - Gemini Edition`.
- Executable used: `vectorcraft-gemini.exe` in that checkout (85,402,624 bytes). Native save output reports generator `VectorCraft 1.2.0`.
- ZIP SHA-256: `01CA07DF3C31DB90C344431349D79797E165F16BE35D6C41B8E22A9FF21C3F12`.
- EXE SHA-256: `BFF8F17BB1FF36716383725B3916E88511C04645E5D1A758FA7ED8A10A2C63A3`.

Before this documentation edit, 15 relevant local files were byte-hash compared against their ZIP entries: Cargo.toml; format/lib.rs and atomic.rs; doc/lib.rs and setup.rs; tools/distort/arap.rs; ui-egui/chrome.rs, panels/layers.rs, panels/history.rs, prefs_dialog.rs, codex_panel.rs and canvas.rs; apps/vectorcraft/main.rs; the original workflow; and integrations/README.md. All matched. This confirms the inspected source sample, not every ZIP entry or a reproducible binary build.

The executable predates the ZIP. Matching version strings do not prove it includes every later source change. Infinite-canvas controls, separate desk/sheet colors and the revised splash exist in the source; their complete current behavior was not validated by this audit. Do not convert source presence or an edition note into a blanket shipped/accepted claim.

Existing application edits were left intact. The original ZIP and executable were not replaced or rebuilt.

### Existing baseline follow-up

The runnable baseline already exists; release bookkeeping is not an unfinished bootstrap milestone. A documentation follow-up rechecked `vectorcraft-gemini.exe --version` (output `vectorcraft 1.2.0`) and its SHA-256, which still matches the EXE hash above. The existing save/reopen and undo/redo evidence remains the recorded baseline; those runtime fixtures were not rerun for this follow-up.

The checkout HEAD is [storytold/vectorcraft commit 4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd](https://github.com/storytold/vectorcraft/commit/4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd), confirmed through GitHub's public commit API. The current `Cargo.lock` SHA-256 is `8BDD57E227A762A68F2561B02A936678F096F4301F873C14DA55B50BDD5C9D2B`. Local Gemini edits are present in the checkout and must be preserved; the upstream commit does not establish which local edits are included in the binary.

No rebuild, dependency change, Git commit, fetch/pull or replacement of the existing executable was required for this documentation correction. Future changed releases should retain their own build records and regression results without treating the initial build as missing.

## Storage is already implemented

| Capability | Source evidence | Verification here |
| --- | --- | --- |
| Native serialization | [format/lib.rs](../crates/format/src/lib.rs), lines 1-29 and 49: versioned JSON metadata, optional gzip, v4 out-of-line binary data, preservation of unknown fields, linked-image proxies and optional embedded assets/PDF/profiles. | Plain v4 file and gzip file reopened without warnings; layer name and both shapes retained. Binary-blob/linked-asset fixtures were not exercised here. |
| Atomic writes | [format/atomic.rs](../crates/format/src/atomic.rs), lines 1-46: a same-directory temporary file, `sync_all`, then replacement; cleanup on failure. [fileio/mod.rs](../crates/engine/src/cmd/fileio/mod.rs), lines 719-720, calls this writer. | Successful background saves observed. Failed-write/power-loss fault injection was not run; do not promise power-loss durability on every filesystem. |
| Save lifecycle | [fileio/save.rs](../crates/engine/src/cmd/fileio/save.rs): shared Save/Save As/Copy/Template planning, encoding and saved-state handling. [background.rs](../crates/ui-egui/src/background.rs): worker jobs. | `app.save` returned a background job; completion was verified by file reading, clean document state and reopen. |
| Recovery | [engine recovery.rs](../crates/engine/src/cmd/recovery.rs), lines 1-32: document and metadata copies, instance areas/locks, restore/discard, removal after save. [UI recovery.rs](../crates/ui-egui/src/recovery.rs), lines 1-5 and 73 onward: timer, background write and startup offer. | A modified fixture produced a 27,993-byte recovery copy; listing showed it; saving removed it. Automatic interval timing and crash/restart restoration were not tested. |
| Preferences | [main.rs](../apps/vectorcraft/src/main.rs), lines 140-207: Windows `%APPDATA%/VectorCraft/ui.json`, load/restore and atomic save. | Disk persistence inspected in source. Live test deliberately disabled user preferences, so no claim of cross-restart verification. |
| Scratch-drive choices | [prefs_dialog.rs](../crates/ui-egui/src/prefs_dialog.rs), lines 358-374; preference fields in [engine/lib.rs](../crates/engine/src/lib.rs). | Enumerated C, D, E, G, H and L drives; applying D returned `D:\` from `prefs.get`. A repository-wide Rust search found no allocation/write-path consumer of `scratch_primary` or `scratch_secondary` beyond preference declaration/defaults and UI/tests. Disk routing is therefore not established. |

Conclusion: preserve native storage. SQLite/WAL/CBOR is not the current native format and is not an obligatory migration. A stored scratch-drive preference is a different capability from a working disk-backed scratch allocator; neither should be confused with the existing document save/recovery system.

## Existing UI and tools versus the earlier checklist

| Earlier item | Actual comparison |
| --- | --- |
| Build X/Y/W/H, fill/stroke and per-tool contextual controls | Already implemented in [chrome.rs](../crates/ui-egui/src/chrome.rs), lines 218-251 and 373-395. The test instance had `control_bar: false`; enabling it revealed the bar. Rotation is in the Transform popover. The remaining task is layout/defaults or an optional inline-angle control. |
| Build a layer/object manager | [layers.rs](../crates/ui-egui/src/panels/layers.rs), lines 1-29, already provides hierarchy, visibility, lock, rename, drag/reorder, targeting and selection. A C4D-inspired treatment is a presentation change, not a second data hierarchy. |
| Add HD layer previews and readable text | Gemini already implemented 76-point default rows, up-to-72-point displayed thumbnails, larger row text and 2x physical-size thumbnail rendering (capped at 256 pixels). See [layers.rs](../crates/ui-egui/src/panels/layers.rs), lines 72-83, 139, 608 and 859-893; [panel options](../crates/ui-egui/src/dialogs/layers_panel_options.rs) includes HD Preview/custom sizes. Runtime screenshots confirmed large previews, but names can still truncate in the existing narrow panel. |
| Add blend modes / history | Existing [Transparency panel](../crates/ui-egui/src/panels/transparency.rs) and [History panel](../crates/ui-egui/src/panels/history.rs) supply these functions. History selects existing undo/redo states; it is not a time-based animation system. Rename undo/redo passed live. Inline layer controls or a scrub-style history UI would be enhancements. |
| Implement Puppet Warp / ARAP | [arap.rs](../crates/tools/src/distort/arap.rs), lines 1-16, explicitly implements the SIGGRAPH 2005 two-step ARAP method. [puppet.rs](../crates/tools/src/distort/puppet.rs) and engine commands support editable pins/rest state. Static quality improvements are possible; a new basic solver or animation system is not required. |
| Implement Pencil / Brush / Width / Liquify / Mesh / Blend | Existing [freehand gestures](../crates/tools/src/draw2/gesture.rs), [freehand options](../crates/ui-egui/src/dialogs/freehand.rs), [brush definitions](../crates/brush/src/lib.rs), [width.rs](../crates/tools/src/distort/width.rs), [liquify.rs](../crates/tools/src/distort/liquify.rs) and [meshblend.rs](../crates/tools/src/meshblend.rs) are extension points. This audit did not visually accept every tool. |
| Implement Live Paint / Shape Builder | [builder.rs](../crates/tools/src/builder.rs) already implements both. The upstream [roadmap](../ROADMAP.md), especially the parity table's Path operations row, still lists Live Paint gap options and Shape Builder gap detection as gaps. Those are specific improvements, not missing whole tools. |
| Build tracing from zero | Image Trace make/expand/release, presets, color/fill modes and stroked centre-line parameters are exposed by the running command registry. Compare proposed NeuralTrace against this baseline before replacing it. No trace-quality benchmark was run here. |
| Build MCP infrastructure | [control.rs](../crates/ui-egui/src/control.rs) already exposes engine commands, document inspection, tools, gestures, dialogs, screenshots and save/open. [integrations/README.md](../integrations/README.md) documents the separate live MCP path. The [embedded Codex panel](../crates/ui-egui/src/codex_panel.rs) is explicitly read-only: document advice, not canvas edits. Remaining work is panel-to-approved-edit wiring, cancellation and packaging. |
| Adopt a whole node-graph document | [doc/lib.rs](../crates/doc/src/lib.rs), lines 1-5, describes a persistent `Arc<Node>` tree with cheap undo snapshots; appearance and live-object mechanisms already exist. [doc/graph.rs](../crates/doc/src/graph.rs), lines 1-22, implements charts, not a procedural effects graph. Optional hidden semantic relationships should extend this baseline only where required. |

## Live diagnostic record

Launched only the audit instance on localhost port 17979 with `VECTORCRAFT_NO_PREFS=1`. No user document was opened. All drawing/file writes were limited to the audit fixture folder. The app was asked to quit after the checks; no normal preferences were saved.

1. Created a 600 x 500-point document, a rectangle and an ellipse. Inspection reported three nodes including the layer.
2. Saved the plain fixture: native format `vectorcraft`, version 4, generator `VectorCraft 1.2.0`, 27,979 bytes. Reopened with no warnings and both shapes present.
3. Enabled the existing Control bar and captured it with the Layers panel.
4. Set an isolated recovery folder, renamed the layer and wrote/listed a recovery copy. It was removed by the normal save lifecycle, leaving no recovery copies.
5. Saved with compression: 1,907 bytes, gzip magic `1F-8B`. Reopened clean with three nodes and layer name `Persisted audit layer` intact.
6. Changed the layer name; Undo restored `Persisted audit layer`, Redo restored the temporary name, and a final Undo returned the fixture to its saved state.
7. Opened Performance & Storage, inspected enumerated drives and applied D as Primary. This demonstrates dropdown/preference behavior, not disk-cache routing.

Evidence files:

- [Large layer previews](verification/workload-audit-2026-10-10/layers-and-canvas.png).
- [Existing Control bar and large previews](verification/workload-audit-2026-10-10/control-bar-and-layers.png).
- [Scratch-drive preference dropdowns](verification/workload-audit-2026-10-10/scratch-drive-preferences.png).
- [Plain native fixture](verification/workload-audit-2026-10-10/storage-roundtrip.vectorcraft).
- [Compressed native fixture](verification/workload-audit-2026-10-10/storage-roundtrip-compressed.vectorcraft).

Screenshots were returned by the app at 2160 x 1350 pixels and inspected without cropping. The app reports GPU-backed DX12 presentation; [render/Cargo.toml](../crates/render/Cargo.toml) still uses `vello_cpu` for vector artwork. This is not a GPU artwork-renderer benchmark.

No rebuild, Rust test-suite rerun, model call, live MCP-client handshake, large-document performance run, foreign-format fidelity test, or full recovery fault-injection test was performed. The successful small fixture does not imply all those gates pass.

## Scope correction

The owner excludes real animation. Do not add animation timelines, keyframes or playback, or treat them as deferred obligations. All Stage/3D work is deferred to last and has not yet been designed; the plane/sphere placeholder is not an approved prototype task. The Cinema 4D mockups describe a future multi-canvas spatial-composition concept, and a Blender fork/mature 3D host is an option for later evaluation, not a selected engine or current implementation. Static artwork deformation remains in scope. The [updated workload](VECTORCRAFT-MASTER-WORKFLOW-NO-GPU.md) preserves existing storage and includes the requested component breakdown and Preferences & Scratch Disk Management Panel (7.3); these additions are plans, not changes to the capabilities verified by this audit.

The owner accepts the conventional drawing tools as an excellent baseline and directs the drawing milestone toward ML-assisted capabilities beyond their current Illustrator-style workflows, not a generic repair/parity pass. Full MCP and embedded Codex editor control is also a product requirement: all implemented tool/action capabilities should be accessible to the AI through the existing command/control architecture. The current separate MCP editing path and read-only embedded panel remain the audited implementation boundary; the expanded milestone does not claim that full panel-driven control or the ML enhancements are already implemented.
