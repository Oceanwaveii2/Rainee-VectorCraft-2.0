# Recorded Cintiq canvas replay

The existing two owner recordings are now reusable tests of VectorCraft's native canvas input consumer. Routine regression checks use these recordings instead of asking the owner to repeat the physical Cintiq trial. Their original samples remain local and are not approved training data. GPU companion work is outside this batch.

## Results

Both recordings pass with Rainee Pencil and Rainee Dry Ink at packet batch sizes 32 and 512. Each pair produces identical document layers, retains finite artwork, restores Paintbrush after rear-tip erasing, records zero queue drops, and passes native save/reopen plus undo/redo. Eight runs replay 99,420 packets from 24,855 unique recorded samples. The sessions contain 35 and 22 complete strokes; the replay retains 35 and 22 undo steps respectively. This tests selected natural brushes, not every brush or assistance tool.

Separate replay of the recorded bracket keys passes six open-bracket and sixteen close-bracket presses through the tool assignment handler. Hold/release restoration and unchanged document contents are checked, including overlapping held keys. Pen and key replay are separate checks, not a reconstruction of their combined event chronology.

The two local replay tests pass. Regression verification additionally passes ten native-input tests, five button-assignment tests, seven prediction tests, the installed learned-model worker test, eleven cleanup tests and two actual cleanup-worker tests. The full UI WASM check passes. These 38 scoped test results do not replace whole-workspace CI, reference-style evaluation or testing every tool against these recordings. The eight drawing runs and key replay completed in 285.19 seconds of test runtime, not recorded drawing time or a responsiveness benchmark.

The first test compared pressure as generic JSON numbers and failed because the recorder serialized its f32 value with a shorter decimal representation. Comparing typed `PenSample` values resolves that harness error without changing pressure data or editor behavior. No editor defect was demonstrated by the completed replay comparisons.

## Coordinate and timing contract

Original raw packets are re-normalized and compared with their stored normalized values before drawing. One documented uniform affine placement maps recorder client coordinates into an 800 by 600 headless canvas. Pressure, tilt, flags, device identity and QPC timestamps remain unchanged; placed screen/client coordinates are test coordinates. Neither session has scenario labels, so slow curves, fast strokes and reversals are not assigned guessed labels.

The tests consume recorded timestamps without real-time sleeping. They exercise the native queue and mounted egui canvas dispatch, not Windows driver delivery, current focus behavior, screen presentation or input-to-photon latency. Conventional and learned prediction have separate numerical tests; the full-session brush replay uses default prediction settings and does not establish model quality. The owner's earlier physical feedback remains valid for that physical trial.

## Local artifacts

Run output: `D:\DevData\Rainee\verification\cintiq-canvas-replay-2026-10-11-run1`. It contains four editable native documents and the complete replay log. The two pencil documents were reopened through a separate native instance on port 8006, rendered as 1200 by 900 PNGs and visually inspected. Session 1 also completed nonempty SVG and PDF export. These scribble recordings test editing behavior, not reference-style fidelity or the requested 80 to 90 percent brush coverage.

Fixture SHA-256 values remain unchanged:

- Session 1: `E07C1F11D2BED9C82098944D6C367CA52830C53095BACA71E632C47587101AA9`.
- Session 2: `6C07D6E37FCF1EBD421F0C1F3B082E8606331549CF76417779A45518CAF76FE6`.

## Reuse

Run `integrations/cintiq-editor/Test-RecordedCintiq.ps1` from the checkout. It verifies fixture hashes, routes builds and temporary files to D, chooses a fresh output directory and refuses to replace earlier results. It invokes two explicitly local ignored Rust tests. The originals stay outside distributed fixtures, and no data is uploaded. No active owner editor window is driven or closed by the harness.

## Remaining work

Extend this recorded-data suite to controlled brush comparisons, assisted cleanup and junction workflows. Improve brush fidelity next, then editable advanced ink appearance, investigate cleanup quality and consolidate a dependable build. Repeat physical testing only for a specific uncovered input or presentation issue; do not make another identical Cintiq trial a prerequisite for ordinary coding.
