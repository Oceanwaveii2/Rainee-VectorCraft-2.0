# Rainee assessment of VectorCraft as a 2D canvas donor

**Assessment date:** 2026-10-09
**Audience:** Rainee project owner and Gemini review
**Scope:** Read-only source assessment; no product or renderer adoption decision.

## Bottom line

VectorCraft is a substantial existing 2D vector application, so it can shorten Rainee's path to evaluating conventional vector workflows and improving the 2D canvas. It should be treated as a donor/evaluation codebase, not as proof that Rainee's product shell, document model, 3D Stage, RailClone-style generator, or production renderer is already solved.

The inspected renderer crate explicitly describes itself as a `vello_cpu` backend. This is not the GPU Vello path discussed in the Rainee research. VectorCraft's roadmap also lists 3D as absent and marks performance as unverified, with a prior loaded-machine 50k-path measurement missing its listed target. Do not promise GPU brush performance or production suitability without fresh, idle-machine measurements on target hardware.

## What the source already gives us

- A broad conventional 2D vector editing surface, including path/shape tools, selection, layers, appearance, document I/O, and an egui desktop interface.
- A persistent document tree: `crates/doc/src/lib.rs` describes artboards, layers, and objects as a persistent tree whose edits clone only the path to the changed node.
- A dedicated brush crate at `crates/brush`. `BrushKind` includes calligraphic, scatter, art, pattern, and bristle families. Brush definitions are stored in the document's `unknown["brushes"]` data; `stroke_pieces` is shared by renderer output and an expand-to-objects command. This gives a practical extension point, but Gemini should verify how much original input/pressure data remains editable for each brush type before recommending a non-destructive appearance stack.
- A renderer based on `vello_cpu` (`crates/render/Cargo.toml`, with CPU pixmap composition in `crates/render/src/ink.rs`). This provides an immediate CPU rendering baseline; it is not evidence of GPU acceleration.
- A functioning nested Layers panel in `crates/ui-egui/src/panels/layers.rs`, including thumbnails, visibility/lock columns, target/selection controls, and panel options. Rainee can evaluate an enlarged, more legible preview without inventing the whole panel interaction model.
- A command-oriented engine/UI contract documented in `AGENTS.md`: user-visible behavior should go through commands, and the UI should remain a thin view over engine state. Preserve these boundaries when evaluating extensions.

## Where the graphics research fits

| Research | Fit to VectorCraft | Risk / dependency |
|---|---|---|
| Ciallo, SIGGRAPH 2024 | Strong research direction for editable vanilla, stamp, and airbrush-like vector strokes. | GPU-oriented implementation cannot be assumed to plug into the CPU renderer; prototype a separate backend and benchmark first. [ACM](https://dl.acm.org/doi/10.1145/3641519.3657418) |
| GPU-friendly Stroke Expansion, HPG 2024 | Could inform broad/wide stroke geometry once a GPU path exists. | HPG paper, not SIGGRAPH; compute backend, synchronization, geometry parity, and fallback work required. [arXiv](https://arxiv.org/abs/2405.00127) |
| Lightweight Vector Watercolor, I3D 2012 | A reasonable medium-risk research path to a bounded, procedural watercolor appearance. | I3D paper, not SIGGRAPH. The paper is not a drop-in package; design tile/state/cache and editability rules. [ACM](https://dl.acm.org/doi/10.1145/2159616.2159627) |
| NeuBE, SIGGRAPH Asia 2022 | Optional model-driven brush/material exploration after a conventional brush baseline. | Model/runtime/weights, training-data provenance, packaging, hardware, latency, and offline use must be verified. Keep it out of the live stroke path. [ACM](https://dl.acm.org/doi/10.1145/3550454.3555472) |

Practical order: first refine standard paths, tools, layer readability, and deterministic brushes; then test procedural texture/watercolor as isolated experiments; only later benchmark a GPU brush backend. Do not replace an editable path/layer workflow with flattened raster output or expose a graph-first UI. A hidden semantic/appearance graph can remain an implementation option under the familiar layer panel.

## Rainee-specific limits

- Historical first-milestone notes included a plane/sphere Stage placeholder. The later owner decision defers Stage implementation and selects Goo Engine for that future work; see the [2026-10-11 decision](../../../_START-HERE/RAINEE-STAGE-ENGINE-DECISION-2026-10-11.md). The current phase is 2D/stylus work, not that prototype. RailClone-style generator placement and the UI/canvas/scene integration remain future planning; VectorCraft does not complete them.
- Use VectorCraft's conventional 2D behavior to save time and establish a reference surface, while retaining Rainee's product identity, original icons/assets, and architecture decisions.
- Rainee's engine baseline and renderer decision documents govern Rainee. This donor assessment does not promote Vello GPU, remove the CPU fallback, or authorize a renderer rewrite.
- Keep machine-learning work optional and off the input/presentation hot path. Do not require a large model to draw, edit, save, or reopen a document.

## Current worktree caution

At assessment time the VectorCraft working tree already contains uncommitted edits outside this document. The inspected status included changes to the chrome, layer options/panel, preferences, state, menus, a new Codex panel, and an `integrations/` directory. These are pre-existing user work and must not be staged, reset, overwritten, or bundled into a future patch without a separate explicit scope.

Those edits include the enlarged layer-preview work and a scratch-drive preference UI. The actual storage/runtime effect of the selected drive has not been established by this assessment; describe it as a preference/UI change until its consumers and behavior are traced. Likewise, the Codex integration README identifies a preview/read-only panel; do not describe it as a full bidirectional editor control plane without verification.

This note itself is an assessment document only. It does not modify `ROADMAP.md`, source modules, dependencies, or test status. Any later feature task should follow VectorCraft's `AGENTS.md`, including its clean-room, asset licensing, command, panic-safety, and quality-gate rules.

## Local source map

- `ROADMAP.md` — current status, gaps, and feature direction; source claims are self-assessed and should be checked against behavior.
- `AGENTS.md` — donor-specific rules and clean-room constraints.
- `crates/doc/src/lib.rs` — document model modules and persistent-tree contract.
- `crates/brush/src/lib.rs` — brush definitions, document storage key, and stroke-piece interface.
- `crates/render/Cargo.toml` and `crates/render/src/ink.rs` — current CPU Vello renderer.
- `crates/ui-egui/src/panels/layers.rs` — Layers panel implementation and preview rendering.
- Rainee's [graphics research pack](../../../03-CORE-RESEARCH-2026-09/RESEARCH-PACKS-2026-09-25-26/README.md) — local research inventory; original papers remain the primary local sources.
