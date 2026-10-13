# StripMaker: verified Gurobi-free native research backend

October 11, 2026. This backend report records the native research build and small-fixture checks. The subsequent optional editor integration is tracked in [editable cleanup implementation and acceptance](EDITABLE-SKETCH-CLEANUP-2026-10-11.md); the backend checks alone are not editor acceptance.

The isolated lab now executes the author's geometric features, normalization, embedded classifiers, grouping and curve/width fitting through an independent SCIP/OSQP CPU adapter. This removes the Gurobi license requirement for this tested path. It does not implement a new VectorCraft cleanup tool, establish arbitrary-sketch equivalence to Gurobi, or change the renderer. Existing author source, model weights, editor changes and drawings remain untouched.

## Backend choice and mathematical contract

| Model shape | Route | Verification before exposing a result |
| --- | --- | --- |
| Signed binary orientation, linear models, other supported nonconvex quadratics | SCIP 10.1.0 | Optimal termination; finite values; binary rounding within 1e-6; original bounds and constraints checked |
| Continuous quadratic objective with linear constraints | OSQP 1.0.0, builtin CPU algebra | Positive-semidefinite check on at most 512 active quadratic variables; `OSQP_SOLVED`, not solved-inaccurate; original bounds and constraints checked |
| Unsupported expression, mixed model references, infeasible/unbounded/limited solve | Explicit error | No solution is exposed; previous solution validity is cleared |

SCIP has a linear objective API. The adapter models a quadratic objective as `min t` subject to `f(x) <= t`; absolute-value constraints use both inequalities and a quadratic equality, not an L1-only relaxation. [SCIP FAQ](https://scipopt.org/doc/html/FAQ.php), [SCIP nonlinear API](https://scipopt.org/doc/html/cons__nonlinear_8h.php).

The first SCIP-only implementation passed 30 adapter checks and solved the fixture's orientation, but its continuous parameterization hit the 30-second limit with an unbounded dual bound. That result was rejected. It was not accepted as a fit or fixed by loosening success criteria. Adding the specialized QP route resolved the tested case.

OSQP minimizes `0.5*x'P*x + q'x` subject to `l <= A*x <= u`. The adapter doubles diagonal quadratic coefficients, supplies unchanged cross coefficients in the upper triangle, preserves linear terms, moves constraint constants into bounds, and adds variable bounds as identity rows. The omitted objective constant does not affect optimization and remains included when reporting the original objective. The author sometimes omits square-expansion constants; negative objective values alone do not indicate failure. [OSQP C interface](https://osqp.org/docs/interfaces/C.html), [solver settings](https://osqp.org/docs/interfaces/solver_settings.html).

OSQP is compiled with MSVC, double precision, int32 indices and a static library; a compile-time ABI check enforces those types. CUDA/MKL are off. QP tolerances are 1e-7 absolute/relative, with polishing, at most 20,000 iterations and at most 30 seconds per solve. Positive-semidefinite testing allows only a 1e-12-scaled roundoff allowance; failed or larger certificates retain the bounded SCIP route, which may reject the job. No universal convergence claim is made.

Both routes check original variable bounds to 1e-5 and constraint residuals to 1e-5 times the recorded expression scale. Models are capped at 20,000 variables and 100,000 constraints; SCIP additionally uses a 100,000-node and 1 GiB solver-memory limit. Test child processes have a 90-second wall-clock deadline and are terminated on timeout. OSQP is not claimed to have a separate process-memory sandbox. Untrusted drawing ingestion and stricter editor-job limits are future work.

## Pinned provenance and storage

| Component | Version / revision | Location |
| --- | --- | --- |
| StripMaker author source | `810a61a03c2177a61f8e32b8ce72c33dccfac49f` | `D:\DevData\Rainee\research\StripMaker` |
| SCIP official Windows SDK | 10.1.0 | `D:\DevData\Rainee\tools\scip-10.1.0\scipoptsuite-10.1.0-win-x64` |
| OSQP author source | v1.0.0, `236713ce9a56c182ac3230d52108f952afce1523` | `D:\DevData\Rainee\research\osqp-1.0.0` |
| QDLDL | v0.1.8, `138fdac58b9cd1c4137ff1b99152c8108a6cff5b` | `D:\DevData\Rainee\cache\stripmaker-cmake\osqp\qdldl-src` |
| OSQP MSVC install | Static builtin CPU algebra | `D:\DevData\Rainee\tools\osqp-msvc-1.0.0` |

The [official SCIP archive](https://github.com/scipopt/scip/releases/download/v10.1.0/scipoptsuite-10.1.0-win-x64.zip) SHA-256 matches its GitHub release digest: `E71BAFB8CE3C26386AE0D93CDD51867F21EB74B70986BBD696B7B864A3C355FF`. The build verifies it before extraction. [Pinned OSQP source](https://github.com/osqp/osqp/tree/236713ce9a56c182ac3230d52108f952afce1523), [QDLDL source](https://github.com/osqp/qdldl/tree/138fdac58b9cd1c4137ff1b99152c8108a6cff5b).

The downloaded OSQP Windows release archive was inspected but is **not used by this build**: it supplies a MinGW import library and lacks the generated configuration header required by the C API. Building the pinned source with MSVC avoids guessing its binary types. No downloaded font, artwork/training dataset, service, global PATH setting, CUDA/OpenCL component or GPU renderer was added.

All three source checkouts were verified clean after building/testing. Wrapper CMake substitutes only the narrow independent adapter and research `Context` policy, and wraps the three author mains with exception-to-error reporting. The author feature definitions, normalization, forest code and fitting mathematics are unchanged. Gurobi-specific tuning/retry parameters are not falsely mapped onto other solvers. The editor's Rust/WASM dependency graph is unchanged.

The bootstrap Python environment contains setuptools for legacy pybind11 configuration; Python bindings and training are not built. It lives at `D:\DevData\Rainee\venvs\stripmaker-build`. Builds, dependency caches and process TEMP/TMP remain on D. The SDK DLL search path is set only for build/test processes.

## Executed checks

Final normal run: [results.json](D:/DevData/Rainee/build/stripmaker-scip/checks-def76ce2a2574676b157386d30cf9898/results.json). It records input/executable hashes and individual process durations. The build log is [build-final.log](D:/DevData/Rainee/build/stripmaker-scip/build-final.log).

| Check | Result | Single-run process duration |
| --- | --- | ---: |
| Adapter mathematics/errors | 34 passed | 0.380 s |
| Annotated overlapping bundle | One fitted curve, 77 samples; width fitting output also produced | 0.096 s |
| Native learned local grouping: bundle | Three originals -> one group and one fitted proposal | 0.189 s |
| Native learned local grouping: separated strokes | Three originals -> three groups and three fitted proposals | 0.057 s |

The adapter checks include 20 signed six-variable binary objectives compared with exhaustive enumeration, known constrained QPs, re-solving with stale-result rejection, matrix/cross-term coefficients, semidefinite QP plus L1 variables, exact absolute value, nonconvex fallback, infeasibility, unboundedness, zero deadline, invalid bounds, nonfinite coefficients, mixed models and degree overflow.

The bundle includes one reversed stroke. Every fitted sample is at most **0.946 fixture units** from the central reference polyline, below the fixed two-unit test threshold. This is a directed geometric check, not a proof of corner or endpoint fidelity. The separation fixture includes a deliberate end gap and a distant parallel stroke. Grouped original IDs, all points and timestamps match the input numerically within 1e-9; original counts remain three. Fitted proposal counts match proposed groups. SVG path commands, finite geometry and width output are checked, not merely file existence.

Debug run [output directory](D:/DevData/Rainee/build/stripmaker-scip/checks-a4ea46caea21462abe6a4de9c64d6b37/results.json) confirms SCIP handles binary orientation and OSQP handles parameterization/width QPs. The initial 228-variable QP terminated solved in 300 iterations, with primal residual approximately 3.23e-6 and dual residual 5.49e-8, then passed the original-constraint checks. Small-fixture durations are not sustained editor latency or GPU performance measurements.

Local Release executable SHA-256 values, after the final rebuild:

| Artifact | SHA-256 |
| --- | --- |
| `clustering-solve.exe` | `F3029840132FFE65D73AB2D800947BA6F6C2227246DBFE8966323D569BF8C053` |
| `clustering-endend.exe` | `EC16243A2560115B9413AA9A7620EC84C1E2DC0BCB60E349A34F41E25A2EC1AF` |
| `strokestrip.exe` | `E308AA462F1E22CE440425BD7F3A13AE5862E83A1A8D3F5CFB3A10288A64A76E` |
| `scip-adapter-tests.exe` | `7B5B4AB4E8AD446CD92AC74161A0451C078EBC0B2A156F375D41427D838D60A0` |

The three worker CLIs are in `D:\DevData\Rainee\build\stripmaker-scip\author\Release`; tests are in the parent `Release` directory. PE dependency inspection shows SCIP/TBB and Windows/MSVC runtimes, not a Gurobi DLL. OSQP is statically linked. Hashes identify these local binaries; byte-identical rebuilds across environments are not claimed.

## Reproduce

From `D:\0\Rainee drawing app\ArtCraft-Foundation\VectorCraft - Gemini Edition`:

```powershell
.\integrations\stripmaker-lab\Build-SCIP.ps1
.\integrations\stripmaker-lab\Test-SCIP.ps1
```

MSVC 2022 x64, CMake, Git and the configured D-drive Rust/uv environment are required. A fresh dependency fetch needs network access. The build checks pinned clean author/OSQP/QDLDL sources, verifies the SCIP archive, builds OSQP and the narrow native adapter, and runs the mathematical checks. Tests run bounded child jobs with process-local DLL paths and create new output directories; existing results are not overwritten.

Implementation: [wrapper/build script](../../integrations/stripmaker-lab/Build-SCIP.ps1), [fixture runner](../../integrations/stripmaker-lab/Test-SCIP.ps1), [adapter](../../integrations/stripmaker-lab/scip/backend.cpp), [tests](../../integrations/stripmaker-lab/scip/tests.cpp), [lab README](../../integrations/stripmaker-lab/README.md).

## Next milestone and unverified gates

The next milestone is **selected stroke-bundle cleanup preview in VectorCraft**, using the verified geometry/feature behavior as a research oracle for the Rust editor implementation. Capture selected stable IDs and document revision, preserve originals, process asynchronously, and show original/proposed groups and editable replacement curves. Accept is one undoable command; reject and cancellation change nothing. UI and MCP use the same discoverable command and unavailable/error states. Test stale revisions, manual edits during processing, undo/redo, save/reopen and export on owner artwork.

Not yet verified: owner-drawing quality, general solver parity with Gurobi, contextual split/merge or endpoint/junction behavior, broad variable-width quality, editor worker packaging, Cintiq interaction with this new tool, and UI/MCP/persistence acceptance. `clustering-endend` is compiled only. The existing Rust forest parity checks remain separate inference evidence; native fixture success does not mean the Rust feature pipeline has been ported.

The standalone StrokeStrip diagnostic SVG's viewBox clips the test curve, and its SCAP canvas header does not retain the original canvas size. Do not use these diagnostics as editor document bounds. `clustering-solve` produces proposals in the original canvas coordinates and dimensions; coordinate/object correspondence still requires editor tests. Width-envelope SVG output does not automatically become editable pressure/nib metadata in VectorCraft.

SCIP and OSQP need no Gurobi license. Release packaging still needs notices/dependency review: OSQP/QDLDL are Apache-2.0, its bundled AMD is BSD-3-Clause, and the SCIP binary suite and author pipeline contain additional dependencies. Existing Triangle restrictions are not removed by swapping the solver. This is a private research route, not a declaration that the whole package is cleared for redistribution. [OSQP license](https://github.com/osqp/osqp/blob/v1.0.0/LICENSE), [QDLDL license](https://github.com/osqp/qdldl/blob/v0.1.8/LICENSE).

GPU companion, Vello changes, Smart Inker, animation and 3D Stage remain outside this task.
