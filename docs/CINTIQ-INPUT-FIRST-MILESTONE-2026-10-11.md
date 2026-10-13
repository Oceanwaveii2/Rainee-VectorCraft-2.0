# Cintiq input repair and first drawing enhancement milestone

The subsequent [editable sketch cleanup integration](verification/EDITABLE-SKETCH-CLEANUP-2026-10-11.md) adds an optional background worker, preview, shared UI/MCP commands and reversible acceptance for ordinary stroke bundles. Its physical Cintiq and owner-artwork acceptance is separate from the input repair below.

The native Windows Ink focus gate is patched and the owner confirmed the front-tip / rear-eraser / front-tip test works on the Cintiq Pro 24. Capture now checks current Win32 focus before reading pointer history and during each input frame, rather than waiting for focus messages to arrive through a queued-message hook. This passes the reported rear-tip scenario, not every Wacom hardware acceptance check.

## Input repair

The app binds its window while it is hidden. The previous adapter saved that initial focus state and relied on queued `WM_SETFOCUS`/`WM_KILLFOCUS` observations to change it. Windows can dispatch sent messages separately from the queued message returned to winit's hook; this could leave the native adapter permanently gated off. This is a source-supported explanation for the earlier zero-packet editor telemetry, not proof that every possible input issue has been resolved.

The adapter now queries [GetFocus](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getfocus) and the foreground window before the pointer-history gate. Focus must belong to the bound editor window or one of its children, with that editor in the foreground. Focus loss clears queued packets and interrupts the owned gesture. Returning to the editor does not resume the abandoned stroke. The implementation retains winit's handling of controls and keyboard-mapped buttons; no WndProc replacement, driver remapping or synthetic pen injection is involved. [Microsoft message dispatch documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-peekmessagew).

Source: [native adapter](../apps/vectorcraft/src/native_pen.rs) and [canvas dispatcher](../crates/ui-egui/src/canvas/native_pen.rs).

The local control channel's `ui.inspect` now includes `penInput.capture`:

- `bound` and `focused`: adapter binding and current native focus.
- `queuedFocusMessages`: focus messages actually seen by the queued hook.
- `pointerMessages` and `unfocusedPointerMessages`: messages seen and rejected at the focus gate.
- `history`: received/accepted/invalid/repeated/truncated packet counters.

These diagnostics do not generate strokes and are not saved in documents. Physical rear-tip flags remain separate from side buttons mapped to keyboard shortcuts. The default rear-tip tool is Eraser; an assigned rear-tip tool is restored to the original front-tip tool after the rear gesture completes.

## Verification results

The patched native editor received and accepted **831 Windows Ink packets**, with zero invalid packets, repeated history, truncated history or dispatcher drops. The queued hook observed zero focus messages, while the new polling path recovered focus after the initially hidden window became active. The live document history contains Paintbrush and Eraser operations, and the current tool returned to Paintbrush. The owner replied "ah it's working" to the requested front-tip / rear-eraser / front-tip sequence. Returning to Codex made the editor's live focus flag false, as expected.

Nineteen targeted tests pass: five native adapter tests, seven canvas/native pen tests and seven button/settings tests. The canvas regression checks real artwork changes and tool restoration, not just an eraser flag. Native `cargo build --locked -p vectorcraft`, the UI WASM check and app-only Clippy with `--no-deps -- -D warnings` pass. The broader dependency Clippy gate is blocked by four existing `collapsible_if` warnings in `crates/doc/src/setup.rs`; those unrelated color-setting changes were preserved. Full-suite and sustained latency acceptance are not claimed.

Package: [VectorCraft Cintiq Focus Fix](../dist/VectorCraft-Cintiq-Focus-Fix-2026-10-11/README.md). Its executable SHA-256 is `DB86D2817272DB4A801866E0441773EBDF58244680B3576A229FB458D4C33967`. The source base is `4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd` plus the existing local edition changes and this patch, not an upstream release. The dependency lock SHA-256 is `B0E8DA95C91EC56D8C29A479979DE1B852145311E4DCC70D136E78DEC2D188CC`.

The running test used isolated preferences and a new unsaved document; it did not change the owner's Wacom driver mappings or saved app preferences. The last live inspection reported the drawing unsaved. A subsequent process check found the test process no longer running; this agent did not close it, save or discard the drawing. The detected presentation adapter was NVIDIA GeForce RTX 4080 SUPER (DX12); this is not evidence of a new GPU effects engine.

## Physical test coverage and remaining checks

In the patched build, on a disposable document:

1. Draw with the front tip in Paintbrush and vary pressure and tilt.
2. Lift, turn the pen over, and erase across that stroke with the rear tip.
3. Lift, turn it back, and draw again. Paintbrush must be restored.
4. Switch to another application while a stroke is unfinished, then return. The abandoned gesture must not connect to a new stroke.
5. Test both side buttons using the explicitly configured app assignments; these are separate from rear-tip handling.

The basic rear-eraser/tool-return scenario above is owner-confirmed in the actual editor. Explicit side-button assignments, interrupted-stroke application switching, duplicate/jump checks and measured pressure/tilt/latency remain separate checks; the brief reply does not establish each of them. Inspect live native counters and actual artwork changes. Synthetic command gestures and the standalone recorder cannot satisfy a hardware gate. Save the test drawing if it should be kept; do not close or discard it merely to finish verification.

## First feature milestone after input acceptance

**Editable sketch cleanup with intelligent vector refinement.** This is the first new drawing capability, not a rebuild of the accepted conventional tools. The [technology reevaluation](../../../_START-HERE/RAINEE-VECTORCRAFT-EXPERIMENTAL-TECHNOLOGY-REEVALUATION-2026-10-11.md) provides the broader sequence.

### Selected stroke bundle consolidation

Implementation progress: the [isolated classifier lab](verification/STRIPMAKER-CLASSIFIER-SDK-2026-10-11.md) runs both released classifiers with exact C++ parity on 1,158 synthetic probes. The follow-up [SCIP/OSQP backend](verification/STRIPMAKER-SCIP-OSQP-2026-10-11.md) now runs native feature extraction, grouping and curve/width fitting without Gurobi. It passes 34 adapter checks and two small synthetic sketch fixtures, preserving original samples and deliberate separation. The endpoint CLI is compiled, not behaviorally accepted. Owner drawings, broader fitting quality, asynchronous editor preview/accept/reject, cancellation, UI/MCP commands and persistence acceptance remain open gates.

Evaluate [StripMaker](https://www.cs.ubc.ca/labs/imager/tr/2023/stripmaker/) grouping with [StrokeStrip](https://www.cs.ubc.ca/labs/imager/tr/2021/StrokeStrip/) fitting. Begin with a user-selected set, not automatic whole-document rewriting. Validate feature extraction and released classifier behavior before treating a smaller Rust adapter as a reproduction of the complete author pipeline.

Show proposed groups and fitted editable paths over preserved originals. Keep stroke widths, object correspondence and intentional separations. Provide accept/reject, cancellation and one logical undo transaction. Processing is asynchronous and must not delay ordinary live ink.

### Intended junction suggestions

Evaluate [SketchConnectivity](https://www.cs.ubc.ca/labs/imager/tr/2022/SketchConnectivity/) for proposed joins and region closures after bundle consolidation works. Display endpoints and intended changes before acceptance. Do not automatically close hatching, hair, deliberate gaps or adjacent parallel strokes.

### Controls and AI access

Register both enhancements through the existing command/tool system, with parameter schemas, enablement and unavailable states. MCP and the embedded model use the same stable-ID operations as the UI. Reject stale document revisions and preserve undo/redo, save/reopen and export. Originals remain available until the owner accepts replacement.

### Milestone acceptance

Owned fixtures cover rough bundles, intentional gaps, junctions, cross-hatching, stylized hair and variable widths. Record incorrect proposals as well as successful cleanup. Accepted results are editable vectors, can be undone/redone, survive save/reopen and export correctly. No available model, unsupported input or cancelled job may disable normal drawing.

Smart Inker remains dropped. GPU companion integration is last; this milestone preserves Vello CPU and does not implement a 3D Stage or animation.
