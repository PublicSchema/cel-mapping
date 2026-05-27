# crosswalk-functions-cel

`crosswalk-functions-cel` adapts the pure helpers from `crosswalk-functions`
into CEL-visible functions. It owns CEL registration, CEL value conversion,
arity checks, request fallback resolution, and helper hot-path security limits.

## When to use it

Use this crate when you are building a CEL `Context` and need Crosswalk helper
functions registered in it.

Most application code should not depend on this crate directly. Use
`crosswalk-cel` for standalone expression evaluation or `crosswalk-core` for
full mapping evaluation.

## Install

This package is not published yet. Use a workspace or git dependency:

```toml
[dependencies]
crosswalk-functions-cel = { git = "https://github.com/PublicSchema/cel-mapping" }
```

## Quick start

```rust
use crosswalk_functions_cel::{helper_metadata, HelperArity};

let helpers = helper_metadata();
assert!(helpers.iter().any(|h| h.name == "text_lower"));
assert!(helpers.iter().any(|h| matches!(h.arity, HelperArity::Exact(_))));
```

Direct CEL context registration is intentionally lower-level:

```rust
use cel::Context;
use crosswalk_functions_cel::{register_stdlib, FunctionRegistry};
use std::sync::Arc;

let mut ctx = Context::default();
register_stdlib(&mut ctx, Arc::new(FunctionRegistry::new()));
```

## Public API surface

Registration and metadata:

- `register_stdlib`
- `register_crosswalk_functions`
- `helper_metadata`
- `HelperMetadata`
- `HelperArity`

Context and security:

- `FunctionRequestContext`
- `set_eval_ctx`
- `clear_eval_ctx`
- `take_warnings`
- `FunctionSecurityLimits`
- `BudgetGuard`

Compatibility exports:

- `crosswalk_functions`
- `FunctionRegistry`

## Boundaries

This crate owns CEL adapter semantics only. Pure helper behavior belongs in
`crosswalk-functions`. Full expression compile/evaluate/preview behavior belongs
in `crosswalk-cel`.

The adapter preserves CEL-visible compatibility behavior, including:

- Missing/null coercion for helper inputs.
- Runtime fallback for country, timezone, and today when context is installed.
- Empty-string result for `text_regex_extract` when a match or capture is absent.
- Warning collection through helper context.

## Error model

CEL helper failures are returned through CEL execution errors because this crate
is inside the CEL adapter layer. Public standalone APIs should wrap those errors
in `crosswalk-cel` types before crossing crate boundaries.

## Testing

```bash
cargo test -p crosswalk-functions-cel
cargo clippy -p crosswalk-functions-cel --all-targets -- -D warnings
cargo doc -p crosswalk-functions-cel --no-deps
```
