# Stylus ML deployment audit

Current integration update: the [combined experimental tool build](verification/THREE-TOOL-UPGRADES-2026-10-11.md) adds opt-in AOSP display prediction, the StripMaker-bundled SketchConnectivity junction adapter and NeuBE raster appearance. The dated selection results below describe the earlier research gate, not the latest editor integration. Hardware quality acceptance, portability and redistribution gates remain separate.

October 11, 2026. Scope: select usable research components for VectorCraft's 2D drawing tools while building native Cintiq recording and replay. This is a deployment decision record, not a claim that learned prediction or assisted inking runs in the editor. Existing conventional tools remain the baseline. No animation or 3D Stage work is included.

Primary author pages, repository trees, source files and dependency declarations were checked live. Repository revisions and classifier hashes below identify the inspected artifacts. The pretrained AOSP motion model now executes in a separate Rust replay laboratory; no research drawing dataset was imported and no model was trained. Public availability alone is not permission to redistribute code, weights or drawings.

## Selection result

| Component | Verified artifacts | Decision for this build |
| --- | --- | --- |
| Learned stroke-head prediction | AOSP supplies a pinned TFLite model with inherited license declarations and NOTICE; N-euro permissions and Xiaomi/Samsung releases remain unresolved | AOSP executes in causal replay. Initial adaptation does not beat the existing predictor; keep it experimental and outside the editor. |
| Smart Inker | Author paper, supplement and demonstration; no verified deployable interactive model | Removed from implementation scope at the owner's request. Retained below only as a historical research reference. |
| StripMaker | Two released classifiers execute in an isolated Rust lab; SCIP/OSQP-backed native feature/grouping/fitting now passes two synthetic sketch fixtures and 34 adapter checks | First candidate for selected stroke-bundle consolidation. Gurobi-free research path works; owner-drawing quality and editor preview/acceptance remain pending. Endpoint CLI is built, not behaviorally verified. Redistribution remains a separate release gate. |
| SketchConnectivity | C++ implementation, embedded classifier and Python checkpoint | Next candidate for local junction suggestions. Do not silently reconnect the entire drawing. Solver-off configuration exists, but does not remove the Triangle licensing gate. |

The AOSP network is the first executed pretrained pen-head candidate. The vector-refinement classifiers remain concrete CPU candidates, not learned pen-head predictors or sketch-aware raster ink networks. Their existence does not make the complete applications portable or license-cleared.

## Recording and replay foundation: implemented

The separate [Cintiq recording tool](verification/CINTIQ-NATIVE-INPUT-2026-10-11.md) preserves native timestamps, screen/client pixels, pressure, tilt, phase, eraser and delivered button state. F1–F6 label slow curves, fast strokes, corners, reversals, pressure changes and button exercises. Window-local keyboard events also capture the owner's configured `[` / `]` shortcuts without turning them into fabricated barrel flags.

The first finished session has 17,987 accepted samples and 35 complete strokes. It predates scenario labels and keyboard-event capture; it stays **unlabelled**. A second finished session has 6,868 samples and 22 complete strokes and verifies delivery of both configured bracket shortcuts. No scenario markers were used in that session either. Both derived local fixtures retain every accepted raw and normalized sample and their source SHA-256. Captures and derived human fixtures are ignored by Git. Exported records remain `approved_for_training: false`; nothing is uploaded or automatically treated as training consent.

Replay uses a virtual native-QPC clock and unchanged ordered sample batches. Synthetic slow/fast/corner/reversal/pressure fixtures compare different batch sizes. This is deterministic headless playback for repeatable tests, not an on-screen replay, a trained model or an input-to-photon latency measurement. Physical pixels must still be mapped through the actual editor's DPI/canvas transform before becoming drawing-tool fixtures.

## Learned prediction candidates

### Xiaomi self-distillation model: promising architecture, missing deployment package

The MobileHCI 2025 paper describes a small multitask stylus latency-compensation model. Its reported size and device timing are research results on the authors' mobile hardware, not measurements on this Cintiq or Windows editor. A licensed source implementation, checkpoint and training dataset were not verified in the checked paper page and primary-source searches. The article's license must not be interpreted as a license for unreleased software or weights. [ACM paper](https://dl.acm.org/doi/10.1145/3743742).

Before selecting it, obtain exact preprocessing, sample-rate assumptions, input channels, units, prediction horizon, loss functions, model architecture, checkpoint rights and evaluation splits. If no usable release is available, an independent implementation needs a separately licensed dataset and validation; it is not a quick checkpoint installation.

### AOSP pretrained motion model

The [learned prediction evaluation](verification/STYLUS-ML-PREDICTION-2026-10-11.md) records the pinned weights, license declarations, Rust evaluator and initial Cintiq results. The 179,532-byte model uses five history steps and five predicted steps. The trial runs actual pretrained inference without a large ML runtime, but custom-kernel parity with TensorFlow Lite and Wacom sensor adaptation remain acceptance gates. At 14 ms the guarded adaptation has mean endpoint error of 8.94 and 9.07 physical pixels across the two sessions, compared with 8.60 and 8.74 for the existing predictor. It is not enabled in the editor and did not use either recording for training. This additional mobile open-source candidate is not a SIGGRAPH component. [Pinned AOSP artifacts](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/data/etc/input/).

### N-euro: actual checkpoint, unresolved permissions and legacy environment

Inspected author repository revision: `d25ab5ace85ffb418e97d93ba997cfedbbd33970`. The tree contains a demonstration notebook, `timeseries.py`, small demonstration CSV files and `best_model_0202_small_model_10.pth` (7,367 bytes). The README describes a demonstration and future fuller release; the notebook uses PyTorch, PyTorch Lightning and PyTorch Forecasting with a patched `timeseries.py`. No license file was found in the inspected tree. [Author repository](https://github.com/QijiaShao/N-euro/tree/d25ab5ace85ffb418e97d93ba997cfedbbd33970).

Do not infer redistribution or training-data rights from the public files. Do not load an untrusted PyTorch checkpoint into the main app. A separate evaluation environment, permission confirmation, pinned compatible dependencies, architecture inspection and safe checkpoint conversion would be required before considering ONNX or another production inference runtime. The available demo CSV files are not proof of a diverse Cintiq drawing dataset. This is a different research model from Xiaomi's 2025 model.

### Samsung GRU-CNN: research lead, not a verified dependency

Samsung's HCII 2020 page describes a trajectory predictor using recent input events and additional pen channels. Its published mobile latency is not a Windows performance guarantee. The checked primary page links the paper, not a verified original source/weights/dataset release. A third-party project citing the paper is not the authors' implementation. [Samsung research page](https://research.samsung.com/research-papers/Active-Stylus-Input-Latency-Compensation-on-Touch-Screen-Mobile-Devices), [paper DOI](https://doi.org/10.1007/978-3-030-50726-8_32).

### Prediction acceptance gate

Record labelled sessions at more than one drawing speed and pressure, including stops, sharp corners, reversals and stationary pressure changes. Keep missing channels explicitly unavailable. Define native-time units, sampling irregularity, document transforms and the horizon, initially 12–16 ms. Preserve session/artist boundaries in train/validation/test splits; adjacent windows from the same stroke must not leak into both training and testing.

Evaluate held-out endpoint error, overshoot at corners/reversals, pause behavior, inference cost and displayed latency against the current predictor at the same horizon. Offline future samples can supply evaluation targets; only past samples may enter inference. Predictions affect temporary stroke-head ink only. Saved paths, undo history and committed pressure must remain unchanged. Hardware input-to-photon validation is a separate gate after editor integration.

## Smart Inker: interactive raster assistance

**Scope decision:** Smart Inker is dropped. The following architecture notes are historical; they do not allocate implementation work or authorize a substitute model.

The SIGGRAPH 2018 system uses a rough sketch and user-guidance tools to produce cleaned raster linework. The author page supplies the paper, supplement and demonstration, but an interactive runnable source/weights/dataset package was not verified. The separately published 2016 sketch-simplification code is not this interactive system. [Author project](https://esslab.jp/~ess/research/inking/), [full paper](https://esslab.jp/~ess/publications/SimoSerraSIGGRAPH2018.pdf), [supplement](https://esslab.jp/~ess/publications/supplemental/SimoSerraSIGGRAPH2018_supp.pdf), [author's illustration projects](https://esslab.jp/~ess/themes/illustration/).

The demonstration includes artwork with differing rights. Paper figures and teaser images are not an approved training dataset. Required assets are the interactive architecture and preprocessing, hint-channel definitions, compatible checkpoint, licensing, training sketch/ink/hint pairs, and a measured region-size/latency budget.

VectorCraft integration can be raster-first: capture a bounded rough-sketch region and user hints, paint ordinary ink immediately, run assistance asynchronously, then show a separate preview with accept/reject. Bind a job to document ID, region, source revision and cancellation token. Reject stale results after edits. Accept into an explicitly identified raster layer through an undoable editor command; preserve original strokes and sketch. Any raster-to-vector tracing is a separate opt-in operation. These are coding goals, not implemented features.

## StripMaker: selected stroke-bundle consolidation

Inspected revision: `810a61a03c2177a61f8e32b8ce72c33dccfac49f`. The repository provides C++ code and embedded pretrained random forests. Its own `src`/`python` code is Apache-2.0 and its StrokeStrip source is MIT; those labels do not cover every dependency or artwork. [Pinned repository and README](https://github.com/squidrice21/StripMaker/tree/810a61a03c2177a61f8e32b8ce72c33dccfac49f), [project license](https://github.com/squidrice21/StripMaker/blob/810a61a03c2177a61f8e32b8ce72c33dccfac49f/LICENSE.txt).

Inspected classifier artifacts, hashed from exact retrieved source bytes:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `src/classifier/forest.cpp` | 2,544,007 | `21E989DD0DF4E77BD809CE383B491466D9CCF98C4CEC7191DC4DB7D759597EFC` |
| `src/classifier/forest_sec.cpp` | 1,046,645 | `1A883AE603BB98757B6812E0E7E8B059EB0F6C6B6D4BB8C0A872F68264CC7D6A` |

The C++ forests permit investigating inference without loading a Python pickle, but feature definitions, ordering and thresholds must match exactly. SCAP inputs contain stroke/strip identity; timestamps are ignored by this pipeline. It is not a pen-prediction dataset or model.

The [author data archive](https://www.cs.ubc.ca/labs/imager/tr/2023/stripmaker/StripMakerData.zip) responds successfully and reports 106,920,431 bytes. It was not downloaded or inspected here. Drawing permissions are specified individually in the referenced train/validation and test YAML files; permission to use the archive's contents is therefore still unverified. Record the final archive hash, per-drawing terms, split and provenance before training.

The author's original full build uses C++20, CMake and required Gurobi, alongside Cornucopia, modified StrokeStrip and bundled SketchConnectivity. Its declarations fetch Eigen, CLI11, JSON, pybind11, span-lite, spdlog and TBB. The legacy training environment pins Python 3.8 and scikit-learn 1.0.2; do not assume the current Python installation reproduces it. Our isolated hybrid adapter removes the Gurobi runtime dependency for the tested native path, not the remaining training and release compatibility work. [Pinned CMake](https://github.com/squidrice21/StripMaker/blob/810a61a03c2177a61f8e32b8ce72c33dccfac49f/CMakeLists.txt), [training environment](https://github.com/squidrice21/StripMaker/blob/810a61a03c2177a61f8e32b8ce72c33dccfac49f/python/environment.yml).

Its bundled SketchConnectivity includes Triangle. Triangle's source header limits commercial distribution and requires an arrangement with its author for commercial use. This restriction survives the outer Apache/MIT labels. It is a release gate, not a conclusion that private research experiments are prohibited. [Bundled Triangle terms](https://github.com/squidrice21/StripMaker/blob/810a61a03c2177a61f8e32b8ce72c33dccfac49f/external/SketchConnectivity/3rdparty/Triangle/Triangle/triangle.c).

First integration goal: selected-bundle proposals using a reviewed classifier/feature subset and existing Rust geometry, with a separately validated curve fitter. Measure parity against reference examples before calling this a StripMaker integration. Preserve selected object IDs, widths, corners, hatching and deliberate gaps. Present replacement curves and originals together; accepting creates an undoable change, rejecting changes nothing. A classifier alone is not the complete grouping/fitting pipeline.

Local implementation update: both classifiers now pass 1,158 inference parity probes against the original C++ code, with zero maximum absolute error. The signed Windows Gurobi 13.0.3 SDK is extracted to D and the three author reference CLIs build without modifying author source. The native one-variable solve fails with license error 10009; the separate limited Python solver succeeds. See the [verification report](verification/STRIPMAKER-CLASSIFIER-SDK-2026-10-11.md) for exact scope, scripts and remaining gates. This supersedes the SDK availability gap, not the sketch-quality or editor acceptance requirements.

Gurobi-free follow-up: the independent [SCIP 10.1.0 + OSQP 1.0.0 CPU adapter](verification/STRIPMAKER-SCIP-OSQP-2026-10-11.md) now executes the unchanged author feature definitions, normalization and native classifiers on two original synthetic sketches. It produces one consolidated fit for overlapping strokes and three separate fits for deliberately separated strokes. Original grouped samples remain unchanged. Mathematical/error tests pass (34); limited results are rejected. This supersedes the native license blocker for this alternative, not equivalence to Gurobi on arbitrary drawings or finished UI/MCP integration. No Gurobi license purchase is needed for this path.

## SketchConnectivity: intended-junction suggestions

Inspected revision: `386cab84e940be1da3ac6e8696ef46f8c7d44389`. Its own code is Apache-2.0. The tree provides `src/sketching/forest.cpp` (941,305 bytes; SHA-256 `86E347198435511662C19F32084A61A31A99CBA3DEFFA5C0EAEB848FCD898DD7`) and `python/sketching/resources/classify_junction_models-current.pickle` (1,229,291 bytes). The pickle was not loaded. [Pinned repository](https://github.com/enjmiah/SketchConnectivity/tree/386cab84e940be1da3ac6e8696ef46f8c7d44389).

Its VPaint VEC inputs are dense `xyw`, optionally `xywt`. Preserve stroke identity and width when adapting editable paths. Epoch-style VEC time is not native QPC; conversion needs an explicit clock contract, not an assumed equivalence.

The [author dataset archive](https://www.cs.ubc.ca/labs/imager/tr/2022/SketchConnectivity/SketchConnectivityData.zip) responds successfully and reports 25,364,728 bytes. It was not downloaded or opened. The README points to per-drawing permission records in `data/inputs_train+validate.yml`; dataset rights and contents remain unverified.

C++17/CMake and pybind11 are used with Eigen, fmt/spdlog, zlib, Cairo, clipping/XML/JSON utilities, Graphics Gems and Triangle. `WITH_GUROBI` defaults to ON but the actual CMake file allows OFF and gates StrokeStrip dependencies. This is worth testing for local junction classification; it does not establish equivalent global optimization without Gurobi. Triangle is still included independently, so its distribution restrictions remain. The complete transitive license audit is not finished. [Pinned CMake](https://github.com/enjmiah/SketchConnectivity/blob/386cab84e940be1da3ac6e8696ef46f8c7d44389/CMakeLists.txt), [Triangle terms](https://github.com/enjmiah/SketchConnectivity/blob/386cab84e940be1da3ac6e8696ef46f8c7d44389/3rdparty/Triangle/Triangle/triangle.c).

First integration goal: confidence-ranked junction suggestions within selected strokes, using exact feature definitions and validated classifier outputs. Show endpoints and candidate joins; do not silently close intentional gaps or rewire hatching. Apply individually or to an approved proposal set through normal editor commands. Test rejection, stale proposals, undo/redo and save/reopen. Global topology optimization is a later separate decision.

## Coding order and stop conditions

1. Finish the native measured input and expressive pressure/tilt brushes, shared UI/AI controls, then test the current editor build on the owner's Cintiq. The [expressive brush implementation and acceptance checklist](verification/EXPRESSIVE-CINTIQ-BRUSHES-2026-10-11.md) distinguishes automated routing from physical tablet acceptance.
2. The causal replay interface and initial AOSP/baseline comparison are implemented. Next validate runtime parity and sensor preprocessing, then use newly labelled held-out sessions. Do not ship a learned predictor until applicable permissions, preprocessing, native display behavior and held-out improvement are demonstrated.
3. Evaluate released learned brush appearance after editable pressure/nib behavior. NeuBE is a concrete SIGGRAPH Asia candidate with linked weights, but needs an isolated runtime and an evaluation-only deployment decision; see the expressive brush notes. Smart Inker remains dropped, not a pending implementation dependency.
4. Evaluate StripMaker's selected-bundle feature/classifier subset before implementing an artist-approved consolidation tool. Do not bundle solver/Triangle dependencies without resolving their terms.
5. Evaluate SketchConnectivity's local junction classifier after bundle refinement. Keep every proposal reversible and originals intact.

New assistance actions must register their discoverable AI controls alongside their UI controls. Jobs require cancellation, bounded input/output, useful errors and document-revision binding. ML inference must not block pen capture or manual drawing. CPU evaluation comes first for these small classifiers; no CUDA, OpenCL or GPU companion engine is needed merely to run a random forest. GPU effects remain outside this stylus milestone.

Unknown model rights or missing weights are a deployment blocker, not permission to fabricate model behavior or advertise an ordinary filter as ML. A paper's timings are hypotheses to test here. No full-editor acceptance or ML quality claim follows from the recorder's automated tests.
