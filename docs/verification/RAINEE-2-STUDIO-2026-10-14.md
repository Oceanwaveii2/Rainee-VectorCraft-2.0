# Rainee 2.0 Studio / desktop UI repair — October 14, 2026

## Scope and preservation rules

Continue from `builds/VectorCraft-2.0-Latest.exe` (b008 Optics). Preserve the full desktop toolbar, original stock icon assets, selectable 1.0/2.0 order, expandable vector hierarchy, and independent Artboard/transparent-sheet and Backdrop controls, including with artwork selected. UI darkness never changes document colors. Existing owner changes and open owner documents are not discarded.

## Repairs implemented

- Brush Studio uses bounded three-column child regions and a fixed-height header. A long settings label cannot grow the containing window. Section navigation has blue active rows, stock icons and readable titles.
- Studio's native whole-window drag is disabled. Only its explicit header grip moves it. Pen/touch jitter in a subsection or settings area cannot drag the window. Settings/navigation use wheel and scrollbar scrolling without touch-drag interception. Other research/cleanup/Codex/About windows use title-bar-only dragging; anchored modal windows retain their explicit header handling.
- Nine definition-backed sections: Shape, Stroke path, Grain, Rendering, Dynamics, Color dynamics, Properties, Pen input and About this brush. Only parameters supported by the selected engine are active. Blank sections explain applicability rather than presenting nonfunctional controls.
- Preview Undo and Clear act on disposable strokes, not the document. Done saves through `brush.options`; stale/document guards remain. Renaming updates existing brush references and is undoable. Cancel and Reset draft preserve their distinct meanings.
- Experimental fluid controls are visibly disabled until connected to rendering. Their previous source is retained, not represented as working brush physics.
- Circular inner saturation/value field with a visible marker and reachable color extremes; five harmony modes; RGB/HSB/CMYK/Lab Value controls. Hex editing uses the shared buffered commit field, preserving incomplete typed values.
- Truncated names with tooltips in layer/brush rows; checkbox-style visibility while keeping hierarchy and command routing; saved Recent order; lighter card spacing; desktop Brush Library heading.
- Rainee 2.0 preferences use labelled blue pill switches. 1.0 keeps legacy checkboxes. Dark glass tint stays charcoal over bright/white artwork; scattering/lensing cache and Reduce Transparency remain.

## Evidence / status

Evidence root: `D:\DevData\Rainee\verification\rainee-2-studio-2026-10-13` (the run began October 13).

The b008 baseline reproduced content-driven enlargement when changing Studio pages. Headless regression covers repeated subsection/idle frames at 1440×900 / 1× and 1024×768 / 2×, touch jitter, deliberate header dragging, unchanged art and brush rename/undo. The first full suite reported 877 passed, one newly missing localization, two ignored; the missing translation was supplied in all ten complete catalogs. An intermediate recompile was deliberately interrupted after final source refinements; it is not a passing run. Final build/test/native results will be recorded below after execution.

## Still missing — do not label 2.0 physically or visually complete

1. Per-brush pressure/tilt curves, taper profiles and stabilization editors. Pen input currently links to tool preferences; Studio's pad is position-only. Existing tool pen behavior is not a substitute for per-brush graphs.
2. Dedicated shape/grain source editing/import, source thumbnails and pattern-tile tools inside Studio. Vector source editing remains on the document canvas.
3. Live wet-media rendering: fluid transport, pigment mixing, wet-on-wet bleed, drying/pooling and bristle dynamics must be wired to editable strokes and brush serialization. Standalone experimental fluid modules and disabled sliders do not prove this.
4. Full primary/secondary layer-selection semantics, nondestructive hover blend previews and a richer layer-options surface. Existing hierarchy, modifier selection, rename/reorder/visibility and inline opacity/blend controls remain available.
5. True stacked-window glass sampling and morphing transitions. Current glass samples the canvas/background only, with bounded CPU scattering and existing GPU mesh compositing; it does not sample other windows' text or controls.
6. Proven 80–90% brush-reference coverage, complete end-to-end 1.0 acceptance, sustained large-document performance and full accessibility/localization acceptance. Targeted tests and screenshots are not those broader proofs.
7. The retained Artboard/Backdrop quick pickers still mutate setup directly in `chrome.rs`. They need a dedicated command/undo transaction; their visibility and existing behavior are preserved, not represented as newly validated transaction semantics. Do not remove them while correcting this.

## Public design / implementation references

- [Procreate Brush Studio](https://help.procreate.com/procreate/handbook/brushes/brush-studio): attribute/settings/drawing-pad organization and reversible editing.
- [Procreate Studio settings](https://help.procreate.com/procreate/handbook/brushes/brush-studio-settings): reference capabilities; unsupported engines are not given decorative functional-looking settings.
- [egui 0.36.2 Window](https://docs.rs/egui/0.36.2/egui/containers/struct.Window.html): explicit window drag-area policy. Local source confirms the touch default can allow whole-window dragging and a title-less window has no native header to constrain it.
- [Apple Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/): design inspiration, not a proprietary compositor implementation.
