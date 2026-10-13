# Expressive Cintiq brushes and owner-machine verification

VectorCraft now has an optional editable pressure-width and tilt-nib mode for Pencil and Paintbrush. This is deterministic brush geometry, not a learned brush model. Native Windows Ink history reaches the canvas instead of being reduced to one pointer position per frame. The owner's existing Cintiq capture establishes that the hardware supplies pressure, tilt, timestamps and eraser state; live acceptance of this new editor integration remains separate.

The GPU companion stays deferred. The owner confirms the later GPU phase is selective acceleration for useful rendering/presentation and raster/research effects, not a fully GPU canvas conversion; see the [clarified scope](../GPU-EFFECTS-COMPANION-SCOPE-2026-10-11.md). Smart Inker is removed from the implementation scope at the owner's request. Original recordings and older research remain intact; no owner recording is approved for training or uploaded.

## Hardware and actual acceleration

On 2026-10-11, local device inspection found Wacom Pointer driver `4.0.0.0` and Wacom Tablet driver `4.0.0.4`. These are PnP component versions, not a claim about the Wacom installer package version. The owner identifies the display as Cintiq Pro 24 `DTK-2420`; the exact pen model is still unverified.

`nvidia-smi` identifies **NVIDIA GeForce RTX 4080 SUPER**, **16,376 MiB VRAM**, driver **617.14**. The native app's default build enables wgpu, which presents the UI and canvas texture on a GPU. Its document rasterizer is still **Vello CPU**. Neither Vello GPU document rendering nor production CUDA/OpenCL effects are integrated by this work. The disconnected `gpu-effects` prototype is not enabled or resumed.

The [owner-specific launcher](../../integrations/cintiq-editor/Launch-Cintiq-RTX4080.ps1) requests this NVIDIA adapter and high-performance preference through the existing selector. DX12 is preferred by the existing Windows selection policy, with other usable adapters/backends retained as fallbacks. The requested adapter is not proof of the selected adapter: check `ui.inspect.graphicsAdapter` after startup.

“Full Wacom support” is not an acceptance claim. Windows Ink is implemented; a Wintab backend and dedicated ExpressKey/Touch Ring/touch APIs are not added here. Existing keyboard/mouse driver mappings remain in the normal UI route. Tilt azimuth is not barrel rotation, which remains unavailable until the actual pen capability is confirmed. Microsoft's [POINTER_PEN_INFO contract](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-pointer_pen_info) defines the measured channels and flags.

## Existing hardware evidence

Re-reviewed local capture: `integrations/cintiq-input-probe/captures/cintiq-1791713023544-99848.jsonl`, SHA-256 `11ed0520bdfb2e45fd64216269ed35c39bf5917eb749c0cac444a67f22215dd2`.

- 6,868 valid timestamped samples; 22 complete contacts, no incomplete strokes.
- Pressure range 0–0.78125; measured X tilt −17° to 48°, Y tilt −22° to 30°.
- 367 eraser samples, 192-DPI recorded window, seven duplicate history packets removed.
- No invalid or truncated packets in this recording.
- No barrel-button flag. Auxiliary key-downs include virtual keys 219 and 221 (bracket-key mappings). The owner pressed both buttons. The messages are evidence of keyboard input, but do not by themselves prove which physical button generated each mapping.

This is real hardware capture evidence, not proof of live drawing quality, latency or button correctness in the new editor build. Seventeen pen-tip contacts are eligible for the new recorded-input brush replay; five eraser/inverted contacts are excluded from that brush-appearance test.

## Implemented behavior

The native hook reads `GetPointerPenInfoHistory` on the owning event thread, validates/deduplicates raw packets, and retains physical coordinates, QPC timestamps, pressure masks, both tilt masks, eraser/inversion and button metadata. The canvas maps physical client pixels through display scale, zoom, pan and view rotation. Tilt conversion rotates the projected shaft rather than the angular components. Unavailable channels remain explicitly unavailable in `ui.inspect.penInput`; the legacy tool input uses full width when pressure is missing.

Claimed native drawing contacts suppress the promoted frame-level pen/mouse duplicate. Packet batches retain the samples while avoiding an intermediate expensive path-fit preview for every packet. Focus loss, capture loss and queue overflow cancel the in-progress native stroke rather than connecting across a missing interval. Inverted/eraser input temporarily selects the existing eraser tool and restores the previous tool on completion/cancellation.

Pressure generates an editable width profile. Tilt generates an editable elliptical nib profile; the default brush library resolves the reserved `Rainee Expressive Pen` even in documents with custom libraries. These profiles drive the same generated geometry for display and PNG/SVG/PDF export. Native save retains the centerline and profiles. SVG/PDF rendering geometry is not a promise of native editable nib metadata in another application.

Pressure response is `minimum + (1 - minimum) * pressure^gamma`. Upright measured tilt produces a round nib; increasing tilt flattens it toward the configured minimum axis ratio. Missing tilt uses the configured fixed angle and ratio. This is a controllable geometric approximation, not a simulation of physical bristles or an inferred pen roll.

Expressive mode replaces the selected appearance brush for the new stroke. It currently requires a solid centered stroke without arrowheads, and deliberately does not auto-extend an existing path. Unsupported appearance combinations return an error without committing a changed document. Defaults are off; ordinary drawing retains the established path. Hold Ctrl **at the start** to bypass expressive mode for that stroke. Escape cancels a preview; one completed stroke creates one undo action.

## UI and AI controls

Double-click Pencil or Paintbrush, or invoke `tool.options`. The options dialog is scrollable so its existing controls and confirmation buttons remain accessible.

| Option | Meaning | Range/default |
| --- | --- | --- |
| `pressureWidth` | Measured pressure controls editable width | false |
| `pressureGamma` | Pressure response curve | 0.1–4 / 1 |
| `pressureMin` | Minimum width fraction | 0.01–1 / 0.1 |
| `tiltNib` | Measured tilt controls ellipse | false |
| `nibRoundness` | Minimum measured ratio; fixed ratio if tilt absent | 0.01–1 / 0.2 |
| `nibAngle` | Missing-tilt orientation on the page | −180°–180° / 0° |

`tool.setOption` changes the same settings used by the UI, per tool and across preference save/reload. `ui.pointer` and MCP `pointer_gesture` accept normalized `pressure`, timestamp `time`, and optional `tilt: [x,y]` in document-axis degrees. `path.freehand` accepts `penSamples` aligned to input points and an `expressive` options object. It rejects malformed samples and more than 16,384 entries. `stroke.setAdvanced` accepts `nib: [[t,angle,ratio], ...]` or null; the existing width-editing path edits the width profile. A dedicated UI editor for individual nib stations is not included.

```json
{"command":"tool.setOption","params":{"tool":"paintbrush","values":{"pressureWidth":true,"pressureGamma":1,"pressureMin":0.1,"tiltNib":true,"nibRoundness":0.2}}}
```

Registered command descriptions, current options and native channel status are discoverable through the existing control/MCP system. These additions do not establish full editor-wide AI-control acceptance, embedded-model inference or unrestricted filesystem permissions.

## Verification status

Completed checks so far:

- 26 brush tests, including intermediate pressure swell on a straight centerline and shortest ellipse-orientation interpolation.
- 291 tools tests, including existing input/gesture behavior.
- Ten engine tests, including the explicitly enabled owner-recording replay: pressure/nib persistence, native roundtrip, undo/redo, stationary pressure, bypass, cancellation, malformed requests, PNG/SVG/PDF geometry, option persistence and batched/per-packet equivalence.
- Four native canvas-dispatch tests: fast contact without egui hover, exclusive commit, interruption cancellation, bounded queue and view-rotation tilt mapping.
- One MCP expressive-gesture test, covering discovery, measured channels, editable appearance and undo/redo.
- Three freehand-options dialog tests, including existing control rendering, confirmation and per-tool isolation.

The owner-recording replay passed for all 17 eligible pen-tip strokes, retaining width/nib profiles, one undo per stroke and exact native roundtrip. It took 19.21 seconds for the ten-test engine run on this loaded development machine; this is not a real-time brush or latency benchmark.

`cargo build --locked -p vectorcraft` completed, and the resulting native app is running on **NVIDIA GeForce RTX 4080 SUPER (Dx12, DiscreteGpu)**, as reported by its live `ui.inspect`. Registered expressive command descriptions and current options were also checked through the live control endpoint. The development executable is `D:/DevData/Rainee/build/editor/debug/vectorcraft.exe`, SHA-256 `46EB39FB5E6405C30E41973424EDC8DAC69D5226D738094B2868BC64FBCDEA2E`; dependency lock SHA-256 `B0E8DA95C91EC56D8C29A479979DE1B852145311E4DCC70D136E78DEC2D188CC`. Native launcher syntax and the scoped tracked diff whitespace check passed.

The engine/UI `wasm32-unknown-unknown` check passed. Strict selected-package Clippy encountered pre-existing `collapsible_if` findings in `crates/doc/src/setup.rs`, then an existing `double_ended_iterator_last` finding in `crates/ui-egui/src/dialogs/splash.rs`. Those unrelated owner edits are preserved; the strict repository lint gate is not reported clean. The selected library/binary diagnostic rerun passed with `-D warnings -A clippy::collapsible_if -A clippy::double_ended_iterator_last`; these are command-line allowances only, not changes to repository lint policy.

Full repository CI, sustained performance, actual input-to-photon measurements, a newly rebuilt live MCP-client handshake and new live Cintiq brush acceptance are not yet established. The owner test window was prepared with Paintbrush, pressure/tilt enabled, 24 pt stroke weight and assistance/prediction disabled for the initial comparison. At preparation, its live native packet count was zero; the existing recording is not substituted for this new hands-on acceptance.

Reproduction from this checkout:

```powershell
. ./integrations/storage/Use-DDrive.ps1 -Target Editor
cargo test --locked -p vectorcraft-brush -p vectorcraft-tools --lib
cargo test --locked -p vectorcraft-engine --lib tests_expressive_stylus -- --include-ignored
cargo test --locked -p vectorcraft-ui-egui --lib native_pen
cargo test --locked -p vectorcraft-ui-egui --lib dialogs::freehand
cargo test --locked -p vectorcraft-mcp --lib expressive
cargo check --locked -p vectorcraft-engine -p vectorcraft-ui-egui --target wasm32-unknown-unknown
cargo build --locked -p vectorcraft
./integrations/cintiq-editor/Launch-Cintiq-RTX4080.ps1
```

The owner-recording test is ignored by default because it requires that private local capture. `--include-ignored` runs it explicitly; absent captures fail rather than substituting synthetic input. Build artifacts, temporary files and dependencies use D. The source base is `4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd` plus a pre-existing dirty worktree and these uncommitted changes; the base revision alone does not identify this executable.

## Live Cintiq acceptance

Use the new development build, not the older Gemini executable. Enable pressure/tilt for Paintbrush and choose a visible stroke weight. Compare mode enabled/disabled and Ctrl-start bypass on the same shapes:

1. Slow pressure ramps, fast curves, stationary pressure changes, sharp corners and reversals. Check taper, lag and whether intended geometry survives.
2. Upright pen versus tilt in both axes, including a rotated canvas and 192-DPI display. Check nib orientation and missing-channel fallbacks.
3. Inverted eraser, both physical side buttons, ordinary mouse drawing, navigation and focus loss. Confirm one stroke per contact and the expected driver-mapped action.
4. Edit widths, undo/redo, native save/reopen, then PNG/SVG/PDF export. Compare visible contours; keep an original editable document.
5. Inspect `ui.inspect.penInput` for measured channel availability and dropped packets, and `graphicsAdapter` for the selected RTX/backend. Capture latency separately; a high reported FPS is not input-to-photon evidence.

Control access is opt-in for trusted local clients: add `-ControlPort 9441` to the launcher. Use `-NoPreferences` for an isolated test that does not read/write the owner's preferences. The loopback endpoint is unauthenticated; leave it off when not testing AI control.

## Released learned appearance candidate: NeuBE

NVIDIA's **Neural Brushstroke Engine**, SIGGRAPH Asia 2022, has released inference code and a model download. It generates raster brush appearance controlled by stroke geometry/style/color, rather than predicting the pen head or changing an editable vector centerline. This makes it a candidate asynchronous appearance layer, not a replacement for this immediate native ink path. [Author repository](https://github.com/nv-tlabs/brushstroke_engine).

Reviewed repository revision: `093d3c7b047ef160314336c2034348c87f537a4b`. The authors link [model files](https://nvidia-caat.s3.us-east-2.amazonaws.com/neube/models.zip); they have **not** been downloaded, hashed, loaded or benchmarked in this work. The documented runtime is legacy Python/PyTorch/CUDA, tested by the authors on Linux, with a model reported to fit comfortably in 12 GB VRAM. That suggests a plausible fit on the owner's 16 GB card, not proof of Windows compatibility, speed or memory use here. [Pinned README](https://github.com/nv-tlabs/brushstroke_engine/blob/093d3c7b047ef160314336c2034348c87f537a4b/README.md), [pinned dependencies](https://github.com/nv-tlabs/brushstroke_engine/blob/093d3c7b047ef160314336c2034348c87f537a4b/requirements.txt).

The supplied NVIDIA source license permits research/evaluation under its noncommercial terms; it is not an MIT/Apache production component. Preserve notices and evaluate in isolation before deciding distribution. [Pinned license](https://github.com/nv-tlabs/brushstroke_engine/blob/093d3c7b047ef160314336c2034348c87f537a4b/LICENSE).

Next gate: evaluate the pinned model in a D-drive environment, verify Windows/runtime compatibility, measure inference and memory on the RTX, and assess visual quality before adding an asynchronous, cancellable preview/accept workflow keyed to document revision. Owner drawings are not implicitly training data. The earlier AOSP predictor remains outside the editor because it did not outperform the current predictor on the reviewed recordings; see [prediction results](STYLUS-ML-PREDICTION-2026-10-11.md). StripMaker and SketchConnectivity retain their separate deployment gates in the [audit](../STYLUS-ML-DEPLOYMENT-AUDIT-2026-10-11.md).
