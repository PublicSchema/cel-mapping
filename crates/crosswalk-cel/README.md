# crosswalk-cel

`crosswalk-cel` is the standalone CEL boundary for Crosswalk. It compiles,
evaluates, and previews expressions with Crosswalk helper functions registered,
while keeping upstream CEL engine internals private.

## When to use it

Use this crate when you want to:

- Evaluate a single CEL expression against JSON root bindings.
- Compile an expression once and evaluate it many times.
- Build editor previews with syntax, rewrite, and evaluation diagnostics.
- Share expression behavior between runtimes without pulling in v0.1 mapping
  YAML or PublicSchema parsing.

Use `crosswalk-core` if you need the full v0.1 mapping runtime. Use
`crosswalk-publicschema` if your input is a PublicSchema v0.2 mapping document.

## Install

This package is not published yet. Use a workspace or git dependency:

```toml
[dependencies]
crosswalk-cel = { git = "https://github.com/PublicSchema/crosswalk" }
serde_json = "1"
```

## Quick start

```rust
use crosswalk_cel::{
    evaluate_cel_expression_with_input, SecurityLimits, StandaloneExpressionInput,
};
use serde_json::json;
use std::sync::Arc;

let input = StandaloneExpressionInput::new([
    ("source".to_string(), json!({ "count": 2 })),
    ("ctx".to_string(), json!({})),
]);

let value = evaluate_cel_expression_with_input(
    "source.count + 1",
    input,
    &SecurityLimits::default(),
    Arc::new(Default::default()),
)
.unwrap();

assert_eq!(value, json!(3));
```

## Public API surface

Compile and evaluate:

- `compile_expr`
- `CompiledCel`
- `evaluate_cel_expression`
- `evaluate_cel_expression_with_input`
- `evaluate_compiled_expression_with_input`
- `StandaloneExpressionInput`

Editor preview:

- `preview_cel_expression`
- `preview_cel_expression_with_input`
- `ExpressionPreviewResult`
- `ExpressionIssue`
- `ExpressionPhase`

Security and diagnostics:

- `SecurityLimits`
- `CompileError`
- `StandaloneEvalError`
- `MappingError`
- `ErrorCode`
- `ErrorSeverity`

Path and output utilities are public for compatibility with `crosswalk-core`:
`expr`, `missing`, `output`, and `paths`.

## Boundaries

This crate owns expression-level behavior: security limits, diagnostics,
missing-aware path injection, root binding validation, JSON conversion, and CEL
helper registration.

It intentionally does not expose `cel::Value`, `cel::Program`, or
`cel::ExecutionError` as public boundary types. Public callers pass JSON root
bindings and receive JSON values or Crosswalk error types.

It also intentionally does not parse v0.1 mapping YAML or PublicSchema mapping
documents. Those belong in `crosswalk-core` and `crosswalk-publicschema`.

## Error model

Compile-time failures use `CompileError`. Fail-fast evaluation uses
`StandaloneEvalError`. Preview APIs return `ExpressionPreviewResult` and include
issues instead of throwing for syntax and evaluation problems.

## Testing

```bash
cargo test -p crosswalk-cel
cargo clippy -p crosswalk-cel --all-targets -- -D warnings
cargo doc -p crosswalk-cel --no-deps
```
