# crosswalk-functions

`crosswalk-functions` contains pure deterministic helper functions used by
Crosswalk mapping runtimes. It has no CEL dependency and is the lowest-level
crate in the helper stack.

## When to use it

Use this crate when you want to:

- Call helper functions directly from Rust without a CEL engine.
- Reuse text, email, phone, date, ID, redaction, JSON, or code-system semantics.
- Add or test helper behavior independently from CEL adapter coercion.

Use `crosswalk-functions-cel` if you need CEL registration, arity validation,
missing/null behavior, request fallback resolution, or warning collection.

## Install

This package is not published yet. Use a workspace or git dependency:

```toml
[dependencies]
crosswalk-functions = { git = "https://github.com/PublicSchema/cel-mapping", features = ["date", "phone", "redaction"] }
```

## Feature flags

Default features include `std`, `text`, `regex`, `email`, `ids`, `json`, and
`codes`.

Optional feature groups:

- `date`
- `phone`
- `redaction`

The crate keeps heavy optional dependencies behind these features.

## Quick start

```rust
use crosswalk_functions::{email, text};

assert_eq!(text::normalize_space(" Ada   Lovelace "), "Ada Lovelace");
assert_eq!(email::email_domain("ada@example.org"), Some("example.org".to_string()));
```

## Public API surface

Modules are feature-gated:

- `text`: trim, case normalization, slug, regex replace/extract.
- `email`: normalize, domain extraction, validation.
- `ids`: stable SHA-256 hash, prefixed slug, clean ID.
- `json`: parse and stringify JSON.
- `codes`: `CodeSystemRegistry`, `CodeEntry`, typed code-system documents, ISO preload.
- `date`: date and datetime parse/format helpers.
- `phone`: E.164 normalization and phone validation.
- `redaction`: masking and redaction helpers.

Shared errors:

- `FunctionError`
- `InfallibleFunctionError`

## Boundaries

This crate owns pure helper semantics only. It should not depend on CEL,
`serde_yaml`, runtime thread-local context, mapping compilation, or binding
adapters.

Important example: direct `text::regex_extract` returns `Ok(None)` when no match
is found. The CEL-visible helper in `crosswalk-functions-cel` adapts that into
the compatibility empty-string behavior.

## Error model

Fallible helpers return `FunctionError` with a stable machine-readable `code`
and human-readable `message`. Tests should assert error codes before prose.

## Testing

```bash
cargo test -p crosswalk-functions
cargo test -p crosswalk-functions --features date,phone,redaction
cargo clippy -p crosswalk-functions --all-targets --all-features -- -D warnings
cargo doc -p crosswalk-functions --no-deps --all-features
```
