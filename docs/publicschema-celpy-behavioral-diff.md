# PublicSchema celpy Behavioral Diff

Status: Phase 0 template. To be filled before Phase 4 ships.
Spec reference: `spec-publicschema-v0.2.md` §13.
Implementation plan reference: `implementation-plan-publicschema-v0.2.md` §3.3.

This document tracks known and expected behavioral differences between the
current hosted `celpy` transform path and the `cel-mapping` Rust runtime.

It MUST be reviewed and accepted before the Phase 4 hosted runtime migration
begins (spec §13, implementation plan Phase 4 gate). Each section records the
expected v0.2 behavior (the spec-correct answer), the current `celpy` behavior
(to be filled during Phase 4 shadow runs), the verification status, and the
path to a parity fixture once fixtures exist.

## How to fill this document

During Phase 4 shadow runs:

1. Run the same mapping + input through both `celpy` and `cel-mapping`.
2. Compare outputs, logs, and errors.
3. Fill in "Current celpy behavior" for each section.
4. Set status to `matches`, `intentional-diff`, or `bug-to-fix`.
5. Write a parity fixture at the path listed under "Test fixture" and commit it.

Status values:

- `unverified`: shadow run not yet done; celpy behavior unknown.
- `matches`: cel-mapping and celpy produce the same result.
- `intentional-diff`: behavior deliberately diverges; reason recorded here.
- `bug-to-fix`: cel-mapping has a defect that must be fixed before Phase 4 flips.

---

## 1. Missing vs null

### 1.1 Absent source path vs JSON null

**Expected v0.2 behavior:** When a source JSON Pointer path does not exist in
the input, the runtime produces a structured `missing` status for that rule. The
binding `source` receives the internal Missing sentinel, not JSON `null`.
Helpers that distinguish missing from null (such as `present`, `missing`,
`blank` in the Rust stdlib) behave differently for each. A `null` written
explicitly in the source JSON is a different value from a missing path.

**Current celpy behavior:** TBD (to be observed in Phase 4 shadow runs).

**Status:** unverified

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/missing-vs-null.json` (to be created)

### 1.2 Missing propagation into CEL expressions

**Expected v0.2 behavior:** Missing sentinel propagates into CEL expressions.
Accessing a missing value in a formula that does not handle it explicitly
(e.g. via `coalesce` or `default`) causes a `formula_error` status, not silent
null substitution.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 1.3 Optional vs required missing behavior

**Expected v0.2 behavior:** A rule without `required: true` and without
`on_missing` set uses `omit` as the default missing policy. A rule with
`required: true` or `on_missing: fail` produces a `missing` error that sets
`ok: false` on the output (spec §8.2).

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/missing-optional-omits.json` (to be created)

---

## 2. JSON Pointer reads/writes

This section covers the edge cases defined in spec §7.

### 2.1 Missing intermediate path in source read

**Expected v0.2 behavior:** Reading `/answers/respondent_first` when
`/answers` does not exist in the input produces a missing status, not a panic
or null. This is consistent with spec §7.1: "Missing paths produce a structured
missing status, not an immediate host panic."

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 2.2 Array index source read (numeric segment)

**Expected v0.2 behavior:** A source path `/items/2` reads the element at
index 2. If `items` exists but has fewer than 3 elements, the result is missing.
If `items` is not an array, the result is a type error.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 2.3 Array append target write (`-`)

**Expected v0.2 behavior:** A target path ending in `/-` appends to the array
at the parent path. If the parent array does not yet exist, it is created as an
empty array and the value is appended as element 0. If the parent exists as a
non-array, this is a `write_error` (spec §7.2).

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/array-append.json` (to be created)

### 2.4 Missing intermediate object creation on target write

**Expected v0.2 behavior:** Writing to `/a/b/c` when `/a` and `/a/b` do not
exist creates both intermediate objects automatically. If the next segment after
a missing node is numeric or `-`, an array is created; otherwise an object
(spec §7.2).

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 2.5 Out-of-bounds array write with padding

**Expected v0.2 behavior:** Writing to `/items/3` when `/items` is `[1, 2]`
pads indices 2 with `null` and writes index 3. Writing to an existing
non-array at that path is a `write_error` (spec §7.2).

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 2.6 Tilde-escaped segments (`~0`, `~1`)

**Expected v0.2 behavior:** RFC 6901 decoding is applied: `~1` decodes to `/`,
`~0` decodes to `~`. Decoding is done before segment lookup.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

---

## 3. Date/time formats

### 3.1 Format token dialect: celext v1 vs Rust ICU-subset

**Expected v0.2 behavior:** celext v1 helpers (`parseDate`, `formatDate`, etc.)
use the profile format dialect: `YYYY`, `YY`, `MM`, `DD`, `HH`, `mm`, `ss`.
The Rust stdlib (`date_parse`, `date_format`, `date_parse_datetime`) uses an
ICU-subset dialect: `yyyy`, `MM`, `dd`, `HH`, `mm`, `ss`. These are NOT
identical. The Phase 0 porting work (see `publicschema-helper-parity.md`
section 2) must bridge the two dialects when registering `parseDate` and
`formatDate` aliases in Rust.

**Current celpy behavior:** Uses the profile dialect tokens (`YYYY`, `DD`).

**Status:** intentional-diff (token dialect translation required; see helper parity doc)

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/date-format-tokens.json` (to be created)

### 3.2 ISO-8601 string input to formatDate

**Expected v0.2 behavior:** `formatDate` accepts an ISO-8601 string as the
first argument (in addition to a timestamp value). Python `helpers.py` line
69-70 handles this with `datetime.fromisoformat`. JS accepts `string` or `Date`
object. Rust `date_format` (builtins.rs line 503) accepts an ISO-8601 date
string or RFC 3339 datetime string.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** Covered by cel-parity fixture `01-formatDate.json` and `13-formatDate.json`

### 3.3 Offsetless datetime input

**Expected v0.2 behavior:** When `parseDateTime` (or `date_parse_datetime`) is
called on an offsetless datetime string (e.g. `"2024-01-15 10:30:00"`) with no
embedded timezone offset, the runtime requires `ctx.timezone` to be set. If
`ctx.timezone` is absent, the call returns an error. This contract is
established by the Rust test `date_parse_datetime_two_arg_with_explicit_format`
at `tests/core_contracts.rs` line 451, which uses `timezone: "Asia/Bangkok"`.

**Current celpy behavior:** TBD. celpy may default to UTC for offsetless
datetimes rather than requiring `ctx.timezone`.

**Status:** unverified (likely intentional-diff if celpy defaults to UTC)

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/date-parse-datetime-offsetless.json` (to be created)

### 3.4 Ambiguous local time (DST overlap)

**Expected v0.2 behavior:** When converting an offsetless datetime in a timezone
that has a DST overlap (e.g. clocks fall back), the Rust runtime calls
`tz.from_local_datetime(&ndt).single()` and returns an error for ambiguous
local times (builtins.rs line 477: "ambiguous local"). There is no silent
disambiguation.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 3.5 Output format of timestamps

**Expected v0.2 behavior:** `date_parse_datetime` and `parseDateTime` return an
RFC 3339 string (e.g. `"2024-01-15T10:30:00+07:00"`). `date_parse` and
`parseDate` return a date string in `YYYY-MM-DD` format. These are strings, not
timestamp objects, in the Rust runtime.

**Current celpy behavior:** TBD (celpy may return a `celpy.celtypes.TimestampType` object).

**Status:** unverified (likely intentional-diff; celpy returns a native timestamp type while Rust returns a string)

**Test fixture:** TBD

---

## 4. Numeric conversion and JSON safe integer range

### 4.1 Rust rejects JS-unsafe integers

**Expected v0.2 behavior:** The Rust runtime rejects integers outside the
JavaScript safe integer range `[-9007199254740991, 9007199254740991]` when
converting CEL values to JSON output. Integers outside this range cause a
`write_error` or serialization error. This contract is established by the test
`cel_to_json_rejects_js_unsafe_integers` at `tests/core_contracts.rs` line 116,
using `JSON_SAFE_INT_MAX = 9007199254740991` from `src/output.rs` line 12.

**Current celpy behavior:** TBD. Python integers are unbounded; celpy likely
permits large integers in output JSON.

**Status:** unverified (likely intentional-diff; Rust enforces JS-safe range, Python/celpy does not)

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/numeric-js-safe-range.json` (to be created)

### 4.2 Float-to-int truncation behavior

**Expected v0.2 behavior:** `type_int` on a float requires the float to be
integral (no fractional part). A float with a fractional part returns an error
"float must be integral" (builtins.rs line 97). `num_floor` and `num_ceil`
truncate explicitly.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 4.3 Integer overflow on UInt -> Int conversion

**Expected v0.2 behavior:** Converting a `UInt` value larger than `i64::MAX` to
`Int` via `type_int` produces an "integer overflow" error (builtins.rs line 92).

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

---

## 5. Helper behavior differences

This section links to the `publicschema-helper-parity.md` gap section and
records the specific behavioral differences that affect hosted runtime parity.

### 5.1 `coalesce`: whitespace-only string handling

**Expected v0.2 behavior:** Rust `coalesce_impl` calls `present_value`, which
returns `false` for whitespace-only strings (helper.rs `is_blank_string`). A
whitespace-only string is therefore skipped and not returned as the first
"present" value.

**Current celpy behavior:** Python `_coalesce` checks `a != ""` but does NOT
strip whitespace. A whitespace-only string such as `"  "` is returned as a
non-empty, non-null value.

**Status:** intentional-diff (Rust is stricter; Python/JS treat whitespace-only as present)

**Test fixture:** TBD

### 5.2 `normalizePhone`: validation strictness

**Expected v0.2 behavior:** Rust `phone_normalize` uses libphonenumber via the
`phonenumber` crate and calls `pn.is_valid()`. An invalid number (syntactically
acceptable but not a real number for the given region) produces an error
"invalid phone number" (phone.rs line 29).

**Current celpy behavior:** Python falls back to a heuristic that does NOT call
`phonenumbers.is_valid_number`. JS always uses the heuristic. Both Python and
JS may return a formatted but invalid number.

**Status:** intentional-diff (Rust is stricter on validation; celpy and JS-based implementations are permissive)

**Test fixture:** TBD

### 5.3 celext v1 helper name gaps

The following helpers have different names in Rust and must be aliased or ported
before Phase 4. Until they are registered under their celext v1 names,
production formulas using them will fail to compile in the Rust runtime:

- `parseDate` (Rust: `date_parse`)
- `formatDate` (Rust: `date_format`)
- `parseDateTime` (Rust: `date_parse_datetime`)
- `formatDateTime` (Rust: `date_format`)
- `normalizePhone` (Rust: `phone_normalize`)
- `defaultIfEmpty` (Rust: `default`)
- `regexMatch` (Rust: `text_matches`)
- `lookupCode` (no Rust equivalent with matching signature)
- `mapCode` (no Rust equivalent with matching signature)
- `splitName`, `joinName`, `fhirReference`, `iso3166`, `iso4217`, `iso639`, `regexExtract`, `coalesceList`, `struct`, `listOf`: all absent in Rust

See `publicschema-helper-parity.md` section 10 for the full table.

**Status:** bug-to-fix (must be resolved before Phase 4 flips production traffic)

**Test fixture:** `publicschema-helpers.rs` test suite (to be created per implementation plan §3.2)

---

## 6. Error wording and redaction

### 6.1 Production privacy mode suppresses source values in error messages

**Expected v0.2 behavior:** In `production` privacy mode, resolved source
values MUST NOT appear in error messages or diagnostics. Error messages use
generic descriptions: "required value missing", "formula error in rule
`<rule_id>`". Resolved input and output are suppressed from the rule log.
(Spec §9.1.)

**Current celpy behavior:** TBD. The current hosted transform likely includes
source values in some error message paths.

**Status:** unverified (likely intentional-diff if celpy exposes source values in errors)

**Test fixture:** TBD

### 6.2 Authoring privacy mode includes resolved values on request

**Expected v0.2 behavior:** In `authoring` privacy mode with
`include_resolved_values: true`, the rule log includes `resolved_input` and
`resolved_output` fields. Exception messages that contain source values are
still redacted.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

### 6.3 Error message format

**Expected v0.2 behavior:** Rust error messages from helper functions follow
the pattern `"<function_name>: <description>"` (e.g.
`"date_parse: trailing input"`). celpy may use different formatting.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

---

## 7. Diagnostic ordering

**Expected v0.2 behavior:** In `collect` error mode, diagnostics are collected
in rule evaluation order (document order). Each rule contributes at most one
error or one warning per rule log entry. Mapping-level errors and warnings are
reported after rule-level errors and warnings.

**Current celpy behavior:** TBD.

**Status:** unverified

**Test fixture:** TBD

---

## 8. Transformation-log ordering

**Expected v0.2 behavior:** The `log` array in the transform output contains
one entry per evaluated rule, in document order (spec §5.4). Skipped rules
(wrong-direction formulas, disabled rules) are included with status `skipped`.
Omitted rules (missing optional source) are included with status `omitted`.

**Current celpy behavior:** TBD. The current hosted implementation may not
expose a per-rule transformation log at all, or may order entries differently.

**Status:** unverified

**Test fixture:** TBD

---

## 9. Duplicate target writes

**Expected v0.2 behavior:** When two rules in a mapping both write to the same
target JSON Pointer, the last rule in document order wins (last-write-wins). In
`authoring` privacy mode, a warning diagnostic is emitted for the overwritten
write (spec §5.4). In `production` mode, no warning is emitted.

Example: if rules A and B both write to `/given_name`, and B appears after A
in the document, B's value is the final output value and A's write is discarded
with a warning.

**Current celpy behavior:** TBD. The current hosted implementation may produce
different behavior (e.g. first-write-wins, merge, or an error).

**Status:** unverified

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/duplicate-target-last-write.json` (to be created; listed in implementation plan §3.1)

---

## 10. Intentional non-parity decisions

This section records places where `cel-mapping` deliberately diverges from
`celpy` behavior and those divergences have been accepted.

### 10.1 No identity fallback on formula error

**Expected v0.2 behavior:** If a formula fails (CEL compile error, evaluation
error, helper exception), the rule produces a `formula_error` status and no
output is written for that rule. The source value is NEVER silently copied to
the output as a fallback identity. (Spec §8.2: "The runtime MUST never silently
fall back to identity when a non-identity formula fails.")

**Current celpy behavior:** TBD. celpy may fall back to identity on some
formula errors.

**Status:** intentional-diff (fail-closed is required by spec)

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/formula-error-fails-closed.json` (to be created)

### 10.2 Wrong-direction formula is an error, not identity

**Expected v0.2 behavior:** If a rule has a formula only for `from_target` and
the execution direction is `to_target`, the rule is a `formula_error` (not
omitted, not identity). This is distinct from "no formula at all", which is
identity. (Spec §5.3.)

**Current celpy behavior:** TBD. celpy may treat the wrong-direction case as
missing-formula identity.

**Status:** intentional-diff (required by spec)

**Test fixture:** `crates/cel-mapper-core/tests/fixtures/publicschema-parity/wrong-direction-no-identity.json` (listed in implementation plan §3.1)

### 10.3 JS-safe integer range enforcement

Documented in section 4.1 above.

### 10.4 Whitespace-only string treatment in coalesce

Documented in section 5.1 above.

### 10.5 Phone number validation strictness

Documented in section 5.2 above.

---

## Appendix: open items before Phase 4 approval

The following must be resolved before this document can be accepted and Phase 4
can proceed:

1. All "TBD" cells in sections 1-9 must be filled from shadow-run observations.
2. All helper name gaps from section 5.3 must be resolved (ported or explicitly
   deferred with a documented workaround).
3. Every `intentional-diff` item must have an explicit approval note (who
   approved, when, why).
4. Every `bug-to-fix` item must have a linked issue or a fix committed before
   the production traffic flip.
5. At least one parity fixture must exist for each section marked
   `intentional-diff` or `bug-to-fix`.
