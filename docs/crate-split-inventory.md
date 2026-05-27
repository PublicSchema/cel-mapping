# Crate Split Inventory v0.3

Status: implementation inventory for `spec-crate-split-v0.3.md`.

## Helper Semantics

Current CEL helper registration moved to `mapping-functions-cel`; pure deterministic helper semantics live in `mapping-functions` where the v0.3 API exists. The adapter preserves current CEL names, arity checks, null/missing coercion, warnings, and compatibility behavior.

Important preserved behavior:

- `text_regex_extract` returns an empty string through CEL when the pattern does not match or a capture group is missing.
- Direct Rust `mapping_functions::text::regex_extract` returns `Ok(None)` when the pattern does not match.
- `date_parse_datetime` resolves offset-less datetime fallback from adapter request `timezone`.
- `date_today` resolves adapter request `today`.
- `phone_*` helpers use the explicit country argument first, then adapter request `country` when the argument is null or missing.
- PublicSchema evaluation currently does not install helper request context before expression evaluation. This pre-existing behavior is preserved; changing it would alter fallback semantics for PublicSchema formulas and needs a focused spec/test decision.

## Helper Ownership

- `mapping-functions`: text, email, phone, date, IDs, redaction, JSON parse/stringify, code-system registry, code entries, typed code-system documents, and ISO preload.
- `mapping-functions-cel`: CEL `Value` conversion, helper registration, arity validation, missing/null compatibility, request fallback resolution, warning collection, and helper metadata.
- `cel-evaluator`: standalone expression compile/evaluate/preview, security limits, expression diagnostics, missing-path injection, CEL/JSON conversion, and reusable compiled-expression wrappers.
- `publicschema-mapping`: PublicSchema v0.2 document parsing, compile/evaluate, value mappings, JSON Pointer reads/writes, deterministic hash, logs, warnings, and privacy-aware log values.
- `cel-mapper-core`: v0.1 mapping runtime and compatibility facade. It still preserves existing public core paths for current bindings and callers.

## Public Types And Re-Export Paths

Existing `cel_mapper_core` public paths retained:

- `MappingRuntime`, `RuntimeOptions`, `EvaluationInput`, `MappingOutput`
- `CompiledMapping`, `CompiledCel`, `ErrorMode`
- `CodeEntry`, `CodeSystemRegistry`
- `CompileError`, `MappingError`, `ErrorCode`, `ErrorSeverity`
- `StandaloneExpressionInput`, `StandaloneEvalError`, `ExpressionPreviewResult`, `ExpressionIssue`, `ExpressionPhase`
- PublicSchema facade types and functions currently exported from `cel_mapper_core`

New direct crate paths:

- `mapping_functions::{text,email,phone,date,ids,redaction,json,codes}`
- `mapping_functions_cel::{register_stdlib,register_mapping_functions,helper_metadata}`
- `cel_evaluator::{compile_expr,evaluate_cel_expression,preview_cel_expression,SecurityLimits}`
- `publicschema_mapping::{compile_publicschema_mapping,evaluate_publicschema_mapping}`

## Serde And Trait Notes

- PublicSchema public options and outputs preserve their existing serde derive behavior.
- Error code enums remain serde-compatible with current JSON casing.
- `FunctionError.code` is the stable machine-readable helper error contract; tests assert codes instead of prose for extracted helpers.
- `CodeEntry` keeps `id`, `label`, `aliases`, and flattened JSON `extra` metadata.

## Accepted Diagnostic Wording Changes

No diagnostic wording changes are intentionally accepted in this implementation. Existing core fixture output and PublicSchema parity tests are expected to remain unchanged.

## Pitfalls Logged

- The existing PublicSchema evaluator did not set thread-local helper context. This was surprising because generic and standalone evaluation do install context. The split preserves that behavior to avoid unreviewed drift.
- `serde_yaml` remains above `mapping-functions`; the leaf crate accepts typed `CodeSystemDocument` values and uses a simple embedded ISO loader without `serde_yaml`.
- `cel::ExecutionError` remains inside adapter/evaluator internals. Public standalone APIs expose stable evaluator errors instead.
