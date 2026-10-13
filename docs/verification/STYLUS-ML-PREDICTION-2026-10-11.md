# Cintiq learned prediction evaluation

The first pretrained pen-head model runs in the Rust replay laboratory. This Windows adaptation did not beat VectorCraft's existing predictor on either owner recording. A later [combined tool build](THREE-TOOL-UPGRADES-2026-10-11.md) adds an opt-in native display-only worker; that integration does not overturn the quality result below. Original recordings and saved drawing geometry are unchanged, and no training was performed.

## Model and provenance

The model is fetched from AOSP frameworks/native revision `4f463a6b1de9198963dc6aff74154a504ba3f8f6`. Its 179,532-byte TFLite file has SHA-256 `66dd372bbbc7717cf64067c1257117d35d6a2c727506ea39cb1cb51b83d03bd7`. The graph contains 123 tensors and 88 operations: concatenation, fully connected layers and GELU activations. It consumes five history steps for distance, relative direction, pressure, tilt and orientation, and returns five steps of distance, direction and pressure. These graph counts were inspected by the local evaluator. [Pinned model and configuration](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/data/etc/input/), [model adapter](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/libs/input/TfLiteMotionPredictor.cpp).

The model directory explicitly inherits `frameworks_native_license`; the root declaration lists Apache-2.0, BSD, MIT, Unicode-DFS and legacy notices. The downloader retains both declarations and the complete NOTICE. This is stronger distribution metadata than the unresolved N-euro checkpoint, but it is not evidence of training-dataset rights or a claim that every bundled file has one universal license. Production redistribution still needs the applicable notices reviewed and included. No training dataset is downloaded. [Directory license declaration](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/data/etc/input/Android.bp), [root license declaration](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/Android.bp), [NOTICE](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/NOTICE).

This is an additional open-source mobile ML candidate, not a SIGGRAPH paper or an implementation of Smart Inker, StripMaker or SketchConnectivity. Their separate deployment gates remain in the [research audit](../STYLUS-ML-DEPLOYMENT-AUDIT-2026-10-11.md).

## Implementation and measurement contract

The portable Rust evaluator accepts only the pinned model size and hash. It uses checked FlatBuffer reads and a restricted FLOAT32 operation set; it does not execute pickle files, custom kernels or a general model importer. `libm` supplies the error function for exact-form GELU. Its kernels still require numerical comparison with TensorFlow Lite before editor integration. [TensorFlow schema](https://github.com/tensorflow/tensorflow/blob/v2.20.0/tensorflow/compiler/mlir/lite/schema/schema.fbs).

Model history is resampled at the configured 4,166,666 ns interval, using only observations that have already arrived. Future samples supply targets only. More than 20 ms between interpolation samples disables that window; unavailable pressure or tilt is not filled with invented measurements. Prediction is reset for every stroke, and eraser/inverted strokes are excluded from this initial pen-tip model trial. The model's predicted pressure is not committed or used to alter recorded pressure. [AOSP configuration](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/data/etc/input/motion_predictor_config.xml), [prediction path](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/libs/input/MotionPredictor.cpp).

Windows tilt angles are adapted to the Android convention using InputReader's sine/cosine conversion. This derived orientation is not barrel rotation. Display rotation and physical sensor equivalence remain hardware checks. [Windows tilt definition](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-pointer_pen_info), [Android tilt conversion](https://android.googlesource.com/platform/frameworks/native/+/4f463a6b1de9198963dc6aff74154a504ba3f8f6/services/inputflinger/reader/mapper/TouchInputMapper.cpp).

All six methods are scored on identical targets at 12, 14 and 16 ms. Abstention falls back to the current measured point and is included in the error, so selective coverage cannot hide hard cases. Methods are hold-last-position, unbounded last velocity, the existing predictor, that predictor with stabilization enabled, raw AOSP output, and guarded AOSP output. Guarding uses the existing predictor's direction eligibility and its 24-logical-pixel displacement cap. The baseline runs in 96-DPI logical units at zoom 1; the model operates in physical screen pixels. No parameter search or fitting was performed on these recordings.

The existing tracker was extracted into a shared module without changing its equations. The laboratory calls that same implementation; it does not substitute a reconstructed baseline. Reports include source hashes, inference timing, coverage, mean/p95/p99/max endpoint error and per-stroke results. They are not pen-to-photon measurements or estimates of commercial tablet compatibility.

## Initial results

Session 1 supplies 28 eligible strokes; seven eraser/inverted strokes are excluded. Session 2 supplies 17 eligible strokes; five are excluded. Both sessions remain unlabelled. At 14 ms there are 15,087 and 5,639 common target positions respectively.

| Method | Session 1 mean error in pixels | Session 1 p95 | Session 2 mean error in pixels | Session 2 p95 |
| --- | ---: | ---: | ---: | ---: |
| Hold measured position | 24.41 | 53.02 | 23.39 | 54.74 |
| Unbounded last velocity | 13.90 | 25.76 | 16.54 | 29.96 |
| Existing predictor | **8.60** | **19.92** | **8.74** | **21.74** |
| Existing predictor with stabilization | 40.66 | 100.22 | 41.99 | 108.84 |
| Raw AOSP adaptation | 9.23 | 25.21 | 9.53 | 27.08 |
| Guarded AOSP adaptation | 8.94 | 24.12 | 9.07 | 25.20 |

Each two-session release trial executed 20,192 model inferences. Core inference averages ranged from approximately 101 microseconds in the initial trial to 133–145 microseconds in the packaged trial; p95 ranged from 112–114 to 189–217 microseconds. Concurrent host load was not controlled. These timings exclude feature extraction, rendering and display delivery. Unbounded velocity produced large outliers, so average error alone is not an adequate safety gate. Stabilization at the current test settings added substantial endpoint lag; it should not be presumed beneficial for every stroke.

The model's mean and p95 errors were worse than the existing predictor in this adaptation. That is a reason not to enable it by default, not proof that learned prediction is ineffective. Runtime parity, coordinate conventions and new labelled held-out scenarios must be settled before drawing a broader conclusion. [Session 1 report](stylus-ml-reports/session-1-v0.3.json), [session 2 report](stylus-ml-reports/session-2-v0.3.json). The earlier same-algorithm trial reports are retained alongside these files.

## Reproduce on D

From the VectorCraft Gemini Edition root:

```powershell
. ./integrations/storage/Use-DDrive.ps1 -Target Probe
./integrations/cintiq-input-probe/Fetch-AospModel.ps1
cargo test --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml
cargo clippy --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml --all-targets -- -D warnings
cargo build --release --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml
& './integrations/cintiq-input-probe/Cintiq-ML-Replay-v0.3.exe' --evaluate-prediction './integrations/cintiq-input-probe/captures/cintiq-1791713023544-99848.jsonl' --model './integrations/cintiq-input-probe/models/aosp/motion_predictor_model.tflite' --output 'D:/DevData/Rainee/prediction-new-report.json'
```

Choose a new absolute output filename; existing reports are never overwritten. The pinned downloader also refuses to replace mismatched existing artifacts. Models are ignored downloads, not committed binary fixtures. The local v0.3 executable SHA-256 is `42223CCD48517DC536967670FBEF3350CEFA9BF814F03A6169EFC1767E7BF893`. A rebuild need not have that byte-identical hash.

There are 33 passing probe checks on this machine, including the installed model and owner-capture evaluation tests; those two optional tests require the local artifacts and are not evidence of inference when artifacts are absent. Five editor stylus regression tests pass after the shared-module extraction. Full editor CI and native ML display integration are not completed by this laboratory build.

## Next coding gates

1. Compare the Rust kernels against TensorFlow Lite on fixed feature tensors and malformed-input tests. Freeze output tolerances before native display integration.
2. Verify Wacom tilt and physical/logical coordinate adaptation. Record new labelled slow/fast/corner/reversal/pressure sessions, reserved for evaluation; do not silently approve them for training.
3. Integrate timestamped native packets into the actual canvas, then test prediction as a cancellable, temporary overlay with a measured input-to-photon budget. Saved paths must remain identical with prediction toggled.
4. Promote a learned model only when held-out quality and tail errors improve over the existing predictor. Training or fine-tuning on owner data requires explicit dataset approval and session-level splits.
5. Smart Inker is dropped at the owner's request. Continue expressive brushes and selected-vector-refinement work with reversible commands and discoverable AI controls. Learned appearance remains separate from deterministic pressure/nib behavior; see [expressive Cintiq brushes](EXPRESSIVE-CINTIQ-BRUSHES-2026-10-11.md).
