# Repository Overview

## 1. High-Level Purpose
- Rhai-powered Greentic component implementing the standard `greentic:component/node@0.4.0` interface.
- Executes user-provided Rhai scripts against a generic invocation envelope (config, msg, payload, state, connections), persists state mutations, and returns a structured component result payload/control/error as JSON.

## 2. Main Components and Functionality
- **Path:** src/lib.rs  
  - **Role:** Component entrypoint wiring guest exports and high-level invocation handling.  
  - **Key functionality:** Exposes manifest, lifecycle hooks, and invoke/invoke-stream implementations; parses invocation JSON, normalizes config/payload/state, executes scripts via `script_engine`, computes state updates (object-level diffs), builds `ComponentResult`, and maps errors to `NodeError` for the guest ABI.  
  - **Key dependencies / integration points:** Uses `greentic-interfaces-guest` (wasm32) for exports, and `greentic-types::ChannelMessageEnvelope` for incoming messages.
- **Path:** src/model.rs  
  - **Role:** Shared data models for config, invocation envelope, control/error/result, and __greentic directive parsing.  
  - **Key functionality:** Defines `ScriptConfig` (script/result_mode/on_error), `InvocationEnvelope` normalization, component control/error/result shapes, and helpers to extract routing directives.
- **Path:** src/script_engine.rs  
  - **Role:** Rhai runtime glue.  
  - **Key functionality:** Converts JSON ↔ Rhai Dynamic, injects `msg`, `payload`, `state`, `connections` into scope, evaluates scripts, captures final state even on errors, extracts __greentic directives, and reports conversion/runtime failures.
- **Path:** src/bin/hash_wasm.rs  
  - **Role:** Utility binary to compute a blake3 hash for the built wasm artifact.  
  - **Key functionality:** Reads a file path argument and prints `blake3:<hex>`; used to update `component.manifest.json`.
- **Path:** schemas/  
  - **Role:** JSON Schemas for component configuration and invocation I/O.  
  - **Key functionality:** `component.schema.json` now requires `script` with optional `result_mode`/`on_error`; `io/input.schema.json` documents invocation envelope fields; `io/output.schema.json` describes `ComponentResult` payload/state_updates/control/error.
- **Path:** component.manifest.json  
  - **Role:** Greentic component manifest describing identity, world, supported profiles, capabilities, limits, artifact path, and placeholder hash for the built wasm.  
  - **Key dependencies / integration points:** References built artifact at `target/wasm32-wasip2/release/component_script_rhai.wasm`, uses the describe export `get-manifest`, and now includes a computed blake3 hash.
- **Path:** tests/ and src/lib.rs tests  
  - **Role:** Conformance and unit tests validating manifest content and scripting behavior.  
  - **Key functionality:** Cover wrapped outputs, __greentic routing, state persistence on script error, serialization fallback, and manifest world/name.

## 3. Work In Progress, TODOs, and Stubs
- None currently noted in code comments; scripts already execute against the full envelope. Future enhancements may expand error/control semantics or diff-based state updates.

## 4. Broken, Failing, or Conflicting Areas
- None observed. `ci/local_check.sh` (fmt, clippy, test) passes and wasm release build completed.

## 5. Notes for Future Work
- Consider more sophisticated state diffing/patch semantics if the host prefers JSON Patch or merge semantics beyond simple object diffs.
- Extend error/control semantics to honor additional host routing conventions if introduced in newer Greentic interfaces.
- If the invocation/result contract stabilizes in greentic-interfaces, align envelope/result structs directly with provided types rather than ad-hoc JSON parsing.
