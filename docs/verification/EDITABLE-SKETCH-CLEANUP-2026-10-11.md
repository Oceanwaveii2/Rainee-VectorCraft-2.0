# Editable sketch cleanup implementation and acceptance

VectorCraft now has an experimental selected-stroke cleanup integration using the pinned StripMaker research worker with SCIP and OSQP. It consolidates ordinary vector stroke bundles into editable centerline paths. It is not Smart Inker, junction correction, natural-media painting or GPU acceleration.

## Supported artwork

Select 2 to 64 consecutive sibling paths with identical ordinary stroke appearances. They must be open, visible and unlocked, without fills, masks, effects, live shapes, pressure profiles, tilt nibs or brush appearances. Unsupported artwork is rejected rather than silently stripped of its appearance. Each input path is capped at 4096 samples, with 16384 samples across the selection and bounded control-polygon length.

The first integration uses the native worker's fitted centerline samples as editable polyline anchors. Singleton groups keep their original geometry exactly. This does not yet produce minimal cubic Bezier representations or transfer expressive pressure and nib data onto a consolidated bundle.

## Preview and acceptance

The Sketch Cleanup button opens the experimental panel. Generate Preview starts a background worker; cyan centerlines show proposed geometry without changing the document. Curve sampling tolerance, maximum deviation and timeout are configurable. Accept creates cleaned paths and moves their source strokes into hidden Cleanup Originals groups, retaining source IDs. The edit is one undo step. Reject and Cancel leave artwork unchanged.

The job is bound to its document UID and original document-content snapshot. Pan, zoom and selection can continue without replacing that content snapshot. Content changes, document switches and closure revoke the proposal. Acceptance checks target snapshots again, including during command-journal replay. Resolved geometry is stored in the acceptance journal entry, so replay does not rerun the research solver.

Proposal validation checks finite geometry, complete and unique source membership, supported appearance, bounded output and distance in both directions. Segment interiors are sampled with a conservative distance margin; a comparison budget rejects oversized validations. These guards limit geometric displacement, not artistic interpretation. A creator must inspect proposed grouping, particularly near hatching, corners and junctions.

## UI and MCP commands

| Command | Purpose |
| --- | --- |
| `window.sketchCleanup {open?: bool}` | Open, hide or toggle the controls in the native UI |
| `sketch.cleanup.start {ids?, tolerance?, maxDeviation?, timeout?}` | Capture a bounded selection and generate a preview |
| `sketch.cleanup.status {}` | Poll state, document binding, backend availability and proposal geometry |
| `sketch.cleanup.cancel {}` | Request worker cancellation and clear pending work |
| `sketch.cleanup.reject {}` | Clear the preview without an artwork edit |
| `sketch.cleanup.accept {job?}` | Apply the current proposal as one reversible edit |

Start defaults are 0.25 pt curve sampling, 4 pt maximum deviation and a 30-second deadline. The command registry exposes all parameters through the existing MCP `list_commands` and `run_command` tools. The optional `proposal` acceptance parameter carries a resolved journal plan; it must match the current source nodes and pass the same safety checks. It cannot select an executable or arbitrary output path.

## Research worker installation

The Rust editor does not link author C++ code into its process. An explicitly requested cleanup job invokes the installed Windows research worker at `D:\DevData\Rainee\build\stripmaker-scip\author\Release\clustering-solve.exe`. Its SHA-256 must be `F3029840132FFE65D73AB2D800947BA6F6C2227246DBFE8966323D569BF8C053` before execution. Runtime DLL search is process-local. Jobs and diagnostics stay under `D:\DevData\Rainee\scratch\cleanup`.

The worker is optional and currently machine-specific, not a portable production dependency. Other platforms report it unavailable. Cancellation and deadlines terminate the child; malformed or unsuccessful output never becomes an accepted edit. No Gurobi runtime, license, GPU renderer replacement or filesystem capability is exposed by cleanup commands. The [native backend verification](STRIPMAKER-SCIP-OSQP-2026-10-11.md) records the author revision, models and solver contract.

## Verification

Eight cleanup unit tests pass, plus two optional real-worker integration tests and one MCP-discovery test. Coverage includes preview/rejection/cancellation without edits, content staleness, unsupported expressive strokes, malformed/duplicate/missing targets, one-step undo/redo, native save/reopen, SVG and PDF export, and resolved-plan replay. Nine existing expressive-stylus tests and four spatial/stylus tests also pass; the physical recording test remains a separate ignored hardware test.

The packaged native app was exercised through its control channel and a live stdio MCP client connected to that exact instance. Initialization, command discovery, background cleanup, cancellation, rejection, stale-content rejection, acceptance and undo/redo passed. Native reopening retained the cleaned node exactly; SVG and PDF artifacts were written. The whole-window preview and trial screenshots were inspected without cropping.

| Native artwork fixture | Result | Time to terminal state |
| --- | --- | --- |
| Three overlapping strokes, one reversed | One proposed cleaned group | 0.224 seconds |
| Intentional gaps and a distant stroke | Three separate groups; no consolidation accepted | 0.170 seconds |
| Separated hatching | Three separate groups; no consolidation accepted | 0.165 seconds |
| Sharp corner bundle | Fit rejected because it discarded too much original geometry | 0.284 seconds |
| Curved bundle | One proposed cleaned group | 0.326 seconds |
| Branched hair curves | Three separate groups | 0.462 seconds |

These times describe six small synthetic fixtures on this machine, not large-document performance or owner-artwork quality. The rejected corner is a retained regression case, not a successful corner-preserving cleanup. Fitting protected corners and consolidating expressive strokes remain follow-up work.

Artifacts and machine-readable results are under `D:\DevData\Rainee\verification\editable-cleanup-package`, including `artwork-results.json`, `mcp-live-results.json`, `accepted.vectorcraft`, `accepted.svg`, `accepted.pdf`, `preview.png` and `cintiq-trial.png`. The ready trial tab is named Cleanup Cintiq Trial. Physical Cintiq cleanup acceptance is pending owner feedback.

The runnable app and CLI are packaged under `D:\DevData\Rainee\build\cleanup-native-test\Editable-Cleanup-2026-10-11`. The tested app SHA-256 is `1DFDE4448F337F9CCA78B1381F3E3BF1F1774CD028097887BBEF91781D10C775`; the dependency-lock hash is `B86E993FC274D2D1FA4983D667E8F8DB3BB8C2DFB8E0794CD368E9CD45D18C3F`. The build is based on source revision `4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd` with the existing local edition changes and this cleanup integration. It is an experimental machine-specific package, not an upstream release.

WebAssembly engine checking and dependency-layer checking pass. Engine Clippy passes with warnings denied. The broader UI Clippy check completes with five existing warnings in the native pen adapter, canvas-color controls and splash code, outside this cleanup implementation.

```powershell
. ./integrations/storage/Use-DDrive.ps1 -Target Editor
cargo test -p vectorcraft-engine --lib cleanup --offline
cargo test -p vectorcraft-engine --lib cleanup_real_worker --offline -- --ignored --nocapture
cargo test -p vectorcraft-mcp sketch_cleanup --offline
cargo build -p vectorcraft --offline
```

For a disposable native instance with no owner documents open, run [Test-EditorCleanup.ps1](../../integrations/stripmaker-lab/Test-EditorCleanup.ps1). It checks discovery, records broader artwork outcomes, captures the preview and exercises acceptance, undo/redo, native reopening and finite SVG/PDF export. [Test-McpCleanup.ps1](../../integrations/stripmaker-lab/Test-McpCleanup.ps1) performs the live MCP handshake, cancellation/rejection/staleness checks and reversible editing. Physical Cintiq cleanup testing and representative owner artwork remain acceptance requirements; the previous rear-eraser confirmation does not establish cleanup quality.

## Implementation locations

- [Engine lifecycle and validation](../../crates/engine/src/cleanup.rs)
- [Pinned isolated worker](../../crates/engine/src/cleanup/worker.rs)
- [Shared commands](../../crates/engine/src/cmd/cleanup.rs)
- [Thin UI controls](../../crates/ui-egui/src/cleanup_panel.rs)
- [Engine acceptance tests](../../crates/engine/src/tests_cleanup.rs)

The source drawings, user preferences and Wacom driver mappings are not changed by verification on a disposable instance.
