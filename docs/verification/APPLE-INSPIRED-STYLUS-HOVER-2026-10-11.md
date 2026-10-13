# Apple inspired stylus input and hover

This Windows/Cintiq batch adds native hover feedback and timing diagnostics to the existing measured input and provisional prediction pipeline. It does not implement proprietary Apple Pencil firmware, PencilKit rendering, Apple prediction weights or Apple-platform hardware features.

## Public software principles

[Apple coalesced-touch handling](https://developer.apple.com/documentation/uikit/implementing-coalesced-touch-support-in-an-app) retains intermediate measurements. VectorCraft already consumes Windows Ink history in chronological order and rejects repeated history/promoted mouse duplication. [Apple predicted-touch reconciliation](https://developer.apple.com/documentation/uikit/incorporating-predicted-touches-into-an-app) keeps future predictions separate from actual samples. The existing revision/epoch correction and committed-path tests remain in place. [Apple Pencil hover](https://developer.apple.com/documentation/applepencil) motivates pre-contact feedback; this implementation uses only channels supplied by Windows Ink.

## New controls and feedback

Pencil and Paintbrush expose `hoverPreview` through their independent persistent tool settings and the ordinary `tool.setOption` command. Their Tool Options dialog has a Native pen hover footprint checkbox. It is off by default so saved preferences do not silently change. Enabling it shows a nominal-width ellipse at measured pen hover coordinates. Measured tilt changes the ellipse when Tilt nib is enabled; missing tilt uses a round footprint. The configured fallback nib angle contributes its orientation.

This is a nominal nib envelope, not an exact preview of arbitrary Art-brush artwork, measured hover pressure, or a physical bristle simulation. It does not measure hover distance. No roll/squeeze channel is synthesized. Feedback expires after 100 ms and clears on interruption/contact; painting checks the bound document UID. Hover packets do not commit paths or add undo steps.

## Timing diagnostics

`ui.inspect.penInput.timing` reports UI queue residence and dispatch CPU time. It begins when the adapter hands packets to the UI queue, so it does not include earlier native capture waits or presentation.

`ui.inspect.penInput.capture.packetAgeAtUiDrain` compares Windows Ink's native performance counter with the current system counter. It reports newest/oldest packet ages for the measured batch, cumulative mean/max/count and the last measured batch's newest age. Missing clocks and future timestamps are not converted to invented measurements. These diagnostics persist locally for inspection but are not saved into artwork. None of these values measures physical input-to-photon latency.

Existing pressure/tilt profiles, actual-sample preservation, prediction controls, eraser restoration and ordinary manual tools remain unchanged. The experimental AOSP predictor remains separate from Apple technology and is not promoted as better than the conventional predictor.

## Remaining Apple inspired work

Independent presentation-time measurement and heavy-document latency acceptance remain open. Hover feedback is a footprint, not a full material preview. Windows Ink does not expose UIKit's estimated-property correction protocol through this adapter, so no equivalent late sensor-update support is claimed. Physical Cintiq tests must verify responsiveness, hover orientation, front/rear-tip switching and duplicate prevention. Actual Apple Pencil hardware comparison is not available on this machine.

## Verification

Owner physical trial feedback: "Works pretty well." Subsequent read-only inspection of the port-8004 trial captured 13,139 native samples, zero invalid samples and zero UI queue drops. Native timestamp-to-drain mean was 15.54 ms (max 330.60 ms); UI queue residence mean was 3.09 ms (max 190.40 ms). These include hover/history and scheduling stalls, not physical input-to-photon measurements. The trial drawing was preserved. The earlier pending-acceptance statement below records the initial state, superseded by this subjective hardware feedback; heavy-document and presentation-time acceptance remain open.

Owner follow-up: an Eraser option `eraseAll` bypasses selection through the existing `path.eraseRegion` command. Off preserves the existing selected-or-all-if-none behavior. On targets all editable paths; existing locked/hidden protection and undo geometry operations remain in place. The option is persisted and exposed through `tool.setOption`, and the Freehand/Eraser options dialog labels it "Erase all editable paths, regardless of selection". Closed filled shapes use region subtraction; open vector-brush paths use centerline splitting retaining brush identity, not arbitrary rendered-footprint raster subtraction. Final verification/package results for this follow-up are recorded separately.

Eraser follow-up verification: two new engine tests pass for selection bypass/default preservation and locked/hidden protection; the tool dispatch test, two settings discovery tests and three freehand dialog tests pass. Native build and full UI WASM check pass. Separate runnable package: `D:\DevData\Rainee\build\Apple-Inspired-Eraser-All-2026-10-11\vectorcraft.exe`, SHA-256 `1FE5F9DA9EDF74FA25F8A994F1886C96852843C88799B8253B84818856DD17E9`. A fresh port-8005 document exercised the live eraser command, undo/redo and `eraseAll` option reporting; its options dialog is open. This is synthetic native verification, not owner Cintiq acceptance of the new eraser setting.

Focused native-dispatch, per-tool option persistence, hover expiry/no-art-mutation, focus/eraser restoration, native clock checks and existing prediction tests are the acceptance suite for this batch. Native package, result counts and remaining limitations are recorded after the final checks. The clock implementation uses `web-time` in the UI so the new per-frame measurement does not introduce an unsupported standard-library clock on WASM.

Final automated results: ten UI native-dispatch tests, three freehand dialog tests, six Windows capture/focus/clock tests, two tool-settings discovery tests and five prediction-reconciliation tests pass. The installed-model test remains explicitly ignored in this batch; previous model-worker evidence is not a new hardware or latency result. Native build and the full UI crate's WASM check pass. No owner drawing or Windows/Wacom preference was altered.

The final package is `D:\DevData\Rainee\build\Apple-Inspired-Stylus-2026-10-11`. Executable SHA-256: `CA06768D8001FACF1959DFA69C45E6DF03EFB5C2C89261097290F87E7627CD9D`. It includes the existing Codex bridge script and retains its machine-local runtime dependencies. A separate trial package/process uses control port 8004 and the fresh Apple-inspired Cintiq Trial document. Its options were inspected live: hover preview, pressure width and tilt nib enabled, conventional prediction 14 ms, learned prediction off. Initial capture reports zero physical packets and no clock-derived age samples; this is connection/configuration evidence only. Physical acceptance was requested and remains pending.
