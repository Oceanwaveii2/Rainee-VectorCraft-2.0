# Rainee 2.0 UI — first native implementation

## Layers reference pass — latest package

All supplied Procreate images were used as local visual references for the existing 2.0 styling; this implementation pass focuses on Layers. No supplied artwork or proprietary assets are shipped.

2.0 now has thumbnail-led rounded blue-selected rows, readable names, right-side visibility and compact blend badges. Disclosure retains vector hierarchy. Rename, row selection with existing modifiers, drag-reorder and contextual Lock/Options/Target Appearance remain available. The 1.0 row renderer and toolbar are unchanged. The removed target/selection-square columns remain in 1.0; 2.0 offers appearance targeting through its row context menu instead of duplicating the old chrome.

Blend badges open an inline section above the row list, not a popup and not a variable-height expansion interleaved beneath each row. It names and binds the exact document/node, clears stale document targets, and applies object-level transparency using explicit ids and item:null. Opacity has a full-width blue track; six filtering categories expose the renderer's 16 supported blend modes. Clicking a mode commits an undoable change; hover-preview transactions and a distinct primary/secondary layer model are still pending. Visibility uses existing commands; locked rows show a compact cue and expose unlock through context actions.

21 Layers tests passed during the pass, including 2.0 non-mutating frames at 1x/2x DPI and cross-document inline-target cleanup. Final Windows dev build and WASM UI check passed after opacity-track refinements. Native testing of the final package on port 8032 passed workspace order/switch-back without artwork changes. Screenshots were inspected: blue row/badges, inline controls and visible opacity change. A synthetic click set Layer 1 opacity to approximately 0.502; Undo restored it. Clicking Multiply set that layer's blend; Undo restored the full document JSON. Initial immediate queries arrived before queued UI events and did not prove failure; final assertions waited for the next presented frames. Physical pen interaction, exhaustive 2.0 rename/reorder, save/reopen/export and full UI parity are not claimed.

Latest package: D:\DevData\Rainee\build\VectorCraft-Rainee-2.0-Layers-Verified-2026-10-12. SHA256: B671E19BCD400FFCC91A14954769E1335540FEAF7B13344D885CAE767B195B98. Evidence: D:\DevData\Rainee\verification\rainee-2-layers-verified-2026-10-12 (rainee-2.png, inline-controls.png, opacity-ui.png and 1.0 retained screenshot). Earlier candidates retained; no owner drawings or running windows closed.

## Visual refinement pass — current package

Owner-supplied Procreate reference images informed panel proportions and styling only; no reference artwork, proprietary presets or icons are bundled. The regular Rainee 1.0 toolbar and workspace remain.

Passes: (1) dark charcoal panel skin with rounded cards and blue active tabs/rows; (2) neutral white-on-charcoal renderer-backed brush silhouettes, names above previews and a larger library allocation; (3) dark three-column Brush Studio with blue Properties / Shape-Nib / Texture-Ink navigation and a white-on-dark disposable pad. Categories expose existing supported definition fields, not invented functionality. Original artwork colors remain unchanged: white preview conversion happens only in UI preview textures. Full Procreate authoring pages, circular inner color field, advanced pressure-page UI and primary/secondary layer model remain incomplete.

Three brush-panel tests passed. Windows optimized dev build, WASM UI check, and scoped git diff whitespace checks passed. Fresh native control tests on port 8029 verified workspace ordering/switch-back without changing artwork. Native screenshots of the workspace and Brush Studio were visually inspected. A synthetic screen-space drag produced a visible white Drawing Pad stroke; document layer JSON was unchanged. This supersedes the earlier no-presented-frame limitation for this scenario, not for all window configurations. Physical Cintiq pad dynamics and full UI acceptance are not claimed.

Evidence: D:\DevData\Rainee\verification\rainee-2-visual-2026-10-12 (rainee-2.png, rainee-1-retained.png, studio-blue.png, studio-pad-drawn.png). Current package: D:\DevData\Rainee\build\VectorCraft-Rainee-2.0-Visual-Pass-2026-10-12. Editor SHA256: 9EFBA6A5238AA2F35DFB48116D726ACDE54DCD99E4FD17520C5B87CE9CC5BD3C. Launch.ps1 uses local Python and the packaged bridge kit, not a portable installer. Older packages preserved.

## Brush Studio follow-up

The follow-up adds a disposable renderer-backed Drawing Pad (up to eight strokes, 512 normalized samples per stroke), Clear, document-bound drafts and conflict checks against the original brush definition before Apply. Apply uses brush.options and is undoable. Recent/Pinned and active engine-type set now serialize with UI preferences; missing names are ignored by the document library filter. Previous first-pass limitations below are historical: the pad, persisted pins, and cross-document protection are no longer pending.

Three brush-panel tests pass: existing drag-and-drop, preference serialization, and isolated pad / undo / stale-definition / cross-document checks. Position-only pad input does not claim native pressure/tilt calibration, wet-media latency, full per-engine authoring pages or primary/secondary layer semantics. Shared drawing canvas and 1.0 toolbar remain unchanged.

The prior running debug executable was renamed in the same build directory to vectorcraft-running-preview-2026-10-12.exe so Windows could link the update without closing its window; its contents were preserved. No owner drawings or older packaged preview were removed.

Windows dev build and WASM UI check passed. Fresh native workspace tests passed on control port 8028. The follow-up's workspace screenshot succeeded, but screenshots after attempting to open Studio failed twice with "no frame was presented" even after a focus request. Telemetry remained responsive, reported no UI error and reported the pen window unfocused. This does not prove a Studio visual regression or successful drawing-pad interaction; visual/hardware pad acceptance remains pending. Unit tests verify the pad's isolation/renderable document and edit guards; they are not end-to-end pen tests.

Updated package: D:\DevData\Rainee\build\VectorCraft-Rainee-2.0-Studio-Preview-2026-10-12. SHA256: 7E863E0531EFA0517213E570D4E6A1A1E80606C4FC9E91449BB22C4B530F13DF. Native workspace evidence: D:\DevData\Rainee\verification\rainee-2-studio-2026-10-12. Prior first-preview package remains available.

Owner decision: add a selectable Rainee 2.0 immediately beneath Rainee 1.0, retain the 1.0 toolbar, and use the Procreate-inspired panel direction. This is a first implementation, not completion of the entire RevC/RevD design bundle.

## Implemented

- Workspace selector and Window → Workspace list include 2.0 below 1.0. A saved workspace preserves the new panel flag. Switching back restores the original panel renderer. Toolbar/control settings are identical; original toolbar source was not replaced.
- Color modes: Disc with hue ring and saturation/value field, existing Classic spectrum, basic harmony swatches, hex/HSB Value controls, existing document swatches in Palettes. Edits route through the existing paint/gradient-target and undo commands.
- Brush Library: two columns, actual rendered stroke previews and names, search, existing engine-type sets, session-local Recent/Pinned. Selection applies the existing brush command. Double-click preview or Edit opens a first Brush Studio window with numeric/boolean definition fields, renderer-backed preview, and Apply/Cancel via brush.options.
- Layers: larger rows in 2.0 without overwriting 1.0's row preference, and quick access to existing transparency controls. Existing hierarchy/disclosure, selection, lock, reorder and virtualization remain.
- CPU rendering and GPU-assisted navigation remain. No Stage or animation work.

## Explicitly pending

Full Brush Studio per-engine pages, disposable drawing pad, nested brush fields/dynamic curves, persistent custom brush sets and pins, detachable Color Companion, complete harmony modes, exact CMYK/Lab Value UI, and separate Primary/Secondary layer editing semantics. Layer blend access currently uses the existing selection-targeted transparency panel, not per-row live-hover blend transactions. Hue sliders currently use ordinary command changes; full grouped slider-gesture polish is pending. Studio drafts should be completed or cancelled before changing documents; cross-document draft binding needs hardening. No full Procreate feature-parity claim.

## Verification

- 6 workspace unit tests passed, including new ordering, toolbar equivalence, switch-back and JSON saved-workspace checks.
- 3 existing working-dock tests passed, including non-mutating switching and 1x/2x DPI canvas layout for 1.0. These are not yet exhaustive 2.0 DPI tests.
- Windows dev build and WASM UI check passed. Release compilation was intentionally stopped; package is an optimized dev-profile preview, not release/performance certification.
- Native app control test passed ordering and 1.0/2.0 switches without artwork changes. Screenshots visually inspected for 2.0 and Brush Studio; no repeated hardware Cintiq test requested.
- No full CI, full brush authoring, save/reopen of newly edited brushes, or end-to-end model editing acceptance claimed for this UI change.

Evidence: D:\DevData\Rainee\verification\rainee-2-ui-2026-10-12. Reproduction: integrations/Test-Rainee2UI.ps1 against a fresh app with --control 8027.

Package: D:\DevData\Rainee\build\VectorCraft-Rainee-2.0-UI-Preview-2026-10-12. Editor SHA256: 91F4E3EDAFA75B24704F52E3E86D53A061FE3C23FAB5CFD380770A979C585CD1. Launch.ps1 configures the local bridge kit and installed Python runtime; not a standalone installer. Existing verified 1.0/GPU package preserved.
