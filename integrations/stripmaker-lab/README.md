# StripMaker research lab: classifiers and Gurobi-free native solving

This isolated research lab runs the two released StripMaker random forests in Rust and runs the author's native feature/grouping/fitting pipeline through an independent **SCIP + OSQP CPU adapter**. No Gurobi runtime or license is required for this alternative. The editor now has an optional experimental selected-path preview/acceptance integration, without linking the author C++ into the Rust app or replacing its drawing tools. See [editable cleanup implementation and acceptance](../../docs/verification/EDITABLE-SKETCH-CLEANUP-2026-10-11.md) for its bounded scope and current verification.

The intended feature remains selected stroke bundle consolidation with proposed groups and fitted editable paths, followed by explicit accept or reject. Correct feature extraction and fitting are prerequisites for that feature. The feature must preserve originals, intentional gaps and undo history.

## Current native alternative

```powershell
.\integrations\stripmaker-lab\Build-SCIP.ps1
.\integrations\stripmaker-lab\Test-SCIP.ps1
```

SCIP 10.1.0 handles the signed binary orientation problems, linear models and nonconvex fallback. OSQP 1.0.0 handles continuous quadratic objectives with linear constraints only after a bounded positive-semidefinite check. OSQP is built with MSVC, double precision and 32-bit indices using its builtin CPU algebra; CUDA/MKL are not enabled. The narrow API adapter is independent code, not copied Gurobi SDK code. It retains the author feature definitions, normalization and classifiers; the author checkout stays clean.

The hybrid backend passes 34 mathematical/error checks and two original synthetic sketch fixtures. Three overlapping strokes, including one reversed stroke, form one group and one fitted curve. Deliberately separated strokes form three groups and three fitted curves. Grouped originals retain their IDs, points and timestamps. Width fitting also runs. These small fixtures are not artist-quality or full-pipeline parity acceptance. See the [native backend verification](../../docs/verification/STRIPMAKER-SCIP-OSQP-2026-10-11.md) for results, provenance and limitations.

The first SCIP-only attempt timed out on a continuous fit and rejected its result. Routing verified convex QPs to OSQP resolved that fixture without changing the objective or accepting a time-limited incumbent. All unsuccessful or unvalidated solutions remain unavailable.

Build products are in `D:\DevData\Rainee\build\stripmaker-scip\author\Release`; tests create fresh `checks-*` output directories under the same build root. Dependencies, build-bootstrap Python environment, cache and temporary files remain on D. Runtime PATH changes are process-local. The old Gurobi build below remains a historical comparison, not a dependency of this path.

## Source and models

Use the [StripMaker author repository](https://github.com/squidrice21/StripMaker) at revision `810a61a03c2177a61f8e32b8ce72c33dccfac49f`. The local checkout is `D:\DevData\Rainee\research\StripMaker`. Its `src` classifier files are Apache-2.0; the modified StrokeStrip code is MIT. Training drawings have separate licenses and are not included in this lab. [Author project and dependency instructions](https://www.cs.ubc.ca/labs/imager/tr/2023/stripmaker/).

| Model | Features in author order | Trees | Parsed nodes | Canonical LF SHA-256 |
|---|---:|---:|---:|---|
| `forest.cpp` | 27 | 150 | 38,736 | `21e989dd0df4e77bd809ce383b491466d9ccf98c4cec7191dc4db7d759597efc` |
| `forest_sec.cpp` | 33 | 150 | 17,390 | `1a883ae603bb98757b6812e0e7e8b059eb0f6c6b6d4bb8c0a872f68264cc7d6a` |

The parser checks canonical hashes after normalizing Windows CRLF endings. It accepts only the pinned branch and probability-leaf grammar, with file-size, node-count and depth limits. It never executes downloaded code or loads Python pickle files. Feature rows must contain the exact number of finite values in the author's order.

Classifier inference alone needs no Gurobi. The geometric feature pipeline and fitting use joint parameterization and optimization, so substituting endpoint distances or arbitrary feature values would not reproduce StripMaker.

## Classifier checks

From the VectorCraft source checkout, run:

```powershell
.\integrations\stripmaker-lab\Test-Parity.ps1
$env:CARGO_TARGET_DIR='D:\DevData\Rainee\build\stripmaker-classifier-lab'
cargo clippy --manifest-path .\integrations\stripmaker-lab\Cargo.toml --locked --all-targets -- -D warnings
cargo check --manifest-path .\integrations\stripmaker-lab\Cargo.toml --locked --target wasm32-unknown-unknown --lib
cargo fmt --manifest-path .\integrations\stripmaker-lab\Cargo.toml --check
```

The parity script sets D-drive storage, builds the Rust lab and compiles the pinned author classifier files with MSVC as an independent oracle. Each model receives 579 deterministic probes: one zero vector, 128 pseudo-random vectors and values immediately below, at and above each tree's root threshold. The 1,158 rows matched the original C++ results with maximum absolute error **0**. Four unit tests, strict Clippy, formatting and the WASM library check pass.

These probes test inference, not the quality of grouping real drawings or every branch of every tree. They are not an artist dataset or learned stylus-prediction model.

The native CLI accepts JSON feature arrays, one per line:

```powershell
& 'D:\DevData\Rainee\build\stripmaker-classifier-lab\debug\stripmaker-classifier-lab.exe' `
  'D:\DevData\Rainee\research\StripMaker' local
```

Use `contextual` for the 33-feature classifier. Metadata goes to stderr; predictions or explicit row errors go to stdout. Input rows are bounded to 64 KiB.

## Historical Windows Gurobi SDK route

The owner supplied `C:\Users\User\Downloads\Gurobi\Gurobi-13.0.3-win64.msi`. Its Authenticode signature is valid for Gurobi, LLC. The verified SHA-256 is `F8835DA03F204E1FBE49B8779AD53D7EC5B992396C57EC47A6930D69B520D070`.

The installer is copied to `D:\DevData\Rainee\downloads\gurobi`. Administrative extraction makes the full native SDK available at `D:\DevData\Rainee\tools\gurobi-sdk-13.0.3\gurobi1303\win64` without registering a system installation or modifying global PATH. Headers, the C and C++ import libraries and the signed runtime DLL are present. The owner's original downloads remain untouched; the AMPL packages are not needed for StripMaker.

```powershell
.\integrations\stripmaker-lab\Setup-GurobiSdk.ps1
.\integrations\stripmaker-lab\Test-GurobiSdk.ps1
```

The setup script verifies the supplied package hash and signature. The test compiles and links a small C++ program against the native SDK, reports version 13.0.3 and attempts a bounded one-variable solve. It restores process PATH afterward and prints numerical error codes rather than license details.

**Historical native Gurobi result:** compile and link succeed; solving returns `10009`, meaning no valid license was obtained. That original Gurobi-backed route cannot solve here. The current SCIP/OSQP route above removes this runtime requirement; no license-file action is needed for it. [Official error codes](https://docs.gurobi.com/projects/optimizer/en/current/reference/numericcodes/errors.html).

The separate `gurobipy==13.0.3` environment at `D:\DevData\Rainee\venvs\gurobi-evaluation` solved a one-variable Python model. Its bundled size-limited non-production license expires on 2027-11-29 and does not establish native C++ solver availability or suitability for complete sketches. [Gurobi pip installation](https://docs.gurobi.com/projects/optimizer/en/current/reference/python/overview.html).

## Official Docker alternatives

The [optimizer Dockerfile](https://github.com/Gurobi/docker-optimizer/blob/master/13.0.3/Dockerfile) includes the full Linux optimizer distribution. It is a possible Linux SDK route, not the Windows SDK used here. The [manager Dockerfile](https://github.com/Gurobi/docker-manager/blob/master/13.0.3/Dockerfile) and [compute Dockerfile](https://github.com/Gurobi/docker-compute/blob/master/13.0.3/Dockerfile) package server components. The [Python example](https://github.com/Gurobi/docker-python-example/blob/master/13.0.3/Dockerfile) starts from `gurobi/python:13.0.3_3.10`; it does not supply the Windows C++ development libraries. Installing containers would not itself satisfy the licensing requirement.

No Docker images, server services, remote solver access or GPU changes were installed for this native research setup.

## Historical Gurobi-backed author reference build

`Build-Author.ps1` configures and builds `clustering-solve`, `clustering-endend` and `strokestrip` from the clean pinned checkout, using MSVC 19.41 and the extracted Gurobi 13.0.3 SDK. All three Release builds succeed. The author source remains unchanged.

```powershell
.\integrations\stripmaker-lab\Build-Author.ps1
```

Executables are under `D:\DevData\Rainee\build\stripmaker-author\Release`. Runtime DLL search must include the SDK `bin` directory and `D:\DevData\Rainee\build\stripmaker-author\msvc_19.41_cxx20_64_md_release` for TBB. Keep these settings process-local. Compilation is not a successful grouping or fitting test; the native license probe must pass before processing drawings.

Build adaptations are configuration-only: an explicit Gurobi 13 import-library path; CMake's legacy policy compatibility switch; `setuptools==80.9.0` for the old pybind11 configure step; a matching spdlog 1.5.0 package installed to a D-drive prefix; `/EHsc` exception handling; and Microsoft's specific old iterator deprecation suppression. The script uses forward-slash CMake cache paths. Optional SketchConnectivity Gurobi features are disabled, but StripMaker and StrokeStrip still require Gurobi. No solver was replaced and no feature definitions or trained weights were changed.

The native CLIs do not require building the Python bindings. Those bindings and the author's Python 3.8 training/report environment remain separate compatibility work; this Python 3.13 environment is not a reproduction of the training setup. Dependency fetches require network access on a fresh build. The tested source revision and local artifact hashes are recorded in the [verification report](../../docs/verification/STRIPMAKER-CLASSIFIER-SDK-2026-10-11.md).

## Feature acceptance still required

The optional editor adapter now supplies a bounded asynchronous job, stable source IDs, document-content guards, preview, cancellation, accept/reject, one-step undo, save/reopen and export tests. The [editable cleanup report](../../docs/verification/EDITABLE-SKETCH-CLEANUP-2026-10-11.md) distinguishes these checks from owner-artwork and Cintiq acceptance. Continue testing corners, hatching, stylized hair, junctions and variable widths, recording rejected proposals as well as good ones. Agreement with the licensed Gurobi backend is not verified. Do not use the standalone StrokeStrip diagnostic SVG viewBox as the editor's canvas bounds; it clips the original fixture, while the grouped SCAP proposal preserves source coordinates.
