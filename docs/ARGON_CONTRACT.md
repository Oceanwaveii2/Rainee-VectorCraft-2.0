# VectorCraft 2.0 / Rainee Pairing Contract for Argon

> **Purpose:** This document specifies the architectural constraints, lint requirements, and communication protocol for collaborator agents (Argon) writing code for the VectorCraft 2.0 / Rainee engine.

---

## 1. Rust Compiler & Lint Contract (Strict)

All code contributed to VectorCraft must satisfy these invariants:

1. **Forbid Unsafe Code:**
   * Root `Cargo.toml` enforces `#![forbid(unsafe_code)]`.
   * **Do NOT use `bytemuck::Pod` or `bytemuck::Zeroable`** (their derive macros expand to `unsafe impl`).
   * **Use safe little-endian serializers** (e.g., `to_wgsl_bytes()` or `to_le_bytes()`) into `Vec<u8>`.
   * No `std::mem::transmute`, raw pointers, or `unsafe` blocks under any circumstances.

2. **Zero-Panic Clippy Policy:**
   * The workspace enforces `unwrap_used = "deny"`, `expect_used = "deny"`, and `panic = "deny"`.
   * **Do NOT call `.unwrap()` or `.expect()`** in production paths.
   * **Do NOT use direct indexing** (e.g., `map[&k]` or `slice[i]`). Use `.get()`, `.get_mut()`, and pattern matching or `if let Some(...)`.
   * Propagate errors via `Result<T, E>` and `?`, or fall back to safe defaults via `.unwrap_or(...)` / `.unwrap_or_default()`.

3. **Core Dependencies & Versions:**
   * `egui` = `0.36.2`
   * `vello_cpu` = `0.2.0`
   * `serde_json` = `1.0`
   * `kurbo` = `0.13.1`

4. **Module Wiring & Non-Destructive Extension:**
   * Prefer adding isolated extension modules (e.g., `super_studio_ext.rs`, `km_optics.rs`) rather than making sweeping edits to existing core UI files.
   * Explicitly specify the `pub mod <name>;` wiring for `lib.rs` or `mod.rs`.

---

## 2. Preferred Delivery Format (Unified Git Patch)

Because Argon's cloud sandbox cannot directly push to GitHub over TCP, delivery must be cleanly readable by the local agent.

**Format Choice A: Unified Git Patch (Preferred)**
```patch
diff --git a/crates/render/src/example.rs b/crates/render/src/example.rs
new file mode 100644
--- /dev/null
+++ b/crates/render/src/example.rs
@@ -0,0 +1,20 @@
+//! Example enhancement
+#![forbid(unsafe_code)]
+...
```

**Format Choice B: Drop-in File Block**
If outputting complete new files, prefix each file clearly:
```text
### FILE: crates/render/src/my_module.rs
<code>
### EXPORT: crates/render/src/lib.rs
pub mod my_module;
```

---

## 3. Remote Verification Feedback Loop

When code is submitted:
1. The local agent applies the patch and runs `cargo check --workspace`.
2. The local agent pushes to GitHub `main` where GitHub Actions CI tests it on a clean Windows runner.
3. If any compiler error occurs, the local agent will return the exact 5–10 line diagnostic:
   ```text
   error[E0308]: mismatched types
     --> crates/render/src/my_module.rs:42:15
     |
   42 |     let x: f32 = val;
     |                  ^^^ expected `f32`, found `f64`
   ```
4. Argon only needs to output the targeted fix for that diagnostic rather than regenerating entire files.
