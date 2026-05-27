# crosswalk

Deterministic **CEL** mapping runtime: YAML mapping specs → compiled expressions → JSON in/out. Rust core with optional **WASM** and **Python** bindings.

Full behaviour and vocabulary are defined in **[`spec.md`](./spec.md)** (v0.1). This README is the **implementation** entry point: layout, commands, and bindings.

For the proposed PublicSchema-native runtime refactor, see **[`spec-publicschema-v0.2.md`](./spec-publicschema-v0.2.md)** and the implementation plan in **[`implementation-plan-publicschema-v0.2.md`](./implementation-plan-publicschema-v0.2.md)**.

For the proposed crate-boundary refactor and reusable helper-function crates, see **[`spec-crate-split-v0.3.md`](./spec-crate-split-v0.3.md)**.

Phase 0 gate documents: **[`docs/publicschema-helper-parity.md`](./docs/publicschema-helper-parity.md)** (celext v1 helper inventory and Rust porting status) and **[`docs/publicschema-celpy-behavioral-diff.md`](./docs/publicschema-celpy-behavioral-diff.md)** (celpy behavioral diff template, to be filled before Phase 4).

## Which package should I use?

Most Rust application code should start with **`crosswalk-core`**. Use the
lower-level crates only when you need their narrower boundary.

| Need | Use |
|------|-----|
| Compile and evaluate v0.1 mapping YAML in Rust | [`crosswalk-core`](./crates/crosswalk-core/README.md) |
| Evaluate or preview standalone CEL expressions | [`crosswalk-cel`](./crates/crosswalk-cel/README.md) |
| Call deterministic helper functions without CEL | [`crosswalk-functions`](./crates/crosswalk-functions/README.md) |
| Register helper functions into a CEL context | [`crosswalk-functions-cel`](./crates/crosswalk-functions-cel/README.md) |
| Compile and evaluate PublicSchema v0.2 mappings | [`crosswalk-publicschema`](./crates/crosswalk-publicschema/README.md) or the facade in `crosswalk-core` |
| Use Crosswalk from Python | [`crosswalk`](./crates/crosswalk-python/README.md) |
| Use Crosswalk from browser or TypeScript | [`crosswalk-js`](./packages/js/README.md) backed by [`crosswalk-wasm`](./crates/crosswalk-wasm/README.md) |

## Documentation map

| Document | Purpose |
|----------|---------|
| [`docs/architecture.md`](./docs/architecture.md) | Crate ownership, dependency direction, and boundary rules |
| [`docs/extension-guide.md`](./docs/extension-guide.md) | How to add helpers, CEL behavior, PublicSchema behavior, or bindings without breaking boundaries |
| [`docs/release-checklist.md`](./docs/release-checklist.md) | Pre-release checks for Rust crates, Python wheels, and JS/WASM package output |
| [`docs/crate-split-inventory.md`](./docs/crate-split-inventory.md) | v0.3 split inventory, preserved pitfalls, and public re-export paths |

## Layout

| Path | Role |
|------|------|
| [`crates/crosswalk-functions`](./crates/crosswalk-functions/README.md) | Pure deterministic helper functions, typed code-system registry, and ISO preload data with no CEL dependency |
| [`crates/crosswalk-functions-cel`](./crates/crosswalk-functions-cel/README.md) | CEL adapter: helper registration, CEL value coercion, helper metadata, and request fallback resolution |
| [`crates/crosswalk-cel`](./crates/crosswalk-cel/README.md) | Standalone CEL compile/evaluate/preview boundary, expression diagnostics, security limits, and helper registration |
| [`crates/crosswalk-publicschema`](./crates/crosswalk-publicschema/README.md) | PublicSchema v0.2 property-mapping compile/evaluate runtime, JSON Pointer writes, hashes, logs, and privacy diagnostics |
| [`crates/crosswalk-core`](./crates/crosswalk-core/README.md) | Compatibility facade and v0.1 records mapping runtime |
| [`crates/crosswalk-wasm`](./crates/crosswalk-wasm/README.md) | `wasm-bindgen` wrapper (JSON string API) |
| [`crates/crosswalk-python`](./crates/crosswalk-python/README.md) | PyO3 extension `crosswalk` + [`examples/`](./crates/crosswalk-python/examples/) |
| [`packages/js`](./packages/js) | TypeScript + `wasm-pack` script targeting `wasm-pkg/` |
| [`.github/workflows/ci.yml`](./.github/workflows/ci.yml) | `fmt` / `clippy` / tests, WASM build + TS, `maturin` + `pytest` + examples |

Workspace `default-members` include the reusable split crates, core, and WASM so `cargo test` exercises the Rust runtime without requiring Python dev libraries. The Python crate remains a workspace member for `cargo test -p crosswalk-python` (cdylib-only; use **`pytest`** after `maturin develop`).

Compatibility import paths remain available through **`crosswalk_core`** during the migration. New Rust consumers can import pure helpers from **`crosswalk_functions`**, standalone expression APIs from **`crosswalk_cel`**, PublicSchema runtime types from **`crosswalk_publicschema`**, and CEL helper registration/metadata from **`crosswalk_functions_cel`**.

## Workspace verification

```bash
cargo fmt --all -- --check
cargo clippy -p crosswalk-core -p crosswalk-wasm --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Python and JS/WASM have additional package-level checks documented in
[`crates/crosswalk-python/README.md`](./crates/crosswalk-python/README.md) and
[`packages/js/README.md`](./packages/js/README.md).

## Quick start (Rust)

```bash
cargo test -p crosswalk-core -p crosswalk-wasm
cargo doc -p crosswalk-core --open
```

Important types: **`MappingRuntime`**, **`CompiledMapping`**, **`EvaluationInput`**, **`MappingOutput`**, **`ExpressionPreviewResult`** (see `crosswalk_core::errors` and `preview_cel_expression`).

### Compile once, evaluate many

```rust
let rt = MappingRuntime::new(RuntimeOptions::default());
let compiled = rt.compile_mapping(yaml)?;
let out = rt.evaluate(&compiled, EvaluationInput { source, context });
```

### Standalone expression (no mapping YAML)

- **`evaluate_cel_expression`** / **`MappingRuntime::evaluate_cel_expression`**: same bindings as a field expression (`source`, `ctx`, stdlib); takes **`EvaluationInput`**; returns `Result<JsonValue, StandaloneEvalError>`.
- **`preview_cel_expression`** / **`MappingRuntime::preview_cel_expression`**: **editor-oriented**: same `expr` + **`EvaluationInput`**; always returns **`ExpressionPreviewResult`** with `author_expression`, optional `rewritten_expression`, `value`, `issues` (syntax uses CEL’s full diagnostic + line/column in the **rewritten** buffer), and `notes` for tools/LLMs.

The crate-root free functions **`crosswalk_core::evaluator::evaluate_cel_expression`** / **`preview_cel_expression`** take raw `source` / `ctx` JSON plus `SecurityLimits` and a code-system registry for lower-level use.

## Python

See **[`crates/crosswalk-python/README.md`](./crates/crosswalk-python/README.md)** for `maturin develop`, `pytest`, and runnable **`examples/*.py`**.

Summary:

- **`MappingRuntime`**, **`CompiledMapping`**, **`MappingCompileError`**
- Dict-first API: **`evaluate`**, **`evaluate_compiled`**, **`preview_expression`**, **`evaluate_expression`**
- JSON-string helpers retained for interop: **`evaluate_json`**, **`set_limits_json`**, etc.

## JavaScript / WASM

Requires [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and the **`wasm32-unknown-unknown`** target. From **`packages/js`** (so `--out-dir` resolves next to this package):

```bash
cd packages/js && npm ci && npm test
```

The **`test`** script builds WASM, builds TypeScript, and runs Node smoke tests
against the generated package. The **`build:wasm`** script runs `wasm-pack`
with the correct `--out-dir` for this repo layout and prefers rustup-managed
Rust when it is installed.

For **idiomatic TypeScript** (`Crosswalk`, camelCase, object I/O) and **copy-paste examples**, see **[`packages/js/README.md`](./packages/js/README.md)**. The WASM class exposes JSON-oriented methods such as **`evaluate_json`**, **`compile_mapping_meta`**, **`set_limits_json`**, **`set_runtime_options_json`**, **`evaluate_expression_json`** (single CEL expression, `{"ok":true,"value":…}` / `{"ok":false,"error":…}`), and **`preview_expression_json`** (editor-oriented `ExpressionPreviewResult` JSON). See `crates/crosswalk-wasm/src/lib.rs` and `packages/js/src/index.ts` for helpers.

## Runtime options vs mapping YAML

Mapping document **`errors.mode`** (`strict` / `collect` / `lenient`) wins when set. If it is omitted, **`RuntimeOptions::default_errors_mode`** applies (see `MappingRuntime::compile_mapping`).

## Licence

See workspace `Cargo.toml` (`license` field).
