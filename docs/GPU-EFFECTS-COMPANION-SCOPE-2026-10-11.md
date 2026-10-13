# GPU effects companion research and coding scope

October 11, 2026. Owner clarification: retain the current Vello CPU canvas and add optional GPU paths only where useful. This is not a fully GPU-accelerated canvas migration. Vello GPU remains an optional candidate for eligible rendering work; a separate effects executor may process raster paint and research-driven appearance textures without replacing editable document geometry. The disconnected prototype remains deferred while native Cintiq/expressive-brush work is verified.

## Owner scope clarification: selective acceleration

Latest owner sequence: **UI references and interface work first, then GPU brush/effect and pan/zoom support, then Live Trace enhancement**. The owner clarified that the interrupted rollback should not stand. The [selectable Rainee 1.0 workspace](verification/RAINEE-SELECTABLE-UI-1.0-2026-10-11.md) is restored, with research panels under Window → Experimental. No GPU change is included. High-DPI viewport presentation is part of that UI/interaction quality target, not a proprietary Apple implementation. The [wet-media follow-up](verification/WET-MEDIA-PROFILES-COVERAGE-PERFORMANCE-2026-10-11.md) supplies a bounded CPU diffusion/deposition effect and correctness fixtures for later companion evaluation; its limits are not erased by moving computation to a GPU.

- Keep conventional tools, editable vector geometry, the existing canvas and CPU fallback. No full renderer replacement is authorized by this phase.
- Measure pan/zoom presentation and reuse of cached artwork before adding another renderer: the existing native UI/compositing path already uses wgpu. Moving/scaling a resident image differs from regenerating its artwork; acceleration claims must identify which work improves.
- Evaluate optional Vello GPU for supported workloads and a companion engine for raster paint, filters, material/pigment effects and suitable SIGGRAPH implementations. Do not imply Vello GPU is a generic arbitrary-effects engine.
- Compare WGSL/wgpu, Skia/SkSL and Windows-specific Direct2D candidates on actual masks, colored transparency, blend/isolation boundaries, working color space and export behavior. The owner's concern about Vello GPU transparency is a test requirement, not a verified blanket finding.
- CUDA/OpenCL are workload-specific options on the owner's RTX 4080 SUPER, not a requirement for every effect. Include synchronization and transfers in comparisons.
- Device/render/ML work follows stylus acceptance. The existing disconnected GPU pilot is not production integration and is not resumed by this clarification.

## Decision

Lightweight navigation implementation: use the existing egui-wgpu compositor on the active desktop adapter, not a second vector engine. Resident texture quads are translated/scaled/rotated while CPU raster refreshes are throttled and submitted asynchronously. This extends existing GPU presentation rather than claiming that GPU presentation itself is new. View → GPU-assisted Navigation is saved and accessible through `view.gpuNavigation`; `ui.inspect.gpuNavigation` exposes reuse, submissions and upload counters. Pixel Preview, content/appearance changes and non-GPU/headless paths retain their existing rendering behavior. Skia, Direct2D, CUDA and Vello GPU are not needed just to transform a resident texture; no universal fastest-engine claim is made. The [egui-wgpu renderer API](https://docs.rs/egui-wgpu/0.36.2/egui_wgpu/struct.Renderer.html) documents texture updates and GPU presentation on the existing device. Object-level GPU filters and paint kernels remain separate work.

The initial pilot selected an original Rust and WGSL effects module on the existing wgpu version. That is a small effects engine over a GPU API, not another off-the-shelf vector renderer. It is a candidate shared-device route, not a decision to convert the full canvas. Compare the alternatives below where their ready-made effects or research runtimes justify integration. Keep the current CPU effect path authoritative until an optional path passes parity and integration gates.

Gaussian blur is the first transport, alpha and correctness fixture, not the promised Super Rainee enhancement. Displacement, procedural paper and pigment mixing are subsequent companion effects. Machine-learning inference is a separate optional executor; neither blur nor a compute shader is ML.

## Alternatives

| Candidate | What it supplies | Fit beside Vello | Decision |
| --- | --- | --- | --- |
| wgpu compute and WGSL | Portable programmable compute and GPU textures; native DX12, Vulkan and Metal, browser WebGPU | Uses the same Rust GPU resource types as the current presentation path; we must implement kernels, bounds and scheduling | First backend. Lowest additional graphics-stack complexity, not a ready-made filter catalogue |
| Skia GPU and SkSL | A mature 2D graphics pipeline, image filters, programmable shaders, color filters and blenders | Can render effect output offscreen while Vello retains paths. Requires a Skia context, Rust/C++ bindings and verified native texture interop; Graphite/Dawn is not automatically interchangeable with Rust wgpu | Strong alternative if a large ready-made filter catalogue justifies the integration cost |
| Direct2D effects | Windows GPU effect graphs with built-in and custom pixel/compute operations | Appropriate as a Windows-only offscreen companion, but its D3D11 resource path differs from the app's DX12/wgpu path | Optional Windows adapter, not the portable default |
| NVIDIA NPP and CUDA | Optimized image and signal-processing primitives | Useful for selected heavy filters, especially where data already resides in CUDA. Adds FFI, runtime packaging and graphics-memory/fence interoperability | Benchmark a specific workload later; no CUDA installation or universal speedup assumption |
| OpenCL kernels | Programmable compute, including existing research implementations | Can reuse selected research math, but needs a device/runtime query and explicit resource sharing or transfers | Optional research adapter; not a second implementation of every ordinary filter |

[wgpu's upstream documentation](https://github.com/gfx-rs/wgpu) describes its supported backends and WebGPU basis. [Skia's runtime-effects documentation](https://skia.org/docs/user/sksl/) explains how SkSL contributes shaders, color filters and blending within Skia's pipeline, including color-space and premultiplied-alpha requirements. Skia has a [BSD-style license](https://github.com/google/skia/blob/main/LICENSE); its dependencies and bindings still need a distribution audit before adoption.

[Microsoft's Direct2D effects overview](https://learn.microsoft.com/en-us/windows/win32/direct2d/effects-overview) documents D3D11-based GPU image processing and composable effect graphs. It does not establish zero-copy sharing with this app. [NPP documentation](https://docs.nvidia.com/cuda/npp/index.html) describes CUDA primitives; its sample-code license is not the CUDA library's distribution contract. [CUDA graphics interoperability](https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/graphics-interop.html) requires matching devices, external-memory support and synchronization. Existing safe Rust wgpu textures do not provide that integration automatically.

[Mixwell's SIGGRAPH 2026 release](https://dougjam.github.io/mixwell-2026/code) offers GLSL, HLSL and Houdini-oriented OpenCL implementations. Its [MIT license](https://github.com/dougjam/mixwell-2026/blob/main/LICENSE) makes it a concrete research candidate, but the Houdini example is not a standalone VectorCraft plug-in. A WGSL port would need mathematical parity fixtures and attribution. Mixwell is analytical fluid mixing, not a neural watercolor model or a complete physical wet-paint simulation.

## Actual integration boundary

The checkout's artwork renderer currently uses vello_cpu 0.2. The desktop presents through wgpu 30.0.1. Existing effects in `crates/render/src/fx.rs` render bounded content offscreen, apply filters in appearance order, and composite it back. Those semantics are valuable and must survive GPU routing. Adding this companion does not mean GPU Vello is already integrated.

The intended flow is editable paths and appearance parameters → Vello content or silhouette texture → supported companion passes → ordered composition → canvas. CPU fallback operates on the same effect description. The companion must never blur the entire final screen as a substitute for an object-level blur, or flatten isolated groups before their proper blend boundary.

Current app assumptions about stored premultiplied pixels, ICC conversion and proof rendering must be preserved. The pilot operates on explicitly supplied premultiplied RGBA values in the caller's working space; it performs no implicit gamma or ICC conversion. Its CPU oracle tests the same kernel, not equivalence to every current Vello filter implementation.

## Coding goals

### E1 Resident effects backend

- Feature-gated module in the existing render crate, no replacement renderer or native-format change.
- Accept an existing device and queue. Create reusable compute pipelines once, not once per stroke.
- Keep input, intermediate and output textures resident. No production-path GPU readback or blocking device poll.
- Validate dimensions, buffer lengths, sigma, compute limits and same-executor image ownership before dispatch.
- Implement bounded separable Gaussian blur with transparent borders, half-float intermediate/output textures and premultiplied-alpha math. Sigma zero is identity.
- Return validation/allocation errors rather than silently switching algorithms. The integration caller will retain CPU fallback; E1 alone does not install that router.
- Unit fixtures for parameters and CPU math; explicit hardware tests for actual shader execution, odd image sizes, colored transparency, borders, chaining, identity and rejected input.

Acceptance: optional module compiles, hardware shader output agrees with its CPU oracle within 0.002 absolute channel error, and default CPU renderer tests still pass. Hardware tests must fail if no suitable adapter exists when explicitly requested, not quietly pass through CPU emulation.

### E2 Live canvas integration

- Route one eligible existing object effect through the companion at the correct appearance boundary.
- Reuse the active desktop device and register the output texture for presentation; preserve masks, DPI, pan/zoom and ordered composition.
- Add bounded texture pooling, aggregate VRAM accounting, revision/settings/working-space cache keys and cancellation of stale jobs.
- Expand input tiles by the full effect support radius and crop after processing. A cropped source with missing padding must not create tile seams or omit offscreen influence.
- Route unsupported blend/filter graphs and device failures to the existing CPU path with an explicit diagnostic reason. Preserve export until separately verified.
- Make backend controls and diagnostics discoverable through existing UI commands/control/MCP, without a separate document-edit path.

Acceptance: mounted canvas comparisons include nested groups, masks, transparency, partly offscreen sources, zoom/DPI changes, edited parameters, undo/redo, save/reopen and export. Shader parity alone does not satisfy this milestone.

### E3 Effects beyond the Vello backend

Add displacement and procedural paper/granulation through registered editable parameters, then evaluate a licensed Mixwell port. Scope any physical pigment model separately: color mixing, wetness and paper absorption are not obtained just by calling a blur. Keep research results undoable and reproducible from stored parameters, raw brush input and required material assets.

Acceptance: each effect has independent CPU/reference fixtures, transparent-edge handling, tile-support rules, command discovery, persistence and export behavior. No animation timeline or 3D Stage is included.

### E4 Optional NVIDIA and OpenCL adapters

Only introduce CUDA NPP, TensorRT or OpenCL for a demonstrated bottleneck or a research implementation that needs it. Compare transfer-inclusive performance against the shared-device WGSL path. Keep unavailable runtime states explicit and preserve portable execution. Do not use unsafe native interop in this checkout without resolving its unsafe-code policy and reviewing a narrow, safe wrapper.

## Performance and release gates

Record adapter, driver, backend, dimensions, sigma and workload. Separate cold pipeline compilation, upload, GPU execution, synchronization and export readback. Report warm median and tail latency over repeated trials; a single correctness readback is not a renderer speedup benchmark. Test with ML inference active before assigning shared VRAM budgets.

Do not add GPU features to the default desktop build until measured end-to-end benefit, current CPU visual parity, safe fallback and packaging pass. Keep authored data and recovery files out of disposable effect caches.

The local `plan/STATUS.md`, `plan/execution-plan.md` and `plan/architecture.md` referred to by AGENTS.md are absent in this checkout; this source-linked scope supplies the task plan without inventing their contents. Existing owner changes remain untouched. The earlier space constraint is resolved: current editor/probe builds, dependencies and temporary files use the verified D-drive paths configured by `integrations/storage/Use-DDrive.ps1`. Do not follow the obsolete E-drive instruction.

## Relationship to prior research

This scope supplements the [tablet and GPU research](../../../_START-HERE/RAINEE-TABLET-ML-WACOM-NVIDIA-GPU-RESEARCH-2026-10-11.md), not its ML training/model/license gates. Device baseline remains the owner-reported Cintiq Pro 24 DTK-2420, with real recorded pressure and tilt; the actual pen model is not yet verified, and shader code cannot invent barrel rotation. See [native expressive-brush verification](verification/EXPRESSIVE-CINTIQ-BRUSHES-2026-10-11.md) for current machine evidence and remaining acceptance gates.

Implementation and verification results are recorded in [GPU effects pilot verification](verification/GPU-EFFECTS-COMPANION-2026-10-11.md).
