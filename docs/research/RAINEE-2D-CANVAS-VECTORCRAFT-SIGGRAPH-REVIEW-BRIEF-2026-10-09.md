# Rainee 2D Canvas + VectorCraft + Graphics Research Review

**Prepared:** 2026-10-09
**Purpose:** Working brief and model handoff; this is not a renderer decision or a new owner lock.

## Short answer

VectorCraft looks useful as a working 2D vector-canvas donor and evaluation surface. Its conventional vector tools, document tree, brush definitions, layer panel, and existing UI can save Rainee from rebuilding basic workflows before testing improvements. It is not yet evidence that Rainee should inherit VectorCraft wholesale: the inspected renderer uses `vello_cpu`, VectorCraft reports 3D as absent, and its roadmap still lists brush depth and performance verification as gaps. Treat it as a source to inspect and adapt, with Rainee's product scope and architecture decisions kept separate.

The safest first gains are familiar canvas and layer ergonomics, deterministic editable brushes, and measured procedural appearance enhancements. A GPU-native Ciallo-style brush is promising, but it is a later renderer experiment, not a drop-in feature for the current CPU renderer. Stateful paint simulation and neural brushes should stay optional research until quality, hardware, latency, provenance, and model-distribution costs are demonstrated.

## Product scope and sequencing

Rainee's owner-defined first milestone remains the functional prototype: an Illustrator-familiar shell with original Rainee icons, Procreate-inspired layer ergonomics, and a contained Cinema 4D-style Stage area with a plane and sphere that can be navigated; the plane can be drawn on with basic tools. Decide whether the RailClone-style generator belongs in a dedicated panel as part of that prototype. It is explicitly not the production drawing engine or final 3D feature set. See [`RAINEE-START-HERE-MILESTONE-1-2026-09-15.md`](RAINEE-START-HERE-MILESTONE-1-2026-09-15.md).

The current owner direction adds an immediate evaluation focus: use the available VectorCraft 2D canvas to accelerate and improve the conventional 2D experience. This does not cancel the shell/layer/Stage goals, authorize copying VectorCraft wholesale, or settle Rainee's production renderer. The Gemini review should recommend a safe sequence for using the donor while preserving the distinction between prototype UI work, VectorCraft adaptation, and later engine work.

## Evidence checked

- Rainee's supplied research pack is [`RESEARCH-PACKS-2026-09-25-26/README.md`](../03-CORE-RESEARCH-2026-09/RESEARCH-PACKS-2026-09-25-26/README.md), with its original PDFs canonical and extracted text for discovery. Its stated cutoff is September 25–26, 2026; it is not a claim that all newer work has been searched.
- VectorCraft source inspected includes `crates/brush`, `crates/doc`, `crates/render`, `crates/ui-egui`, its `ROADMAP.md`, and `AGENTS.md`. The focused donor assessment is [`rainee-2d-canvas-donor-assessment.md`](../ArtCraft-Foundation/vectorcraft/docs/rainee-2d-canvas-donor-assessment.md).
- Some useful research in the pack is adjacent graphics literature rather than SIGGRAPH proceedings. In particular, GPU-friendly stroke expansion is HPG 2024 and the lightweight watercolor engine is I3D 2012. Cite the actual venue, not “SIGGRAPH” as a blanket label.

## What is most likely to work

| Candidate | Likelihood / cost | Why it fits | Suggested gate |
|---|---|---|---|
| Improve layer previews, row readability, and canvas navigation | High / low | Direct UI improvements; no new rendering research required. Larger previews help when layer contents are hard to distinguish. | Test at common DPI/scaling and narrow/wide panel sizes; previews recognizable, labels readable, controls operable. |
| Extend standard editable path and brush workflows | High / low–medium | VectorCraft already has conventional drawing tools and brush types. | Source path/brush settings remain editable; undo/redo and save/reopen preserve appearance; repeat rendering is deterministic. |
| Procedural stamp, grain, pencil/chalk, or dry-brush appearance | Medium–high / medium | Existing brush families and generated stroke geometry provide a plausible extension point without requiring ML. | Keep source geometry/brush parameters separate from cached detail; bound cache memory; compare latency and visual quality against a small reference set. |
| Lightweight vector watercolor prototype | Medium / medium–high | A procedural vector approach is more plausible for an early interactive tier than full fluid simulation. | Start with a bounded local/tiled effect; maintain editable path input; measure stroke latency, idle refinement, memory, zoom behavior, and export fidelity. |
| Ciallo-inspired GPU vanilla/stamp/airbrush strokes | High research value / high integration risk | Ciallo directly targets expressive yet editable vector strokes, but current VectorCraft rendering is CPU-backed. | Isolate and benchmark an experiment; do not replace the renderer until backend compatibility, parity, device-loss, and sustained-performance gates pass. |

The first two rows are the best near-term bets. The next two are good experiments after baseline behavior is captured. GPU items are high-value only after rendering architecture and hardware gates are explicit.

## What is likely to go worse

- Treating a GPU paper or shader as a direct patch to the present `vello_cpu` renderer. GPU stroke expansion and GPU brush execution need a real GPU compute/render path, synchronization and cache design, fallbacks, and cross-device measurements.
- Starting with full wet-pigment or bristle physics. These methods introduce state, order dependence, memory pressure, more difficult undo/cache invalidation, and a larger fidelity burden than a deterministic procedural brush.
- Making a neural model part of the latency-critical stroke loop. NeuBE is a useful research lead, not a ready-made small-GPU feature: weights, runtime, training/source assets, licensing, latency, memory, device support, and offline behavior all need separate verification. Keep inference optional and off the input/presentation hot path.
- Flattening brush output to a bitmap too early or replacing the familiar layer stack with a visible node graph. Keep artwork editable; if semantic or procedural nodes are adopted, hide them behind the conventional layer/appearance workflow.
- Inferring that VectorCraft's UI or feature percentage proves Rainee-quality interaction, stylus performance, or a complete Stage. Assess behavior and platforms with measured evidence.

## Research anchors

- **Ciallo, SIGGRAPH 2024** — GPU rendering for vanilla, stamp, and airbrush vector strokes. Strong direct research fit for editable expressive brushes; integration depends on a GPU path. [ACM paper](https://dl.acm.org/doi/10.1145/3641519.3657418)
- **GPU-friendly Stroke Expansion, HPG 2024** — parallel stroke expansion designed for GPU processing. Relevant after choosing and validating a GPU execution path. [arXiv paper](https://arxiv.org/abs/2405.00127)
- **A Lightweight, Procedural, Vector Watercolor Painting Engine, I3D 2012** — interactive vector-based procedural watercolor approach. Useful for a deliberately bounded fast watercolor tier; it is not a SIGGRAPH paper. [ACM paper](https://dl.acm.org/doi/10.1145/2159616.2159627)
- **Neural Brushstroke Engine (NeuBE), SIGGRAPH Asia 2022** — learns controllable brush appearance from example images. Treat as optional, higher-risk material-generation research, not a required brush backend. [ACM paper](https://dl.acm.org/doi/10.1145/3550454.3555472)

The research pack also references physical bristle, classic watercolor, and differentiable/vectorization work. Use its bibliography and original PDFs for further review; do not conflate an implementation concept in the research note with a tested Rainee feature.

## Questions for Gemini

The review-only ticket is [`P3-2026-10-09-001.yaml`](../../AI_STUDIO_CONTROL/TASK_QUEUE/P3-2026-10-09-001.yaml). The requested result should:

1. Verify the exact current VectorCraft code paths and summarize what is implemented versus merely named or planned.
2. Rank three practical 2D canvas improvements and two experiments to defer, with concrete module touchpoints, dependencies, risks, and measurable acceptance checks.
3. Recommend a first thin slice that advances Rainee's 2D canvas without locking a renderer or disrupting its layer-first UI.
4. Check publication details and any available research code, data, runtime, hardware assumptions, and redistribution terms from primary sources. Flag anything that cannot be verified.
5. Do not modify code, start a build, commit, or edit the existing VectorCraft worktree as part of this review.

## Documentation / authority note

This brief is a dated assessment, not an owner lock. Rainee's reconciled architecture, explicit owner decisions, and later scoped updates remain authoritative. This new Markdown is linked from `AI_START.md` and `00-INDEX.md`, but has not been reviewed, hash-pinned, or added to the local retrieval index; it is therefore not yet body-searchable there.
