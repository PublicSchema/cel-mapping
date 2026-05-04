# PublicSchema v0.2 Implementation Plan

Status: draft  
Spec: [`spec-publicschema-v0.2.md`](./spec-publicschema-v0.2.md)  
Primary goal: make `cel-mapping` the canonical PublicSchema transform runtime without breaking the existing v0.1 `records` API.

## 1. Operating Principles

- Implement PublicSchema mode as an additive path.
- Start inside `cel_mapper_core::publicschema`; defer a separate crate until after hosted runtime migration.
- Expose Rust, Python, and WASM/TypeScript APIs in Phase 1, because Workbench preview depends on WASM and `publicschema-build` / generated Python validators depend on Python.
- Treat Rust core as the behavioral oracle.
- Port helper behavior before claiming parity.
- Never silently fall back to identity when a non-identity formula fails.
- Keep `celpy` and `@publicschema/cel-js` only as temporary migration fallbacks; they are not peer runtimes.

## 2. Phase Gates

### Phase 0 Gate

Phase 1 cannot start until:

- The parity fixture format is committed.
- The first golden corpus fixtures exist.
- Current `celext v1` helper inventory is listed.
- The hosted `celpy` behavioral-diff template exists.
- Privacy modes are represented in API option structs.

### Phase 1 Gate

Phase 2 cannot start until:

- Rust core can compile and evaluate minimal `property_mappings`.
- Python and WASM bindings expose the new PublicSchema APIs.
- Golden corpus tests pass in Rust, Python, and WASM for the minimal cases.
- Wrong-direction formula tests prove no identity fallback.

### Phase 3 Gate

`publicschema-build` integration cannot ship generated artifacts until:

- `mapping_hash` is implemented or intentionally excluded from generated artifact identity.
- Code-system loading behavior is implemented.
- Helper registry version is included in compile metadata.

### Phase 4 Gate

Hosted runtime migration cannot start until:

- The `celpy` behavioral diff is complete and accepted.
- A feature flag and rollback path are implemented.
- Shadow-run or A/B parity plan is documented.

## 3. Phase 0: Parity And Compatibility Groundwork

### 3.1 Add Golden Corpus Harness

Create:

```text
crates/cel-mapper-core/tests/publicschema_parity.rs
tests/fixtures/publicschema-parity/*.json
```

Fixture schema:

```json
{
  "name": "odk-person-basic",
  "mapping": {},
  "direction": "fieldbridge-odk->profile",
  "source_data": {},
  "ctx": {},
  "options": {
    "privacy": "authoring",
    "include_resolved_values": true,
    "error_mode": "collect"
  },
  "expected": {
    "ok": true,
    "output": {},
    "log": [],
    "errors": [],
    "warnings": []
  }
}
```

Initial fixtures:

- `identity-copy.json`: no formula, source pointer to target pointer.
- `explicit-source-identity.json`: `to_target.expression = "source"`.
- `wrong-direction-no-identity.json`: only `from_target` exists while executing forward.
- `missing-optional-omits.json`: missing optional source omits output.
- `missing-required-fails.json`: missing required source fails.
- `formula-error-fails-closed.json`: bad formula does not copy input.
- `array-append.json`: target path `/identifiers/-`.
- `duplicate-target-last-write.json`: two rules write same target, last wins with authoring warning.
- `ctx-timezone.json`: context binding preserves caller value and fills runtime defaults.
- `code-map-preloaded.json`: code lookup uses preloaded registry only.

Acceptance:

- Rust test loader can run all fixtures and compare canonical JSON.
- Equality ignores only declared volatile fields.

### 3.2 Inventory And Port Helpers

Create:

```text
crates/cel-mapper-core/src/functions/publicschema.rs
crates/cel-mapper-core/tests/publicschema_helpers.rs
docs/publicschema-helper-parity.md
```

Work:

- Inventory current `celext v1` helpers from `publicschema-build`.
- Map helper names, aliases, argument behavior, null/missing behavior, return type, and examples.
- Implement or alias missing helpers in Rust.
- Export helper registry metadata as JSON from Rust.

Initial helper groups:

- text: trim, lower/upper, defaulting, coalesce
- date/time: parseDate, formatDate, parseDateTime, formatDateTime
- phone: normalizePhone
- code systems: lookupCode, mapCode, iso3166, iso4217, iso639
- regex: regexMatch, regexExtract
- names/references where currently used by generated mappings

Acceptance:

- Helper parity tests pass for representative null, missing, invalid-type, and valid inputs.
- Metadata includes `name`, `aliases`, `parameters`, `return_type`, `deterministic`, `missing_behavior`, and `examples`.

### 3.3 Behavioral Diff Template

Create:

```text
docs/publicschema-celpy-behavioral-diff.md
```

Sections:

- missing vs null
- JSON Pointer reads/writes
- date/time formats
- numeric conversion and JSON safe integer range
- helper behavior differences
- error wording and redaction
- diagnostic ordering
- transformation-log ordering
- duplicate target writes
- intentional non-parity decisions

Acceptance:

- Template exists in Phase 0.
- It is filled before Phase 4.

## 4. Phase 1: PublicSchema Mode In Core And Bindings

### 4.1 Core Module Skeleton

Create:

```text
crates/cel-mapper-core/src/publicschema/mod.rs
crates/cel-mapper-core/src/publicschema/document.rs
crates/cel-mapper-core/src/publicschema/compile.rs
crates/cel-mapper-core/src/publicschema/evaluate.rs
crates/cel-mapper-core/src/publicschema/pointer.rs
crates/cel-mapper-core/src/publicschema/bindings.rs
crates/cel-mapper-core/src/publicschema/output.rs
crates/cel-mapper-core/src/publicschema/hash.rs
```

Update:

```text
crates/cel-mapper-core/src/lib.rs
crates/cel-mapper-core/src/runtime.rs
```

Public API types:

```rust
pub struct PublicSchemaMappingDocument;
pub struct CompiledPublicSchemaMapping;
pub struct PublicSchemaCompileOptions;
pub struct PublicSchemaEvaluateOptions;
pub struct PublicSchemaEvaluationInput;
pub struct PublicSchemaTransformOutput;
pub struct PublicSchemaRuleLogEntry;
pub enum PublicSchemaBindingMode;
pub enum PrivacyMode;
```

Runtime methods:

```rust
impl MappingRuntime {
    pub fn compile_publicschema_mapping(
        &self,
        source: &str,
        options: PublicSchemaCompileOptions,
    ) -> Result<CompiledPublicSchemaMapping, CompileError>;

    pub fn evaluate_publicschema_mapping(
        &self,
        mapping: &CompiledPublicSchemaMapping,
        input: PublicSchemaEvaluationInput,
    ) -> PublicSchemaTransformOutput;

    pub fn preview_publicschema_rule_expression(
        &self,
        rule: &PublicSchemaPropertyMapping,
        input: PublicSchemaEvaluationInput,
    ) -> ExpressionPreviewResult;
}
```

Acceptance:

- v0.1 `compile_mapping` and `evaluate` behavior remains unchanged.
- PublicSchema documents can be parsed from YAML and JSON.
- `property_mappings` + `version: "0.1"` is rejected.
- Documents containing both `records` and `property_mappings` are rejected.

### 4.2 JSON Pointer Engine

Implement in `publicschema/pointer.rs`:

- RFC 6901 decode.
- Source read with missing sentinel status.
- Target write with object/array creation.
- Array append with `-`.
- Padding behavior for missing arrays.
- Duplicate-target detection for authoring warnings.

Acceptance:

- Fixtures cover nested objects, arrays, append, missing source, mixed-type write errors, duplicate writes.

### 4.3 Direction And Formula Selection

Implement:

- Direction parser for `source->target`.
- Forward direction uses `to_target`.
- Reverse direction uses `from_target`.
- Any other direction errors.
- Opposite direction with one-direction formula errors.
- No `formula` entry means identity.
- `source` string means explicit identity.

Acceptance:

- Wrong-direction fixture fails closed.
- No-formula fixture copies identity.
- Bad formula fixture does not copy identity.

### 4.4 Binding Construction

Implement `publicschema-v1` binding mode:

- `source`: resolved rule value.
- `root`: full input record.
- `ctx`: normalized context.
- `vars`: mapping/rule variables.
- `index`: null in non-loop v0.2.
- `target`: aliases root when transforming external input to profile.
- `profile`: aliases root when transforming profile input to external target.
- No partial-output binding.

Acceptance:

- Existing-style formula `source + " " + target.last_name` works in forward external-to-profile mode.
- Partial-output reads are impossible unless explicitly provided through `ctx`.

### 4.5 Output And Diagnostics

Implement `PublicSchemaTransformOutput`:

```json
{
  "ok": true,
  "output": {},
  "log": [],
  "warnings": [],
  "errors": []
}
```

Implement statuses:

- `applied`
- `defaulted`
- `omitted`
- `missing`
- `skipped`
- `formula_error`
- `write_error`
- `validation_error`

Acceptance:

- Production privacy suppresses resolved values.
- Authoring privacy includes resolved values only when requested.
- Debug privacy includes full diagnostics.

### 4.6 Deterministic Hash

Implement advisory `mapping_hash` in Phase 1 if cheap; otherwise expose `hash_status = "advisory-unimplemented"`.

Before Phase 3, implement canonical JSON SHA-256 per spec.

Acceptance:

- Hash tests prove object-key ordering does not affect hash.
- Property mapping order does affect hash.

### 4.7 Python Binding

Update:

```text
crates/cel-mapper-python/src/lib.rs
crates/cel-mapper-python/python/cel_mapper/__init__.pyi
crates/cel-mapper-python/tests/test_publicschema.py
crates/cel-mapper-python/examples/publicschema_transform.py
```

Expose:

```python
compile_publicschema_mapping(mapping: str, options: dict | None = None)
evaluate_publicschema_mapping(compiled, source_data, *, direction, context=None, options=None)
preview_publicschema_rule_expression(rule, source_data, *, direction, context=None, options=None)
```

Acceptance:

- Python parity fixtures pass.
- Python API returns plain dict/list JSON-compatible objects.

### 4.8 WASM And TypeScript Binding

Update:

```text
crates/cel-mapper-wasm/src/lib.rs
packages/js/src/index.ts
packages/js/README.md
```

WASM JSON-string methods:

```rust
compile_publicschema_mapping_meta_json(mapping: &str, options_json: &str) -> String
evaluate_publicschema_mapping_json(
    mapping: &str,
    source_json: &str,
    options_json: &str,
) -> String
preview_publicschema_rule_expression_json(
    rule_json: &str,
    source_json: &str,
    options_json: &str,
) -> String
```

TypeScript ergonomic methods:

```ts
evaluatePublicSchemaMapping(mapping, source, options)
previewPublicSchemaRuleExpression(rule, source, options)
getPublicSchemaHelperMetadata()
```

Acceptance:

- WASM parity fixtures pass.
- TypeScript types cover output/log/errors/privacy options.

## 5. Phase 2: Workbench Preview

Target repo:

```text
../publicschema.com/apps/workbench
```

Work:

- Add local dependency on `cel-mapping-js`.
- Configure Next/Vite handling for WASM asset.
- Add client-side runtime loader.
- Use `previewPublicSchemaRuleExpression` in CEL formula editor.
- Replace display-only preview in mapping panes where safe.
- Keep server mapping-test as authoritative until Phase 4.
- Use helper metadata for helper menu.

Likely files:

```text
components/CelFormulaInput.tsx
components/mappings/MappingPreviewPane.tsx
components/MappingRulePreview.tsx
components/mappings/MappingRulesTable.tsx
__tests__/components/CelFormulaInput.test.tsx
__tests__/components/MappingDesigner.test.tsx
```

Acceptance:

- Formula syntax errors render inline without throwing.
- Preview uses same binding semantics as transform.
- Tests mock WASM loader cleanly.

## 6. Phase 3: publicschema-build Integration

Target repo:

```text
../publicschema-build
```

Work:

- Add Python dependency on `cel-mapping`.
- Replace `build.cel.env` internals with `cel_mapper` facade while preserving public classes.
- Replace or supplement `build.cel.typecheck`.
- Generate PublicSchema mapping artifacts compatible with the new runtime.
- Include `mapping_hash`, helper registry version, and referenced paths.
- Keep current generated package APIs stable.

Likely files:

```text
build/cel/env.py
build/cel/typecheck.py
build/profile_compile.py
build/generators/py_validator.py
build/generators/js_validator.py
tests/profile_pipeline/test_cel_env.py
tests/profile_pipeline/test_generator_py_validator.py
tests/profile_pipeline/test_generator_js_validator.py
```

Acceptance:

- Existing profile pipeline tests pass.
- New compile diagnostics match Rust core parity fixtures.
- Generated mapping artifacts include deterministic metadata or explicitly omit it per Phase 3 decision.

## 7. Phase 4: Hosted Runtime Migration

Target repo:

```text
../publicschema.com/apps/core
```

Work:

- Add `cel-mapping` Python dependency.
- Add feature flag, e.g. `PUBLICSCHEMA_TRANSFORM_RUNTIME=celpy|cel-mapping|shadow`.
- Implement adapter from hosted bundle shape to `cel_mapper` inputs.
- Wire code-system registries from profile bundles.
- Replace transform execution path behind flag.
- Shadow-run and compare in non-production.
- Preserve response shape for `/v1/transform` and mapping-test where possible.

Likely files:

```text
apps/core/src/publicschema_core/validation/worker.py
apps/core/src/publicschema_core/api/v1/transform.py
apps/core/src/publicschema_core/api/v1/mapping_test.py
apps/core/src/publicschema_core/validation/cache.py
apps/core/tests/api/v1/test_transform.py
apps/core/tests/api/v1/test_mapping_test.py
```

Acceptance:

- Behavioral diff is accepted before code path flips.
- Feature flag can roll back to current runtime.
- Mapping-test includes authoring logs with resolved values when requested.
- Hosted transform defaults to production privacy.

## 8. Phase 5: Generated Python Validators

Target repo:

```text
../publicschema-build
```

Work:

- Update generated Python validator templates to depend on `cel-mapping`.
- Replace emitted `celpy` transform logic.
- Package mapping artifacts and code registries.
- Define wheel/release matrix.

Acceptance:

- Generated Python validator output matches hosted runtime for golden fixtures.
- Missing wheel/dependency errors are loud and actionable.
- Fail-closed formula behavior is preserved.

## 9. Phase 6: Generated JS Validators

Work:

- Build Node/bundler-compatible WASM target.
- Decide WASM packaging pattern for generated validators.
- Replace generated JS transform logic.
- Deprecate `@publicschema/cel-js` with removal milestone.

Acceptance:

- Generated JS validator parity passes for golden fixtures.
- WASM loading works in Node and supported bundlers.
- `@publicschema/cel-js` is no longer needed for generated transforms.

## 10. Phase 7: YARRRML/RML Export

Work:

- Add optional generator from PublicSchema mapping artifacts to YARRRML/RML.
- Keep RDF export outside the runtime core.
- Document unsupported constructs.

Acceptance:

- Export works for straightforward one-record mappings.
- Runtime behavior remains JSON/profile-first.

## 11. Test Matrix

Run after Phase 1:

```bash
cargo test -p cel-mapper-core -p cel-mapper-wasm
cd packages/js && npm run build:wasm && npm run build:ts
cd crates/cel-mapper-python && pytest -q
```

Run after Phase 3:

```bash
cd ../publicschema-build
uv run pytest
pnpm test
```

Run after Phase 4:

```bash
cd ../publicschema.com
uv run pytest apps/core/tests
pnpm --filter @publicschema/workbench test
```

## 12. Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Helper behavior diverges from current Python/JS implementations | Phase 0 helper inventory and parity fixtures |
| WASM loading blocks Workbench or generated JS validators | Expose Rust/Python first; defer generated JS parity to Phase 6 |
| Hosted runtime breaks production transforms | Feature flag, shadow-run, behavioral diff, rollback |
| Partial-output dependency requests reappear | Keep v0.2 no-partial-output invariant; defer explicit dependency model to v0.3 |
| Wheel packaging slows generated Python validator adoption | Keep hosted runtime migration separate; define wheel matrix before Phase 5 ships |
| Code-system lookup accidentally performs I/O | Restrict helpers to preloaded registries; test missing registry behavior |

## 13. First Implementation Slice

Start with a narrow vertical slice:

1. Add `publicschema` module and document structs.
2. Parse YAML/JSON `property_mappings`.
3. Implement JSON Pointer read/write for objects and append arrays.
4. Implement forward `to_target` formula selection.
5. Evaluate `source` identity and simple CEL formula.
6. Return `output`, `log`, `warnings`, `errors`.
7. Add Rust parity fixtures for identity, explicit source, missing optional, bad formula.
8. Add Python and WASM wrappers for one-shot evaluation.

This slice proves the architecture without waiting on all helpers, generated artifacts, or hosted migration.
