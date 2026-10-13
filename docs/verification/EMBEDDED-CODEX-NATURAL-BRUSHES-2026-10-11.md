# Embedded Codex editing and editable natural brushes

October 11, 2026. This implementation follows the owner's order: embedded editor control, Apple Pencil software lessons adapted to Windows Ink, then natural appearance controlled through vector tools. Source implementation is not physical Cintiq acceptance or proof of matching the supplied reference artwork.

## Embedded editor control

The new panel connects the signed-in Codex CLI through the supplied bridge kit. Four registered model tools inspect the active document, discover commands, request editor operations and inspect a rendered canvas image. The Rust host executes commands on the UI thread through the normal editor route; Python cannot authorize edits or open a separate editor endpoint.

Approval mode pauses each proposed canvas operation. Trusted mode applies ordinary operations for the current request and document; deletion/removal/clear and undo/redo still request approval. File, clipboard, global settings, connector and remote operations are unavailable through this first embedded route. Discovery reports unavailable actions rather than presenting them as missing editor tools. Saving/exporting remains available manually and through the separate trusted local MCP workflow.

Document UID and revision prevent an operation from being applied after a document switch or intervening selection/content change. The model must inspect again after a stale result. Manual drawing gestures retain priority. Cancellation stops future operations; completed changes remain in ordinary undo history, not silently rolled back. A bounded complete gesture command validates all input samples before dispatch and restores its document checkpoint on a dispatch error.

The panel shows connection/model status, streamed reply text, pending approval and recent operations. `codex.status` and `ui.inspect.embeddedCodex` expose up to 64 lifecycle records, completed-operation count, command timing and before/after revisions. Model text and a queued assistant job do not establish a completed artwork edit. Assistant preview status still comes from `assist.status` and `assist.telemetry`.

The adapter retains the kit's disabled shell, patch, connector, network and filesystem tools. A specialized app-server turn allows registered canvas tools without inheriting the old read-only data-tool instructions. Canvas images are sent as actual image content rather than unreadable base64 text. The [official App Server contract](https://learn.chatgpt.com/docs/app-server) defines dynamic tool calls, streaming and interruption.

## Apple Pencil behavior adapted to Windows Ink

The existing input route already captures native packet history, preserves timestamps and measured channels, deduplicates promoted input and separates provisional predictions from saved geometry. The new correction clears a learned prediction as soon as an actual sample arrives. Requests/replies carry a sample revision and stroke epoch; delayed predictions for older samples remain timing diagnostics but cannot reappear on the newer tip.

This implements the transferable [Apple predicted-touch reconciliation principle](https://developer.apple.com/documentation/uikit/incorporating-predicted-touches-into-an-app), not Apple's proprietary predictor, PencilKit brushes, firmware or handwriting models. Windows Ink does not supply UIKit's estimated-property update protocol. No unsupported Wacom barrel roll, squeeze or touch capability is fabricated. Physical input-to-photon measurements remain separate from model inference timing.

## Editable natural brush foundation

`Rainee Pencil` and `Rainee Dry Ink` are original procedural brushes, not image assets copied from the references. Their serializable natural-brush definitions retain media, scale, seed and fiber count. Render, SVG/PDF export and Expand use the existing generated-brush path. Changing the centerline or editable width regenerates the appearance; erasing an open centerline retains brushed path pieces rather than a single baked image.

Measured pressure creates editable width profiles. Nib angle/ratio influence transverse footprint; this is not a physical bristle model or measured barrel rotation. Natural brushes remain selected when expressive input is enabled. `brush.new` accepts `type:natural`; `brush.edit`, `brush.apply` and `brush.setCurrent` use the established command system.

Generation is bounded: 256 input path elements, 512 sampled chords, 16 fibers and 16,384 input points per width/nib profile. At most 256 derived profile knots are inserted. The earlier 128-point profile cap was unsuitable for real high-frequency input and would discard expressive appearance through fallback; it has been corrected, with a 1,000-entry synthetic pressure/tilt fixture. Large unsupported strokes receive a bounded ordinary calligraphic fallback with a diagnostic API, not an unlimited allocation or a silently missing stroke. Fallback approximates at most 32 subpaths and drops expressive profiles. Dashes, arrowheads, noncenter alignment and physical wet-media mixing are not implemented by this contact model.

The target references call for fine graphite construction lines, grainy shading, tapered comic ink and frayed dry-brush edges, with vector-editable width and erasing. The procedural implementation is an initial foundation for those controls, not a claim that full watercolor, colored-pencil layering, pigment mixing or reference-level fidelity is complete. NeuBE remains the separate optional learned raster appearance assistant.

The supplied AI file and reference images stay unchanged in their original locations. They are not bundled, uploaded or used as training data. Visual evaluation uses original test strokes; physical Cintiq testing and recorded replay remain deferred for the combined owner session.

## Verification

Fifteen mocked Python bridge checks pass, including routing, cancellation, timeout, image content, disabled-tool configuration and closing the Codex child before terminal completion. Five host tests pass for approval/undo, rejection, stale state, cancelled queued operations, permissions and bounded telemetry. All six learned-prediction checks pass when the installed model check is explicitly enabled. Two engine natural-brush checks pass for pressure/width retention, erasing, undo and native roundtrip. The final brush module has 40 focused tests covering determinism, width/nib changes, dense profiles, persistence, erased gaps and bounded fallback.

### Live model and native brush results

The packaged native app completed two signed-in model requests through the mounted panel: one approved path creation and one request-scoped trusted path creation. The host recorded the actual commands, before/after revisions and completion; the model received the rendered canvas as image content. Undo/redo and native saving were exercised. A third no-edit request was cancelled; telemetry reached `Cancelled`, with zero completed operations for that request. The native panel screenshot was inspected in full.

Native synthetic gestures created pressure/nib-profiled Pencil and Dry Ink paths. Width editing changed rendered appearance without changing the centerline. Erasing split the selected dry-ink path into editable brushed pieces; undo/redo, save/reopen and SVG/PDF export passed. Initial visual inspection revealed synchronized long contact gaps that looked like dashed technical lines. The refined contact model joins core contacts and staggers small outer-fiber breaks; subsequent PNG inspection confirms removal of that repeated-gap pattern. This remains a fiber-based approximation, not reference-level graphite or wet-media fidelity.

Initial native results are in `D:\DevData\Rainee\verification\codex-editable-natural-2026-10-11`; refined natural-brush fixtures are in `D:\DevData\Rainee\verification\natural-refined-2026-10-11`. Scripted tool gestures are synthetic editor input and are not counted as physical Cintiq packets. Whole-workspace CI and physical hardware latency/quality acceptance remain open. Tools/brush/engine focused Clippy passed; the broader UI run retained six existing warnings outside the new controller.

`Zero Brush File.ai` opened locally with 444 objects and three layers from its PDF-compatible portion. The app warned that some private editing/layer operators and placed-file editing data were unsupported; this is not evidence of complete editable brush import. Original SHA-256 before and after opening was `1b38015d4acfc6a727b4653626784697b859dd74a88e02123bb72b08c6e31592`. No original artwork was saved over or sent to the model.

The old `codex_panel.rs` is retained as historical source but is no longer the active module; `codex_editor.rs` is compiled through the `codex_panel` module path. Existing owner drawing windows, recordings and preferences remain intact. No canvas GPU replacement, 3D Stage or animation work is included.

### Final owner package

The runnable package is `D:\DevData\Rainee\build\Codex-Natural-Owner-2026-10-11`. Its native window was opened with the opt-in local control port 7997. Both the live approved/trusted model test and the synthetic natural-brush test passed against this final package. Results and full-window screenshots are in `D:\DevData\Rainee\verification\codex-natural-owner-2026-10-11`; the expanded natural controls were visually inspected. Native build and WASM checks passed. This optimized debug package is not a release-performance benchmark or a fully portable dependency bundle.

SHA-256 identities:

- `vectorcraft.exe`: `F7E58A48FF9400B6D286B0912B462B2C0C48B2CBBDD27D003A57E0BC3E07BE58`
- `vectorcraft-cli.exe`: `0ACF54F99A5ACD8DB9810A095791C1481B905130C1CE2317837F8036E5D82D66`
- `integrations/codex_bridge.py`: `8872015562B706927146907AF343E648E2B9198C7802F7E7B149DEC361109B56`
- dependency lock: `434895515C790654F6AE71A0285249FEE946A04B76AEEB9449BE51CDBAD26151`

The additional `Sketch File.ai` reference is 380,999,400 bytes. The initial open reached PDF import options. During the subsequent brush research pass, the separate test instance completed import with 3,021 objects, 14 imported layers and clean document state. This does not verify brush-library recovery or appearance fidelity. The original file remained unchanged, with SHA-256 `97893E5496AC75421AE7C083E73BDFDEEE0CF90C43B0CCE0A7CBF027B7E5745A`. It was not saved over, bundled, trained on or sent to the model. See [brush replication research](../BRUSH-REPLICATION-RESEARCH-2026-10-11.md) for the continuing technology and per-preset acceptance plan.
