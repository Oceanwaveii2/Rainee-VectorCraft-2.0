# Brush ink and cleanup build

This batch improves brush deformation, adds editable procedural ink controls, expands safe cleanup input and packages the editor with its MCP CLI and embedded Codex bridge. Existing recorded Cintiq sessions remain the regression input. GPU companion and 3D Stage work are unchanged.

## Brush fidelity

Art brushes now project the measured nib profile onto each local curve normal. Pressure width and tilt deformation apply together; a round nib reproduces the previous geometry. Original Art brush definitions and centerlines are retained. Tests cover a straight flat nib, round-nib compatibility, changing nib orientation along curves and existing pressure-profile deformation.

The local nine-brush matrix exercises straight strokes, S curves, sharp turns and reversals with controlled pressure and tilt. These are authored fixtures rather than new hardware recordings or measurements from another application. They establish editability and geometry behavior, not independent matching of an unknown complete style library. The original owner brush library stays separate from distributed app defaults.

## Editable procedural ink

Natural brushes expose four additional parameters through the Brushes panel and shared command system:

- `inkLoad`, from 0 to 2, changes contact coverage and darkness.
- `edgePooling`, from 0 to 1, darkens contact toward the nib edges.
- `breakup`, from 0 to 1, introduces seeded bristle contact gaps.
- `paperScale`, from 0.125 to 128, changes document-space paper wavelength; the UI slider covers 0.125 to 8.

Neutral values preserve legacy serialization and geometry. Invalid new values are rejected before editing a brush definition. The effects regenerate from the same editable path, width and nib profiles; seed, grain and new parameters persist with the document. Geometry remains bounded by the existing fiber and patch limits. Ink load zero intentionally hides deposition without deleting the source path.

Pooling is procedural edge darkening, not simulated fluid transport. There is no new stateful wet paint, smudging or learned reference fitting. NeuBE remains the separate experimental appearance assistant. More advanced wet-media interactions remain future work.

## Eraser correction

The combined fixture exposed an ordinary long Bézier that a narrow eraser missed. Fixed sampling at 24 positions could skip the entire eraser corridor. The cutter now selects sampling density from a derivative bound and the eraser size, retains exact Bézier subsegments at detected cut boundaries and caps subdivisions for pathological imported geometry. The original failing curve is a regression test. This fixes the demonstrated miss without claiming mathematically exact intersections for every extreme coordinate or tangency.

## Cleanup quality and feedback

Uniform Art and Natural brush centerlines can now use consolidation while retaining their brush definition and hidden originals. Sources still need matching appearances and consecutive sibling placement. Pressure/nib profiles, scatter/pattern/bristle effects, fills, closed paths and masks remain protected by refusal; they are not silently flattened.

`sketch.cleanup.inspect` performs read-only bounded preflight using the same admission checks and settings as Generate Preview. The panel caches the result by document, revision, selection, active-gesture state and settings, displays the refusal reason and disables an ineligible preview request. Existing no-safe-bundle reporting, cancellation, approval, deviation limits and source retention stay in place.

## Build and verification

The brush suite passes 46 tests; focused brush Clippy passes. Five engine brush tests pass, including atomic parameter validation, source preservation, undo and native roundtrip. Thirteen cleanup tests and both installed worker tests pass, covering intentional gaps and hatching as well as retained source identity. The eraser suite passes 14 tests after the long-curve fix. The earlier broad engine run passes 1536 tests with three optional tests ignored; final verification and package details follow below.

The first native matrix run produced the nine-brush and four-ink appearance sheets, which were visually inspected. Its eraser failure was an editor defect, not dismissed as a fixture problem. Results before and after correction remain in separate local directories.

The package is a labelled optimized development build for this Windows machine, not a fully self-contained installer or release-performance certification. It includes the editor, MCP CLI, Python bridge, supplied bridge toolkit and hash-checking launcher. The manifest records executable and Rust-source hashes, dependency lock hash and machine runtime paths. Existing Python/Codex authentication and optional research runtimes remain machine-local. No credentials, font files or owner brush art are bundled.

## Final package checks

Package: `D:\DevData\Rainee\build\VectorCraft-Rainee-Ink-2026-10-11-Verified`. Editor SHA-256: `886EDFCC74284395EE4DBF2E3514821D9164CFF7DB36C862F95322B54D3D0A30`. The manifest confirms the existing Python runtime, StripMaker worker, AOSP model and NeuBE environment paths exist; existence is distinct from a model-quality result. Use `Launch.ps1` for the bundled toolkit and D-drive cache/temp configuration. Local MCP stays off unless a control port is explicitly supplied.

The final package passed `Test-BrushInkBatch.ps1` and a live MCP handshake with discovery, actual ink editing, invalid-option rejection, source preservation, undo/redo and cleanup preflight. Native documents, finite SVG/PDF outputs, PNGs and result JSON are in `D:\DevData\Rainee\verification\brush-ink-final-package-2026-10-11`. The corrected curved-eraser gaps and cleanup source result were visually inspected; the expanded Brushes panel shows all four new controls. Signed-in model inference was not rerun in this batch; the embedded controller and its bridge were packaged without changing their authorization behavior.

Final verification passes 1537 engine tests with three optional tests skipped, all 46 brush tests, 14 eraser checks, the cleanup-panel cache test, five pen-button tests and ten native-input checks. Both installed cleanup-worker tests pass separately. Brush Clippy and the full UI WASM check pass. Two local Cintiq replay tests pass against the updated source: 99420 packet deliveries across eight drawing configurations, recorded button-key handling, identical document layers between batch sizes, save/reopen and undo/redo. Editable replay outputs and log are in `D:\DevData\Rainee\verification\cintiq-canvas-replay-2026-10-11-ink-batch`. No new physical trial was required.

The 5000-path synthetic performance run meets its listed budgets. The 50000-path run exceeds seven budgets: fit-page refresh 135.87 ms, pan refresh 138.87 ms, zoom refresh 43.09 ms, hit testing 2.12 ms, native open 679.52 ms, SVG export 599.78 ms and handle dragging 8.80 ms. Save, 1000-path Unite and segment dragging meet their listed budgets. These are development-build timings taken alongside test activity on Windows with unavailable load-average reporting, not clean release benchmarks or proof of a regression. The log is `performance-50000.log` in the final-package verification directory. Large-document optimization remains open; no GPU replacement was introduced to hide that limitation.

`cargo xtask ci` stops at formatting differences in pre-existing modified workspace files. The Rust files changed in this batch pass their targeted formatting check. No commit, full-workspace CI pass, production performance or complete unknown-library style match is claimed.
