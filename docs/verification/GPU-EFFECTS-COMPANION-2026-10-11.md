# GPU effects pilot verification

## Delivered lightweight GPU navigation follow-up

The light pan/zoom feature is delivered separately from the still-disconnected compute-filter pilot below. It uses the existing egui-wgpu/DX12 compositor and its resident canvas texture; Vello CPU remains the artwork/export renderer. No second graphics engine, CUDA runtime, Vello GPU canvas replacement or live paint kernel was added.

View → GPU-assisted Navigation (command `view.gpuNavigation {enabled?:bool}`) is enabled by default and stored in UI preferences. It reprojects the existing texture during view-only movement, requests an asynchronous sharp refresh after an 80 ms pause, and submits refreshes at least every 160 ms during continuing movement. These are scheduling thresholds, not guaranteed render-completion deadlines. Newly exposed areas outside the old texture can remain incomplete until refresh; zoomed cached pixels can look temporarily softer. Pixel Preview, edits, document/DPI/size changes and rendering-setting changes bypass reuse. Non-wgpu/software/headless paths keep the existing CPU scheduling. Background completions now check content and proof/overprint/mask settings instead of document identity alone; rotation and resized-raster reprojection are corrected. No new additional texture pool or production GPU readback is required.

The host reported **NVIDIA GeForce RTX 4080 SUPER (Dx12, DiscreteGpu)**; Windows driver string was `32.0.16.1714`. A synthetic 1000-path document, 1440×900 logical window and 2160×1350 capture was tested with two alternating 40-step native RPC runs per mode:

| Run | Reuse | Median UI frame ms | P95 UI frame ms | Raster submissions/uploads | RPC loop wall ms |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | Off | 8.72 | 10.28 | 67 / 67 | 2752.66 |
| 2 | On | 2.91 | 3.18 | 17 / 17 | 3531.68 |
| 3 | Off | 10.33 | 12.16 | 40 / 40 | 3104.55 |
| 4 | On | 3.44 | 5.99 | 19 / 19 | 3892.24 |

This demonstrates lower CPU UI-frame work and fewer uploads in this fixture, not universally fastest rendering, sustained 4K performance, GPU timestamp speed or input-to-photon latency. RPC completion is slower with reuse in these runs; it is not presented as an end-to-end speedup. First-run cache/context warm-up differs, so the counts are not normalized per input sample. The settled canvas interior in runs 3/4 is pixel-identical (physical region 506,177–1659,1276); full-window screenshots were visually inspected without alteration. The first harness compared selection/UI state as well as artwork and failed that overbroad assertion; the final harness compares layer artwork separately and passes.

Four navigation-policy tests, 45 canvas tests, two tool eraser tests and six engine eraser tests pass, along with targeted formatting, WASM checking and Windows editor/CLI builds. The engine and tool Eraser setting `eraseAll` now defaults to true in newly initialized tool settings; explicit saved overrides and the direct `path.eraseRegion` API default are unchanged. Native checks confirm default-on eraseAll, unchanged artwork through navigation, undo/redo, save/reopen and finite SVG export. These do not retest physical Cintiq latency or claim complete GPU brush/filter acceleration.

Current package: `D:\DevData\Rainee\build\VectorCraft-Rainee-GPU-Navigation-2026-10-11-Verified`. Editor SHA-256: `4C7F24EAB0F0C4C7A3F96567363BA0DA3BF74C8E0E63101A6CCBFCE52AA7DD0F`. Use its `Launch.ps1`; ordinary launch does not enable a control listener. The isolated test used port 8022 and no owner preferences. Evidence is `D:\DevData\Rainee\verification\gpu-navigation-final-2026-10-11` (`results.json`, `regression.json`, native roundtrip/SVG and four full-window captures). Reproduction: `integrations/Test-GpuNavigation.ps1` against a fresh app and fresh output folder. The Rainee 1.0 selectable UI and Window → Experimental placement are retained. Brush-options-dialog work is unchanged.

Engine choice is based on the [existing egui-wgpu renderer and texture API](https://docs.rs/egui-wgpu/0.36.2/egui_wgpu/struct.Renderer.html) and verified local behavior. A NVIDIA catalog check did not establish a necessary app-specific graphics skill; no skill or new GPU runtime was installed. The compute-filter pilot's separate pending status below still applies.

October 11, 2026. Scope: [GPU effects companion](../GPU-EFFECTS-COMPANION-SCOPE-2026-10-11.md), milestone E1. The module is optional and does not route the live canvas or change current CPU rendering and export.

## Implementation

`vectorcraft-render` feature `gpu-effects` enables `gpu_fx::GpuEffects`. The caller provides its existing wgpu 30.0.1 device and queue. The executor uploads validated premultiplied RGBA8 sources and returns resident Gaussian-blurred RGBA16Float textures. It accepts its own previous output for chained passes. Bounds, sigma and same-executor ownership are checked. Scoped validation, allocation and internal errors return as errors.

The original WGSL kernel uses transparent borders and a normalized separable Gaussian truncated at ceil(3*sigma). Sigma ranges from 0 to 10 source pixels; zero returns the input. Individual images are capped at 4,194,304 pixels. Production pooling and aggregate memory budgets remain E2 work.

## Reproduction

Use the verified D-drive build/dependency routing. The earlier E-drive instruction is superseded.

```powershell
. ./integrations/storage/Use-DDrive.ps1 -Target Editor
cargo test -p vectorcraft-render --features gpu-effects gpu_fx
cargo test -p vectorcraft-render --features gpu-effects hardware_gaussian_parity -- --ignored --nocapture
cargo test -p vectorcraft-render
```

Hardware tests require a discrete GPU and do not pass through a CPU emulation fallback. They exercise tiny/odd image sizes, zero and maximum sigma, colored transparency, borders, chained half-float textures and invalid/foreign-executor inputs. Test-only readback uses finite waits.

## Results

Verification is pending. Do not claim a hardware speedup, live effect routing, full release acceptance or equivalence to every current Vello blur implementation from this implementation alone.
