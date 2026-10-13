# VectorCraft / Super Rainee - corrected workload

Updated 2026-10-10 after inspecting the Gemini v1.2 source ZIP and running `vectorcraft-gemini.exe` against a disposable document. This replaces the earlier workload, not Rainee's governing architecture. See the [capability audit and evidence](VECTORCRAFT-V1.2-CAPABILITY-AUDIT-2026-10-10.md). The [original checklist](history/VECTORCRAFT-MASTER-WORKFLOW-NO-GPU-2026-10-10-original.md) is preserved as history; its architecture bullets were proposals, not verified implementation.

## Scope

Latest owner sequence, October 11: **UI → GPU-assisted brush/effect execution and pan/zoom → Live Trace**. The owner clarified that the rollback was interrupted and requested the [Rainee 1.0 workspace](verification/RAINEE-SELECTABLE-UI-1.0-2026-10-11.md) restored. Sketch Cleanup and Research Tools remain under Window → Experimental. High-DPI viewport quality spans UI sizing and GPU presentation, not a claim to Apple proprietary technology. CPU geometry/rendering and fallback remain; 3D/Stage and animation stay outside this phase. This sequence supersedes earlier GPU-last wording where that would place Live Trace ahead of GPU work. Prior delivery evidence is the [wet media, profiles, brush coverage and performance report](verification/WET-MEDIA-PROFILES-COVERAGE-PERFORMANCE-2026-10-11.md); the earlier baseline audit/table below is historical capability evidence, not current implementation status.

Immediate input repair and first feature milestone: [Cintiq focus repair and editable sketch cleanup](CINTIQ-INPUT-FIRST-MILESTONE-2026-10-11.md). The patched editor accepted 831 native packets and the owner confirmed the real rear-eraser/tool-return test works. Broader button, focus-interruption and latency checks remain separate. The first feature milestone is selected bundle consolidation followed by intended-junction suggestions, with originals, preview, undo and full command/MCP access retained.

Current research companion: [tool-by-tool experimental technology reevaluation and GPU-last coding goals](../../../_START-HERE/RAINEE-VECTORCRAFT-EXPERIMENTAL-TECHNOLOGY-REEVALUATION-2026-10-11.md) (2026-10-11). It updates research readiness and sequencing, not delivered-feature status: Smart Inker remains dropped and optional GPU integration remains the final phase. Its initial zero-packet diagnosis is superseded by the scoped input verification linked above, not a claim of full stylus acceptance. Earlier research and verification records are preserved.

Use the working VectorCraft editor as the starting point. Perfect the 2D canvas and its interface, then add the Super Rainee capabilities that are actually missing.

- The owner accepts the current conventional drawing tools as an excellent working baseline, comparable to Illustrator for their workflows. The drawing milestone is ML-assisted capabilities beyond that baseline, not a conventional-tool repair or parity pass. Address a conventional bug separately if it is reported or introduced by an enhancement.
- No real animation: no keyframes, animation timelines, playback, or animation engine. History is undo/redo navigation, not animation.
- Static Puppet Warp and other artwork deformation remain drawing/editing tools; they do not require an animation system.
- All Stage/3D implementation is deferred to last. On 2026-10-11 the owner selected Goo Engine as the future staging engine, rather than building a new 3D engine. Its source is downloaded separately, not built or integrated here; see the [official decision](../../../_START-HERE/RAINEE-STAGE-ENGINE-DECISION-2026-10-11.md). UI/canvas integration still requires future planning.
- The owner reports the infinite canvas and independent backdrop/artboard colors completed. Keep them off the new-build list; compatibility and regression checks remain valid.
- Keep the existing CPU vector renderer. The owner clarified selective, optional GPU acceleration for useful pan/zoom presentation, eligible Vello GPU work and raster/SIGGRAPH effects through a suitable companion; no fully GPU canvas conversion. See [GPU effects companion scope](GPU-EFFECTS-COMPANION-SCOPE-2026-10-11.md). The disconnected pilot remains deferred until stylus acceptance; alternative-engine transparency concerns require fixtures, not assumed blanket conclusions.

## What we already have - do not rebuild it

| Area | Actual baseline | Work still justified |
| --- | --- | --- |
| Document storage | Native format v4: JSON metadata with optional binary blobs and gzip; save/open, Save As/Copy, linked and embedded assets, atomic file replacement, background saves, recovery copies and saved preferences. Plain and compressed save/reopen passed in the running app. | Preserve this system and add regression fixtures as new features land. No mandatory SQLite/WAL/CBOR migration. |
| Layers / object hierarchy | Nested layers, groups and objects; visibility, lock, reorder, rename, appearance targeting and rendered previews. Gemini already added HD Preview rows, larger text and DPI-aware thumbnails. | Refine readability and layout; add explicit type badges or convenient blend/opacity controls where useful. Do not create another object tree. |
| Control bar | Fill/stroke controls, stroke weight/profile, opacity, context/tool options, Transform popover and inline X/Y/W/H. It was hidden by default in the test instance; enabling it displayed the controls. | Improve defaults, discoverability and space allocation. Rotation is available in Transform; inline angle is an optional layout change, not a missing transform engine. |
| Blending and history | Transparency/blend settings and an undo/redo History panel already exist. Live rename undo/redo passed. | Bring existing controls closer to the layer workflow if needed. A scrub-style history control would be a new presentation of existing history, not a new storage or animation system. |
| Conventional drawing | Pen, Pencil, Paintbrush, brush definitions, smoothing/fidelity options, Width, Live Paint, Shape Builder, Liquify, Puppet Warp, Gradient Mesh and Blend are implemented and accepted by the owner as the working drawing baseline. | Add optional ML assistance and new capabilities beyond the conventional workflow. Preserve the existing tools; a generic repair/parity pass is not an assigned milestone. |
| Puppet Warp | A paper-derived 2D ARAP solver and editable pins already exist. | Retain Puppet Warp. Develop the separate Smart Deform tool and its original icon when the selected assistance methods are ready; do not rebuild basic ARAP or add animation. |
| Raster-to-vector | Image Trace has presets, fill/stroke modes, editable trace groups, Expand and Release. | Benchmark its line-art quality before adding NeuralTrace or a research replacement. |
| Automation | The live JSON control channel and separate MCP support already expose editing operations. The embedded Codex panel currently offers only read-only document advice. | Connect the panel to approved operations through the existing command system; do not rebuild MCP from zero. |
| Scratch-drive selection | Primary/Secondary dropdowns enumerate drives and retain their preference values. Selecting `D:\` worked in the isolated app. | Complete the requested Preferences & Scratch Disk Management Panel (7.3): directory selection, free-space gauges and safe Purge Cache actions. Disk-backed cache routing needs its own implementation and verification; native document storage already works. |

## Work packages, in practical order

Effort labels describe relative scope, not measured calendar estimates. No reliable completion date follows from this audit alone.

### 0. Existing runnable baseline - complete

Use the existing [vectorcraft-gemini.exe](../vectorcraft-gemini.exe), which reports VectorCraft 1.2.0, as the working baseline. A runnable build is already available, and plain/compressed save-reopen and undo/redo fixtures are already recorded in the [capability audit](VECTORCRAFT-V1.2-CAPABILITY-AUDIT-2026-10-10.md). This is completed baseline work, not another build/bootstrap milestone or a prerequisite for continuing the 2D upgrades.

The checkout's upstream base is [storytold/vectorcraft commit 4cf912f](https://github.com/storytold/vectorcraft/commit/4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd), verified against GitHub. The local Gemini modifications remain separate from that upstream commit. The existing executable's SHA-256 still matches the audited artifact; these records identify the baseline without claiming that GitHub contains the local modifications or proves an exact binary-to-source match.

For future changed builds, retain the source revision, local changes, dependency lock, executable hash and build commands as routine release records. Reuse the existing save pipeline and regression fixtures when features change. No mandatory rebuild, new storage implementation or repeated baseline milestone is needed now.

### 1. Interface and layer readability - small to medium

Start with the existing HD Preview mode. Refine the Layers area so large thumbnails do not squeeze long names into unreadable fragments. Check sensible panel widths, row sizes, text sizing and high-DPI behavior; preserve compact mode for larger documents.

Make the existing Control bar easy to find. Decide which controls belong inline and which remain in popovers. Reuse the layer/object hierarchy for a C4D-inspired presentation, rather than introducing a competing Object Manager model. Add convenient type/blend/opacity access only after checking the existing panels.

Include component 7.3, Preferences & Scratch Disk Management Panel, in this interface milestone. Extend the existing drive selectors into directory selection, live free-space gauges and a safe one-click Purge Cache action. The panel must report which cache consumers actually use the chosen directories; a saved preference alone is not scratch routing.

Acceptance: representative illustration layers are distinguishable at ordinary viewing distance, names remain usable, selection/lock/reorder still work, and large-document scrolling stays responsive. This is an improvement task; the layer panel and HiDPI renderer are already built.

### 2. Machine Learning Assisted Drawing Beyond Illustrator

Detailed owner-requested research: [ML tablet drawing, Apple Pencil, Wacom, NVIDIA/CUDA, Vello and OpenCL](<../../../_START-HERE/RAINEE-TABLET-ML-WACOM-NVIDIA-GPU-RESEARCH-2026-10-11.md>) (2026-10-11). This separates intelligent drawing assistance from basic smoothing, evaluates optional GPU additions with the CPU renderer retained, and records model/data/license gates. It does not mark these features implemented or authorize replacing the working renderer.

The [GPU effects companion research and coding scope](GPU-EFFECTS-COMPANION-SCOPE-2026-10-11.md) compares shared-device WGSL, Skia, Direct2D, CUDA NPP and OpenCL. The current coding milestone is an optional resident-texture executor with correctness fixtures; live canvas routing and research brush effects are separate acceptance stages.

Build on the drawing tools the owner already considers excellent. The purpose of this milestone is to add intelligent assistance and new creative controls beyond conventional vector editing. Existing Pencil, Brush, Width, Liquify, Puppet Warp, Mesh, Blend, Live Paint and Shape Builder remain usable in their current form; the upstream parity backlog is not a prerequisite or the work assignment for this milestone.

Proposed enhancement families:

- **Intelligent vector inking:** infer intended stroke groups and suggest consolidated curves, corners and terminals from loose multi-pass drawing. Distinguish vector-native consolidation algorithms from any learned grouping model; StrokeStrip is a possible geometric foundation, not itself an ML model.
- **Style-conditioned stroke width:** suggest editable taper/weight profiles from an approved style reference and drawing dynamics, with manual width handles retained.
- **Smart Deform:** a separate static 2D tool with a new original icon for assisted pin/joint suggestions, bend constraints and shape-aware deformation. Keep ordinary Puppet Warp available. Do not imply that learned anatomy or overlap handling is already implemented.
- **Feature-aware Liquify:** investigate learned feature guidance that helps preserve important corners, contours and line-weight relationships while deforming artwork. Keep the existing brush modes; do not promise exact topology preservation or zero added anchors.
- **Mesh, shading and Blend assistance:** suggest mesh control points/colors from references and meaningful contour correspondence for static blends. Separate learned methods from geometric optimization and preserve manual editing.
- **Intelligent paint-region assistance:** suggest likely regions or boundary continuations in sketch linework, with visible, reversible suggestions. This adds interpretation beyond exact geometric filling; it is not a requirement to repair accepted Live Paint or Shape Builder behavior first.

Select the first implementation from these families after confirming its paper/code source, usable model/data, license and hardware requirements. Research selection and integration are part of this milestone, as detailed in work package 5. Raster-assisted Clean Sketch is optional and distinct from native vector inking; AutoDrawer remains raster-first.

Acceptance: an implemented enhancement offers a useful new capability or reduces repeated manual work on agreed illustrations. Its output stays editable, previewable and reversible, works through MCP, and survives save/reopen. Compare assistance quality and latency with the accepted baseline; the baseline does not need to fail a bug fixture to justify a new capability. Existing tools must continue to work without the model. Scope/effort depends on the selected enhancement rather than an assumed medium-sized conventional repair pass.

### 3. Full MCP and Embedded Codex Editor Control

Make MCP UI Designer - Vector Work Edition a full AI-operated vector editor, suitable for the owner's Illustrator-replacement workflows. The AI must be able to discover and operate every implemented drawing tool, layer/object action, appearance control and document workflow exposed by the application, including the new ML tools as they are built. Open-source access lets us extend the command surface wherever a feature needs finer control.

Reuse the existing [live control channel](../crates/ui-egui/src/control.rs), [MCP server](../crates/mcp/src/lib.rs), engine commands and undo history. The separate MCP path already supports editing; the current [embedded Codex panel](../crates/ui-egui/src/codex_panel.rs) is read-only. Connect that panel and its existing model bridge to the same editing capabilities rather than building a competing controller or document state. The [integration notes](../integrations/README.md) describe the current boundary.

Required improvements:

- **Complete capability discovery:** expose command/tool names, parameters, current options, enablement and unsupported states. New tools must register their AI controls alongside their UI implementation; do not require a manually maintained subset of drawing tools.
- **Fine-grained vector control:** address stable object IDs and anchors/handles; create and edit paths, styles, widths, transforms, layers, groups, masks and selections. Support tool selection, options and pointer/gesture operations through existing mechanisms when a higher-level command is insufficient.
- **High-level workflows:** translate requests such as creating an editable composition, restyling selected strokes or arranging layers into ordered operations using those same primitives. Keep the targets, intended changes and progress visible.
- **Live context and visual feedback:** inspect the active document, selection, hierarchy, properties and tool state; render/screenshot the canvas and inspect the result after execution. Refresh context after edits, and reject or replan operations based on stale document state. Bind the controller to the intended app instance/document.
- **Controlled multi-step edits:** group logical actions into appropriate undo steps, report partial completion and roll back supported failed operations. Preserve the ability to undo/redo and save/reopen the result. Add cancellation, timeouts, progress and useful error messages for model calls and editor jobs; keep manual editing available.
- **User-controlled permissions:** offer preview/approval and trusted editing modes scoped to the authorized document/actions. Trusted ordinary canvas edits should not require approval for every gesture. Destructive actions, unsaved-work discard, file replacement and destinations outside approved paths retain explicit safeguards. Full tool access does not mean unrestricted filesystem or remote access.
- **Portable integration:** package the bridge/runtime dependencies, show connection/authentication status, and make reconnect/error recovery understandable without requiring the source checkout. Local MCP endpoints remain opt-in for trusted clients.

Acceptance: a live MCP client and the embedded model panel both operate the intended running document. Capability-discovery checks account for every registered tool/action and its parameters or explain why it is unavailable. Representative end-to-end workflows cover path creation/editing, layer organization, appearance/width changes, tool gestures, preview inspection, undo/redo, save/reopen and export; cancellation, permissions and stale-state failures are also exercised. New ML commands participate as they land. One successful edit is a smoke test, not acceptance of full editor control. Advice text or a queued operation does not count as a completed canvas edit.

### 4. 3D Stage Deferred and Not Yet Designed

Stage requires a separate future planning phase. The earlier one-plane/one-sphere description was a placeholder, not an approved architecture or a ready implementation task. Do not build a Stage prototype, assign a 3D workload, or estimate its implementation during the current 2D phase.

Preserve the owner's future concept:

- Separate the application UI from the drawing board so editable canvases can be positioned in 3D space while the controls remain independent.
- Support multiple drawing canvases, each with its own layers, and arrange/stack them at different depths for a Disney-style multiplane composition workflow.
- Use camera/navigation views to compose the spatial arrangement of those canvases. This describes static artwork composition; animation timelines, keyframes and playback remain excluded.
- The host-engine choice is now Goo Engine, by the owner's official 2026-10-11 decision. Source acquisition is complete in a separate reference checkout; no application fork, build or integration is started. The future phase must still select the appropriate version and define the UI/scene/document boundary. Do not budget a new 3D engine from scratch or assume the integration/time savings are already proven.

The owner's Cinema 4D workspace mockups are visual concept references: [spatial/side view](<C:/Users/User/AppData/Local/Temp/codex-clipboard-4314d4ec-431d-4ba9-822e-24290bf61967.png>) and [camera/composition view](<C:/Users/User/AppData/Local/Temp/codex-clipboard-c2b379ec-f9fe-4ad2-9cb4-94f1a0aaca5f.png>). They illustrate drawings placed at different depths, not a functioning Rainee Stage or proof that the editor/engine integration is solved. Do not use them as an implementation blueprint.

Future planning must settle UI/canvas separation, canvas ownership, spatial editing/composition and engine reuse before defining a prototype or acceptance tests. RailClone-style Generator placement belongs to that later planning phase. Leave this area untouched until the owner explicitly resumes Stage work.

### 5. Research Selection and Integration for the ML Drawing Milestone

Research selection supports the core drawing enhancements in work package 2; it is not postponed behind a conventional-tool repair pass. The earlier checklist's stroke consolidation, sketch simplification, style-conditioned widths, mesh fitting, static blending, assisted deformation and feature-aware Liquify describe capability goals. Select actual papers/models for those goals rather than treating every earlier neural-sounding label as a verified software package.

For each candidate, record the paper/code source, license, available data or weights, input/output contract, whether it uses ML or geometric optimization, CPU/GPU feasibility, measured quality and a comparison with the accepted tool. Define its editable output, UI controls and MCP commands together. Keep the source artwork intact and allow bypassing the assistance.

Start with one usable static enhancement that adds a capability or saves meaningful manual work, then extend the selected families. A broken baseline is not required. Auto-Color and Remove BG need algorithms and evaluation fixtures if selected, not just menu buttons. AutoDrawer may produce raster artwork with vector conversion later; raster-first remains valid. None of this adds an animation pipeline.

Smart Deform is a proposed separate static 2D tool with a new original icon, not a replacement for the existing Puppet Warp. Assisted pins, bend constraints and mesh-inversion safeguards need independent evaluation; anatomy or overlap handling is not established by the current solver.

Simo-Serra sketch simplification belongs primarily to AutoDrawer research and, optionally, an explicit VectorCraft Clean Sketch command. It is not a native vector inker; see component 6.1 for the proposed raster-assisted route and its limits.

Acceptance: the selected method demonstrates a useful added capability on agreed illustrations, meets its hardware/latency budget, and integrates into editable, undoable UI and MCP workflows. Unverified model names and unlicensed/unavailable weights are not implementation dependencies. Research candidates are not claimed as shipped features.

## Architecture guardrails

Keep the persistent document tree, appearance/live-object mechanisms, native storage and command history. Hidden semantic or procedural relationships may be added behind the familiar layer system when a feature needs them; that does not imply replacing the document format or rewriting all objects as a graph. `doc/graph.rs` currently means chart tools, not a procedural graph.

Artboards are not proof of a finished multi-stage/per-canvas ownership system. Define that future mapping and version any new fields when the actual feature is built; reuse existing serialization and compatibility mechanisms where possible.

The vector artwork renderer uses `vello_cpu`; the application already uses GPU-backed UI/presentation. “No GPU” in the package name does not mean there is no GPU anywhere in the app. Production GPU rendering and all Stage/NPR/Unreal integration remain outside this immediate workload. The scratch-management panel is in scope; its disk-cache consumers and performance work follow the component gates below. Animation is excluded, not deferred.

## First deliverable

Starting from the existing runnable baseline, finish the layer/control-bar usability pass, including Preferences & Scratch Disk Management (7.3), deliver the first selected ML-assisted drawing capability, and establish an end-to-end embedded Codex editing workflow toward full MCP control. Keep storage and the accepted conventional drawing tools intact. A first model-driven edit is an integration step; work package 3 defines the broader acceptance requirements.

## Master Milestone & Component Breakdown

The component IDs below retain the supplied milestone structure. They describe extensions to the current app, not a requirement to rebuild its working systems. Performance figures in the supplied draft are targets to test, not measured results. Stage/3D and animation are outside this 2D list.

Source: the owner's [supplied component breakdown](<C:/Users/User/.codex/attachments/3726c623-9163-4599-be9a-05399b45fbc2/Pasted text.txt>) and section 7.3 request, 2026-10-10. Baseline status comes from the linked capability audit and the source paths below.

### 1. Core Engine & Document Architecture

#### 1.1. Artboard Metadata Decoupling

Keep canvas spatial mode at document/setup level rather than requiring an `is_infinite` field in every Artboard constructor. The current [Document helpers](../crates/doc/src/lib.rs) use `setup.infinite_canvas`; [setup metadata](../crates/doc/src/setup.rs) and native serialization must remain compatible with existing files. The infinite-canvas feature is owner-reported complete, so this item is a compatibility/regression check rather than another implementation of that feature.

Acceptance: old native documents still open, new settings survive save/reopen, and PDF, Affinity and drawing-tool constructors compile without per-artboard changes.

#### 1.2. Structural Undo/Redo Memory Budget

Preserve the existing persistent `Arc<Node>` tree and cheap document snapshots. Measure retained memory during long sessions before considering fine-grained deltas or another persistent-tree library. Evaluate a configurable memory cap, with 512 MB as an initial candidate, and predictable pruning that preserves the current state and save/dirty-state behavior.

Acceptance: a long-session fixture records memory use and the available undo range; pruning does not break redo, saving or active edits. Do not replace structural sharing that already exists.

#### 1.3. Spatial R-Tree / BVH Scene Acceleration

Profile the existing culling and hit-testing paths, then add or improve an incremental shape-bounds index where it helps. Updates must track edits, groups, clipping and appearance bounds. Query cost depends on the visible result count as well as the tree; do not promise a universal O(log N) query or 95% rejection rate.

Acceptance: compare pan, zoom, hit-test and edit timings on the same large documents before and after indexing, including mostly visible scenes.

### 2. Viewport & Rendering Performance

#### 2.1. Tile-Based Dirty Region Redraws

Evaluate 64 x 64 or 128 x 128 physical-pixel damage tiles against the current canvas cache. An edit invalidates every tile affected by strokes, effects, clipping or masks; view changes and global appearance changes must invalidate or reproject safely.

Acceptance: measured redraw savings without stale pixels, seams or clipped effects. Tile size is a benchmark choice, not a fixed requirement.

#### 2.2. Off-Thread Asynchronous Rasterization

Extend the existing native [render worker](../crates/ui-egui/src/render_worker.rs), which already renders heavy documents away from the UI thread. Evaluate job cancellation, stale-result rejection, latest-view scheduling and buffering rather than rebuilding background rendering. The current worker is native-only; WASM needs a separate compatible path.

Acceptance: measure input latency and frame times on 120/144 Hz hardware under a heavy document. Monitor refresh rate is a target, not a guaranteed result.

#### 2.3. SIMD Path & Curve Flattening

Profile curve evaluation, subdivision and clipping before adding vectorized implementations. Preserve a portable scalar path, runtime capability checks where necessary, and numerical/error-tolerance parity across AVX2, SSE, NEON and WASM targets.

Acceptance: targeted benchmarks improve without changing path bounds, corners, clipping or exported geometry beyond the declared tolerance.

### 3. Startup Optimization & Scratch Disk Subsystem

#### 3.1. Font Discovery Binary Cache

Evaluate a versioned `fonts.bin` discovery/metadata cache inside an app-owned scratch subdirectory. Cache validity must include changes to installed fonts, file metadata and the cache schema. Keep live font discovery and newly installed fonts working; do not copy or bundle font files as part of this cache.

Acceptance: measure cold and warm startup on the same system and verify invalidation. The draft's 80-90% speedup is an unverified target.

Source implementation started on 2026-10-10: versioned compressed discovery metadata, installed-file/root/schema invalidation, lazy font loading and startup scratch routing with fallback. Existing scratch preferences select this consumer's location on the next launch; they do not yet route thumbnails or color transforms. Local discovery measurements over 1,760 faces gave median cache-cold 344.9 ms and cache-warm 137.9 ms across three trials (about 60% faster for discovery, not whole-app startup). Tests and remaining acceptance work are recorded in [Startup and Scratch Verification](verification/STARTUP-SCRATCH-2026-10-10.md). The existing packaged executable has not been replaced.

#### 3.2. Incremental Crash Recovery Journal

Retain the existing document recovery copies. Evaluate an append-only command journal only if recovery granularity needs improvement. Specify checkpoints, flush policy, sequence numbers, compatibility and recovery from a truncated record. A WAL adds I/O and cannot promise zero overhead or zero lost strokes without a tested durability contract.

Acceptance: fault-injection tests document the maximum loss window and restore behavior. Recovery records are protected data, never disposable Purge Cache targets. No native-format migration is required.

#### 3.3. Layer Thumbnail Disk Cache

Evaluate disk caching as an addition to existing rendered/HiDPI previews. Key entries by content/revision, rendering settings and DPI; invalidate them after edits. A 128 x 128 preview is a candidate cache tier, not a limit on the enlarged layer preview.

Acceptance: large layer trees load and scroll faster, previews reflect edits, and purged entries regenerate correctly at the selected display size.

#### 3.4. CMYK / ICC Color LUT Cache

Evaluate caching computed color transforms while preserving current ICC conversion behavior. Include source/destination profile hashes, rendering intent, black-point compensation, precision and cache version in the key. Choose interpolation appropriate to the transform dimensionality rather than assuming one tetrahedral method covers every RGB/CMYK case.

Acceptance: measured acceleration with agreed color-error tolerances and correct invalidation after color-setting changes. Store only disposable computed data here; preserve document-embedded profiles and original profile files.

### 4. Infinite Spatial Canvas & Navigation

Implementation update, October 11, 2026: the existing infinite board and colors are retained. The finite and infinite grids now share an adaptive renderer, and rotated screen bounds include all four corners. Existing renderer culling remains in place. Save/reopen, finite exports and effect-aware culling have regression fixtures. See [implementation and verification](verification/SPATIAL-CANVAS-STYLUS-2026-10-11.md).

#### 4.1. Infinite Board Mode

Owner-reported complete, together with independent backdrop/artboard colors. Retain normal PDF/print artboards as export frames and verify bounds, save/reopen and finite export behavior. Do not rebuild the canvas/color feature.

#### 4.2. Adaptive Dot & Line Grid

Evaluate zoom-dependent major lines and fading minor dots against existing grid behavior. Grid spacing, contrast and physical-pixel rendering should remain readable across zoom levels without aliasing or discontinuous jumps.

Acceptance: visual fixtures cover low/high DPI, extreme zoom and both finite and infinite documents.

#### 4.3. Spatial Viewport Culling & Coordinate Mapping

Keep screen, document and artboard coordinates consistent across pan, zoom, selection, snapping and export. Share bounds/culling work with 1.3 rather than introducing a competing index. This is 2D viewport work, not a 3D Stage.

Acceptance: offscreen objects are culled safely, while effects reaching into the viewport and art crossing export frames remain visible.

### 5. Smart Stylus & Geometric Drawing Engine

Implementation update, October 11, 2026: Pencil and Paintbrush now have independent, saved controls for optional 1 Euro stabilization, display-only prediction, magnetic perspective attraction and Shift angle locks. Defaults preserve ordinary drawing. Existing 1/2/3-point grids remain; 4-point cylindrical and 5-point hemispherical reference grids are selectable in Define Grid and save with the document. The curved modes do not attach or warp artwork. Synthetic input tests cover jitter, filter lag, prediction expiry, corner/reversal suppression, bypass and unchanged committed paths; pen-hardware latency and usability acceptance remain open. See [controls, limits and verification](verification/SPATIAL-CANVAS-STYLUS-2026-10-11.md).

#### 5.1. 1 Euro Speed-Adaptive Jitter Filter

Evaluate an optional speed-adaptive stabilizer alongside existing freehand fidelity/smoothing controls. Tune the trade-off between low-speed jitter removal and fast-stroke lag, with bypass and per-tool settings. Do not promise complete tremor removal or zero lag.

Acceptance: recorded slow/fast stylus fixtures compare line quality, corner retention and latency with the existing input path.

#### 5.2. Forward Predictive Extrapolation

Evaluate a configurable one-frame prediction window, initially 12-16 ms, for the displayed stroke head. Correct predictions as actual samples arrive; do not silently commit extrapolated points as authoritative geometry. Handle pauses, reversals and sharp turns.

Acceptance: latency measurements show a benefit without overshoot artifacts or changed committed paths.

#### 5.3. Multi-Point & Curvilinear Perspective Grids

Reuse existing Perspective Grid tools and presets. Verify current 1-, 2- and 3-point coverage before adding missing behavior. Treat 4-point cylindrical and 5-point fisheye/hemispherical modes as additional 2D grid/projection tools, with horizon/vanishing controls and clear mode selection.

Acceptance: projection, interaction and save/reopen fixtures cover each supported mode. These grids do not require a 3D Stage.

#### 5.4. Magnetic Axis & Ray Snapping

Extend existing snapping with perspective-ray attraction, adjustable strength and optional 15/30/45-degree angle locks. Respect tool modifiers, intentional freehand input and a clear bypass.

Acceptance: recorded gestures demonstrate predictable entry/exit from snapping without jumps or unintended path changes.

### 6. Modern Live Trace Suite

#### 6.1. Centerline Extraction & Optional Sketch Cleanup

Existing [Image Trace commands](../crates/engine/src/cmd/buildcmds.rs) already support stroked centerlines and editable expansion. Improve junction handling, curve fitting and variable-width reconstruction against that baseline; recovered width is an estimate, not the original stylus pressure history. Verify exact Noris/Simo-Serra paper and code attribution before selecting a research implementation.

[Simo-Serra et al., Learning to Simplify (SIGGRAPH 2016)](https://esslab.jp/publications/SimoSerraSIGGRAPH2016.pdf) cleans raster sketches; vectorization is a separate step. It is primarily an AutoDrawer candidate, not a replacement for VectorCraft's vector inker.

An optional VectorCraft Clean Sketch command could use:

`Selected vector strokes or imported sketch -> temporary grayscale bitmap -> ML cleanup -> Image Trace -> previewed new vector paths`

This is a proposed integration. The app already exposes [rasterization](../crates/engine/src/cmd/menucmds.rs) and tracing, but the model adapter and combined workflow are not implemented. Rasterize a temporary copy/offscreen render, not the user's original selection. Preserve originals, coordinate scale and layer placement, show a before/after preview, and apply the accepted result as one undoable operation. Sampling/tracing can change fine details, topology, anchor placement and widths; original vector metadata and stroke identity do not survive automatically. Normal vector editing remains vector-native. Model/runtime licensing, CPU/GPU memory use and quality need evaluation; no real-time guarantee follows from the paper.

#### 6.2. Boundary & Junction Cleanup

Evaluate topology-aware T/Y junction handling, corner preservation, deliberate gap closure and self-intersection cleanup. Verify exact Favreau/Noris references before assigning their methods to this feature. Expose tolerance controls and preserve intentional openings rather than claiming 100% watertight output for arbitrary sketches.

Acceptance: junction, gap, thin-line and corner fixtures compare the new paths and fill regions with current Image Trace and Live Paint.

#### 6.3. Diffusion Curves & Smooth Gradient Vectorization

Evaluate diffusion curves as a separate painterly-vector representation, including editing, layer behavior, rendering and export/fallback support. This is not a direct substitute for ordinary filled paths or the existing gradient mesh. Pin the Orzan paper, algorithm/code license and representation contract before implementation.

Acceptance: smooth-gradient examples demonstrate better reconstruction than current tracing without silently losing editability or export compatibility.

#### 6.4. Interactive Live Trace Preview

Reuse existing Image Trace controls and view modes. Add improvements where needed for threshold, corner fidelity and smoothing changes: cancellable worker jobs, debouncing, stale-result rejection and a clearly provisional source-overlay preview.

Acceptance: interaction stays responsive on agreed image sizes, and accepting a preview creates exactly the inspected result. SIMD and real-time performance require measurements, not a UI label.

### 7. UI, Onboarding & User Experience

#### 7.1. Retro Launch Screen

Double the existing splash duration from 9.5 to 19 seconds, stretching the current ticker-stage timings proportionally. Retain instant skip (Click/Space/Esc) and Replay Launch Screen behavior. This is a timing adjustment to the existing splash, not another splash implementation; ticker labels do not prove that the named subsystems exist.

#### 7.2. Larger Layer Previews & Readable Layer Text

Retain the owner's large-preview/readable-text requirement and existing HD Preview implementation. Finish usable panel width, adjustable row/thumbnail size, long-name handling and HiDPI rendering, while preserving compact mode and responsive scrolling. Cross-reference work package 1; this is not a second layer system.

#### 7.3. Preferences & Scratch Disk Management Panel

User-facing selector for primary and secondary scratch directories with free disk space gauges and one-click **Purge Cache** actions.

- Extend the existing Primary/Secondary drive dropdowns with folder selection and clear resolved paths. Keep defaults/reset available and allow Secondary to be disabled.
- Show free/total space for each underlying volume, app-cache usage and low-space/unavailable warnings. Refresh space readings without blocking the UI; two directories on the same volume do not provide independent capacity.
- Validate writability and use an app-owned cache subdirectory within the selected folder. Define which consumers use Primary, how Secondary fallback works, and whether changing a path applies immediately or after active jobs finish. Never imply routing is active before those consumers are wired.
- Provide one-click Purge Cache for known disposable categories such as generated thumbnails, computed LUTs, font-discovery metadata and temporary cleanup previews. Report actual bytes freed, skipped in-use entries and errors; caches must regenerate on demand.
- Purge only validated, app-owned cache entries. Never delete the selected drive/folder wholesale or traverse links outside the cache boundary. Exclude saved artwork, linked assets, user fonts, original/embedded ICC profiles, preferences, undo history, recovery copies and recovery journals.
- Coordinate purge with active readers/writers and retain required in-flight data. Disk-full, detached-drive and permission failures must preserve document editing and existing recovery behavior.
- Persist approved directory choices through the existing preferences system. Keep document Save/Open/Save As independent of scratch-cache selection.

Acceptance: choices persist; displayed free-space gauges reflect the selected volumes; a disposable cache fixture is written to the configured app-owned directory and can be safely purged; recovery/artwork fixtures remain byte-identical; purged thumbnails/LUTs rebuild; unavailable Primary follows the documented fallback policy. These are future implementation tests, not completed capabilities.
