# cel-mapping

Deterministic **CEL** mapping runtime: YAML mapping specs → compiled expressions → JSON in/out. Rust core with optional **WASM** and **Python** bindings.

Full behaviour and vocabulary are defined in **[`spec.md`](./spec.md)** (v0.1). This README is the **implementation** entry point: layout, commands, and bindings.

For the proposed PublicSchema-native runtime refactor, see **[`spec-publicschema-v0.2.md`](./spec-publicschema-v0.2.md)** and the implementation plan in **[`implementation-plan-publicschema-v0.2.md`](./implementation-plan-publicschema-v0.2.md)**.

Phase 0 gate documents: **[`docs/publicschema-helper-parity.md`](./docs/publicschema-helper-parity.md)** (celext v1 helper inventory and Rust porting status) and **[`docs/publicschema-celpy-behavioral-diff.md`](./docs/publicschema-celpy-behavioral-diff.md)** (celpy behavioral diff template, to be filled before Phase 4).

## Layout

| Path | Role |
|------|------|
| [`crates/cel-mapper-core`](./crates/cel-mapper-core) | Rust library: compile mapping YAML, evaluate, stdlib, limits, expression preview |
| [`crates/cel-mapper-wasm`](./crates/cel-mapper-wasm) | `wasm-bindgen` wrapper (JSON string API) |
| [`crates/cel-mapper-python`](./crates/cel-mapper-python) | PyO3 extension `cel_mapper` + [`examples/`](./crates/cel-mapper-python/examples/) |
| [`packages/js`](./packages/js) | TypeScript + `wasm-pack` script targeting `wasm-pkg/` |
| [`.github/workflows/ci.yml`](./.github/workflows/ci.yml) | `fmt` / `clippy` / tests, WASM build + TS, `maturin` + `pytest` + examples |

Workspace `default-members` are **core + wasm** so `cargo test` does not require Python dev libraries; the Python crate remains a workspace member for `cargo test -p cel-mapper-python` (cdylib-only; use **`pytest`** after `maturin develop`).

## Quick start (Rust)

```bash
cargo test -p cel-mapper-core -p cel-mapper-wasm
cargo doc -p cel-mapper-core --open
```

Important types: **`MappingRuntime`**, **`CompiledMapping`**, **`EvaluationInput`**, **`MappingOutput`**, **`ExpressionPreviewResult`** (see `cel_mapper_core::errors` and `preview_cel_expression`).

### Compile once, evaluate many

```rust
let rt = MappingRuntime::new(RuntimeOptions::default());
let compiled = rt.compile_mapping(yaml)?;
let out = rt.evaluate(&compiled, EvaluationInput { source, context });
```

### Standalone expression (no mapping YAML)

- **`evaluate_cel_expression`** / **`MappingRuntime::evaluate_cel_expression`** — same bindings as a field expression (`source`, `ctx`, stdlib); takes **`EvaluationInput`**; returns `Result<JsonValue, StandaloneEvalError>`.
- **`preview_cel_expression`** / **`MappingRuntime::preview_cel_expression`** — **editor-oriented**: same `expr` + **`EvaluationInput`**; always returns **`ExpressionPreviewResult`** with `author_expression`, optional `rewritten_expression`, `value`, `issues` (syntax uses CEL’s full diagnostic + line/column in the **rewritten** buffer), and `notes` for tools/LLMs.

The crate-root free functions **`cel_mapper_core::evaluator::evaluate_cel_expression`** / **`preview_cel_expression`** take raw `source` / `ctx` JSON plus `SecurityLimits` and a code-system registry for lower-level use.

## Python

See **[`crates/cel-mapper-python/README.md`](./crates/cel-mapper-python/README.md)** for `maturin develop`, `pytest`, and runnable **`examples/*.py`**.

Summary:

- **`MappingRuntime`**, **`CompiledMapping`**, **`MappingCompileError`**
- Dict-first API: **`evaluate`**, **`evaluate_compiled`**, **`preview_expression`**, **`evaluate_expression`**
- JSON-string helpers retained for interop: **`evaluate_json`**, **`set_limits_json`**, etc.

## JavaScript / WASM

Requires [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and the **`wasm32-unknown-unknown`** target. From **`packages/js`** (so `--out-dir` resolves next to this package):

```bash
cd packages/js && npm ci && npm run build:wasm && npm run build:ts
```

The **`build:wasm`** script runs `wasm-pack` with the correct `--out-dir` for this repo layout.

For **idiomatic TypeScript** (`CelMapper`, camelCase, object I/O) and **copy-paste examples**, see **[`packages/js/README.md`](./packages/js/README.md)**. The WASM class exposes JSON-oriented methods such as **`evaluate_json`**, **`compile_mapping_meta`**, **`set_limits_json`**, **`set_runtime_options_json`**, **`evaluate_expression_json`** (single CEL expression, `{"ok":true,"value":…}` / `{"ok":false,"error":…}`), and **`preview_expression_json`** (editor-oriented `ExpressionPreviewResult` JSON). See `crates/cel-mapper-wasm/src/lib.rs` and `packages/js/src/index.ts` for helpers.

## Runtime options vs mapping YAML

Mapping document **`errors.mode`** (`strict` / `collect` / `lenient`) wins when set. If it is omitted, **`RuntimeOptions::default_errors_mode`** applies (see `MappingRuntime::compile_mapping`).

## Licence

See workspace `Cargo.toml` (`license` field).
