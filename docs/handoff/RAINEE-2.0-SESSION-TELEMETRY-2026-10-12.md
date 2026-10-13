# Rainee 2.0 session telemetry index — October 12, 2026

## Raw thread records

The package includes three unmodified Codex rollout JSONL files under `handoff/session-telemetry/raw/`. All three carry the same Codex thread ID, `01a02ddc-b1d2-74b3-9edb-f9277dd55871`; the October 11 record contains earlier context in that same long-running thread. The raw logs include user/assistant turns, tool calls and outputs, native app-control methods/results, and other same-thread context. They are intentionally not filtered because the owner asked for the entire session. Their sizes are substantial because captured image and tool payloads are embedded; use this index for the Rainee UI test summary.

Expected raw files:

- `rollout-2026-10-11T03-51-39-01a02ddc-b1d2-74b3-9edb-f9277dd55871_01a12a96-f6af-7882-ab07-ea473b456c8c.jsonl` — 1,332,685,501 bytes; SHA-256 `F34AE5526B28551389723C879AEDB22F506020A88FF1ACA1A0DAFE19932D3191`.
- `rollout-2026-10-12T00-28-09-01a02ddc-b1d2-74b3-9edb-f9277dd55871_01a12f03-05ac-7773-be58-75dba9cb9d60.jsonl` — 28,928,703 bytes; SHA-256 `B603516F2FB85371D28E775B4F7798DA78E8D0E18E785858EDD39349EAA10D25`.
- `rollout-2026-10-12T08-23-36-01a02ddc-b1d2-74b3-9edb-f9277dd55871_01a130b6-4d8c-7f01-b3f9-9cc8c985b8d1.jsonl` — 82,546,149 bytes at snapshot time; SHA-256 `72F01F741EFC613F55F76FD240B8801F073DE3E768AB044D4D35B002E1F06E7E`.

The last file was still open for this active Codex thread when the package was assembled. Its copy is a byte-for-byte snapshot through the read point; later final-response events are naturally not in that snapshot. Verify each copy against the SHA-256 values above.

There was no separate native application telemetry export saved by the test harness. The native app-control requests/results are in the raw rollout traces; screenshots and the verification report are included separately. Do not treat generated mockup pixels as telemetry.

## Native evidence and checks

### Artboard / Backdrop restoration

- Build: `VectorCraft-Rainee-Board-Colors-Restored-2026-10-12`, executable SHA-256 `16401DD0B0E2FB0F886A91FA7896641D99021AB6F3823B0CB00FE9110503CB26`.
- Fresh synthetic native test on control port `8033`, process PID `32400` at the captured test point.
- Verified workspace selector order and switching between 1.0 and 2.0 without changing the synthetic artwork.
- Selected-art captures for both workspaces show the Artboard and Backdrop chips in the control bar.
- This was a visibility/restoration check, not a test of color-history undo/save semantics.

### Layers pass

- Native test used a separate synthetic document on control port `8032`, process PID `37908` at its captured test point.
- UI input is queued: assertions were made after the next frames rather than immediately after clicking.
- Opacity was changed to approximately `0.502`; Undo restored the default.
- The selected layer was switched to `Multiply`; Undo restored the full prior document JSON state.
- Captures show the blue selected row, blend badge, inline opacity controls, and the retained 1.0 workspace.
- The test does not cover exhaustive rename/reorder, save/reopen/export, full hardware input, or visual parity.

### Brush Studio and workspace

- Prior native visual pass showed the current 2.0 workspace and Brush Studio. A synthetic drag drew a visible stroke on the disposable pad; active document layer JSON remained unchanged.
- The pad accepts positional input for a bounded number of strokes/samples. This test does not establish calibrated pen pressure/tilt behavior or wet-media fidelity.
- Workspace order/switch-back and the original toolbar configuration were verified. The generated Brush Studio image is a concept, separate from the actual native screenshot.

### Build-level checks

- `cargo build -p vectorcraft` passed.
- `cargo check -p vectorcraft-ui-egui --target wasm32-unknown-unknown` passed.
- Scoped `git diff --check` passed, with a CRLF warning only.
- The Layers pass had 21 UI tests pass, including 1x/2x DPI non-mutating frames and stale-document target cleanup.
- Full CI, release build, exhaustive full-editor acceptance, large-document performance, physical Cintiq validation, and complete Procreate-like theme coverage were not claimed.

## Native screenshot files

In `handoff/native-verification/`:

- `Rainee 1.0-selected.png` and `Rainee 2.0-selected.png`: latest board-color control visibility captures.
- `rainee-2-layers.png`, `inline-controls.png`, `opacity-ui.png`, and `rainee-1-retained.png`: native Layers evidence.
- `rainee-2-visual.png`, `studio-blue.png`, and `studio-pad-drawn.png`: native workspace and Brush Studio evidence.

All native screenshots come from synthetic test documents or UI states. They are evidence of the interactions listed above, not a new feature test for the generated canvas artwork.
