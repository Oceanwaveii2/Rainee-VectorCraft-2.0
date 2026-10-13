# StripMaker classifier and native SDK verification

October 11, 2026. The first sketch-cleanup milestone has an isolated Rust classifier implementation and compiled author reference tools. The native solver cannot yet process sketches because it does not obtain a valid license. Preview, accept/reject and editable consolidation in VectorCraft remain unfinished.

## Verified results

| Check | Result | What it establishes |
|---|---|---|
| Rust unit tests | 4 passed | Inclusive branches, malformed input, model-hash and size rejection, invalid features |
| Author C++ inference parity | 1,158 feature rows, 300 trees, maximum absolute error 0 | Identical results on the tested artificial feature vectors |
| Lab Clippy | Passed with `-D warnings` | Standalone lab lint gate |
| Lab WASM library check | Passed | Classifier library builds for `wasm32-unknown-unknown` |
| Lab formatting | Passed | Standalone Rust format check |
| Windows SDK signature | Valid Gurobi, LLC | Owner-supplied installer and runtime signature |
| Native C++ SDK probe | Compiled and linked; runtime version 13.0.3 | Native development libraries and DLL can load |
| Native C++ one-variable solve | Error 10009 | No valid native solver license obtained |
| Python one-variable solve | Status 2, optimum x = 1 | Installed `gurobipy==13.0.3` executes a model under its restricted non-production license |
| Author native reference tools | Three Release targets built | Grouping, endpoint refinement and fitting CLIs compile |
| PowerShell scripts | Parse checks passed; setup and author-build scripts exercised | Local repeatable setup procedures, not a packaged editor release |

Error 10009 is Gurobi's `NO_LICENSE` code. The Python size-limited license is not evidence that the native reference tools can solve sketches. No license was activated, purchased, transferred into containers or included in project files. [Official error-code reference](https://docs.gurobi.com/projects/optimizer/en/current/reference/numericcodes/errors.html).

## Source and local artifacts

Author source: [StripMaker revision 810a61a](https://github.com/squidrice21/StripMaker/tree/810a61a03c2177a61f8e32b8ce72c33dccfac49f), checked out clean at `D:\DevData\Rainee\research\StripMaker`. The source was not patched for this build. The author CLIs still use the published feature extraction and solver pipeline.

Classifier lab: [source and reproduction instructions](../../integrations/stripmaker-lab/README.md). Its own lock-file SHA-256 is `3C115ED500C4CFDC2BDD12BB7DC5D5EDB8FB1997098E3BD24F9DDF58E4B4C0B1`. It is a standalone workspace; the existing editor manifest and dependency lock were not changed by this work. The author C++ wrapper and SDK probe are research-only, not dependencies of the Rust editor.

SDK root: `D:\DevData\Rainee\tools\gurobi-sdk-13.0.3\gurobi1303\win64`. Installer SHA-256: `F8835DA03F204E1FBE49B8779AD53D7EC5B992396C57EC47A6930D69B520D070`. Administrative MSI extraction returned 0 and preserves the owner's original downloads. Headers, C/C++ import libraries, CLI and signed runtime are available without global PATH or system product registration.

Native reference executables are in `D:\DevData\Rainee\build\stripmaker-author\Release`. The initially successful Release build produced:

| Executable | SHA-256 |
|---|---|
| `clustering-solve.exe` | `92BD3EED589E45F4E2C5222DBA3BBAF87C0B8CE5AB7CF6581E04FAC1B9044877` |
| `clustering-endend.exe` | `5A3B2094CBED998A1C24D80DEA9170FB109A052A340E45686A33CF8B197E2FF0` |
| `strokestrip.exe` | `C5B2B19F27DCA830F71FC6E352BB4DFEE3E4F97C312D59216D97331C05441CEA` |

These are local research artifacts, not a portable VectorCraft build. The reference tools also require the Gurobi and TBB runtime DLLs. They have not processed a sketch, and their Python bindings are not built.

## Test scope

Each classifier has 150 trees. Local inference takes 27 features and contextual inference takes 33, in the exact author order. Each receives one zero vector, 128 deterministic pseudo-random vectors and three values around each tree's root threshold. The tests cover selected branch boundaries and arithmetic, not every branch, authentic extracted features or artist intent.

The parser pins canonical LF source hashes, rejects changed models and bounds file size, tree depth and node count. Feature rows must have the correct length and finite values. JSON-lines input is limited to 64 KiB per row. The parity script requires a clean pinned checkout and verifies model hashes before compiling the original C++ oracle. No pickle, drawing dataset or training procedure is used.

## Build configuration

The author targets compile with MSVC 19.41, Gurobi 13.0.3, the pinned dependency declarations and C++20. The build script passes explicit native Gurobi library paths, compatibility settings for old CMake projects and a D-drive spdlog package prefix. `setuptools==80.9.0` supplies `distutils` for legacy pybind11 configuration under the existing Python 3.13 research environment. Only the native CLIs are built; this does not validate Python 3.13 bindings or reproduce the paper's Python 3.8 training environment.

SketchConnectivity's optional Gurobi setting is OFF; StripMaker and StrokeStrip continue linking Gurobi. The specific old MSVC iterator warning is suppressed, with normal exception handling enabled. No classifier weights, feature definitions or optimization algorithms were changed. All research downloads, dependencies, temporary files and build outputs are on D. Docker servers, CUDA/OpenCL work and editor renderer replacement are excluded.

## Remaining acceptance gates

1. Configure a valid native license and rerun `Test-GurobiSdk.ps1`. A local license path is sufficient; do not paste keys or credentials into chat.
2. Run the author feature extraction, grouping and fitting on small original synthetic sketches with known intended groups, gaps and junctions. Record failures and execution time, not only successful outputs.
3. Compare the Rust classifier on authentic extracted feature vectors. Decide whether to reproduce the remaining geometry in Rust or use a bounded optional native research companion.
4. Add asynchronous selected-object proposals through the existing command registry, binding jobs to the intended document and revision. Expose progress, cancellation, supported parameters and unavailable states to both UI and MCP.
5. Preserve original strokes through preview and rejection. Acceptance must be one undoable operation, with editable paths, variable widths, save/reopen and export tests.
6. Verify on owner-selected artwork, including hatching and stylized hair. Dataset and transitive redistribution terms remain separate from these private experimental tests.

The existing Cintiq focus fix and conventional drawing tools were not changed. Smart Inker stays dropped; GPU support remains the last phase.
