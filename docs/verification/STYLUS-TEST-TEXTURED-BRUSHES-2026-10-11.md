# Stylus tests and textured brush implementation

The automated Apple-inspired stylus behavior suite passed before the textured brush batch. This is Windows Ink and Cintiq-path verification, not a physical Apple Pencil test, PencilKit reproduction or input-to-photon measurement.

## Stylus results

- Five prediction-reconciliation tests pass: actual input replaces temporary ink, obsolete replies cannot return, epochs reset safely, and missing channels clear causal history.
- The explicitly enabled installed AOSP model worker test passes. It is not Apple's predictor.
- Eleven native packet-adapter tests pass for chronology, repeated history, stationary pressure/buttons, missing channels, eraser identity, unsupported roll and promoted-mouse handling.
- Five stabilizer tests pass for jitter/lag behavior, bypass, clock discontinuities, pauses, reversals and snapping boundaries.
- The engine's `learning_is_optional_and_does_not_change_committed_strokes` test passes on the current source. An earlier directly invoked older executable matched zero tests and is not counted as evidence.

Physical Cintiq responsiveness and a true Apple Pencil comparison remain separate acceptance checks. No driver settings or owner drawings were altered for this suite.

## Textured source brushes

Three original presets extend the existing Natural type: Rainee Graphite Grain, Rainee Soft Graphite and Rainee Textured Ink. A `grain` parameter selects granular, paper-modulated deposition; zero preserves the original fiber renderer. Old serialized brush definitions omit the zero field and retain their appearance. Existing owner drawings are not migrated to these new presets.

Deposition consists of bounded vector cells in opacity bins, regenerated from the source path and editable profiles. Document-space paper modulation does not depend on zoom or the viewport. Width/nib profiles change envelope and deposition; profile-derived load is a proxy, not newly captured raw pressure. There is no pigment chemistry, wet mixing or simulated paper erosion. Long strokes share the 8,192-cell budget, so grain coarsens on large inputs.

The Brushes panel exposes Paper grain. `brush.new` discovery documents it; ordinary `brush.edit`, `brush.get`, undo and native persistence remain the control route. The native synthetic test accepts explicit brush names to exercise new presets without changing its original default fixtures.

## Remaining coverage

The supplied Sketch File imports 3,021 objects and 14 layers. Imported names identify body/arm/hip artwork; the normal importer does not recover its brush library. A subsequent isolated metadata/path audit recovered nine brush-section silhouettes from the first 256 MiB of decoded private data. The remaining large drawing payload was not decoded. Zero Brush File was fully decoded for this audit and contains three brush sections. Originals remain unchanged; recovered user geometry stays outside the source/package assets.

The owner clarified that the target brushes are vector objects running along a path. Vector Art brushes are therefore the immediate priority; textured deposition remains optional. Art-brush rendering now respects asymmetric editable width profiles, and expressive freehand retains the chosen Art brush rather than replacing it with a generic pen. Original Hair Taper, Split Ink Art and Frayed Ink Art fixtures also exercise this route.

## Recovered owner shape library

`D:\DevData\Rainee\verification\owner-art-brushes-2026-10-11\owner-art-brush-library.vectorcraft` contains nine local Owner Art Brush candidates derived from the supplied file. Each renders on a curved centerline. All nine pass centerline-preserving width-profile changes; a frayed candidate passes eraser splitting with brush identity retained. Undo/redo, save/reopen with nine retained definitions, SVG and PDF export are exercised.

At a consistent 16x raster scale, normalized binary silhouette intersection-over-union against the recovered SVG shapes is:

| Candidate | Shape overlap |
| --- | --- |
| 01 | 96.45% |
| 02 | 99.32% |
| 03 | 97.57% |
| 04 | 98.40% |
| 05 | 98.32% |
| 06 | 98.50% |
| 07 | 98.38% |
| 08 | 96.78% |
| 09 | 99.88% |

All nine exceed a 90% geometric-overlap threshold for this recovered cohort. Lower-resolution initial measurements were dominated by thin-stroke raster sampling; the consistent high-resolution rerun replaces them. These numbers measure the new Art mapping against conservative monochrome shape recovery, not independent original-application output. They do not establish original colors, opacity, scale guides, direction, dynamic settings, undiscovered styles or artist approval. Thus they are not a claim that 90% of an unknown complete library is finished.

The three fully scanned Zero Brush File sections correspond to Sketch candidates 05, 07 and 08. Their recovered silhouette overlaps are 99.98%, 99.99% and 99.97%, respectively, so those three records do not add three distinct shape targets to this nine-shape cohort. Candidate SVGs and source/straight PNGs remain local in the research and verification directories. They are not app defaults or redistribution assets.

The shape reader parses numeric path data only and never executes embedded programs. It preserves compound holes with an even-odd silhouette representation, but intentionally does not interpret appearance/color metadata. Its metadata scan is bounded, reports partial coverage, and stores local output separately from distributable assets. A future image-only reference can use raster export and vector tracing through the existing editor tools; vector source recovery avoids that extra approximation when available.

## Verification status

The first implementation test run found old-JSON serialization and a stale default-count assertion; both were corrected. The brush suite now passes 44 tests, including legacy defaults, paper modulation, pressure-derived load, geometry budgets, erased gaps and asymmetric Art-brush width deformation. Four engine tests pass for brush identity, native roundtrip, grain undo and erasing. Two stale test-harness assumptions were corrected: use the engine's `brush.options` rather than the UI alias, and accept the eraser's wrapped IDs. Native synthetic texture tests and the nine-owner-brush workflow pass. Rendered texture cells were changed from rectangular to irregular rounded footprints after visual inspection. Full-window capture was unavailable for the covered/hidden test window; native rendered art was inspected. No physical tablet-quality acceptance is claimed.

Focused brush Clippy with `--no-deps`, native build and brush WASM checks pass. Strict dependency-wide Clippy remains blocked by existing `crates/doc/src/setup.rs` warnings, left untouched. The runnable package is `D:\DevData\Rainee\build\Vector-Art-Brushes-2026-10-11`; executable SHA-256 is `7486041292A3B57301D42F5E368091839C16FC6CED8CBEC5C074A04B3BE6A040`. The supplied Codex bridge script is included; its existing machine-local runtime prerequisites remain. The local control test instance uses port 8003.
