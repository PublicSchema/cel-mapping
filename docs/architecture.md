# Crosswalk Crate Architecture

This repository is organized as a small crate family. The split is intended to
make ownership visible, keep helper semantics reusable, and prevent binding
packages from depending on implementation details.

## Dependency direction

The intended direction is:

```text
crosswalk-functions
crosswalk-functions -> crosswalk-functions-cel
crosswalk-functions + crosswalk-functions-cel -> crosswalk-cel
crosswalk-cel + crosswalk-functions -> crosswalk-publicschema
crosswalk-cel + crosswalk-functions + crosswalk-publicschema -> crosswalk-core
crosswalk-core -> crosswalk-python
crosswalk-core -> crosswalk-wasm -> packages/js
```

Lower crates must not depend on higher crates. In practice:

- `crosswalk-functions` has no CEL, YAML, Python, or WASM dependency.
- `crosswalk-functions-cel` depends on CEL and pure helpers, but not on
  `crosswalk-core`.
- `crosswalk-cel` owns standalone expression behavior and keeps upstream CEL
  engine internals behind the crate boundary.
- `crosswalk-publicschema` owns PublicSchema mapping semantics and calls
  `crosswalk-cel`.
- `crosswalk-core` is the compatibility facade and v0.1 mapping runtime.
- `crosswalk-python` and `crosswalk-wasm` are binding layers over
  `crosswalk-core`.

## Crate responsibilities

| Crate | Owns | Must not own |
|-------|------|--------------|
| `crosswalk-functions` | Pure deterministic helper semantics and code systems | CEL adapter behavior, YAML parsing, mapping runtime behavior |
| `crosswalk-functions-cel` | CEL helper registration, value conversion, missing/null compatibility, request fallback context, helper budget guard | Pure helper semantics, standalone expression diagnostics |
| `crosswalk-cel` | CEL compile/evaluate/preview, expression diagnostics, root bindings, security limits, JSON conversion | Mapping YAML, PublicSchema document semantics, Python or WASM bindings |
| `crosswalk-publicschema` | PublicSchema v0.2 compile/evaluate, JSON Pointer reads/writes, value mappings, logs, privacy diagnostics | v0.1 records mapping runtime, binding-specific APIs |
| `crosswalk-core` | v0.1 mapping runtime, compatibility re-exports, host runtime options | New lower-level helper semantics that should be reusable |
| `crosswalk-python` | Python conversion, exceptions, typing, examples, packaging | Runtime semantics |
| `crosswalk-wasm` | Raw wasm-bindgen JSON-string boundary | Runtime semantics or idiomatic TypeScript wrappers |
| `packages/js` | Idiomatic TypeScript wrapper and generated WASM package management | Rust runtime semantics |

## Public boundary rules

Public crate boundaries should expose Crosswalk types and JSON values, not
upstream engine internals. In particular, `crosswalk-cel` keeps `cel::Value`,
`cel::Program`, and `cel::ExecutionError` private to its implementation.

Compatibility re-exports from `crosswalk-core` are allowed while downstream
projects migrate. New code should prefer the narrowest crate that owns the
behavior it needs.

## Runtime context notes

`crosswalk-core::MappingRuntime` installs helper request context and function
security limits for its v0.1 mapping and standalone expression paths.

Direct `crosswalk-publicschema::evaluate_publicschema_mapping` intentionally
preserves existing PublicSchema behavior: it does not install helper request
context or helper hot-path `FunctionSecurityLimits`. This is documented as a
known behavioral boundary in `docs/crate-split-inventory.md`.

## Adding dependencies

Before adding a dependency, check whether it belongs at the crate level:

- Pure parsing or semantics shared by helpers: `crosswalk-functions`.
- CEL-specific adapter support: `crosswalk-functions-cel` or `crosswalk-cel`.
- YAML mapping document behavior: `crosswalk-core` or `crosswalk-publicschema`.
- Host language conversion: `crosswalk-python`, `crosswalk-wasm`, or
  `packages/js`.

Avoid adding dependencies to lower crates just because a higher crate already
uses them.
