# Editable brush replication research

The target is an editable equivalent for every brush in the owner's supplied libraries, not two generic natural-media presets. A brush must retain its centerline, width controls, erasing, undo and saved appearance; a similar static image alone is insufficient. The [full inventory follow-up](verification/WET-MEDIA-PROFILES-COVERAGE-PERFORMANCE-2026-10-11.md) now establishes nine brush sections through EOF, all represented in the separate local owner shape library. Complete geometric-cohort coverage is distinct from matching every original color or dynamic setting.

Implementation update: the owner clarified that these are vector objects running along paths. The [stylus tests and owner Art brush results](verification/STYLUS-TEST-TEXTURED-BRUSHES-2026-10-11.md) measured nine local shapes with reversible editing checks. The later EOF scan supersedes the bounded-prefix uncertainty below; the three Zero Brush File shapes duplicate members of that same nine-shape cohort. The follow-up also adds a bounded wet-media diffusion/deposition effect, not a full fluid/glaze model. Original dynamic settings remain distinct from recovered shape fidelity. Earlier research leads below retain their own deployment status.

## Coverage and inventory

`Sketch File.ai` and `Zero Brush File.ai` stay local and unchanged. PDF-compatible artwork does not necessarily expose private brush definitions or their pressure settings. The earlier Sketch File open reached import options; no complete preset inventory has been recovered. A bounded text inspection finds a declared 11-layer AI document, not a reliable brush list. Layer count must not be confused with preset count. No Adobe application or installed Adobe resources were accessed.

The separate native test instance subsequently completed the Sketch File import: 3,021 objects, 14 imported layers, clean document state. Its SHA-256 remained `97893E5496AC75421AE7C083E73BDFDEEE0CF90C43B0CCE0A7CBF027B7E5745A`. This verifies a local PDF-compatible document import, not recovery of the private brush library; the 11-layer declaration and 14 imported layers are different representations.

The imported document's `brush.list` returned the 17 built-in VectorCraft entries, including Rainee Pencil and Dry Ink, with no current brush. Those are the application's fallback library, not the owner's Illustrator preset inventory. Importing flattened PDF appearance does not solve preset recovery. Next investigate the user-file metadata and rendered reference patches locally, without executing embedded code or importing proprietary procsets.

Create one inventory entry per actual preset: owner label, source hash, family, thumbnail/reference patch location, isolated straight/curved stroke, taper, spacing, overlap, opacity, colorization, pressure/tilt mapping, current reproduction, failed fixtures and approval. Unknown mappings remain unknown rather than guessed. User-provided artwork is a local behavioral reference, not permission to redistribute third-party presets.

## Existing foundation

`crates/brush/src/lib.rs` already contains Calligraphic, Scatter, Art, Pattern, Bristle and Natural variants. Art supports direction, proportional/stretch/between-guide scaling, flips and colorization. Pattern supports side/corner/end tiles and fitting. Reuse these before introducing new engines. Bristle is explicitly a semi-transparent strand approximation, not physical paint. Natural Pencil and Dry Ink are initial procedural fibers, not complete graphite or wet-media models.

| Brush family | Proposed implementation | Missing acceptance |
| --- | --- | --- |
| Solid ink and calligraphy | Existing swept nib plus editable measured width/nib profiles | Reference taper, corners, pressure response and caps |
| Shaped art strokes | Existing artwork warp; preserve ends through guide scaling | Per-reference silhouette, tight turns and variable-width warping |
| Scatter and decorative patterns | Existing stamp/tile placement with deterministic seed | Density, spacing, orientation, corners and colorization |
| Graphite, charcoal and pastel | Paper-coordinate grain plus pressure-dependent deposition and layering | Fine marks, broad tilted shading, accumulation and zoom stability |
| Dry brush and broken ink | Irregular coverage, correlated bristle tracks and ink depletion | Frayed edges without comb-like bands or synchronized gaps |
| Example-matched natural media | NeuBE style fitting; RealBrush-inspired sample synthesis where appropriate | Held-out curved strokes, color/alpha fidelity and local fit performance |
| Wet watercolor and oil interactions | Stateful tiled pigment/fluid or paint model, with ordered replay | Wet overlaps, drying, mixing, erasing and deterministic recomputation |

This table maps technologies to families; it is not an inventory or a claim that the library contains every family.

## Technology evidence

### NeuBE with fitted styles

[NeuBE, SIGGRAPH Asia 2022](https://research.nvidia.com/labs/toronto-ai/brushstroke_engine/) learns geometry-conditioned raster appearance with controllable color and transparency. The [official implementation](https://github.com/nv-tlabs/brushstroke_engine) provides weights, style libraries and `scripts/project_main.py` for fitting a target appearance and its guidance geometry. The documented runtime uses CUDA PyTorch; Windows deployment and responsiveness require local measurements.

The current `integrations/neube-lab/worker.py` selects `engine.random_style(seed)`. That is not reference fitting. Next evaluate the released projected styles, then fit local isolated brush examples with explicit guidance masks. Record the model hash, fitting objective, seed, latent parameters, elapsed time, memory and held-out output. Prefer latent fitting before proposing full retraining. No reference upload or training was performed in this research pass.

The existing support mask dilates guidance by 12 preview pixels. That fixed-pixel corridor can suppress wide bristles or splatter and is not a physical footprint. Make support depend on authored brush width and intended texture extent before judging style fidelity. Saved learned effects need stable latent/model identity and a portable appearance fallback, not a random seed alone.

### Example based synthesis

[RealBrush, SIGGRAPH 2013](https://gfx.cs.princeton.edu/gfx/pubs/Lu_2013_RPW/index.php) synthesizes stroke texture and overlap interactions from captured isolated, overlapping and smudged examples. Its author page links a dataset and paper, but does not establish a ready-to-integrate runtime. It is a useful algorithmic route when actual example textures are available. A flattened stroke does not reveal its original tablet dynamics. Sample rights and implementation effort remain separate from paper availability.

### Practical textured deposition

[libmypaint](https://github.com/mypaint/libmypaint) is an existing ISC-licensed brush engine, not SIGGRAPH research. Its [settings schema](https://github.com/mypaint/libmypaint/blob/master/brushsettings.json) exposes pressure, speed, direction, tilt and grid-coordinate inputs. It is a concrete comparison baseline for dab-based deposition, paper modulation and brush controls. The C implementation and json-c dependency do not meet this project's Rust-only core rule: evaluate externally or implement the relevant behavior in Rust; do not silently link a new C core.

Graphite requires paper interaction and accumulated tone, not merely several gray curves. [Real-Time Pencil Rendering](http://cg.postech.ac.kr/papers/47_Real-Time-Pencil-Rendering.pdf) discusses pencil/paper texture but targets 3D mesh NPR. It supplies texture ideas, not a drop-in interactive tablet brush, and is not a reason to begin 3D Stage work.

### Physical wet media

[Wetbrush, SIGGRAPH Asia 2015](https://wanghmin.github.io/publication/chen-2015-wgb/) simulates bristles, paint and canvas using hybrid particle/density representations and CUDA. The checked author page provides a paper/video, not a verified installable source engine. It belongs to a later paint-effects companion, not replacement of the vector canvas. A stroke edited after interacting with previous paint requires replay of affected later operations; a cached isolated stroke is insufficient.

### Recent research leads

[Birth of a Painting](https://yingjiang96.github.io/DiffPaintWebsite/) describes differentiable Bézier paint reconstruction, geometry-conditioned textures and smudging. The checked project page does not establish deployable code/weights. Treat it as an architectural lead, not an installed tool or proven real-time tablet system.

[StrokeDiff, March 2026 preprint](https://arxiv.org/abs/2603.01103) describes data-efficient oil-stroke generation with Bézier conditioning. Abstract-level evidence does not establish an interactive brush runtime, measured latency, released weights or SIGGRAPH publication. Keep it in the follow-up queue rather than a production dependency.

## Editable appearance contract

Retain the authored path, measured profiles, brush definition, stable seed and appearance parameters. Vector and raster appearance are alternative render products of this source. Width/anchor changes invalidate affected appearance tiles; erasing splits source paths or adds reversible masks. Use document-space paper coordinates so grain does not swim during pan/zoom. Store stroke-local coordinates for brush fibers. Save renderer/model versions and an appearance fallback; export at explicitly selected resolution where pixels are necessary.

For stateful paint, preserve operation order, checkpoints and dependency regions. Undo and path edits replay affected interactions. Keep original vectors editable even when export embeds appearance pixels. Do not claim that every pigment grain is vector geometry.

## Implementation and verification order

1. Recover the local library inventory and isolated reference patches; establish what is actually being matched.
2. Test existing Art/Pattern/Scatter/Calligraphic variants against applicable entries before rewriting them.
3. Add paper-aware deposition and dry-ink coverage to eliminate the current fiber-only visual limitation.
4. Extend the isolated NeuBE evaluation from random seeds to released style libraries and target-conditioned fitting. Promote only styles that pass held-out geometry, color and support tests.
5. Integrate accepted appearance as editable stroke effects with revision invalidation, cancellation, undo and save/reopen.
6. Evaluate wet-media algorithms and GPU acceleration after a correct CPU/reference behavior exists. Keep Vello and conventional editor functionality.

Each preset needs light/heavy pressure, tilted shading where supported, a straight stroke, S curve, acute turn, reversal, overlap and erased segment. Compare silhouette, taper, coverage, grain, color and opacity at matched scale. Also test width/anchor edits, deterministic rerender, low/high DPI, zoom, native roundtrip and SVG/PDF export. Measure cold/warm inference and stroke-head latency independently. Physical Cintiq and artist visual approval are required before marking a preset matched.

No new brush runtime or reference-matched preset was implemented in this research pass. Research remains incomplete until the actual preset inventory and its failed comparisons have been resolved. No single reviewed method establishes exact replication of all supplied styles.
