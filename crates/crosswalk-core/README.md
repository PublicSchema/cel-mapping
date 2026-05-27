# crosswalk-core

`crosswalk-core` is the Rust application facade for Crosswalk. It owns the
v0.1 mapping runtime and keeps compatibility re-exports for consumers that have
not moved to the narrower split crates yet.

## When to use it

Use this crate when you want to:

- Compile and evaluate v0.1 Crosswalk mapping YAML.
- Use a single Rust dependency that exposes mapping runtime, standalone CEL
  helpers, PublicSchema facade functions, security limits, and code systems.
- Build host bindings that need the full mapping runtime, such as Python or
  WASM wrappers.

Use `crosswalk-cel` instead when you only need standalone expression compile,
evaluate, or preview APIs. Use `crosswalk-functions` when you only need pure
helper semantics without CEL.

## Install

This package is not published yet. Use a workspace or git dependency:

```toml
[dependencies]
crosswalk-core = { git = "https://github.com/PublicSchema/cel-mapping" }
serde_json = "1"
```

Inside this repository, sibling crates use a path dependency.

## Quick start

```rust
use crosswalk_core::{EvaluationInput, MappingRuntime, RuntimeOptions};
use serde_json::json;

let yaml = r#"
version: "0.1"
name: demo
records:
  people:
    fields:
      name: "source.name"
"#;

let rt = MappingRuntime::new(RuntimeOptions::default());
let compiled = rt.compile_mapping(yaml).unwrap();
let out = rt.evaluate(
    &compiled,
    EvaluationInput {
        source: json!({ "name": "Ada" }),
        context: json!({}),
    },
);

assert!(out.errors.is_empty());
assert_eq!(out.records["people"][0]["name"], json!("Ada"));
```

## Public API surface

Primary runtime types:

- `MappingRuntime`
- `RuntimeOptions`
- `EvaluationInput`
- `MappingOutput`
- `CompiledMapping`
- `SecurityLimits`

Mapping workflow:

- `MappingRuntime::compile_mapping`
- `MappingRuntime::evaluate`
- `compile_mapping_yaml`

Standalone expression workflow, re-exported from `crosswalk-cel`:

- `evaluate_cel_expression`
- `evaluate_cel_expression_with_input`
- `preview_cel_expression`
- `preview_cel_expression_with_input`
- `StandaloneExpressionInput`
- `ExpressionPreviewResult`

PublicSchema facade:

- `compile_publicschema_mapping`
- `evaluate_publicschema_mapping`
- `preview_publicschema_rule_expression`
- `CompiledPublicSchemaMapping`
- `PublicSchemaEvaluationInput`
- `PublicSchemaTransformOutput`

Code-system facade:

- `CodeSystemRegistry`
- `CodeEntry`

## Boundaries

`crosswalk-core` should remain the compatibility and v0.1 mapping layer. New
shared expression behavior belongs in `crosswalk-cel`. New pure helper
semantics belong in `crosswalk-functions`. New CEL adapter behavior belongs in
`crosswalk-functions-cel`.

The crate intentionally re-exports selected split-crate types so existing
`crosswalk_core::*` imports keep working during the migration.

## Error model

Compilation returns `CompileError`. Mapping evaluation returns `MappingOutput`
with `warnings` and `errors` so callers can support strict, collect, and lenient
workflows. Standalone expression APIs return `StandaloneEvalError` or
`ExpressionPreviewResult`, depending on whether the caller wants fail-fast
evaluation or editor-oriented diagnostics.

## Testing

```bash
cargo test -p crosswalk-core
cargo clippy -p crosswalk-core --all-targets -- -D warnings
cargo doc -p crosswalk-core --no-deps
```

Broader workspace checks are listed in the repository
[`README.md`](../../README.md).
