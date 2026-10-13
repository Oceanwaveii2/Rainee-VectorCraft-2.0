# VectorCraft pen button settings and verification

The local Windows build adds **Edit → Preferences → Pen & Wacom Buttons**. Each side button can select a tool from the existing catalogue, either temporarily while its key is held or permanently when pressed. A separate setting assigns the physical eraser tip when native Windows Ink packets reach the canvas.

## Set up the Cintiq buttons

1. Open the Pen & Wacom Buttons category and enable app-local button assignments.
2. Click **Detect button 1 key**, then press the intended physical pen button. Repeat for button 2. Detection lasts ten seconds and changes only the dialog's working copy.
3. Choose each button's tool and behavior. Button 1 defaults to Eraser with hold-and-restore; button 2 defaults to Paintbrush with press-to-switch. Click OK to apply.
4. If the driver sends an immediate key press and release, choose **Press to switch tool**. Hold-and-restore requires a real held key. If no supported key arrives, configure the Wacom driver's application-specific mapping to an unmodified bracket key or F13–F16, then detect it again.

The earlier owner recorder contained `[` and `]`, but no native barrel-button flag. The default key assignments therefore use those bracket keys. They are not proof of which physical button sends which key. F13/F14 avoid the app's existing bracket-based brush-size shortcuts when the driver can send them.

These settings do not reprogram the Wacom driver. The app cannot distinguish a physical keyboard press from a pen button mapped to the same key. Assignments are disabled by default, exclude dialogs and text editing, and leave modified shortcuts alone. If both assignments use the same key, button 1 wins.

## Stroke and document safety

Tool changes wait until an active stroke finishes. Releasing a temporary tool during a stroke restores the previous tool after completion rather than splitting the stroke. Focus loss cancels an owned unfinished temporary gesture and releases its assignment. Manual tool selection and document switching override temporary restoration.

Settings use `prefs.get`, `prefs.set`, `prefs.list`, and `prefs.reset`, so the existing control channel and MCP can configure them. Tool selection uses `tool.select`; unsupported tool states report the existing engine errors. The settings persist through the existing preference pipeline, outside the document's undo history. Sessions launched with `-NoPreferences` intentionally do not persist settings after exit.

Preference keys are `penButtonsEnabled`, `penButton1Key`, `penButton1Tool`, `penButton1Mode`, the corresponding `penButton2*` keys, and `penEraserTool`. Button keys are `none`, `[`, `]`, and F13–F16. Tool values are registered tool IDs or `none`. Modes are `hold` and `switch`.

## Verification status

Seven button-routing/settings tests, five native canvas pen tests and 26 engine preference tests passed. They cover temporary eraser restoration, permanent switching, preference round trips, old preference compatibility, validation, focus loss, manual tool changes, dialog/text exclusion, release during a drawing gesture, dialog-local detection/timeout and an assigned physical eraser tool. New preference/menu strings have catalogue entries; both menu-coverage tests passed. The engine/UI WebAssembly check and native build passed.

The presented Windows window was inspected at 150% display scaling (2160 × 1350 physical pixels). Setup notes are collapsed so both side-button groups remain visible; scroll for the physical eraser setting. The live adapter reports NVIDIA GeForce RTX 4080 SUPER (Dx12, DiscreteGpu). An injected F14 key selected Eraser through the ordinary shortcut path. A crossing eraser gesture split a path (object count 2 → 3), added one Eraser history action, and undo restored count 2. Native save/reopen preserved count 2; SVG export reported no warnings. This is a synthetic live-app check, not a physical-button or full MCP/model test.

The full UI library run reports **837 passed, 7 failed**. Remaining failures are the two OS-cursor fixtures, narrow titlebar overlap, paper-color fixture, and three broader localization checks (older expressive/splash strings, Codex/perspective messages, and obsolete partial-catalogue labels). Those failures are not suppressed; full repository CI is not clean. New button feature tests pass, but this is a development build, not a release-certified package.

Build: `dist/VectorCraft-Cintiq-Buttons-2026-10-11/vectorcraft.exe` (development/optimized with debug information), SHA-256 `82E1EC7186F0A7BDDEF79C798C9090669E7DD86758DAD6146FBD27646A37BBA9`. Source base is `4cf912fa947f2c2ce5415f2eb704c39b8e31e9dd` plus the existing dirty working tree and this update. Dependency-lock SHA-256 is `B0E8DA95C91EC56D8C29A479979DE1B852145311E4DCC70D136E78DEC2D188CC`. This preserves the earlier Spatial/Stylus package rather than overwriting it.

Reproduce from the checkout:

```powershell
. ./integrations/storage/Use-DDrive.ps1 -Target Editor
cargo test --locked -p vectorcraft-engine --lib tests_prefs
cargo test --locked -p vectorcraft-ui-egui --lib pen_button
cargo test --locked -p vectorcraft-ui-egui --lib canvas::native_pen
cargo check --locked -p vectorcraft-engine -p vectorcraft-ui-egui --target wasm32-unknown-unknown
cargo build --locked -p vectorcraft
```

Normal launches use the existing preferences pipeline. The owner's `%APPDATA%/VectorCraft` junction resolves to `D:/DevData/Rainee/appdata/VectorCraft`. The package launcher requests the RTX adapter; this does not turn the vector canvas into a fully GPU-rendered canvas or add CUDA effects.

Hardware acceptance remains required: identify both physical buttons using detection, verify driver key-down/key-up behavior, erase an ordinary path and undo it, resume the original drawing tool, and confirm there are no duplicate strokes or stuck tools. The previously running test window received ordinary pressure strokes but reported zero native packets; this build's button-key routing does not establish that the native high-rate pen path, tilt or physical eraser tip has passed.

The owner drew 10 Paintbrush and two Blob Brush actions in the isolated verification window during this update. That unsaved document and its window were preserved. Its native packet count still reported zero, so the native high-rate route remains an unresolved hardware integration issue, not a passed Wacom channel test. A separate packaged instance was opened with normal preference saving enabled on opt-in loopback port 9442; its blank Cintiq Button Setup document has the settings dialog open, assignments enabled in the working copy, and button 1 prepared as press-to-switch. Detection and OK still require the owner. The older isolated window is on 9441 and does not save preferences/recovery; save its drawing explicitly before closing it.

## NPR scope remains separate

The reference folder `H:\Rainee NPR Render Research` contains Granblue Fantasy Relink, DNF Duel, Exos Heroes and research documents. Game shader extraction and decompilation remain future reference work. No game was launched or decompiled for the button implementation.

Goo Engine remains the owner's future Stage candidate; 3D integration and animation are outside this update. Its public `goo-engine-v4.4-release` branch is newer than the previously downloaded default branch. The release branch reports Blender 4.4.3 and commit `d281a65ebe227cad2726f79255218dcf88d03f92` dated September 14, 2026. Blender's current stable download is 5.2.2 LTS; its upcoming 5.3 material-lighting nodes deserve a later NPR comparison, not an immediate engine switch.

Sources: [Goo release branch](https://github.com/dillongoostudios/goo-engine/tree/goo-engine-v4.4-release), [Blender downloads](https://www.blender.org/download/), [Blender 5.3 NPR lighting controls](https://developer.blender.org/docs/release_notes/5.3/eevee/).
