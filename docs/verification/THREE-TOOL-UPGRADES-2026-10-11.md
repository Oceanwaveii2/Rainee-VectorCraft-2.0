# VectorCraft experimental drawing tools and telemetry

October 11, 2026. This local Windows build combines optional learned stroke-head prediction, junction suggestions and neural natural brush appearance. Ordinary vector tools and the existing renderer remain available. GPU inference supports the brush assistant; it does not replace the CPU canvas or introduce a 3D Stage.

## Tool behavior

| Tool | Implemented behavior | Important boundary |
| --- | --- | --- |
| Learned prediction | A bounded asynchronous AOSP model worker receives measured timestamped pressure, tilt and screen-position samples. Only temporary displayed ink uses predictions. Pencil and Paintbrush have independent opt-in controls. | The initial offline adaptation did not outperform the baseline. Enabling it is an experiment, not a quality recommendation. Missing channels, discontinuities, stale results and eraser input suppress prediction. |
| Junction suggestions | The StripMaker-bundled SketchConnectivity feature extractors and pretrained forests propose nearby endpoint and T-junction connections. The creator approves individual suggestions. | This is a local candidate adapter, not the complete standalone topology solver. Intentional gaps and hatching must not be automatically connected. |
| Natural brush appearance | The NeuBE style1 model renders a bounded transparent raster appearance over selected ordinary open vector strokes, using an isolated CUDA runtime. Accept creates a group containing the image and hidden original vectors with their stable IDs. | This first adapter supports uniform-width strokes, previews up to 512 pixels and manual regeneration. It is not a live watercolor simulation or a pressure-aware neural painting engine. Release restores the source vectors for editing. |

The Research Tools window separates Prediction, Junctions and Natural brushes into tabs. Source paths remain unchanged during inference. Acceptance is one undoable edit. Content changes or document switches invalidate pending results; cancellation and rejection do not change artwork. Native saves retain the image and original vector group. SVG and PDF export the raster appearance.

## Commands and local telemetry

UI and MCP share `assist.junction.preview`, `assist.brush.preview`, `assist.status`, `assist.telemetry`, `assist.accept`, `assist.cancel`, `assist.reject` and `assist.brush.release`. `window.researchTools` opens the panel. `tool.setOption` controls `learnedPrediction` and `predictionMs`; capability discovery describes parameters and unavailable backends.

`assist.telemetry` retains the last 32 local job lifecycle records. Accept no longer erases backend timing. A new running job does not inherit the previous job's timing; the previous completed record remains separately labelled. Learned tool options expose inference count, skipped submissions, last and maximum model inference time in microseconds, and stale-result count. These are computation diagnostics, not pen-to-photon latency or sustained drawing benchmarks.

The telemetry feed contains job/document identifiers, lifecycle states, messages and backend metadata, not raw pen samples or artwork. Worker diagnostics under `D:\DevData\Rainee\scratch\research-tools` do contain local bounded stroke inputs and generated previews. They remain private local research artifacts; nothing is uploaded or used for training.

## Fixes identified through telemetry and visual inspection

- Backend details disappeared after Accept. Completed timing is now retained with a bounded history and a regression test for attribution to subsequent jobs.
- Unmapped NeuBE output left a residual background. Author UVS mapping improved it, but final screenshot inspection still showed stray off-stroke marks. The adapter now additionally constrains alpha to the authored stroke guidance with 12 preview pixels of dilation and a 2-pixel soft edge. This intentionally limits spill; it is not an author-model change or simulated pigment spreading.
- The original all-in-one panel placed preview acceptance below the fold. Tabs separate workflows, but a full-size image could still push Accept out of view. Approval now precedes the image, whose display height is capped at 240 logical pixels; the underlying generated image keeps its original dimensions.
- Junction acceptance must use the preview's authorized gap tolerance, rather than a fixed maximum unrelated to the request.

## Verification

The first combined native build passed actual junction-worker and NeuBE CUDA workflows: non-mutating preview, approval, undo/redo, stale-state rejection, retained source IDs, native save/reopen, SVG/PDF export, release and cancellation. A live MCP handshake exercised junction approval and learned-option routing. Artifacts are in `D:\DevData\Rainee\verification\three-tools-2026-10-11`.

The restricted Rust model evaluator matched TensorFlow Lite across 48 synthetic feature cases, with maximum absolute output difference `6.67572021484375e-06`. An installed-model test executes the asynchronous worker. These checks establish numerical/integration behavior, not improved drawing quality.

First-build NeuBE telemetry identified an RTX 4080 SUPER. A 464 by 394 preview reported 5.086 seconds for its model/load/mapping/render region and 136,290,816 bytes of peak allocated CUDA tensors. This excludes Python startup and does not measure total device memory or input latency. The test instance had zero live pen packets; frame statistics from it are not Cintiq responsiveness measurements.

Physical Cintiq acceptance and recorded Cintiq replay remain deferred for the combined test phase, as requested. The cleanup quality investigation remains parked; Smart Inker is excluded.

### Final telemetry checked build

The final package is `D:\DevData\Rainee\build\Three-Tool-Upgrades-2026-10-11-telemetry`. Its editor SHA-256 is `3c73343ae5f56639fda4aa31dfa4770d3d82ba57abf903bc967127b3b6b81956`; CLI SHA-256 is `9bba520a0cb2717ccc6b75ff047c91c6cac12134273a18b487ec208e977e40a9`. Cargo.lock SHA-256 is `434895515c790654f6ae71a0285249fee946a04b76aeeb9449be51cdbad26151`. The local test endpoint is 7993, bound to the labelled packaged instance.

Native checks passed again after the appearance constraint and approval-layout fixes. Results and full-window screenshots are in `D:\DevData\Rainee\verification\three-tools-telemetry-2026-10-11`. The screenshot confirms visible approval controls and removal of unrelated distant marks. The 464 by 394 brush fixture reported 2.933 seconds in its model/load/mapping/render timing region and 122,708 nonzero-alpha pixels outside the support removed. These single-run values are diagnostics, not a speedup claim. Near-stroke model texture and imperfections remain subject to artist review.

A live MCP client negotiated protocol 2025-06-18, discovered 32 tools and all eight assistant commands, approved an actual junction edit, generated and accepted a NeuBE appearance, undid/redid it and released the vectors. Completed brush timing remained in the feed after acceptance. Native checks also verified cancellation, stale proposals, native save/reopen and SVG/PDF export. Source validation passed six assistant tests, two learned-model tests including the installed worker, 294 ordinary tool tests (one optional model test skipped in that full run, exercised separately), two appearance-support tests, tools/engine Clippy, WASM engine/UI checks and all 26 dependency-layer checks. Full workspace CI was not run.

The final instance still reported zero live native pen packets at the end of scripted verification. This does not re-test the earlier owner-confirmed eraser fix or establish hardware responsiveness. Current telemetry lets the later combined Cintiq test distinguish received/dropped native packets, model inference work, stale results and assistant failures without treating synthetic fixture results as hardware acceptance.

## Research artifacts and runtime

- [AOSP pinned input model](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/data/etc/input/) provides the pretrained motion candidate. Model SHA-256: `66dd372bbbc7717cf64067c1257117d35d6a2c727506ea39cb1cb51b83d03bd7`.
- [StripMaker pinned source](https://github.com/squidrice21/StripMaker/tree/810a61a03c2177a61f8e32b8ce72c33dccfac49f) supplies the bundled junction features/forests used by the isolated sidecar. [Standalone SketchConnectivity](https://github.com/enjmiah/SketchConnectivity/tree/386cab84e940be1da3ac6e8696ef46f8c7d44389) is a separate implementation, not claimed fully integrated. Local worker SHA-256: `fbe8f994eaa868e9bc59f50c7cfd42734438d04a08c3d325fd5300fb9ec37807`.
- [NeuBE pinned author source](https://github.com/nv-tlabs/brushstroke_engine/tree/093d3c7b047ef160314336c2034348c87f537a4b) supplies the model and CUDA-compatible PyTorch reference kernels. Style1 checkpoint SHA-256: `35f276c79c99fca68a851011ade68721c3bbb8db33fbeabcd514e32dfe61278f`; model archive SHA-256: `6987f50e38300ee4b7c049ebe5865e73f272a6d04d2d4da35354c3cde5732133`. The isolated environment uses Torch 2.6.0+cu124 and torchvision 0.21.0.

The optional research code and weights stay outside the editor package. This is a machine-specific experimental installation, not a portable or redistribution-cleared release. No private recordings were trained on. The original deployment audit retains licensing and dependency gates; local functionality does not erase those gates.

## Reproduction

From the VectorCraft checkout, dot-source `integrations/storage/Use-DDrive.ps1 -Target Editor` before Rust commands. Build `vectorcraft` and `vectorcraft-cli` with the existing dependency lock. Run `integrations/neube-lab/Test-ResearchTools.ps1` only on a fresh disposable instance; it refuses an instance with open documents. Run `Test-McpResearch.ps1` against that labelled test port afterward. `Setup-NeuBE.ps1` prepares the optional pinned research environment; the junction sidecar comes from the isolated StripMaker CMake project.

Source baseline is `4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd` plus the existing dirty checkout and this integration batch. This revision alone does not identify the built executable. Do not overwrite or close owner drawing instances to run these tests.
