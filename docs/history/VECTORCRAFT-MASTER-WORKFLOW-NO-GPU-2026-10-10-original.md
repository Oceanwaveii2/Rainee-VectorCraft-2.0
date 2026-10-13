# VectorCraft v1.2 (Gemini Edition) - Master Workflow (GPU excluded)

## Done
- Infinite canvas
- Independent Canvas Desk Backdrop color (Stage Blue #193C5A, White, Cream, Gray, Black)
- Independent Artboard Sheet color + Transparent Disney Cel mode
- Windows GUI subsystem (no black console window)
- Retro splash extended to 9.5s with paced ticker

## Deferred (not in this package)
- GPU: Vello GPU / wgpu compute, DX12 swapchain, WGSL shaders (VectorCraft stays on vello_cpu)
- 3D Multiplane stage (dead last)
- NPR / Unreal bridge

## Todo - UI
- Top contextual bar: X/Y/W/H/angle, fill/stroke swatches, per-tool options
- Auto-Color menu: Smart Flatting, Color-Hint infill, Turn-Sheet match, De-AI separation
- Remove BG button
- C4D-style Object Manager (hierarchy, visibility dots, lock, inline rename)
- Layer stack with vector/raster badges, blend modes, history scrubber

## Todo - 2D tool upgrades (SIGGRAPH / ML)
1. Puppet Warp (distort/puppet.rs, arap.rs): neural 2D auto-rigging; 2.5D contact-aware ARAP; volume-preserving skinning
2. Pencil/Brush (draw2/gesture.rs): StrokeStrip ribbon fitting; sketch simplification; G2 curvature synthesis
3. Live Paint / Shape Builder (builder.rs): gap-tolerant boundary infill
4. Liquify (distort/liquify.rs): low-rank lattice warping; area-preserving twirl/bloat/pucker
5. Width tool (distort/width.rs): style-conditioned calligraphic profiles
6. Gradient Mesh / Blend (meshblend.rs): deep gradient meshes; optimal-transport morphing

## Architecture
- Non-destructive node-graph document
- Atomic SQLite WAL + CBOR persistence
- CPU reference pipeline
