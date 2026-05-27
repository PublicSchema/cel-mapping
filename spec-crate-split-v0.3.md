# cel-mapping Crate Split v0.3 Refactor Spec

Status: draft  
Audience: `cel-mapping`, `registry-relay`, `registry-witness`, PublicSchema tooling, generated bindings  
Scope: split reusable transformation semantics out of the current `cel-mapper-core` monolith while preserving current runtime behavior

## 1. Purpose

`cel-mapping` currently combines four concerns in one Rust crate:

- generic v0.1 mapping compilation and evaluation
- standalone CEL expression evaluation
- PublicSchema v0.2 property-mapping compilation and evaluation
- mapping helper functions and their CEL adapter code

That has worked while the package had few consumers. It is now becoming a shared runtime for PublicSchema transforms, registry demos, registry relay provenance, and witness-side evidence expressions. The helper functions are especially valuable outside the mapper: deterministic string, code, date, phone, email, ID, and redaction helpers should be reusable without pulling in the full CEL runtime or PublicSchema compiler.

This refactor splits the crate graph now, while external adoption is still low, so later users get clean contracts instead of accidental module boundaries.

## 2. Goals

- Extract pure deterministic helper functions into a reusable crate with no CEL dependency.
- Keep CEL registration and CEL `Value` conversion in a thin adapter crate.
- Move PublicSchema mapping semantics behind a dedicated crate contract.
- Preserve the existing Rust, Python, WASM, and TypeScript public surfaces during the migration through compatibility re-exports.
- Make `registry-relay` and `registry-witness` able to use the parts they actually need.
- Keep every behavioral change covered by golden fixtures or focused unit tests.

## 3. Non-goals

- Do not redesign the PublicSchema v0.2 mapping format.
- Do not change CEL expression syntax or helper names as part of the split.
- Do not introduce a workbook or XLS orchestration runtime in this refactor.
- Do not move to RDF, YARRRML, or JSON-LD as the core execution model.
- Do not make helper functions perform network, database, filesystem, or clock reads.
- Do not drop existing Python, WASM, or TypeScript APIs during the first migration wave.

## 4. Current State

Current package layout:

```text
cel-mapping
  crates/cel-mapper-core
    ast_paths.rs
    budget.rs
    cel_scan.rs
    code_system.rs
    compiled.rs
    compiler.rs
    errors.rs
    eval_ctx.rs
    evaluator.rs
    expr.rs
    functions/
    iso_systems.rs
    lib.rs
    mapping.rs
    missing.rs
    output.rs
    paths.rs
    publicschema/
    runtime.rs
    security.rs
  crates/cel-mapper-python
  crates/cel-mapper-wasm
  packages/js
```

Important existing behavior:

- `cel_mapper_core::MappingRuntime` is the main facade for generic mappings, standalone expressions, and PublicSchema mappings.
- `cel_mapper_core::publicschema` is already implemented inside the core crate.
- `publicschema/mod.rs` directly uses core-owned compile, evaluator, compiled mapping, path, security, and code-system types. This means `publicschema-mapping` cannot be extracted cleanly until those dependencies are available outside `cel-mapper-core`.
- `crates/cel-mapper-core/src/functions/builtins.rs` contains pure helper logic mixed with CEL registration, argument coercion, and `ExecutionError` construction.
- `functions/phone.rs` and date helpers in `functions/builtins.rs` read fallback values from `eval_ctx_get`, including `ctx.country`, `ctx.timezone`, and `ctx.today`.
- `code_system.rs` owns `CodeSystemRegistry`, and helper registration captures an `Arc<CodeSystemRegistry>`.
- `iso_systems.rs` loads built-in code systems into `CodeSystemRegistry`.
- `eval_ctx.rs` is thread-local request state for helper fallback values and warnings.
- Python and WASM bindings call the core facade directly.
- `registry-relay` can consume PublicSchema mapping types for provenance.
- `registry-witness` can consume standalone CEL expression evaluation for evidence checks.

The refactor must treat this as a migration from a working monolith, not as a rewrite.

## 5. Target Crate Graph

Required v0.3 dependency target:

```text
cel-mapper-python / cel-mapper-wasm / packages/js
        |
        v
cel-mapper-core facade
        |
        +--------------------+
        |                    |
        v                    v
publicschema-mapping     cel-evaluator
        |                    |
        +--> cel-evaluator   v
        |              mapping-functions-cel
        |                    |
        v                    v
mapping-functions <----------+
```

Equivalent dependency list:

```text
cel-mapper-core -> publicschema-mapping
cel-mapper-core -> cel-evaluator
publicschema-mapping -> cel-evaluator
publicschema-mapping -> mapping-functions
cel-evaluator -> mapping-functions-cel
mapping-functions-cel -> mapping-functions
```

In this graph, arrows point from consumer to dependency. `publicschema-mapping` depends directly on `mapping-functions` for code systems and pure helper semantics. It does not depend on `mapping-functions-cel`, because CEL registration is an evaluator concern. `cel-mapper-core` can remain the compatibility facade because `publicschema-mapping` depends on `cel-evaluator`, not on `cel-mapper-core`.

`cel-evaluator` is required in v0.3. It is not needed for the first text-helper extraction, but it is a required wave before `publicschema-mapping` can move. If the team decides not to extract `cel-evaluator`, then `publicschema-mapping` must be scoped out of v0.3.

Compatibility target during migration:

```text
external callers
      |
      v
cel_mapper_core re-exports
      |
      +--> publicschema_mapping
      +--> mapping_functions_cel
      +--> generic v0.1 core runtime
```

Out of scope for v0.3 unless a concrete workbook consumer needs it:

```text
mapping-bundle
  workbook, multi-sheet, and multi-record orchestration
```

The first split should avoid creating more crates than needed. The required runtime split is `mapping-functions`, `mapping-functions-cel`, `cel-evaluator`, `publicschema-mapping`, and the existing `cel-mapper-core` facade. `mapping-bundle` should be added only when a concrete workbook or multi-registry consumer needs it.

## 6. Crate Contracts

### 6.1 `mapping-functions`

Purpose: pure deterministic helper functions for mappings and registry transforms.

Rules:

- MUST NOT depend on `cel`.
- MUST NOT depend on PublicSchema mapping document types.
- MUST NOT perform IO, clock reads, randomness, or host callbacks.
- MUST expose typed Rust functions with stable error types.
- SHOULD depend only on deterministic parsing and normalization libraries.
- SHOULD keep null, missing, and CEL coercion semantics out of the pure functions. Those are adapter concerns.
- MUST accept all request-specific fallback values, such as country, timezone, and today, as explicit function arguments.

Suggested modules:

```text
mapping_functions::text
mapping_functions::email
mapping_functions::phone
mapping_functions::date
mapping_functions::codes
mapping_functions::ids
mapping_functions::redaction
mapping_functions::json
```

Initial text/string API:

```rust
pub fn trim(input: &str) -> String;
pub fn lower_ascii(input: &str) -> String;
pub fn upper_ascii(input: &str) -> String;
pub fn title_simple(input: &str) -> String;
pub fn normalize_space(input: &str) -> String;
pub fn remove_accents(input: &str) -> String;
pub fn slug(input: &str) -> String;
pub fn regex_replace(input: &str, pattern: &str, replacement: &str) -> Result<String, FunctionError>;
pub fn regex_extract(input: &str, pattern: &str, group: usize) -> Result<Option<String>, FunctionError>;
```

Initial adjacent APIs:

```rust
pub fn normalize_email(input: &str) -> String;
pub fn email_domain(input: &str) -> Option<String>;
pub fn normalize_phone_e164(input: &str, country_hint: Option<&str>) -> Result<String, FunctionError>;
pub fn parse_datetime(input: &str, pattern: &str, timezone_hint: Option<&str>) -> Result<String, FunctionError>;
pub fn today(today_hint: Option<&str>) -> Result<String, FunctionError>;
pub fn stable_hash_sha256(input: &str, salt: Option<&str>) -> String;
pub fn prefixed_slug(prefix: &str, input: &str) -> String;
```

Code-system API:

```rust
pub struct CodeSystemRegistry;
pub struct CodeSystemDocument;
pub struct CodeEntry;

impl CodeSystemRegistry {
    pub fn new() -> Self;
    pub fn normalize_code(input: &str) -> String;
    pub fn merge_document(&mut self, name: &str, document: CodeSystemDocument) -> Result<(), FunctionError>;
    pub fn merge_documents(&mut self, documents: impl IntoIterator<Item = (String, CodeSystemDocument)>) -> Result<(), FunctionError>;
    pub fn map(&self, system: &str, source: &str) -> Option<String>;
    pub fn reverse_map(&self, system: &str, target: &str) -> Option<String>;
}

pub fn load_iso_systems(registry: &mut CodeSystemRegistry);
```

`CodeSystemRegistry`, `CodeEntry`, normalization, lookup, reverse lookup, and ISO preload logic move with the code helpers because code-system helpers are not CEL-specific. `cel-mapper-core` re-exports the types during the compatibility window.

YAML parsing stays one layer above `mapping-functions`. The leaf crate may expose serde-compatible typed documents, but it MUST NOT require `serde_yaml` in its default dependency graph. `cel-mapper-core` and `publicschema-mapping` parse YAML into `CodeSystemDocument` before calling `mapping-functions`.

Error rules:

- Invalid arguments return `FunctionError`, not strings.
- Errors include stable machine-readable codes.
- `FunctionError` codes are a v0.3 public API contract. Tests assert codes, not prose messages.
- Error messages are suitable for diagnostics but are not a public compatibility contract.
- Redaction helpers must never include the original sensitive value in error text.

Feature flags:

- Default features: `std`, `text`, `regex`, `email`, `ids`, `json`, `codes`.
- Optional features: `date`, `phone`, `redaction`.
- `std` remains enabled by default. `no_std` is not required for v0.3.
- Feature-gated modules still ship in Wave 1. `date`, `phone`, and `redaction` are implemented and tested behind non-default features; `mapping-functions-cel` enables the feature set it needs in its own `Cargo.toml`.

Semantic preservation rules:

- Rust helper names use `remove_accents`; the CEL public helper name remains `text_remove_accents`.
- `regex_extract` returns `Ok(None)` when the pattern does not match. The CEL adapter MUST preserve today's behavior by converting `None` to `Value::String("")`, not `Null`.
- Phone and date helpers never read thread-local evaluation context directly. The CEL adapter resolves `ctx.country`, `ctx.timezone`, and `ctx.today` before calling pure functions.

### 6.2 `mapping-functions-cel`

Purpose: register `mapping-functions` helpers into the CEL runtime.

Rules:

- Owns CEL `Value` conversion, arity validation, missing handling, and `ExecutionError` conversion.
- Owns request-context fallback resolution for helper calls. This is real adapter logic, not just type conversion.
- Exports the same helper names used today, such as `text_trim`, `text_normalize_space`, `email_normalize`, `phone_normalize`, `code_map`, and `id_hash`.
- Keeps namespaced call rewriting compatibility, for example `text.normalize_space(...)` to `text_normalize_space(...)`, wherever that is currently supported.
- Exposes helper metadata for UI, generated docs, and parity tests.

Suggested API:

```rust
pub fn register_mapping_functions(ctx: &mut cel::Context, registry: FunctionRegistry, request: FunctionRequestContext);
pub fn helper_metadata() -> Vec<HelperMetadata>;
```

`FunctionRegistry` should hold code-system registries and any deterministic lookup tables. It must not hold request-scoped state such as privacy mode, current time, or user identity.

`FunctionRequestContext` should hold request-scoped fallback values currently read through `eval_ctx_get`, including:

- `country`
- `timezone`
- `today`
- warning sink

Compatibility rule:

- Until `eval_ctx.rs` is removed or moved into `cel-evaluator`, the adapter may read the existing thread-local context. It must immediately resolve those values and pass explicit arguments to `mapping-functions`.

Layering rule:

- `cel-evaluator` owns expression request context for evaluation and any temporary storage strategy.
- `mapping-functions-cel` owns helper fallback resolution and receives only the helper-facing `FunctionRequestContext`.
- `mapping-functions-cel` MUST NOT depend on `cel-evaluator`; `cel-evaluator` converts its request context into `FunctionRequestContext` when registering helpers.

### 6.3 `cel-mapper-core`

Purpose: generic mapping runtime and compatibility facade.

Responsibilities after the split:

- v0.1 `records` mapping compiler and evaluator.
- `MappingRuntime` facade.
- generic mapping output types, including `EvaluationInput` and `MappingOutput`.
- compatibility re-exports for PublicSchema types during the migration.

Rules:

- MUST depend on `cel-evaluator` after Wave 3.
- MAY depend on `mapping-functions-cel` only during the transition before `cel-evaluator` owns function registration.
- SHOULD NOT contain pure helper implementations after Wave 2.
- SHOULD keep `MappingRuntime::register_standard_functions` behavior stable.
- MUST keep v0.1 mapping document errors and generic `MappingOutput` ownership unless a later spec extracts the generic mapping runtime.
- MUST continue to support existing Rust callers importing PublicSchema types from `cel_mapper_core` until the compatibility window ends.

Compatibility re-export examples:

```rust
pub use publicschema_mapping::{
    CompiledPublicSchemaMapping,
    PrivacyMode,
    PublicSchemaCompileOptions,
    PublicSchemaDirection,
    PublicSchemaEvaluateOptions,
    PublicSchemaEvaluationInput,
    PublicSchemaTransformOutput,
};
```

### 6.4 `cel-evaluator`

Purpose: standalone CEL compile, evaluate, preview, security limits, and mapping function registration.

Responsibilities:

- compile a CEL expression with the project security limits.
- evaluate expressions against root bindings.
- preview expressions with structured diagnostics.
- register `mapping-functions-cel` helpers.
- expose the narrow evaluator API used by both generic mappings and PublicSchema mappings.
- own or re-export the evaluator-adjacent modules currently needed by PublicSchema extraction: reusable compiled expression wrappers, expression compile/evaluate code, `security`, `paths`, `ast_paths`, `missing`, `expr`, `budget`, `eval_ctx`, and expression preview/evaluation errors.

Rules:

- MUST NOT depend on `cel-mapper-core`.
- MUST NOT depend on `publicschema-mapping`.
- MUST own CEL-specific missing-value behavior only when it is independent of a mapping document.
- MUST expose request context explicitly even if it temporarily preserves the current thread-local implementation internally.
- MUST NOT expose `cel::ExecutionError` across public crate boundaries. Adapter and evaluator code convert it into stable `FunctionError`, `ExpressionError`, or mapping-specific diagnostics.
- MUST NOT own `MappingOutput`; that remains with the v0.1 generic mapping runtime in `cel-mapper-core`.
- SHOULD keep generic enough APIs for `registry-witness` evidence expressions.

Suggested API:

```rust
pub struct ExpressionRuntime;
pub struct ExpressionInput;
pub struct ExpressionPreviewResult;
pub struct SecurityLimits;

impl ExpressionRuntime {
    pub fn evaluate(&self, expression: &str, input: ExpressionInput) -> Result<serde_json::Value, ExpressionError>;
    pub fn preview(&self, expression: &str, input: ExpressionInput) -> ExpressionPreviewResult;
}
```

### 6.5 `publicschema-mapping`

Purpose: PublicSchema v0.2 compile and evaluate semantics.

Responsibilities:

- PublicSchema document parsing and validation.
- `property_mappings[]` compilation.
- direction selection.
- JSON Pointer reads and writes.
- value mappings.
- deterministic mapping hash.
- transformation logs, warnings, and privacy-aware diagnostics.
- PublicSchema rule preview support.

Dependencies:

- MUST NOT depend on `cel-mapper-core` while `cel-mapper-core` re-exports PublicSchema types.
- MUST depend on `cel-evaluator` for expression compile, evaluate, and preview.
- MUST depend on `mapping-functions` for `CodeSystemRegistry` and code-system helpers.
- MUST NOT depend on Python, WASM, TypeScript, registry relay, registry witness, or hosted PublicSchema services.

API must match the current core-exported PublicSchema types unless a breaking release is explicitly accepted.

### 6.6 `cel-mapper-python`, `cel-mapper-wasm`, and `packages/js`

Purpose: preserve existing bindings while internals move.

Rules:

- Python and WASM bindings should keep current method names.
- TypeScript wrapper should keep camelCase methods.
- Bindings can import from `cel-mapper-core` compatibility re-exports first.
- After the split stabilizes, bindings MAY import PublicSchema types directly from `publicschema-mapping` if that reduces coupling.

### 6.7 Later `mapping-bundle`

Purpose: workbook and multi-registry orchestration for demos and operational transforms.

This crate is intentionally out of scope for v0.3. It becomes useful when XLS files, multiple registry tabs, code lists, and fixture bundles need a deterministic runtime contract. The current split should leave room for it without inventing it too early.

## 7. Naming Decision

Use `mapping-functions` rather than `cel-functions`.

Rationale:

- The functions should be reusable outside CEL.
- The crate will be useful in registry import/export code, witness normalization, workbook validation, and generated validators.
- CEL is one adapter, not the owner of the semantics.

Use `mapping-functions-cel` for the adapter crate.

Use `publicschema-mapping` for the PublicSchema runtime crate, because it owns PublicSchema mapping document semantics rather than generic mapping semantics.

## 8. Workspace And Release Plan

Rust version:

- Set `workspace.package.rust-version = "1.83"` in the root `Cargo.toml`.
- Every workspace crate must inherit or explicitly repeat `rust-version = "1.83"`.
- The minimum is pinned because current code uses Rust 1.83-compatible thread-local initialization patterns, and new reusable crates should not surprise downstream consumers.

Versioning:

- `mapping-functions`, `mapping-functions-cel`, `cel-evaluator`, and `publicschema-mapping` should use explicit crate versions rather than `version.workspace = true`.
- Initial extracted crate versions should start at `0.1.0`.
- `cel-mapper-core`, `cel-mapper-wasm`, and `cel-mapper-python` may keep the workspace version while they remain distribution facades.

Publish order:

```text
mapping-functions
mapping-functions-cel
cel-evaluator
publicschema-mapping
cel-mapper-core
cel-mapper-wasm / cel-mapper-python
packages/js
```

Workspace membership:

- Add each new crate to `workspace.members` in the wave where it is introduced.
- Add `mapping-functions`, `mapping-functions-cel`, `cel-evaluator`, and `publicschema-mapping` to `default-members` once each crate has tests that do not require Python development libraries.
- Keep `cel-mapper-python` out of `default-members` unless the workspace Python build constraint changes.

Parallel edit coordination:

- Each wave has exactly one integration owner for root `Cargo.toml`, crate `Cargo.toml` files, crate `lib.rs` files, and top-level re-exports.
- Parallel workers may edit implementation modules and tests assigned to them.
- Workers must not independently add `pub mod` lines, workspace members, feature flags, or public re-exports. They hand those changes to the integration owner.

## 9. Migration Strategy

### Wave 0: Spec And Inventory

Deliverables:

- This spec is committed and linked from the README.
- Current helper names are inventoried from `functions/builtins.rs`, including exact no-match, null, missing, fallback, and error-code behavior.
- Public types, trait impls, `From` / `Into` impls, serde derives, generic bounds, and re-export paths are inventoried before any type moves.
- The inventory is recorded in `docs/crate-split-inventory.md` and reviewed before Wave 1 starts.
- PublicSchema helper parity docs identify which helpers move to `mapping-functions` and which stay adapter-only.
- `workspace.package.rust-version = "1.83"` is added before new crates are introduced.
- Feature flags and `FunctionError` code stability are treated as resolved decisions, not open questions.

Review:

- Staff engineer review focuses on crate graph, dependency cycles, compatibility, and test coverage.
- Domain review focuses on whether extracted helpers are realistic for registry and government data workflows.

### Wave 1: Extract Pure Functions

Deliverables:

- Add `crates/mapping-functions`.
- Use explicit `version = "0.1.0"` and `rust-version.workspace = true`.
- Use default features `std`, `text`, `regex`, `email`, `ids`, `json`, and `codes`.
- Move pure text, email, phone, ID, regex, date, JSON, and redaction logic into typed modules.
- Ship `phone`, `date`, and `redaction` behind non-default features in Wave 1 even though they are not default-enabled.
- Move `CodeSystemRegistry`, `CodeEntry`, code normalization, and ISO code-system loading into `mapping-functions::codes`.
- Keep YAML parsing in `cel-mapper-core` and `publicschema-mapping`; pass typed code-system documents into `mapping-functions`.
- Keep existing CEL helper behavior unchanged by calling into the new crate from the current core functions module.
- Add focused unit tests in `mapping-functions`.

Parallel work:

- Worker A extracts text, email, ID, and redaction helpers.
- Worker B extracts phone and date helpers.
- Worker C extracts code-system and JSON helpers.

Integration rule:

- One integration owner edits workspace manifests, crate manifests, `lib.rs`, and re-export surfaces.
- Workers must not edit the same helper module at the same time.
- Each worker lands tests with the extracted module.
- A reviewer validates that CEL behavior remains unchanged after each extraction.

### Wave 2: Add CEL Adapter Crate

Deliverables:

- Add `crates/mapping-functions-cel`.
- Use explicit `version = "0.1.0"` and `rust-version.workspace = true`.
- Move `register_stdlib` and CEL conversion wrappers out of `cel-mapper-core`.
- Move request-context fallback resolution for `country`, `timezone`, `today`, and warnings into the adapter boundary.
- Keep `cel_mapper_core::functions::register_stdlib` as an internal compatibility shim until core modules are updated.
- Add adapter tests proving current helper names, arity behavior, null behavior, missing behavior, and error codes remain stable.
- Add explicit tests that `text_regex_extract` returns `""` on no match through CEL while direct Rust `regex_extract` returns `Ok(None)`.

Parallel work:

- Worker A moves registration and metadata.
- Worker B ports tests for text, email, phone, and date helpers.
- Worker C ports tests for code systems, IDs, redaction, and validation helpers.

Review:

- Reviewer compares golden fixture output before and after the adapter move.

### Wave 3: Extract Evaluator Boundary

Deliverables:

- Add `crates/cel-evaluator`.
- Use explicit `version = "0.1.0"` and `rust-version.workspace = true`.
- Move standalone CEL compile, evaluate, preview, security limits, expression diagnostics, and function registration behind the new crate.
- Move or re-home evaluator dependencies currently imported by PublicSchema: `compiled`, `compiler`, `evaluator`, `security`, `paths`, `ast_paths`, `missing`, `expr`, `budget`, `eval_ctx`, and shared expression errors.
- Keep `cel_mapper_core::evaluator::*` as compatibility re-exports or wrappers.
- Update generic mapping runtime to call `cel-evaluator`.
- Update `registry-witness` compile checks if it imports standalone expression APIs.

Parallel work:

- Worker A moves expression compile and evaluation.
- Worker B moves preview diagnostics and security limits.
- Worker C updates compatibility exports and focused tests.

Review:

- Reviewer confirms standalone expression APIs return the same JSON values and structured diagnostics as before.

### Wave 4: Extract PublicSchema Runtime

Deliverables:

- Add `crates/publicschema-mapping`.
- Use explicit `version = "0.1.0"` and `rust-version.workspace = true`.
- Move PublicSchema document types, compile, evaluate, pointer, hash, and output logic into the new crate.
- Depend on `cel-evaluator` and `mapping-functions`; do not depend on `cel-mapper-core`.
- Keep `cel_mapper_core` re-exporting PublicSchema public types and runtime methods.
- Update Python, WASM, and TypeScript surfaces only where imports require it.

Parallel work:

- Worker A moves document, compile, and hash code.
- Worker B moves evaluate, pointer, log, and privacy code.
- Worker C updates bindings and downstream compile checks.

Review:

- Reviewer confirms there is no behavior drift in PublicSchema parity fixtures.

### Wave 5: Downstream Consumer Checks

Deliverables:

- `registry-relay` compiles against either compatibility re-exports or direct `publicschema-mapping` imports.
- `registry-witness` compiles against standalone expression APIs and can optionally use `mapping-functions` for non-CEL normalization.
- Examples and README snippets are updated.

Parallel work:

- Worker A validates `registry-relay`.
- Worker B validates `registry-witness`.
- Worker C validates Python, WASM, and TypeScript package docs.

Review:

- Reviewer checks that downstream changes are minimal and do not duplicate mapping semantics.

### Wave 6: Compatibility Cleanup

Deliverables:

- Mark compatibility re-exports with documentation explaining their migration path.
- Decide whether to keep `cel-mapper-core` as the long-term facade.
- Remove only compatibility shims that have a documented replacement and a release note.

This wave can be deferred until real consumers have migrated.

## 10. Testing Requirements

Required test layers:

- `mapping-functions` unit tests for every extracted pure helper.
- `mapping-functions-cel` adapter tests for CEL arity, coercion, null, missing, and error behavior.
- `cel-evaluator` tests for expression evaluation, preview diagnostics, security limits, and missing-value behavior.
- Existing `cel-mapper-core` mapping tests.
- Existing PublicSchema golden parity fixtures.
- Python binding tests.
- WASM and TypeScript wrapper tests.
- Downstream compile checks for `registry-relay` and `registry-witness` when those repositories are in scope.
- Compile-time compatibility tests for re-exported types and moved trait impls.

Golden behavior rule:

- A helper extraction is not complete until old and new paths produce byte-for-byte equivalent JSON outputs for existing fixtures, except for explicitly accepted diagnostic wording changes.
- CEL helper compatibility wins over cleaner pure Rust signatures. For example, CEL `text_regex_extract` continues to return `""` on no match even though the pure Rust helper returns `Ok(None)`.
- Diagnostic wording changes are accepted only when the PR updates the affected fixture in the same commit and documents the reason in `CHANGELOG.md` or `docs/crate-split-inventory.md`. Review approval on that PR is the acceptance record.

Recommended commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p mapping-functions
cargo test -p mapping-functions-cel
cargo test -p cel-evaluator
cargo test -p cel-mapper-core -p cel-mapper-wasm
cargo test -p cel-mapper-python
cd packages/js && npm ci && npm run build:wasm && npm run build:ts && npm test
```

If Python dev libraries are unavailable, `cargo test -p cel-mapper-python` may be replaced by the project-supported `maturin develop` plus `pytest` workflow.

## 11. Definition Of Done

The v0.3 crate split is done only when every item below is satisfied. A missing item cannot be waived by calling it a blocker; any intentional scope change requires a spec amendment before the work is marked done.

Required files and workspace metadata:

- Root `Cargo.toml` has `workspace.package.rust-version = "1.83"`.
- `crates/mapping-functions`, `crates/mapping-functions-cel`, `crates/cel-evaluator`, and `crates/publicschema-mapping` exist and are listed in `workspace.members`.
- `mapping-functions`, `mapping-functions-cel`, `cel-evaluator`, and `publicschema-mapping` use explicit `version = "0.1.0"` and inherit or repeat `rust-version = "1.83"`.
- `mapping-functions`, `mapping-functions-cel`, `cel-evaluator`, and `publicschema-mapping` are included in `default-members`.
- `docs/crate-split-inventory.md` exists and records helper semantics, public types, trait impls, serde behavior, generic bounds, re-export paths, ownership moves, and accepted diagnostic wording changes.
- README and crate-level docs name the new crates, their responsibilities, and the compatibility import paths.

Required dependency boundaries:

- `mapping-functions` has no dependency on `cel`, `cel-mapper-core`, `mapping-functions-cel`, `cel-evaluator`, `publicschema-mapping`, Python, WASM, TypeScript, or `serde_yaml` in its default dependency graph.
- `mapping-functions` default features are exactly `std`, `text`, `regex`, `email`, `ids`, `json`, and `codes`.
- `mapping-functions` ships `date`, `phone`, and `redaction` behind non-default features.
- `mapping-functions-cel` depends on `mapping-functions` and does not depend on `cel-evaluator`.
- `cel-evaluator` depends on `mapping-functions-cel` and does not depend on `cel-mapper-core` or `publicschema-mapping`.
- `publicschema-mapping` depends on `cel-evaluator` and `mapping-functions`, and does not depend on `cel-mapper-core` or `mapping-functions-cel`.
- `cel-mapper-core` remains the facade and may depend on `cel-evaluator`, `publicschema-mapping`, and `mapping-functions`.

Required ownership:

- Pure helper logic from `functions/builtins.rs` lives in `mapping-functions`, except CEL-only arity, coercion, missing/null, and error-conversion code.
- `CodeSystemRegistry`, `CodeEntry`, code normalization, and ISO code-system loading live in `mapping-functions::codes`.
- YAML parsing for code systems lives outside `mapping-functions`; callers pass typed code-system documents to the leaf crate.
- `mapping-functions-cel` owns CEL helper registration, CEL `Value` conversion, arity validation, helper metadata, and fallback resolution for `country`, `timezone`, `today`, and warnings.
- `cel-evaluator` owns standalone CEL compile, evaluate, preview, security limits, expression diagnostics, evaluator request context, and evaluator-adjacent modules required by PublicSchema.
- `cel-evaluator` public APIs do not expose `cel::ExecutionError`.
- `MappingOutput`, `EvaluationInput`, and v0.1 mapping document errors remain owned by `cel-mapper-core`.
- PublicSchema document parsing, compile, evaluate, pointer writes, value mappings, canonical hash, logs, warnings, and privacy-aware diagnostics live in `publicschema-mapping`.

Required compatibility:

- Existing helper names and namespaced aliases continue to work.
- CEL `text_regex_extract` returns `""` on no match.
- Direct Rust `mapping_functions::text::regex_extract` returns `Ok(None)` on no match.
- Python, WASM, and TypeScript public method names and JSON shapes are unchanged.
- `cel-mapper-core` re-exports preserve current public import paths for moved PublicSchema and evaluator types.
- Compile-time compatibility tests cover public types, serde behavior, generic bounds, `From` / `Into` impls, and documented re-export paths.
- `registry-relay` and `registry-witness` compile against the resulting API in the shared workspace.

Required verification:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test -p mapping-functions` passes with default features and with `--all-features`.
- `cargo test -p mapping-functions-cel` passes.
- `cargo test -p cel-evaluator` passes.
- `cargo test -p publicschema-mapping` passes.
- `cargo test -p cel-mapper-core -p cel-mapper-wasm` passes.
- Python binding tests pass through the project-supported `maturin develop` plus `pytest` flow, or `cargo test -p cel-mapper-python` if that is the supported local command.
- `cd packages/js && npm ci && npm run build:wasm && npm run build:ts && npm test` passes.
- Existing v0.1 mapping fixtures and PublicSchema golden parity fixtures pass without output changes except accepted diagnostic wording changes recorded in `docs/crate-split-inventory.md`.
- No unrelated files, generated artifacts, lockfiles, or snapshots are changed unless the PR explicitly identifies why the change is required.

## 12. Risks And Mitigations

Risk: over-splitting creates more maintenance cost than value.  
Mitigation: extract only the required runtime crates in order: `mapping-functions`, `mapping-functions-cel`, `cel-evaluator`, and `publicschema-mapping`. Defer only `mapping-bundle`.

Risk: dependency cycles between `cel-mapper-core` and `publicschema-mapping`.  
Mitigation: extract `cel-evaluator` before moving PublicSchema code, then make both crates depend on the evaluator boundary.

Risk: pure helper behavior diverges from CEL behavior.  
Mitigation: adapter tests must compare CEL calls against direct `mapping-functions` calls for representative inputs and explicitly assert any compatibility mapping such as `None` to `""`.

Risk: request-scoped fallback behavior leaks back into pure helpers.  
Mitigation: pure helper APIs accept explicit fallback arguments, and adapter tests cover `country`, `timezone`, and `today` resolution.

Risk: bindings accidentally change JSON shapes.  
Mitigation: keep bindings on `cel-mapper-core` re-exports during the first migration and run binding-level tests before cleanup.

Risk: downstream registry projects import internal modules.  
Mitigation: expose explicit public crates and document imports for `registry-relay` and `registry-witness`.

## 13. Decisions And Remaining Open Questions

Resolved for v0.3:

- `cel-evaluator` is required before `publicschema-mapping` extraction.
- Default `mapping-functions` features are `std`, `text`, `regex`, `email`, `ids`, `json`, and `codes`.
- `FunctionError` codes are a public API contract in v0.3; prose messages are not.
- Rust helper naming uses `remove_accents`; CEL compatibility keeps `text_remove_accents`.
- Direct Rust `regex_extract` returns `Ok(None)` on no match; CEL `text_regex_extract` maps no match to `""`.
- `MappingOutput` and v0.1 mapping document errors stay in `cel-mapper-core`.
- `cel::ExecutionError` is adapter/evaluator-internal and is converted before crossing public crate boundaries.
- Code-system YAML parsing stays above `mapping-functions`; the leaf crate accepts typed code-system documents.

Remaining open questions:

- Should `cel-mapper-core` remain the long-term facade crate, or should callers eventually import `publicschema-mapping` and `mapping-functions` directly?
- Should `cel-evaluator` expose only JSON input/output, or should it expose lower-level CEL `Value` APIs for internal callers?
- Should `mapping-functions` expose JSON helpers through `serde_json::Value`, or should JSON-related helpers live in a separate crate later?

## 14. Recommended First Slice

Start with text/string helpers.

Reason:

- They are the user's explicit reuse target.
- They have broad utility in registry import/export work.
- They are easy to test without PublicSchema fixtures.
- They prove the pure-helper plus CEL-adapter pattern before touching date, phone, code-system, and PublicSchema runtime boundaries.

First-slice acceptance:

- `mapping-functions::text` implements `trim`, `lower_ascii`, `upper_ascii`, `title_simple`, `normalize_space`, `remove_accents`, `slug`, `regex_replace`, and `regex_extract`.
- Existing CEL helpers call the new text functions.
- Existing text helper behavior is unchanged.
- Direct Rust tests and CEL adapter tests both pass.
- README documents that `mapping-functions` is the crate to reuse outside CEL.

## 15. Implementation Plan And Wave Gates

Use one integration owner in every wave. Parallel workers may edit only their assigned modules and tests; the integration owner owns workspace manifests, crate manifests, crate `lib.rs` files, public re-exports, and final merge conflict resolution.

### Wave 0: Inventory And Workspace Groundwork

Parallel work:

- Worker A inventories helper semantics from `functions/builtins.rs`, including null, missing, no-match, fallback, and error-code behavior.
- Worker B inventories public types, trait impls, serde behavior, generic bounds, `From` / `Into` impls, and re-export paths.
- Worker C inventories PublicSchema and downstream couplings in `registry-relay` and `registry-witness`.
- Integration owner adds `workspace.package.rust-version = "1.83"` and creates `docs/crate-split-inventory.md`.

Wave 0 definition of done:

- `docs/crate-split-inventory.md` exists and contains all three inventories.
- Root `Cargo.toml` has `workspace.package.rust-version = "1.83"`.
- Feature flags, error-code stability, regex no-match semantics, and ownership decisions are recorded in the inventory.
- `cargo fmt --all -- --check` passes.

Review checkpoint:

- Staff engineer reviews dependency boundaries, inventory completeness, and manifest changes.
- Domain reviewer confirms helper semantics match real registry and government data workflows.
- Wave 1 cannot start until both reviews are approved.

### Wave 1: Extract `mapping-functions`

Parallel work:

- Worker A extracts text, email, ID, and redaction helpers with tests.
- Worker B extracts phone and date helpers behind non-default features with tests.
- Worker C extracts code-system types, ISO loading, and JSON helpers with tests.
- Integration owner creates `crates/mapping-functions`, features, public modules, workspace membership, and compatibility calls from existing core helper code.

Wave 1 definition of done:

- `mapping-functions` builds with default features and `--all-features`.
- `mapping-functions` has no forbidden dependencies listed in Section 11.
- `CodeSystemRegistry`, `CodeEntry`, typed code-system documents, and ISO loading live in `mapping-functions::codes`.
- CEL behavior is unchanged because existing core helper entry points call the extracted helpers.
- `cargo test -p mapping-functions` and `cargo test -p mapping-functions --all-features` pass.
- Existing `cargo test -p cel-mapper-core` passes.

Review checkpoint:

- Reviewer compares helper inventory against moved modules and confirms every moved helper has tests.
- Reviewer confirms `serde_yaml` is not in the default `mapping-functions` dependency graph.
- Wave 2 cannot start until helper behavior parity is approved.

### Wave 2: Extract `mapping-functions-cel`

Parallel work:

- Worker A moves CEL registration and helper metadata.
- Worker B ports text, email, phone, and date adapter tests.
- Worker C ports code-system, ID, redaction, validation, null, missing, and error tests.
- Integration owner creates `crates/mapping-functions-cel`, features, public API, and the `cel_mapper_core::functions::register_stdlib` compatibility shim.

Wave 2 definition of done:

- `mapping-functions-cel` owns CEL `Value` conversion, arity validation, helper metadata, and fallback resolution for `country`, `timezone`, `today`, and warnings.
- `mapping-functions-cel` does not depend on `cel-evaluator`.
- Adapter tests assert `text_regex_extract` returns `""` on no match through CEL.
- Direct Rust tests still assert `mapping_functions::text::regex_extract` returns `Ok(None)` on no match.
- `cargo test -p mapping-functions-cel` and `cargo test -p cel-mapper-core` pass.

Review checkpoint:

- Reviewer confirms helper metadata and CEL compatibility names are unchanged.
- Reviewer confirms request-context fallback resolution is in the adapter boundary.
- Wave 3 cannot start until CEL parity is approved.

### Wave 3: Extract `cel-evaluator`

Parallel work:

- Worker A moves expression compile, evaluate, missing-value, path, and output conversion internals needed by standalone CEL.
- Worker B moves preview diagnostics, security limits, budget handling, request context, and expression errors.
- Worker C updates `cel-mapper-core` compatibility wrappers and `registry-witness` compile checks.
- Integration owner creates `crates/cel-evaluator`, public API, workspace membership, and re-exports.

Wave 3 definition of done:

- `cel-evaluator` does not depend on `cel-mapper-core` or `publicschema-mapping`.
- `cel-evaluator` public APIs do not expose `cel::ExecutionError`.
- `MappingOutput`, `EvaluationInput`, and v0.1 mapping document errors remain in `cel-mapper-core`.
- Standalone expression evaluation and preview results are unchanged against existing fixtures.
- `cargo test -p cel-evaluator`, `cargo test -p cel-mapper-core`, and the `registry-witness` compile check pass.

Review checkpoint:

- Reviewer confirms no dependency cycle exists and no `cel::ExecutionError` crosses public crate boundaries.
- Reviewer confirms standalone expression JSON and diagnostic shapes are unchanged.
- Wave 4 cannot start until evaluator compatibility is approved.

### Wave 4: Extract `publicschema-mapping`

Parallel work:

- Worker A moves PublicSchema document parsing, compile, hash, and metadata.
- Worker B moves evaluate, pointer writes, value mappings, logs, warnings, privacy, and diagnostics.
- Worker C updates Python, WASM, TypeScript, and `registry-relay` imports through compatibility re-exports.
- Integration owner creates `crates/publicschema-mapping`, public API, workspace membership, and `cel-mapper-core` re-exports.

Wave 4 definition of done:

- `publicschema-mapping` depends on `cel-evaluator` and `mapping-functions`.
- `publicschema-mapping` does not depend on `cel-mapper-core` or `mapping-functions-cel`.
- PublicSchema golden parity fixtures pass without output changes except accepted diagnostic wording changes.
- `cel-mapper-core` compatibility imports for PublicSchema types compile.
- `cargo test -p publicschema-mapping`, `cargo test -p cel-mapper-core`, and the `registry-relay` compile check pass.

Review checkpoint:

- Reviewer confirms PublicSchema runtime behavior, logs, hashes, and privacy diagnostics match pre-extraction fixtures.
- Reviewer confirms compatibility re-exports cover current Rust, Python, WASM, and TypeScript call paths.
- Wave 5 cannot start until PublicSchema parity is approved.

### Wave 5: Bindings And Downstream Validation

Parallel work:

- Worker A validates and updates `registry-relay`.
- Worker B validates and updates `registry-witness`.
- Worker C validates Python, WASM, TypeScript wrappers, examples, and package docs.
- Integration owner updates README, crate docs, migration notes, and compatibility tests.

Wave 5 definition of done:

- `registry-relay` compiles against the resulting API.
- `registry-witness` compiles against the resulting API.
- Python method names and JSON shapes are unchanged in tests.
- WASM and TypeScript method names and JSON shapes are unchanged in tests.
- README, crate docs, and migration notes describe new crate boundaries and compatibility imports.
- Full verification requirements in Section 11 pass.

Review checkpoint:

- Reviewer confirms downstream changes do not duplicate mapping semantics.
- Reviewer confirms public binding surfaces are unchanged.
- Wave 6 cannot start until downstream and binding validation is approved.

### Wave 6: Compatibility Cleanup

Parallel work:

- Worker A audits deprecated compatibility re-exports and docs.
- Worker B audits release notes, migration notes, and inventory updates.
- Worker C reruns downstream compile checks and fixture parity checks.
- Integration owner removes only approved internal shims and finalizes release metadata.

Wave 6 definition of done:

- Only compatibility shims with documented replacements are removed.
- Remaining compatibility re-exports are documented with migration paths.
- `docs/crate-split-inventory.md` and `CHANGELOG.md` record every accepted diagnostic wording or public import change.
- Full Definition of Done in Section 11 is satisfied.

Final review checkpoint:

- Staff engineer signs off on dependency graph, public API compatibility, and full verification output.
- Domain reviewer signs off on helper semantics, PublicSchema parity, and downstream registry fit.
- Work is marked complete only after both final reviews and all Section 11 verification requirements pass.
