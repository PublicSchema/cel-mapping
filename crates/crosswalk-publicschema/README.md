# crosswalk-publicschema

`crosswalk-publicschema` compiles and evaluates PublicSchema v0.2 property
mapping documents. It is a runtime-focused crate for PublicSchema mappings, not
the v0.1 Crosswalk records mapping format.

## When to use it

Use this crate when you want to:

- Compile a PublicSchema v0.2 mapping YAML or JSON document.
- Evaluate property mappings from source to target or target to source.
- Produce transformation logs, warnings, errors, and privacy-aware diagnostics.
- Use PublicSchema behavior without depending on the `crosswalk-core` facade.

Use `crosswalk-core` if you want one compatibility facade for both v0.1 mappings
and PublicSchema mappings.

## Install

This package is not published yet. Use a workspace or git dependency:

```toml
[dependencies]
crosswalk-publicschema = { git = "https://github.com/PublicSchema/cel-mapping" }
serde_json = "1"
```

## Quick start

```rust
use crosswalk_publicschema::{
    compile_publicschema_mapping, evaluate_publicschema_mapping,
    PublicSchemaEvaluationInput, PublicSchemaEvaluateOptions,
};
use serde_json::json;

let mapping = r#"
version: "0.2"
id: demo
property_mappings:
  - source: /person/name
    target: /name/full
"#;

let compiled = compile_publicschema_mapping(mapping, Default::default()).unwrap();
let out = evaluate_publicschema_mapping(
    &compiled,
    PublicSchemaEvaluationInput {
        source: json!({ "person": { "name": "Ada" } }),
        context: json!({}),
        options: PublicSchemaEvaluateOptions::default(),
    },
);

assert!(out.errors.is_empty());
assert_eq!(out.output, json!({ "name": { "full": "Ada" } }));
```

## Public API surface

Compile:

- `compile_publicschema_mapping`
- `CompiledPublicSchemaMapping`
- `PublicSchemaCompileOptions`
- `PublicSchemaCompileMeta`
- `PublicSchemaBindingMode`

Evaluate:

- `evaluate_publicschema_mapping`
- `PublicSchemaEvaluationInput`
- `PublicSchemaEvaluateOptions`
- `PublicSchemaDirection`
- `PrivacyMode`
- `PublicSchemaTransformOutput`
- `PublicSchemaRuleLogEntry`

Editor preview:

- `preview_publicschema_rule_expression`

## Boundaries

This crate owns PublicSchema document parsing, property mapping rules, JSON
Pointer reads and writes, value mappings, deterministic hashes, transformation
logs, and privacy-aware diagnostics.

It depends on `crosswalk-cel` for expression behavior and on
`crosswalk-functions` for code-system data. It intentionally does not depend on
`crosswalk-core` or `crosswalk-functions-cel` directly.

Important preserved behavior: direct PublicSchema evaluation does not install
helper request context or helper hot-path `FunctionSecurityLimits`. Compile-time
expression security limits still apply. If a caller needs the full core runtime
context behavior, use the `MappingRuntime` facade in `crosswalk-core`.

## Error model

Compile failures return `CompileError`. Evaluation returns
`PublicSchemaTransformOutput` with `ok`, `output`, `log`, `warnings`, and
`errors`, allowing callers to inspect partial output and rule-level status.

## Testing

```bash
cargo test -p crosswalk-publicschema
cargo clippy -p crosswalk-publicschema --all-targets -- -D warnings
cargo doc -p crosswalk-publicschema --no-deps
```
