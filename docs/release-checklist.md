# Crosswalk Release Checklist

This project is not externally published yet, but the package split should be
treated as a public boundary before the first release.

## Rust crates

Verify metadata:

- Each publishable crate has `description`, `license`, `repository`, and
  `readme`.
- README examples and package names match the current crate names.
- Public re-export paths in `crosswalk-core` are intentional.
- `docs/crate-split-inventory.md` reflects the actual boundary decisions.

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

If Python dev libraries are unavailable, `cargo test --workspace` may require
using the workspace default members instead of the PyO3 crate. In that case,
record the skipped package and run the Python pytest checks below.

## Python package

Verify:

- Package name is `crosswalk`.
- Import name is `crosswalk`.
- Typing stubs cover exported APIs.
- Examples still run.

Run from `crates/crosswalk-python`:

```bash
uv run --extra dev python -m pytest
for f in examples/*.py; do uv run --extra dev python "$f"; done
uv run --with 'maturin>=1.5,<2' maturin build --release -m Cargo.toml
```

## JS and WASM package

Verify:

- `crosswalk-wasm` raw methods are documented as JSON-string APIs.
- `packages/js` exposes the idiomatic `Crosswalk` TypeScript surface.
- Generated `wasm-pkg` output is not treated as the source of truth.

Run from `packages/js`:

```bash
npm ci
npm test
```

## Public boundary decisions

Before an external release, explicitly confirm:

- Whether `crosswalk-core` compatibility re-exports are permanent or
  transitional.
- Whether direct `crosswalk-publicschema` evaluation should continue to omit
  helper request context and helper hot-path limits.
- Whether all public error shapes and serde casing are stable.
- Whether crate names, Python package name, and JS package name are final.

## Consumer smoke checks

When sibling repositories are available, run the smallest meaningful consumer
checks for:

- `publicschema.com`
- `registry-relay`
- `registry-witness`
- Any other repo with a live `crosswalk-*`, `crosswalk`, or `crosswalk-js`
  dependency.

Record skipped checks with the exact reason.
