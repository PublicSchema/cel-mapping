# crosswalk-wasm

`crosswalk-wasm` exposes the Crosswalk runtime through `wasm-bindgen`. It is
the Rust crate that produces the generated WebAssembly package consumed by the
TypeScript wrapper in `packages/js`.

## When to use it

Use this crate when you are maintaining the WASM boundary itself. Application
TypeScript should usually import `crosswalk-js` from `packages/js`, which wraps
the raw JSON-string methods in object-oriented helpers.

## Install

This crate is built with `wasm-pack` from the JavaScript package:

```bash
cd packages/js
npm ci
npm test
```

The underlying command is:

```bash
wasm-pack build ../../crates/crosswalk-wasm --target web --out-dir ../../packages/js/wasm-pkg
```

## Public API surface

The exported class is `WasmMappingRuntime`.

Runtime configuration:

- `set_limits_json`
- `set_runtime_options_json`

v0.1 mapping APIs:

- `compile_mapping_meta`
- `evaluate_json`

PublicSchema APIs:

- `compile_publicschema_mapping_meta`
- `evaluate_publicschema_json`
- `preview_publicschema_rule_expression_json`
- `get_publicschema_helper_metadata`

Standalone expression APIs:

- `evaluate_expression_json`
- `preview_expression_json`

All methods accept and return strings because this is the raw `wasm-bindgen`
boundary. The high-level TypeScript wrapper parses these into typed objects.

## Boundaries

This crate should stay thin. It should translate strings to Rust values, call
`crosswalk-core`, and serialize results back to JSON. Runtime semantics belong
in Rust crates below it, and idiomatic TypeScript ergonomics belong in
`packages/js`.

## Error model

The raw WASM boundary serializes success and failure into JSON strings rather
than throwing rich Rust errors. The TypeScript wrapper normalizes those shapes
for application code.

## Testing

```bash
cargo test -p crosswalk-wasm
cargo clippy -p crosswalk-wasm --all-targets -- -D warnings
cd packages/js && npm test
```
