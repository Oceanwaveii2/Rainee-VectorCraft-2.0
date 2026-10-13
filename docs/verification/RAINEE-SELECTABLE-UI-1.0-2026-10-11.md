# Rainee 1.0 selectable workspace

Current clarification: the owner interrupted the rollback request and explicitly instructed restoration of the Rainee UI. Its left dock, alternate Home and selectable Rainee 1.0 preset are restored. Sketch Cleanup and Research Tools belong under Window → Experimental, not the top bar. The brief rollback is superseded; brushes, pen improvements and document systems are unchanged.

October 11, 2026. This is an optional workspace layout for the existing VectorCraft editor, not a new document format or a claim that the whole Rainee product has reached version 1.0. The owner requested the Illustrator-style arrangement shown in their editor/Home screenshots, then asked for selectable UI modes.

## Selecting the interface

Choose **Rainee 1.0** from the upper-right workspace dropdown, or **Window → Workspace → Rainee 1.0**. Essentials, Essentials Classic, Painting and the other existing modes remain available. Essentials remains the default for new preferences; this change does not force a new mode into the owner's saved settings.

The preset reuses the existing categorized two-column Tools panel and Control bar. A resizable left working dock contains Color/Swatches, Stroke/Transparency/Gradient and Brushes. Each group scrolls independently so settings remain reachable. Existing panels, commands, property state and icons are reused rather than creating a second editing model. Window panel shortcuts activate the relevant relocated tab; panels can still float through their existing menu. Properties/Layers/Libraries and the remaining utility icons stay on the right. Existing enlarged layer previews and text controls are retained, not reimplemented.

Rainee Home hides drawing chrome and shows New/Open/Home navigation, working preset cards and the real recent-file list. With a document open, **Back to document** returns to the same editing session, including unsaved work, without opening a new copy. Saving a custom workspace retains the left-dock flag and tab choices; selecting another workspace does not edit artwork. Existing preferences and saved workspaces missing the new fields deserialize with the left dock off.

## Reference provenance and limits

The supplied local editor/Home references are `codex-clipboard-95e854e7-e55b-45be-b17c-d8774c805630.png` and `codex-clipboard-0709125b-d509-49f7-aab5-66f34d9b162a.png` in the owner's Temp directory. An extracted earlier UI scope was located at `D:\0\Rainee drawing app\01-APP-1-SUPER-ILLUSTRATOR\Rainee_1.0_UI_UX_Scope_RevD\Rainee_1.0_UI_UX_Scope_RevD`; its retained Illustrator-shell arrangement informed this first pass. The ZIP itself was not located. Those documents are historical reference material, not authority to implement their future camera/compositor/Stage scope.

No Adobe application or installed resources were run, inspected or copied. No screenshot, proprietary icon or brush asset was embedded in this UI. This is arrangement fidelity, not pixel-exact replication: colors follow VectorCraft's existing theme, Home uses existing preset art and recent name/folder rows, and there is no new recent-file thumbnail/type/size table. The right utility strip retains existing controls rather than copying Adobe branding or every reference detail. The three left groups have proportional height budgets, not draggable inter-group dividers.

GPU brush/effect execution, GPU pan/zoom improvements and Live Trace enhancements remain subsequent work. This pass does not alter rendering, pen capture, prediction, wet-media physics or document geometry. Low/high-DPI layout fixtures are not proof of Retina-like presentation performance or complete small-screen usability.

## Verification

Scoped tests pass: five Home tests, including a real egui click returning to an unsaved document; three working-dock tests covering selection, state serialization, panel routing and usable canvas space at 1x/2x DPI; seven existing workspace tests. Targeted changed-file formatting, `cargo check -p vectorcraft-ui-egui --target wasm32-unknown-unknown` and the Windows editor/CLI build pass. These are scoped UI checks, not a new full-engine, hardware or sustained-performance acceptance run.

`integrations/Test-RaineeUi.ps1` uses only a fresh app and its own synthetic three-path document. It checks workspace discovery, switching to/from Essentials without changing a source node, relocated Gradient access, custom-workspace tab restoration and native save. Initial captures made during the unchanged splash were superseded by full-window captures after it ended. Screenshots were visually inspected without cropping or alteration.

The worktree contains pre-existing owner changes. No commit, owner preference reset, source checkout replacement or owner-document discard is performed. The packaged launcher routes temporary/cache data to D, verifies package hashes and keeps the local control endpoint opt-in. It still uses the installed machine runtimes and is not a standalone portable installer or release-performance certification.

## Final local package and native evidence

Restoration follow-up: current package is `D:\DevData\Rainee\build\VectorCraft-Rainee-UI-Restored-Experimental-2026-10-11`, editor SHA-256 `4832AB8245527348F8FE3BA26BE9182E0713C0E3A703DAFBD6D9989E7F9C4BDA`. The original dock, workspace, UI-state and Home implementation files match the preserved UI package's source hashes. The top bar omits Sketch Cleanup and Research Tools; both existing commands are enabled under Window → Experimental. Sixteen scoped tests pass (three dock, five Home, seven workspace, one Experimental menu test), as do targeted formatting, WASM checking and Windows editor/CLI builds. Native evidence at `D:\DevData\Rainee\verification\rainee-ui-restored-2026-10-11` verifies workspace switching, unchanged artwork, saved tabs, relocated Gradient routing, both experimental panels opening and Back to document. Full-window editor/Home screenshots are visually inspected. This current package supersedes the brief previous-UI rollback package; earlier evidence below remains dated history.

Package: `D:\DevData\Rainee\build\VectorCraft-Rainee-UI-1.0-2026-10-11-Verified`. Editor SHA-256: `D186738BDA3DCCEB7AA539ACBD4A544DA6D645281FF02F20A65858E638F012AD`. Launch with the package's `Launch.ps1`; its default launch does not open a control listener. The test instance used port 8016 and `-NoPreferences`, so it did not overwrite the owner's saved UI preferences. The earlier, clean saved synthetic trial window was closed; its files and package were preserved.

Final-package checks pass at `D:\DevData\Rainee\verification\rainee-ui-1.0-final-2026-10-11`: `results.json` records discovery, switching, artwork preservation, relocated panel routing and saved-workspace restoration. `home-return.json` records a native click on Back to document, leaving Home with the same document state. The logical window is 1440 by 900, captured at 2160 by 1350 physical pixels; the drawing canvas is approximately 769 by 733 logical pixels. `rainee-home.png`, `rainee-drawing.png`, `home-with-document.png` and `returned-document.png` are full-window evidence, visually inspected. No routine Cintiq retest was requested for this layout-only change.
