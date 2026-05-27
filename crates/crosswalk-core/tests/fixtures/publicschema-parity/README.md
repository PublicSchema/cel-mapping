# PublicSchema Parity Fixture Corpus

Golden fixtures for the PublicSchema v0.2 mapping runtime. Each `.json` file
describes one scenario. The Rust harness in
`tests/publicschema_fixture_corpus.rs` loads every file here and runs assertions
against the live runtime.

## Fixture format

```json
{
  "name": "short-slug",
  "mapping": { ... PublicSchema v0.2 mapping document as JSON ... },
  "direction": "to_target | from_target",
  "source_data": { ... input record ... },
  "ctx": { ... host context ... },
  "options": {
    "privacy": "authoring | production | debug",
    "errors_mode": "collect | strict | lenient"
  },
  "expected": {
    "ok": true,
    "output": { ... expected output record ... },
    "errors_count": 0,
    "warnings_count": 0,
    "log_statuses": ["applied", "omitted", ...]
  }
}
```

### Relaxed expected shape

The `expected` object uses counts and status sequences rather than full
structural equality on `errors` and `warnings`. This is intentional: error
message wording may evolve without breaking the behavioral contract. To verify
a specific error code, add an inline test in `publicschema_parity.rs` instead.

The `output` field IS compared with deep equality; fixture authors must keep it
accurate.

## Status values (spec §5.2)

| Status | Meaning |
|---|---|
| `applied` | Rule evaluated and output written |
| `defaulted` | Source path absent (optional); no output written. Spec vocabulary: `applied\|defaulted\|skipped`. |
| `missing` | Required source missing; error raised |
| `skipped` | No source path declared for this rule |
| `value_unmapped` | `value_mappings` crosswalk had no deterministic row for the resolved value |
| `formula_error` | CEL compile/eval failure or wrong-direction formula |
| `write_error` | Expression succeeded but target pointer write failed |
| `validation_error` | Reserved for post-transform validation failures |

## Adding a fixture

1. Create a new `.json` file following the format above.
2. Set `"name"` to the same slug as the filename (without `.json`).
3. Run `cargo test -p crosswalk-core --test publicschema_fixture_corpus`.
4. If it fails, fix the fixture until the runtime produces the expected output.
   Do not loosen the harness assertions.

The Rust core is the behavioral oracle. If the fixture disagrees with the
runtime and the runtime behavior is correct per spec, update the fixture.

## Fixture inventory

| File | What it covers |
|---|---|
| `identity-copy.json` | No formula, source pointer to target pointer (pure copy) |
| `explicit-source-identity.json` | `to_target.expression = "source"` is equivalent to no formula |
| `wrong-direction-no-identity.json` | Only `from_target` defined; running `to_target` → `formula_error` |
| `missing-optional-omits.json` | Missing optional source → status `defaulted`, no output |
| `missing-required-fails.json` | Missing required source → status `missing`, `ok=false` |
| `formula-error-fails-closed.json` | Bad formula throws; no identity fallback; output stays empty |
| `array-append.json` | Target path `/identifiers/-`; result is a one-element array |
| `duplicate-target-last-write.json` | Two rules write same target; last wins; one authoring warning |
| `ctx-timezone.json` | Formula reads `ctx.timezone`; caller-supplied value reaches expression |
| `code-map-preloaded.json` | Mapping-local `code_systems` block; `code_map_or_default` lookup works |
| `value-mapping-unmapped.json` | `value_mappings` misses a forward source value and fails closed |
| `value-mapping-reverse-ambiguous.json` | Reverse `value_mappings` target collision fails closed |
