# Cintiq native input test

October 11, 2026. First stylus milestone: inspect native input from the owner's Cintiq Pro 24 DTK-2420 before routing it into drawing tools. The diagnostic is separate from VectorCraft; it cannot alter documents. No ML model, GPU effect, driver change or system-wide input hook is included.

## Capture and replay

The [test application](../../integrations/cintiq-input-probe/) observes native Windows Ink messages on its own event thread. It reads pen history before winit processes the message, reverses the newest-first history into chronological order and records validated unique packets. The [portable contract](../../crates/tools/src/pen_input.rs) retains original coordinates, pointer/device/frame identity, native clocks, channel masks, pressure, tilt, pen flags, pointer button bits and modifier state.

The window displays current sensor values, packet counts, duplicate histories, invalid packets, bounded-history truncation, logging drops and API errors. Native pen packets are the sole authoritative recording stream; ordinary mouse and winit Touch events are not added as duplicate pen samples. Mouse messages still reach window controls. This does not yet change the editor's canvas routing.

Logs are local JSON Lines files, with versioned metadata, raw and normalized samples, diagnostic/lifecycle records and a closing summary. A bounded background writer keeps file I/O out of packet capture. It flushes periodically and on normal close; it is not a crash-durability guarantee. Existing files are never replaced. Close the test window before replaying its recording.

## Hardware test

Open `integrations/cintiq-input-probe/Cintiq-Recording-Replay-v0.2.exe` for the current labelled recorder. The earlier `Cintiq-Input-Test.exe` is retained as the v0.1 diagnostic. Move the window onto the Cintiq if necessary. Its dark panel is an instrument display, not a drawing canvas; strokes are recorded but no ink is painted.

1. Hover above the test area, then touch down and lift several times. Check hover/down/drag/up changes.
2. Draw five slow curves and five fast strokes, including corners and reversals.
3. Draw light-to-heavy pressure ramps; also vary pressure while holding the tip stationary.
4. Tilt left/right and toward/away. Check that each reported tilt axis changes and that missing channels stay unavailable.
5. Flip the pen and touch with the eraser. Check eraser/inverted flags.
6. Press each side button with the tip hovering and touching. Windows or driver mappings may expose one barrel flag or mouse-style buttons rather than two independent physical button identities; record what actually arrives.
7. Use the ordinary mouse. It must not increase the authoritative native pen packet count.
8. Move the window between monitors or change its position; compare screen/client coordinates and reported DPI. This verifies the diagnostic mapping, not the editor's pan/zoom/artboard mapping.
9. Close the window to finish. The default recording is in `captures` beside the executable; an explicit `--output` path is also supported.

```powershell
& '.\integrations\cintiq-input-probe\Cintiq-Input-Test.exe' --replay 'ABSOLUTE-PATH-TO-CAPTURE.jsonl'
```

A replay validates raw/normalized agreement and authoritative packet uniqueness. It is not physical hardware acceptance or input-to-photon latency measurement. An empty recording fails replay. Hardware acceptance requires observed pressure ramps, both available tilt axes, eraser/buttons as exposed by the driver, correct coordinate mapping and no unexplained invalid packets, truncations, logging drops or API errors. The actual pen model and driver configuration still need confirmation. Barrel roll remains disabled unless the active pen's capability is independently confirmed.

If no native packets arrive, keep the recording as evidence and inspect the active Windows Ink/driver route. Do not automatically change Wacom settings, install a driver, or fall back to fabricated tilt or frame-clock timestamps. A selectable Wintab adapter is a later option, not implemented here.

## Automated checks and build

```powershell
. ./integrations/storage/Use-DDrive.ps1 -Target Probe
cargo test --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml
cargo clippy --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml --all-targets -- -D warnings
cargo build --locked --manifest-path integrations/cintiq-input-probe/Cargo.toml
```

The standalone manifest has its own dependency lock and does not build the full editor or GPU stack. The tools crate also exposes the same UI-independent capture contract. Win32 FFI is isolated in the probe's native module with bounded initialized buffers and documented safety contracts, following the desktop app's existing scoped FFI convention. No unsafe code is permitted in the portable tools module.

Fifteen automated tests passed: chronological history, overlapping packet deduplication, window-mapping changes, stationary pressure/button updates, missing-channel handling, API pressure range, pen phases and eraser flags, roll gating, bounded memory/truncation reporting, promotion classification, serialization, logging backpressure, empty-capture rejection, writer/replay agreement and existing-file protection. Native compilation, standalone clippy with warnings denied, targeted formatting checks, tools-crate wasm compilation and the architecture layering check (26 crates) passed. The full editor test suite was not run for this diagnostic.

Packaged diagnostic SHA-256: `E975819471FFCE5EC75D62B84309EB103B1D0A35F0F2AEE880FE2261D8113797`.

The packaged window was launched on this machine. Its live recording, `integrations/cintiq-input-probe/captures/cintiq-1791709827777-36420.jsonl`, contains native hover packets with QPC timestamps, pressure and both tilt axes. The live status subsequently reported drag pressure and eraser input; one observation showed 11,344 accepted packets, five repeated-history packets suppressed, zero logging drops and zero API errors. These are point-in-time observations, not a completed hardware acceptance run or calibrated latency measurement. The recording was still open and had not been replay-validated at this observation.

A window-targeted screenshot was saved at `C:/Users/User/AppData/Local/Temp/codex-shot-2026-10-11_02-11-02.png`. It shows only part of the native window in this multi-monitor configuration; it does not establish full-panel visual acceptance. Complete the remaining hardware checklist and close the window before replaying the finished file.

## Closed recording review

After the owner finished the test, the process had exited and the recording contained a closing summary. Replay passed for all 17,987 accepted samples, with no duplicate authoritative packets or raw/normalized disagreement. The summary reports 17,997 received history packets, ten repeated-history packets suppressed, zero invalid packets, zero bounded-history truncations, zero native API errors and zero logging drops. These counters do not prove that the driver delivered every physical sensor sample.

The recording includes 35 down events, 35 up events, 16,600 drag samples and 1,317 hover samples. Every accepted sample has native QPC time, pressure and both tilt axes; no per-pointer timestamp regression was observed. Normalized pressure ranges from 0 to approximately 0.920, tilt X from -27 to +60 degrees and tilt Y from -34 to +31 degrees. There are 1,420 eraser samples and 1,607 inverted-pen samples. All samples report the same runtime device handle and 192 DPI; moving between displays or DPI values was not demonstrated by this recording.

No barrel-button flag was observed. The owner confirmed pressing both physical side buttons. A subsequent read-only inspection of the active global Wacom profile found the lower side button mapped to `]` and the upper side button to `[`, with shortcut-usage timestamps during the recording. This explains why native pen flags did not identify those physical buttons. The original recorder did not record keyboard messages, so their delivery to that window cannot be reconstructed. The closing summary reports zero pen-promoted mouse messages; ordinary mouse events are not recorded as authoritative pen samples, so this log alone does not establish the complete mouse/UI interaction test.

Although the API support mask advertises a rotation field, normalized barrel rotation remains unavailable: a mask alone does not confirm that the active pen physically measures roll. This recording passes capture/replay integrity and demonstrates pressure, tilt and eraser channels, but full hardware acceptance remains partial pending button and coordinate/mouse checks. Canvas integration is still pending.

Closed recording SHA-256: `6AAA94068E044C358D2CA35F32F80FDC214C8B61E027082F6302146989A2539A`.

## Labelled recording and deterministic replay: v0.2

The new recorder records numerical key codes and mouse-button messages delivered to its own window, including Wacom's configured bracket shortcuts. It does not install a global keyboard hook, record typed text, change the driver or convert shortcut keys into physical barrel flags. Keep the recorder focused during the button exercise. Physical roll remains unavailable.

Choose a scenario with the keyboard **between strokes**, then draw inside the recorder. A mid-stroke label change is retained and marked as mixed labels, not silently corrected. Repeat each movement several times, with both light and firmer pressure where appropriate:

| Key | Scenario | Exercise |
| --- | --- | --- |
| F1 | Slow curves | Slow circles and S-curves, including deliberate pauses. |
| F2 | Fast strokes | Fast straight and curved strokes in several directions. |
| F3 | Corners | L-shaped and zigzag strokes with deliberate sharp corners. |
| F4 | Reversals | Back-and-forth movements and abrupt direction changes. |
| F5 | Pressure changes | Light-to-heavy ramps and stationary-tip pressure changes. |
| F6 | Buttons | Press each side button separately while hovering, then touching; also check the eraser. |

Check the displayed key code/auxiliary counter after each side-button press. Finished capture `cintiq-1791713023544-99848.jsonl` contains separate key-down and key-up messages for both configured shortcuts: codes 219 (`[`) and 221 (`]`). This verifies their delivery to the focused test window without remapping or fabricating physical barrel flags. Profile inspection alone had not established delivery.

```powershell
$taskProbe='D:\0\Rainee drawing app\ArtCraft-Foundation\VectorCraft - Gemini Edition\integrations\cintiq-input-probe\Cintiq-Recording-Replay-v0.2.exe'
& $taskProbe --review 'ABSOLUTE-PATH-TO-CLOSED-CAPTURE.jsonl'
& $taskProbe --replay 'ABSOLUTE-PATH-TO-CLOSED-CAPTURE.jsonl'
& $taskProbe --export-fixtures 'ABSOLUTE-PATH-TO-CLOSED-CAPTURE.jsonl' --output 'ABSOLUTE-PATH-TO-NEW-FIXTURE.json'
```

Review reports packet quality, phases, complete/incomplete strokes, scenario counts, native timestamps, pressure/tilt ranges, DPI, delivered keyboard codes and provenance. Replay feeds unchanged packets in native-time order to a virtual-clock consumer without sleeping or injecting editor input. Batch sizes are tested for identical authoritative samples. This does not yet replay a drawn canvas or measure rendering latency.

Readers reject unsupported/repeated headers, missing closing summaries, truncated/oversized records, duplicate packets, changed normalized data and timestamp regressions. Fixture export also rejects capture-quality warnings and inconsistent QPC clocks. Canceled/orphan strokes remain explicitly incomplete. Existing outputs are never replaced. A recording larger than 64 MiB must be split into shorter sessions.

The finished v0.1 recording was exported locally to `integrations/cintiq-input-probe/fixtures/cintiq-2026-10-11-session-1.json`: 17,987 unchanged accepted samples, 35 paired complete strokes, zero incomplete strokes and no capture-quality warnings. Replay spans 121,124,735 microseconds of recorded native time. No labels were guessed for the old recording; it remains unlabelled and contains no auxiliary keyboard events.

Derived fixture SHA-256: `E07C1F11D2BED9C82098944D6C367CA52830C53095BACA71E632C47587101AA9`. It includes raw/normalized samples, stroke spans, source hash and `approved_for_training: false`. These are local potential training records, not an approved or published dataset. Captures and fixtures are excluded from Git; no data is uploaded. Physical screen/client coordinates are not document coordinates.

Recorder verification: **25 tests passed**, standalone clippy with warnings denied passed, and the native debug build succeeded. The tests cover five original synthetic scenario trajectories, chronological/deduplicated replay, pressure preservation, label handling, cancel/incomplete strokes, shortcut keys remaining separate from ink, malformed/truncated files and no-overwrite exports. The packaged build replayed the owner's finished sessions successfully; the second session confirms both configured side-button shortcuts. Full editor CI, on-screen ink replay and trained-model evaluation remain pending.

Packaged v0.2 SHA-256: `E651E8F64EF2E7156FF30DEBEE89E8B06CE8ED8F290354BDE33580D91A72E464`. This is a standalone debug test package, not a new VectorCraft release. The v0.1 executable and original recording are preserved.

The second finished recording also passed review and virtual-clock replay: 6,868 accepted packets, 22 complete strokes, no incomplete strokes, seven repeated histories suppressed and no quality warnings. All samples retain QPC time, pressure and both tilt axes; 367 eraser samples are present. Pressure reaches 0.78125. Its 48 auxiliary messages include six key-down events for `[` and sixteen for `]`, with corresponding key-up events retained. No native barrel flag is invented. This confirms the configured shortcut route, not two independent physical-button sensor bits.

Second recording SHA-256: `11ED0520BDFB2E45FD64216269ED35C39BF5917EB749C0CAC444A67F22215DD2`. Exported local fixture `fixtures/cintiq-2026-10-11-session-2.json` SHA-256: `6C07D6E37FCF1EBD421F0C1F3B082E8606331549CF76417779A45518CAF76FE6`. The second session also remains unlabelled because no F1–F6 marker was used. Across both recordings there are 24,855 accepted samples and 57 complete strokes; this is not yet the labelled multi-session benchmark or an approved training dataset.

The [ML deployment audit](../STYLUS-ML-DEPLOYMENT-AUDIT-2026-10-11.md) records verified model/code/data availability, pinned classifiers, licenses and coding gates. No ML assistance is implemented by this recorder. [Development storage setup](../../integrations/storage/README.md) routes new local builds and process temporary files to D.

After storage migration, the standalone suite was rerun offline with the existing lockfile and D-drive Cargo, Rustup, build and process-temp paths. All 25 tests, clippy with warnings denied and the native debug build passed. The rebuilt executable successfully replayed both closed sessions; portable-package and fixture hashes were unchanged. Rebuilt executable `D:\DevData\Rainee\build\Cintiq-Input-Probe-2026-10-11\debug\cintiq-input-probe.exe` SHA-256: `334D81EFB506823E0F10C166B8406BEA761DFE3F1A8D1D5D120BE2A88A556E8F`. It is not a replacement release of the editor.

## Next integration

After the hardware recording passes review, reuse the adapter on the actual editor window, map physical client pixels through DPI and the canvas transform into document coordinates, extend tool events with optional measured pen attributes and drain native batches without per-frame point loss. Install one explicit pen source for canvas gestures while retaining mouse-driven UI controls. Add mounted drawing fixtures for duplicate suppression, cancellation on lost capture, tool changes, undo/redo, save/reopen and pressure/tilt retention before beginning ML prediction or assisted inking.

The diagnostic does not claim that the canvas already receives tilt/eraser packets, that all hardware samples are recoverable, or that ML assistance exists.

## API sources

[Microsoft pen history](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getpointerpeninfohistory) specifies newest-first coalesced history, same-thread access and the need to read it before retrieving another message. [Pen information](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-pointer_pen_info) specifies the 0–1024 pressure scale, tilt/rotation units and support masks. Advertised tablet pressure levels are not the normalization denominator for this API.

[Pointer information](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-pointer_info) defines pointer/frame identity, raw coordinates and native tick/performance-counter timestamps. Those clocks are retained as delivered; they are not replaced with UI-frame time or presented as calibrated sensor-to-photon latency.

[Microsoft pen and mouse messages](https://learn.microsoft.com/en-us/windows/win32/tablet/system-events-and-mouse-messages) documents the extra-info signature used to distinguish pen-promoted mouse messages from actual mouse/touch input. The installed winit 0.30.13 Windows event-loop and message-hook source was inspected to choose the capture boundary.

[Wacom's input documentation](https://developer-support.wacom.com/hc/en-us/articles/12844524637975-Wintab) discusses user-configurable controls and differences between available pen channels. The owner's local active profile was inspected read-only; no preference file or serial/device identifiers were copied into the project.
