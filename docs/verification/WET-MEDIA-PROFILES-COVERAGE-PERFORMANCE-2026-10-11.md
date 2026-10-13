# Wet media brush coverage and performance

The local Rainee build adds a bounded wet-media simulation over editable artwork, pressure and tilt transfer during approved sketch consolidation, a complete metadata inventory of the supplied Sketch File, and lower-allocation SVG writing. It retains the CPU vector renderer and existing manual tools. Full fluid physics, original brush color/dynamic matching and several large-document budgets remain open.

The owner's next sequence is **UI, then GPU-assisted brush effects and pan/zoom, then Live Trace**. UI layout changes wait for the owner's forthcoming visual references. High-DPI viewport polish belongs to the UI/presentation work: sharper physical-pixel previews, icons and text, appropriate resolution during navigation and monitor-scale changes. This is an Apple-inspired quality target on Windows, not Apple proprietary technology, hardware or PencilKit reproduction. The GPU companion remains selective and optional; no Stage or animation work is included.

## Editable wet media

`paint.wetMedia` appears in the existing effect catalogue, Effect menu and Appearance editing path. Its controls are `spread` in document points, `water`, per-step `drying`, `paper` absorption variation and `steps`. Water zero bypasses it. `effect.list`, `effect.apply` and `effect.setParams` expose the same operation through MCP; no separate manually maintained AI tool list was introduced.

The Rust kernel advances water, mobile premultiplied pigment and deposited pigment fields. Pairwise wet-cell diffusion conserves pigment before output quantization. Water evaporates, paper variation changes deposition, and settled pigment no longer moves. A distance mask limits spread. Applying it to a group mixes the already composited source colors across neighbouring cells; independent objects do not share water. Width/anchor/source edits regenerate the result instead of baking over the original vectors. Undo, native save/reopen and finite exports keep their existing paths.

This is an offline diffusion/deposition snapshot with at most 512 squared cells and 64 steps. Large rasters use a disclosed coarser grid. It does not implement water velocity, shallow-water pressure, a continuous wet-on-wet painting timeline, rewetting, smudging, spectral/subtractive pigment chemistry or independent glaze optics. Overlap hidden by an opaque source stroke cannot recover the underlying pigment from the composited input. The earlier Natural brush `edgePooling` remains procedural edge darkening, distinct from this simulation.

The primary research reference, [Curtis and colleagues' Computer Generated Watercolor](https://grail.cs.washington.edu/projects/watercolor/), describes independently simulated shallow-water glazes and Kubelka-Munk compositing. This smaller implementation is not a reproduction of that complete SIGGRAPH system. The distinction sets the remaining wet-media work, rather than presenting a diffusion kernel as fully physical watercolor.

## Pressure and tilt consolidation

Cleanup now accepts bounded pressure/nib profiles on otherwise eligible open Art, Natural and non-random Calligraphic strokes. Base appearance and sibling constraints still apply. Differing dynamics are allowed, but the policy is explicit: **project the bundle leader's authored knots, do not average all sources**.

Knots map from the leader's flattened arc length to the fitted centerline. A reversed fit reverses knot order and swaps asymmetric left/right widths; nib orientation remains in document coordinates. All knot values and width discontinuities are retained. Non-monotone/ambiguous mappings, invalid profiles and excessive projection work refuse acceptance transactionally. Every original stroke, including its own profiles, stays in the hidden backup group. Stale-document checks and the existing preview/approval/cancellation path remain active.

The preflight and status expose the policy; preview messaging states that leader pressure/tilt will transfer. The cyan preview remains a centerline proposal, not a fully rendered dynamics comparison. Artists should inspect the accepted result and can undo it.

## Complete identified shape inventory

The streaming inventory read the supplied Sketch File compressed private stream to EOF: **539316833 decoded bytes, nine begin-brush markers and nine end-brush markers**. The file hash remained `97893E5496AC75421AE7C083E73BDFDEEE0CF90C43B0CCE0A7CBF027B7E5745A`. The scan also includes markers in the uncompressed header. It keeps bounded chunks, does not execute embedded programs and reports a limit if decoding exceeds 8 GiB or 120 seconds.

All nine identified shapes already exist in the separate local owner library. The three Zero Brush File silhouettes duplicate members of that cohort. Earlier [normalized silhouette comparisons](STYLUS-TEST-TEXTURED-BRUSHES-2026-10-11.md) range from 96.45 to 99.88 percent, with every recovered shape above 90 percent. The new full scan establishes the denominator for **this nine-shape cohort**; it does not turn those geometric scores into proof of original colors, opacity, scale guides, direction, dynamic settings or every style in visual reference images.

The fresh 36-case matrix covers all nine shapes on straight paths, S curves, corners and reversals with controlled pressure/tilt. Width editing, erasing, undo/redo, native persistence and SVG/PDF export pass. These authored fixtures establish editable behavior, not an independent original-application visual oracle. Owner artwork remains outside bundled defaults and packages.

## Large documents

The renderer retains sorted viewport intervals built from its existing effect-aware cull bounds. Queries preserve painter order; uncertain bounds remain traversed, and the retained parent prevents address reuse. The cache has an entry cap and frame-age eviction. Clipping/knockout paths retain their original handling. Pixel-equivalence fixtures cover navigation, transparency, blurred bounds and edits. Dense-document timing does **not** establish a material viewport speedup.

SVG path writing now appends coordinates directly into the output string instead of allocating intermediate point/segment strings. Number formatting, rounding and path syntax are unchanged. The entire 36-case brush-matrix SVG is byte-identical before and after this change: SHA-256 `D591371E0653354EC4EC1467D665A3D514D73BB8DD919E5E640A33CE7A73E63B`.

The latest sequential synthetic runs use four renderer threads, a 2880 by 1800 frame and the same Windows machine. Owner applications remain open, Windows load-average reporting is unavailable and these are development builds, not sustained input-to-photon or release-performance measurements.

| Operation | Prior package 50000 paths | Final package 50000 paths | Budget |
| --- | ---: | ---: | ---: |
| Fit refresh | 133.80 ms | 140.31 ms | 100 ms |
| Pan refresh | 132.53 ms | 145.15 ms | 100 ms |
| 400 percent zoom refresh | 41.09 ms | 42.87 ms | 16 ms |
| Hit test | 2.17 ms | 2.20 ms | 2 ms |
| Save | 146.71 ms | 178.05 ms | 300 ms |
| Open | 711.92 ms | 652.63 ms | 300 ms |
| SVG export | 645.55 ms | 465.27 ms | 500 ms |
| Unite 1000 paths | 108.73 ms | 87.30 ms | 150 ms |
| Handle drag | 6.49 ms | 9.50 ms | 8 ms |
| Segment drag | 8.86 ms | 5.00 ms | 8 ms |

SVG export clears its budget in this run. Six final-package budgets remain over target; none are marked complete. The 5000-path run passes all ten budgets, including 15.64 ms zoom refresh. Safe visibility traversal is delivered, but a renderer-throughput improvement is not claimed. GPU pan/zoom presentation and effect execution follow the approved UI pass; they will be profiled separately from CPU scene regeneration and loading.

## Verification and package

The effects suite passes 85 tests, the engine suite 1540, the renderer suite 170, SVG library tests 15 and the effect-dialog fixture one. The broad engine/renderer suites and replay ran before the final allocation-only SVG patch; that last patch passed its SVG library tests, final-package native/MCP checks and exact brush-matrix SVG comparison. Targeted changed-file formatting, effects Clippy with warnings denied and the UI WASM check also pass. Optional research/performance tests omitted by ordinary suites are not counted as passing. Recorded Cintiq replay passes separately against both preserved sessions: 24855 distinct packets across eight brush/batch configurations, 99420 packet deliveries, button-key handling, identical document layers across batch sizes, native roundtrip and undo/redo. Replay is not a new hardware trial or a physical latency measurement.

Native checks exercise visible wet/bypass differences, grouped distinct red/blue source pigments, width/source preservation, undo/redo, save/reopen, finite SVG/PDF export and an actual installed-worker pressure/tilt bundle. The initial mixing fixture accidentally recolored its selected source stroke; the corrected harness deselects before setting defaults and asserts distinct source colors. The corrected results supersede that first fixture image. Live MCP discovers the effect and executes/undoes it through the running editor, alongside ordinary ink validation and cleanup preflight. Signed-in model inference was not retested; this batch retains the existing embedded controller.

Final local package: `D:\DevData\Rainee\build\VectorCraft-Rainee-Wet-2026-10-11-Verified`. Editor SHA-256: `8C2CD96BEFD5A39B68C839D38612301A222E4F1492C25F9303FACB5845F098B3`. `Launch.ps1` checks manifest hashes and routes caches/temp to D. The manifest records source/executable/lock hashes and machine runtime paths; it is not a standalone portable installer or release certification. Existing owner builds/documents are preserved. No commit is made in the dirty working tree.

Artifacts remain on D:

- `verification\wet-media-inventory-2026-10-11\sketch-full-inventory.json` for the full scan.
- `verification\wet-brush-matrix-2026-10-11` for the 36-case matrix, persistence/export and MCP evidence.
- `verification\wet-media-native-2026-10-11-color-corrected` for corrected native water/pigment and dynamics fixtures.
- `verification\wet-media-final-package-2026-10-11` for final-package native/MCP checks.
- `verification\large-document-2026-10-11-wet` for sequential timing logs, executable hashes and failed-budget exit codes.
- `verification\cintiq-canvas-replay-2026-10-11-wet` for recorded native replay logs and editable outputs.

Those paths are relative to `D:\DevData\Rainee`, not the repository's documentation directory.
