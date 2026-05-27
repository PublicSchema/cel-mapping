# Crosswalk Extension Guide

Use this guide when adding behavior to the crate family. The main rule is to put
the change in the crate that owns the semantics, then adapt it outward.

## Add a pure helper function

1. Add the deterministic implementation to `crates/crosswalk-functions/src`.
2. Put optional dependencies behind feature flags in
   `crates/crosswalk-functions/Cargo.toml`.
3. Return `FunctionError` with a stable machine-readable `code` for fallible
   behavior.
4. Add focused tests in `crates/crosswalk-functions/tests` or module tests.
5. If the helper should be available in CEL, add adapter behavior in
   `crosswalk-functions-cel`.

Do not add CEL `Value` handling or mapping runtime context to
`crosswalk-functions`.

## Add or change a CEL-visible helper

1. Keep pure semantics in `crosswalk-functions` where possible.
2. Register the CEL helper in `crosswalk-functions-cel`.
3. Update `helper_metadata`.
4. Add adapter tests for arity, missing/null behavior, warnings, fallback
   context, and CEL-visible compatibility results.
5. Add integration tests through `crosswalk-cel` or `crosswalk-core` when root
   binding or runtime context behavior matters.

Be explicit about differences between direct Rust semantics and CEL-visible
semantics. For example, direct regex extraction can return `None` while the CEL
helper returns an empty string for compatibility.

## Add standalone expression behavior

1. Put compile/evaluate/preview changes in `crosswalk-cel`.
2. Keep public inputs and outputs in terms of Crosswalk types and JSON values.
3. Do not expose upstream CEL engine types across the public boundary.
4. Re-export through `crosswalk-core` only if existing facade users need the
   path.
5. Add tests in `crates/crosswalk-cel/tests` and compatibility tests in
   `crates/crosswalk-core/tests` when re-export behavior matters.

## Add v0.1 mapping behavior

1. Put v0.1 YAML parsing, compile, or evaluation changes in `crosswalk-core`.
2. Preserve `ErrorMode` behavior: strict, collect, and lenient callers expect
   different error collection semantics.
3. Add focused tests in `crates/crosswalk-core/tests`.
4. Verify host bindings if the behavior is exposed to Python or WASM.

## Add PublicSchema behavior

1. Put PublicSchema document behavior in `crosswalk-publicschema`.
2. Use `crosswalk-cel` for expression compile/evaluate behavior.
3. Use `crosswalk-functions` for code systems and pure helper data.
4. Keep logs, warning shape, privacy handling, and direction behavior stable.
5. Add or update parity fixtures under the PublicSchema fixture directories when
   behavior should be shared across core and direct PublicSchema paths.

Watch the documented context boundary: direct PublicSchema evaluation does not
install helper request context or helper hot-path limits.

## Add Python binding behavior

1. Add runtime behavior in Rust crates below `crosswalk-python` first.
2. Keep `crosswalk-python` focused on conversion, exceptions, typing stubs, and
   examples.
3. Update `python/crosswalk/__init__.pyi` when the Python API changes.
4. Add pytest coverage in `crates/crosswalk-python/tests`.
5. Run at least:

```bash
cd crates/crosswalk-python
uv run --extra dev python -m pytest
```

## Add WASM or TypeScript behavior

1. Add runtime behavior in Rust crates below `crosswalk-wasm` first.
2. Keep `crosswalk-wasm` as a thin JSON-string boundary.
3. Add idiomatic object/camelCase APIs in `packages/js/src`.
4. Update TypeScript tests in `packages/js/tests`.
5. Run:

```bash
cd packages/js
npm test
```

## Review checklist

- Does the change live in the lowest crate that owns the semantics?
- Are compatibility re-exports still intentional?
- Did any public error, serde casing, or binding method shape change?
- Are direct Rust, CEL-visible, Python, and JS/WASM semantics documented where
  they differ?
- Are focused tests present for the changed behavior?
