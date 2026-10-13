# VectorCraft — Gemini Edition
**Rainee 2D Canvas Donor Engine with Infinite Spatial Grid & Modern Research Suite**

---

### 🌟 About This Edition
This edition of VectorCraft is enhanced with:
1. **Unbounded Infinite Spatial Canvas ($\mathbb{R}^2$):**
   * Decoupled artboards from restrictive document boundary clipping.
   * `Document.setup.infinite_canvas` metadata toggle with New Document preset.
   * Adaptive procedural dot matrix grid in `crates/ui-egui/src/canvas.rs` that smoothly subdivides across zoom octaves with single-pass GPU mesh batching.
   * Figma/Concepts-style subtle artboard boundary frames with persistent name labels.

2. **Hardened Classic Retro Splash Screen:**
   * Embedded fallback artwork via `include_bytes!` in `crates/ui-egui/src/dialogs/splash.rs`.
   * Drag-and-drop custom artwork directly onto the launch screen.
   * Time-guarded skip handling (instant Space / Escape, protected mouse click dismissal).

3. **Restored Workspace Health:**
   * Clean `Artboard` struct definition across all crates (`doc`, `engine`, `pdf`, `affinity`, `tools`).
   * Backwards-compatible document serialization.

4. **Master Research Upgrade Targets:**
   * 1€ speed-adaptive hand-jitter filter (*Casiez et al.*).
   * 12–16ms predictive forward stylus extrapolation.
   * Modern SIGGRAPH Live Trace Suite (skeletal centerline Béziers, junction snapping, and diffusion curves).
   * Scratch disk caching (binary font discovery cache, zero-cost crash recovery WAL, layer thumbnails).
