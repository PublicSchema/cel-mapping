# cel-mapping PublicSchema v0.2 Refactor Spec

Status: draft  
Audience: `cel-mapping`, `publicschema-build`, `publicschema.com`, generated validator packages  
Scope: refactor `cel-mapping` into the native deterministic mapping runtime for PublicSchema

## 1. Purpose

`cel-mapping` v0.2 should become the canonical runtime and compiler for PublicSchema transformation semantics.

The current v0.1 runtime is a good generic CEL mapping engine, but PublicSchema needs more than generic record emission. It needs a runtime that understands profile/system mappings, JSON Pointer reads and writes, directional formulas, authoring diagnostics, transformation logs, quality metadata, generated validator parity, and hosted API behavior.

The goal is not to discard PublicSchema's mapping model. The goal is to move its execution semantics into one deterministic Rust core with Python and JavaScript/WASM bindings.

```text
PublicSchema mapping YAML
        |
        v
cel-mapping compile
        |
        v
compiled mapping / diagnostics / manifest
        |
        +--> Workbench preview
        +--> publicschema-build validation and generated artifacts
        +--> hosted /v1/transform and mapping-test APIs
        +--> generated Python validators
        +--> generated JS/WASM validators
```

## 2. Design Decision

PublicSchema's mapping shape remains the authoring and API contract.

`cel-mapping` becomes the native execution engine for that contract.

This means v0.2 must support PublicSchema-style `property_mappings[]` directly, rather than requiring every caller to translate into the v0.1 `records.*.fields` format.

The v0.1 `records` format may remain useful for generic ETL-style record emission, but the PublicSchema mode is first-class.

PublicSchema mode is additive. The v0.1 `records` API remains supported indefinitely unless a future major version explicitly deprecates it.

## 3. Non-goals

- Do not make RDF triples the core runtime abstraction.
- Do not adopt YARRRML as the native PublicSchema mapping format.
- Do not preserve Python `celpy` or handwritten JS CEL evaluators as peer runtimes.
- Do not require Workbench or generated validators to implement mapping semantics locally.
- Do not hide runtime behavior behind product-specific code in `publicschema.com`.

## 4. Relationship To YARRRML

YARRRML is an important adjacent standard and should influence interoperability work, but it should not define the native PublicSchema transform runtime.

YARRRML is designed as a YAML representation for RML/R2RML/RDF generation. Its central concepts are RDF subjects, predicates, objects, classes, datatypes, sources, iterators, function maps, and predicate-object maps.

PublicSchema's runtime problem is primarily:

```text
source JSON/system payload -> profile/canonical JSON object
```

with validation, deterministic diagnostics, release gating, generated validators, and hosted transform APIs.

Therefore:

- PublicSchema mappings MAY export to YARRRML/RML when targeting RDF or JSON-LD artifacts.
- `cel-mapping` MAY borrow ideas from YARRRML, such as sources, iterators, conditions, external references, and function metadata.
- `cel-mapping` MUST NOT make `predicateobjects` or RDF triples the core transform model.
- CEL remains the primary expression language for PublicSchema transforms.

## 5. Core Concepts

### 5.1 Mapping Document

A PublicSchema runtime mapping document describes a transformation between a source system and a target system.

Minimum shape:

```yaml
version: "0.2"
id: odk-intake-to-profile
source: fieldbridge-odk
target: profile
runtime:
  bindings: publicschema-v1
property_mappings:
  - rule_id: given-name
    source: /answers/respondent_first
    target: /given_name
    quality: exact
    formula:
      to_target:
        expression: source
```

The runtime MUST support this shape directly.

Version rules:

- Documents with `property_mappings` are PublicSchema runtime mappings.
- `version: "0.2"` selects this spec.
- Missing `version` on a document with `property_mappings` is accepted in compatibility mode and treated as `"0.2"` with a warning.
- `version: "0.1"` with `property_mappings` is an error.
- Documents with `records` and no `property_mappings` use the existing v0.1 compiler.
- Documents containing both `records` and `property_mappings` are an error unless a future version explicitly defines mixed mode.

### 5.2 Property Mapping

A property mapping is the primary executable unit.

Required or recommended fields:

```yaml
rule_id: given-name
profile_concept: Person
profile_property: given_name
source: /answers/respondent_first
target: /given_name
direction: bidirectional
quality: exact
formula:
  to_target:
    expression: source
  from_target:
    expression: source
```

Runtime-significant fields:

- `rule_id`: stable identifier used in logs and diagnostics.
- `source`: JSON Pointer into the input record.
- `target`: JSON Pointer into the output record.
- `formula`: optional CEL expression by direction.
- `value_mappings`: optional crosswalk rows applied after formula/identity evaluation. Forward execution maps `source_value` to `target_value`; reverse execution maps `target_value` back to `source_value`.
- `direction`: optional authoring metadata. Execution chooses a concrete direction.
- `quality`: copied into logs and diagnostics.
- `required`: optional boolean controlling missing/error behavior.
- `on_missing`: optional policy for absent source values.
- `on_error`: optional policy for formula/runtime failures.
- `default`: optional fallback value.

Authoring metadata such as `profile_concept`, `profile_property`, `target_path`, `notes`, `rationale`, and provenance MAY be preserved by compile metadata and logs in later phases. It does not affect v0.2 expression evaluation directly.

### 5.3 Direction

The runtime MUST compile and evaluate a mapping for an explicit direction.

Example direction:

```text
fieldbridge-odk -> profile
```

Formula selection rules:

- If execution direction equals the mapping document's `source -> target`, prefer `formula.to_target.expression`.
- If execution direction equals the mapping document's `target -> source`, prefer `formula.from_target.expression`.
- Any other execution direction is an error.
- If only one directional expression is present, the opposite direction is undefined and MUST NOT fall back to identity.
- If no `formula` entry of any kind is present on the rule, the rule is an identity copy from the resolved source value.
- The string expression `source` is an explicit identity formula.

Example:

```yaml
source: fieldbridge-odk
target: profile
formula:
  to_target:
    expression: source
  from_target:
    expression: profile.given_name
```

For `fieldbridge-odk -> profile`, use `to_target`. For `profile -> fieldbridge-odk`, use `from_target`.

If the formula direction is ambiguous or missing for a non-identity transform, the compiler MUST emit an error for execution contexts and SHOULD emit a structured diagnostic for authoring contexts.

### 5.4 Evaluation Order

Rules are evaluated in document order.

v0.2 MUST NOT expose partially built output to CEL expressions. This preserves v0.1's principle that CEL expressions cannot depend on output construction order.

For v0.2:

- `profile` and `target`, when present, refer to input-side compatibility aliases, not partially built output.
- Multiple rules may write to the same target pointer; default behavior is last-write-wins in document order, with a warning diagnostic in authoring contexts.
- A future v0.3 MAY introduce explicit dependencies and partial-output reads, but only with a declared dependency model.

## 6. Binding Semantics

Bindings must be standardized now, before existing accidental names become permanent.

### 6.1 Rule-level Bindings

For each property mapping rule:

- `source`: value resolved from the rule's `source` JSON Pointer.
- `root`: full input record.
- `ctx`: host-provided context object.
- `vars`: object containing variables declared for the mapping or rule. If none are declared, `{}`.
- `index`: loop index when iterating, otherwise `null`.

For PublicSchema compatibility:

- `profile`: compatibility alias to the full input record when the input is a profile/canonical record; otherwise absent unless compatibility mode defines it.
- `target`: compatibility alias to the full input record when the input is an external target/source-system record; otherwise absent unless compatibility mode defines it.

When the runtime is transforming an external source-system record to a profile/canonical output:

- `source` is the resolved input value.
- `root` is the full input record.
- `target` aliases `root` in `publicschema-v1` compatibility mode.
- `profile` is absent unless explicitly provided in `ctx`.

When the runtime is transforming a profile/canonical record to an external target-system output:

- `source` is the resolved profile value.
- `root` is the full profile input record.
- `profile` aliases `root` in `publicschema-v1` compatibility mode.
- `target` is absent unless explicitly provided in `ctx`.

### 6.2 Compatibility Mode

Current PublicSchema formulas may use:

```cel
source + " " + target.last_name
```

where `source` means the resolved source value and `target` means the full input record.

v0.2 MUST provide a compatibility mode for this behavior.

Recommended option:

```yaml
runtime:
  bindings: publicschema-v1
```

Binding mode precedence:

1. Explicit compile option wins.
2. `runtime.bindings` in the mapping document wins when no compile option is provided.
3. The PublicSchema compile entry point defaults to `publicschema-v1`.
4. The generic v0.1 compile entry point keeps v0.1 binding behavior.

Any document containing `property_mappings` and no explicit binding mode is treated as PublicSchema mode and defaults to `publicschema-v1`.

Future cleaner bindings MAY use names like:

- `value`: resolved source value.
- `input`: full input record.

But those names are not required for v0.2. A future `output` binding is allowed only if the runtime also introduces an explicit dependency model; v0.2 has no partial-output binding.

### 6.3 Context Binding

`ctx` is a JSON object supplied by the host. It follows v0.1 context semantics:

- If omitted or null, `ctx` is `{}`.
- If a non-object JSON value is supplied, it is wrapped as `{ "value": <input> }`.
- Runtime options such as timezone and locale MAY populate missing keys, but MUST NOT overwrite caller-provided keys.

### 6.4 Variables

v0.2 supports optional CEL variables at mapping and rule scope:

```yaml
vars:
  today: ctx.today
property_mappings:
  - rule_id: example
    vars:
      normalized: text.trim(source)
    source: /name
    target: /name
    formula:
      to_target:
        expression: vars.normalized
```

Rules:

- Mapping-level vars are evaluated once per transform before rules.
- Rule-level vars are evaluated after the rule source pointer is resolved and before the rule formula.
- Rule-level vars override mapping-level vars with the same key for that rule only.
- Vars are evaluated in document order within their scope.
- Var failures follow `on_error` for the current rule when rule-scoped, and mapping-level error mode when mapping-scoped.

## 7. JSON Pointer Semantics

PublicSchema mappings are pointer-to-pointer transforms. v0.2 must make JSON Pointer behavior first-class.

### 7.1 Source Reads

`source` paths MUST be RFC 6901 JSON Pointers.

Rules:

- Empty string `""` MAY represent the whole input document only if explicitly allowed.
- Slash-prefixed paths such as `/given_name` and `/answers/respondent_first` are normal.
- `~0` decodes to `~`.
- `~1` decodes to `/`.
- Numeric segments index arrays.
- Missing paths produce a structured missing status, not an immediate host panic.
- Missing values propagate into CEL as the same internal Missing sentinel used by v0.1, so helpers can distinguish missing from JSON `null` where supported.

### 7.2 Target Writes

`target` paths MUST be RFC 6901 JSON Pointers.

Rules:

- Missing intermediate objects are created.
- Numeric segments index arrays only when the parent is already an array.
- `-` appends to an array.
- Mixed-type writes produce deterministic errors.
- Out-of-bounds array writes produce deterministic errors.
- Null values may be written, omitted, or defaulted according to rule policy.
- If a missing intermediate node must be created, the next segment decides the container type: numeric or `-` creates an array; any other segment creates an object.
- Writing `/items/3` when `/items` does not exist creates `items` as an array, pads indices `0..2` with `null`, and writes index `3`.
- Writing `/items/3` when `/items` exists as a non-array is a write error.
- Writing an array index greater than `len` on an existing array pads with `null` up to that index.

### 7.3 Path Diagnostics

All runtime issues SHOULD include:

- `rule_id`
- `source_path`
- `target_path`
- expression path when relevant
- JSON Pointer path when relevant
- line and column for CEL syntax issues when available

## 8. Error And Missing Policies

### 8.1 Mapping-level Error Mode

The runtime SHOULD continue to support mapping-level error modes:

- `strict`: stop on first error.
- `collect`: evaluate as much as possible and collect errors.
- `lenient`: downgrade optional field errors to warnings where configured.

PublicSchema v0.2 inherits v0.1 strict behavior: strict mode stops at the first error. Authoring tools that need a full log SHOULD use collect mode.

### 8.2 Rule-level Policies

Each property mapping MAY define:

```yaml
required: true
on_missing: fail | omit | null | default
on_error: fail | omit | null | default
default: "unknown"
```

Default policy:

- Missing optional source: `omit`.
- Missing required source: `fail`.
- Formula error in strict mode: `fail`.
- Formula error in collect mode: collect error and omit output for that rule.
- Formula error in lenient mode: warning plus `null` or `omit`, depending on `on_error`.

The runtime MUST never silently fall back to identity when a non-identity formula fails.

Clarifications:

- `default` is an explicit fallback value and is allowed.
- Identity-on-error is forbidden.
- "No formula" means no `formula` entry of any kind on the rule.
- A formula present only for the opposite direction means the current direction is undefined and MUST NOT be treated as no-formula identity.

### 8.3 Status Values

Rule log status values SHOULD include:

- `applied`
- `defaulted`
- `omitted`
- `missing`
- `skipped`
- `value_unmapped`
- `formula_error`
- `write_error`
- `validation_error`

Status precedence:

- `formula_error`: CEL compile/evaluation failed, including helper-thrown validation functions such as `require(...)` or `validate.required(...)`.
- `value_unmapped`: a `value_mappings` crosswalk had no row for the resolved scalar value, or reverse execution found multiple source values for the same target value.
- `write_error`: expression succeeded but target JSON Pointer write failed.
- `validation_error`: reserved for future post-transform validation failures attached to a rule, not for CEL helper failures.

## 9. Runtime Output

Transform evaluation MUST return structured output suitable for Workbench, hosted APIs, and generated validators.

Recommended JSON shape:

```json
{
  "ok": true,
  "output": {
    "given_name": "Amina"
  },
  "log": [
    {
      "rule_id": "given-name",
      "source_path": "/answers/respondent_first",
      "target_path": "/given_name",
      "expression": "source",
      "resolved_input": "Amina",
      "resolved_output": "Amina",
      "status": "applied",
      "quality": "exact",
      "issues": []
    }
  ],
  "warnings": [],
  "errors": []
}
```

Fields:

- `ok`: false when error-severity diagnostics prevent a valid transform.
- `output`: transformed JSON object.
- `log`: one entry per evaluated or skipped rule.
- `warnings`: mapping-level warning diagnostics.
- `errors`: mapping-level error diagnostics.

Privacy mode MUST allow callers to suppress `resolved_input` and `resolved_output`.

Hosted runtime defaults SHOULD avoid echoing PII in error messages. Mapping-test and local development modes MAY include resolved values when explicitly requested.

### 9.1 Privacy Modes

Privacy modes are:

- `production`: suppress `resolved_input`, suppress `resolved_output`, redact source values from error messages.
- `authoring`: include resolved values when `include_resolved_values` is true; redact exception messages that may contain source values.
- `debug`: include resolved values and full diagnostics; never use by default in hosted production.

Default privacy mode:

- Hosted transform: `production`.
- Mapping-test endpoint: `authoring`.
- Workbench local preview: `authoring`.
- Generated validators: `production`.
- Local CLI/test harness: `debug` only when explicitly requested.

## 10. Expression Preview

Expression preview is a first-class editor feature.

The existing v0.1 standalone preview API remains available:

```text
preview_expression(expr, source, ctx, bindings) -> ExpressionPreviewResult
```

This is the PublicSchema-aware binding wrapper around the existing v0.1 `preview_cel_expression` behavior. It uses the same parser, standard library, Missing sentinel, limits, and structured issue shape.

For PublicSchema rule preview:

```text
preview_rule_expression(mapping_rule_object, sample_record, direction, ctx) -> ExpressionPreviewResult
```

`mapping_rule_object` is the parsed property-mapping object, not a rule id or YAML fragment. It MUST contain enough fields to resolve the rule source pointer and select the directional formula.

Preview MUST:

- Return structured syntax errors without throwing.
- Return line/column when available.
- Return rewritten expression when the runtime rewrites namespaced calls.
- Evaluate against the same bindings used by transform.
- Include notes helpful to editors and AI tools.
- Include source/target path hints when available.

Workbench should use this API for inline formula feedback.

## 11. Compile API

`cel-mapping` should expose compile APIs for both generic v0.1 records and PublicSchema v0.2 property mappings.

Rust:

```rust
let rt = MappingRuntime::new(RuntimeOptions::default());
let compiled = rt.compile_publicschema_mapping(yaml, CompileOptions::default())?;
let out = rt.evaluate_publicschema_mapping(&compiled, input)?;
```

Python:

```python
from cel_mapper import MappingRuntime

rt = MappingRuntime()
compiled = rt.compile_publicschema_mapping(mapping_yaml)
out = rt.evaluate_publicschema_mapping(compiled, source_data, direction="odk->profile")
```

TypeScript/WASM:

```ts
const mapper = await CelMapper.create();
const out = mapper.evaluatePublicSchemaMapping(mappingYaml, sourceData, {
  direction: "fieldbridge-odk->profile",
  includeResolvedValues: true,
});
```

The PublicSchema APIs SHOULD initially live on the same public `MappingRuntime` / `CelMapper` wrapper for ergonomics, backed internally by a `publicschema` module. A separate crate is deferred; see package structure.

PublicSchema mapping documents MAY be supplied as YAML or JSON. The runtime detects the format by the first non-whitespace character: `{` or `[` means JSON; anything else is parsed as YAML.

Compile result metadata SHOULD include:

- mapping id
- version
- source system
- target system
- available directions
- rule count
- expression count
- warnings
- deterministic hash
- referenced source paths
- referenced target paths

For Phase 1, `available directions` and `expression count` are advisory and MAY be omitted from binding APIs. Before Phase 3 ships generated artifacts, the deterministic hash MUST be specified as below.

### 11.1 Deterministic Hash

`mapping_hash` is:

```text
sha256(canonical_json({
  "schema": "cel-mapping-publicschema-v0.2",
  "mapping_id": id,
  "version": version,
  "source": source,
  "target": target,
  "runtime_bindings": resolved_binding_mode,
  "helper_registry_version": helper_registry_version,
  "property_mappings": property_mappings_in_document_order
}))
```

Canonical JSON uses sorted object keys, UTF-8, no insignificant whitespace, and preserves array order. `property_mappings` are not sorted because document order is evaluation order.

If a binding cannot produce canonical JSON in Phase 1, it MUST label the hash as advisory and MUST NOT use it for cache identity or release determinism.

## 12. PublicSchema Build Integration

`publicschema-build` should use `cel-mapping` for:

- CEL expression compilation.
- Mapping formula diagnostics.
- Mapping artifact generation.
- Optional deterministic compiled mapping artifacts.
- Generated validator runtime dependencies.

The existing `build.cel.env` and `build.cel.typecheck` APIs MAY remain as compatibility facades, but they should delegate to `cel-mapping`.

Generated artifacts SHOULD include the canonical PublicSchema mapping JSON/YAML plus, optionally:

```text
mappings/<id>.json
mappings/<id>.compiled.json
mappings/<id>.manifest.json
```

The compiled artifact format should be stable enough for generated validators but not necessarily hand-authored.

Phase 3 MUST NOT ship until deterministic hashing is implemented or explicitly marked out of scope for generated artifacts.

## 13. Hosted PublicSchema Integration

`publicschema.com/apps/core` should use `cel-mapping` for:

- `/v1/transform/...`
- mapping test endpoint
- draft branch compile checks
- formula preview endpoints
- release readiness gate for CEL type checks

The hosted worker should not use `celpy` for PublicSchema mapping transforms after the migration.

The hosted worker should preserve current API response compatibility where possible.

Before Phase 4 starts, the project MUST produce a written behavioral diff between the current `celpy` transform path and `cel-mapping`. The diff MUST cover:

- missing-field semantics
- `null` vs missing behavior
- JSON Pointer read/write edge cases
- date/time helper formats
- numeric conversion behavior
- helper-function behavior differences
- error wording and redaction
- error/warning ordering
- transformation-log ordering
- duplicate target writes

Phase 4 is gated on approving this diff and recording intentional differences.

Mapping-test mode SHOULD call the runtime with:

```text
include_log = true
include_resolved_values = true
privacy = authoring
```

Hosted transform mode SHOULD call the runtime with:

```text
include_log = request option
include_resolved_values = false by default
privacy = production
```

Phase 4 rollout MUST include a feature flag and rollback path:

- shadow-run `cel-mapping` beside the current runtime where feasible
- compare canonical outputs and diagnostics against parity fixtures
- flip production traffic only after parity gates pass
- retain the feature flag until generated validators and hosted runtime share the same engine

## 14. Generated Validator Integration

Generated Python validators SHOULD use the Python binding.

Generated JavaScript validators SHOULD use the WASM binding once Node/bundler packaging is settled.

Until Phase 6 lands, `@publicschema/cel-js` MAY remain as a deprecated temporary fallback for generated JS validators. It is not a peer runtime, and Phase 6 MUST set a removal milestone.

Parity tests MUST compare:

- Rust core
- Python binding
- WASM/TypeScript binding
- hosted worker output
- generated Python validator output
- generated JS validator output, when enabled

### 14.1 Parity Oracle

Parity tests use a golden corpus checked into the repository.

Recommended location:

```text
tests/fixtures/publicschema-parity/*.json
```

Fixture shape:

```json
{
  "name": "odk-person-basic",
  "mapping": { },
  "direction": "fieldbridge-odk->profile",
  "source_data": { },
  "ctx": { },
  "expected": {
    "ok": true,
    "output": { },
    "log": [ ],
    "errors": [ ],
    "warnings": [ ]
  }
}
```

Equality rules:

- Compare canonical JSON after sorting object keys.
- Preserve array order.
- Ignore elapsed time, runtime version strings, and non-stable diagnostic detail fields explicitly marked volatile.
- Normalize line endings in diagnostic messages.
- Rust core is the oracle for new behavior.
- The celpy behavioral diff defines intentional non-parity with the legacy runtime.

Generated JS validator parity is deferred until Phase 6; it MUST NOT block Phase 1.

## 15. Package Structure

Initial internal structure:

```text
crates/cel-mapper-core/src/publicschema
  PublicSchema mapping document model
  property_mappings compiler
  directional formula selection
  PublicSchema transform logs
  compatibility bindings
```

PublicSchema mode starts as a module inside `cel-mapper-core`.

A separate crate is deferred until after Phase 4, when the hosted runtime has migrated and the API boundary is proven.

Possible future split:

```text
crates/cel-mapper-core
  generic CEL evaluation
  JSON value conversion
  JSON Pointer read/write
  diagnostics
  security limits

crates/cel-mapper-publicschema
  PublicSchema mapping document model
  property_mappings compiler
  directional formula selection
  PublicSchema transform logs
  compatibility bindings

crates/cel-mapper-wasm
  WASM API over both generic and PublicSchema APIs

crates/cel-mapper-python
  Python API over both generic and PublicSchema APIs

packages/js
  ergonomic TypeScript wrapper
```

## 16. Security And Determinism

The runtime MUST maintain:

- no I/O inside CEL
- deterministic helper functions
- bounded expression length
- bounded output size
- bounded record count
- no unbounded recursion
- no hidden clock access except through `ctx`
- safe JSON integer handling across Rust, Python, and JavaScript

Code-system lookups MUST use preloaded registries, not network calls.

### 16.1 Code-System Loading

Code systems are loaded into the runtime before evaluation.

Allowed loading paths:

- Compile-time embedded registry from mapping document `code_systems`.
- Host-registered registry on `MappingRuntime` before compile.
- Precompiled bundle registry loaded by hosted API or generated validator at startup.

Not allowed:

- CEL helpers performing network, database, or filesystem lookup during evaluation.
- Per-expression lazy external I/O.

Runtime behavior:

- Compile merges mapping-local code systems over host-registered registries.
- Duplicate code-system keys are deterministic: mapping-local definitions override host defaults and emit a warning unless explicitly allowed.
- Hosted request handlers SHOULD reuse precompiled registries from the loaded profile bundle, not register code systems per rule.
- Generated validators SHOULD embed or package the registry alongside mapping artifacts.

## 17. Helper Functions

PublicSchema helper functions should be implemented once in Rust and exposed consistently to all bindings.

The helper registry should cover current PublicSchema `celext v1` helpers, including date/time, text, phone, code lookup/mapping, regex, and defaulting helpers.

Compatibility aliases MAY be provided for existing formulas.

Helper metadata SHOULD include:

- name
- aliases
- parameter names
- parameter types when known
- return type when known
- deterministic flag
- null/missing behavior
- examples

Workbench and AI tools should be able to consume this metadata for helper menus and suggestions.

Metadata SHOULD be exported as JSON from Rust. A concrete schema can land in Phase 2, but the Rust registry remains authoritative.

Porting current PublicSchema `celext v1` helpers to Rust is a Phase 0 prerequisite for non-trivial PublicSchema mapping parity.

## 18. Migration Plan

### Phase 0: Parity and compatibility groundwork

- Build the parity fixture harness and equality rules.
- Port current PublicSchema `celext v1` helper functions to Rust.
- Define helper behavior diffs against current Python/JS helper implementations.
- Define privacy modes in binding APIs.
- Produce the hosted `celpy` behavioral diff.
- Decide deterministic-hash implementation status for generated artifacts.

### Phase 1: PublicSchema mode in `cel-mapping`

- Add PublicSchema document structs.
- Add JSON Pointer read/write module if not already sufficient.
- Add directional formula selection.
- Add `evaluate_publicschema_mapping`.
- Add structured per-rule logs.
- Add compatibility bindings for current PS formulas.
- Expose PublicSchema APIs through Rust, Python, and WASM/TypeScript bindings.

### Phase 2: Workbench preview

- Use WASM `previewExpression` and `previewRuleExpression`.
- Replace display-only CEL preview with runtime-backed preview.
- Keep server mapping-test as authoritative until hosted runtime migration lands.
- Export helper metadata JSON for Workbench helper menus.

### Phase 3: `publicschema-build`

- Delegate compile/typecheck to `cel-mapping`.
- Keep existing Python facade types for compatibility.
- Generate canonical mapping artifacts plus optional compiled manifests.

### Phase 4: Hosted runtime

- Replace `celpy` transform execution in `publicschema_core.validation.worker`.
- Preserve API shape.
- Add parity tests against old expected fixtures.
- Ship behind a feature flag with shadow-run or A/B parity where feasible.
- Keep rollback to the current runtime until parity gates pass.

### Phase 5: Generated Python validators

- Replace emitted `celpy` transform logic with `cel_mapper`.
- Keep fail-closed behavior.
- Add package dependency and wheel strategy covering at least Linux x86_64, Linux ARM64, macOS ARM64/x86_64, and the supported Python minor versions.

### Phase 6: Generated JS validators

- Add Node/bundler-compatible WASM packaging.
- Replace `@publicschema/cel-js` transform execution.
- Keep `@publicschema/cel-js` only as a deprecated temporary fallback.
- Set a removal milestone once WASM packaging and parity pass. It is not a peer runtime.

### Phase 7: Optional YARRRML/RML export

- Add generator from PublicSchema mapping artifacts to YARRRML/RML for RDF interoperability.
- Do not make YARRRML the native runtime input.

## 19. Open Questions

Resolved in this draft:

- Accept both YAML and JSON PublicSchema mapping documents.
- `source` is always the resolved rule value; `root` is always the full input.
- Partial output reads are not available in v0.2.
- Strict mode stops at the first error; authoring tools should use collect mode.
- v0.1 `records` remains supported as an additive API.
- Privacy mode defaults are defined in §9.1.
- Helper metadata is Rust-authoritative and exported as JSON.

Deferred:

1. How should bidirectional mappings represent asymmetric source and target JSON Pointers?
2. What is the minimum Node/WASM packaging target needed for generated JS validators?
3. Should compiled mapping artifacts be portable/stable, or treated as runtime-internal cache objects?
4. What exact wheel matrix and release process should generated Python validators require?
5. When should the future `value` / `input` / `output` binding mode replace `publicschema-v1` compatibility defaults?

## 20. Success Criteria

The refactor is successful when:

- PublicSchema mappings execute through one Rust-core runtime.
- Workbench formula preview and hosted transform use the same expression semantics.
- `publicschema-build` compile diagnostics match runtime behavior.
- Generated Python validators match hosted transform output.
- Generated JS validators match once WASM packaging lands.
- Current `celpy` and handwritten JS CEL evaluator behavior is no longer authoritative.
- Mapping-test logs include rule-level inputs, outputs, statuses, and diagnostics.
- Runtime errors never silently fall back to identity transforms.
- YARRRML remains available as an interoperability/export concern without driving the core runtime model.
