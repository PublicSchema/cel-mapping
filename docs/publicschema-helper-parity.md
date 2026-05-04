# PublicSchema celext v1 Helper Inventory

Status: Phase 0 working document.
Spec reference: `spec-publicschema-v0.2.md` §17.
Implementation plan reference: `implementation-plan-publicschema-v0.2.md` §3.2.

This document inventories every helper in the canonical celext v1 registry
(`publicschema-build/build/cel/helpers.py`) and tracks porting status across
the three runtimes.

Source files:

- Python canonical: `apps/publicschema-build/build/cel/helpers.py`
- JS mirror: `apps/publicschema-build/packages/cel-js/src/helpers.ts`
- Rust (cel-mapper-core stdlib): `apps/cel-mapping/crates/cel-mapper-core/src/functions/builtins.rs`

The Python registry is the canonical source for celext v1 signatures. The Rust
core is the behavioral oracle for v0.2 going forward (spec §17).

Parity test fixtures (JS): `packages/cel-js/tests/fixtures/cel-parity/*.json`

## Implementation status key

- Present: implemented and exercised by tests or parity fixtures.
- Stub: registered but raises `NotImplementedError` / throws by default (registry-backed helpers).
- Gap: defined in one runtime, absent in another.
- Partial: present but behavior differs in a known way (see notes column).

---

## 1. Text / defaulting / coalesce

### coalesce

| Field | Value |
|---|---|
| Name | `coalesce` |
| Aliases | none |
| Signature | `coalesce(a: any, b: any, ...) -> any` (variadic) |
| Return type | any |
| Null/missing behavior | skips `null`, `undefined`, and empty string `""`; returns the first argument that is none of those; returns `null` when all arguments are skipped |
| Deterministic | yes |
| Python | Present (`helpers.py` line 76) |
| JS | Present (`helpers.ts` line 128) |
| Rust | Present as `coalesce` (`builtins.rs` line 49); delegates to `coalesce_impl` in `helpers.rs` line 82 |
| Example | `coalesce(a, b, c)` where `a=null, b="hello", c="world"` returns `"hello"` (fixture `02-coalesce-first.json`) |
| Notes | Python treats empty string as empty; JS adds `undefined` as an additional skip case. Rust `coalesce_impl` calls `present_value` which also treats whitespace-only strings as blank. This is a behavioral difference: Python and JS do NOT skip whitespace-only strings, Rust does. Flag as intentional-diff candidate for behavioral-diff doc. |

### defaultIfEmpty

| Field | Value |
|---|---|
| Name | `defaultIfEmpty` |
| Aliases | `default` (Rust stdlib exposes `default` with the same semantics; `defaultIfEmpty` is the celext v1 name) |
| Signature | `defaultIfEmpty(a: any, b: any) -> any` |
| Return type | any |
| Null/missing behavior | returns `b` when `a` is `null`, empty string `""`, or empty list `[]`; otherwise returns `a` |
| Deterministic | yes |
| Python | Present (`helpers.py` line 84) |
| JS | Present (`helpers.ts` line 137) |
| Rust | Present as `default` in builtins.rs line 56; `defaultIfEmpty` is not registered under that name in Rust. Gap: celext v1 callers using the name `defaultIfEmpty` will not resolve in Rust. |
| Example | `defaultIfEmpty(val, "fallback")` where `val=""` returns `"fallback"` (fixture `11-defaultIfEmpty.json`) |
| Notes | Gap: Rust stdlib registers this as `default`, not `defaultIfEmpty`. Formulas using `defaultIfEmpty(...)` will fail to compile unless an alias is added. |

### coalesceList

| Field | Value |
|---|---|
| Name | `coalesceList` |
| Aliases | none |
| Signature | `coalesceList(a: list, b: list, ...) -> list` (variadic) |
| Return type | list |
| Null/missing behavior | skips `null` arguments and empty lists; returns first non-empty list; returns `[]` when no non-empty list found |
| Deterministic | yes |
| Python | Present (`helpers.py` line 365) |
| JS | Gap: `coalesceList` is not in `helpers.ts` or the JS `REGISTERED_HELPERS` registry. |
| Rust | Gap: `coalesceList` is not registered in builtins.rs. |
| Example | `coalesceList([], items, fallback)` returns `items` if `items` is non-empty. |
| Notes | Gap in JS and Rust. Needs porting to both. |

---

## 2. Date / time

All date/time helpers in celext v1 are UTC-only and locale-independent. The Rust
runtime uses `chrono` with tz-aware parsing via `chrono_tz`. See the
behavioral-diff doc (`publicschema-celpy-behavioral-diff.md`) for the full
comparison of format tokens and offsetless-input handling.

### Format token mapping

celext v1 uses a profile format string dialect. The token-to-strftime mapping
for Python is in `helpers.py` lines 31-48. JS uses the same tokens mapped to
regex groups (`helpers.ts` lines 19-81). Rust uses an ICU-subset translation
(`builtins.rs` `icuish_to_chrono` lines 33-41):

| Profile token | Python strftime | JS regex group | Rust chrono |
|---|---|---|---|
| `YYYY` | `%Y` | `(?<year4>\d{4})` | `%Y` (via `yyyy`) |
| `YY` | `%y` | `(?<year2>\d{2})` | `%y` (via `yy`) |
| `MM` | `%m` | `(?<month>\d{2})` | `%m` |
| `DD` | `%d` | `(?<day>\d{2})` | `%d` (via `dd`) |
| `HH` | `%H` | `(?<hour>\d{2})` | `%H` |
| `mm` | `%M` | `(?<min>\d{2})` | `%M` |
| `ss` | `%S` | `(?<sec>\d{2})` | `%S` |

Note: Rust `builtins.rs` translates from ICU-style (`yyyy`, `MM`, `dd`, `HH`,
`mm`, `ss`) to chrono. The `icuish_to_chrono` function also handles `XXX` (TZ
offset). celext v1 Python and JS use the profile dialect tokens above. These two
dialects overlap but are NOT identical. See behavioral-diff doc.

### parseDate

| Field | Value |
|---|---|
| Name | `parseDate` |
| Aliases | none |
| Signature | `parseDate(s: string, fmt: string) -> timestamp` |
| Return type | timestamp (UTC datetime) |
| Null/missing behavior | errors on `null`/missing `s` or `fmt` |
| Deterministic | yes |
| Python | Present (`helpers.py` line 57, registered line 431) |
| JS | Present (`helpers.ts` line 112) |
| Rust | Gap: `parseDate` is not registered as `parseDate` in builtins.rs. The Rust stdlib registers `date_parse` (line 429) using ICU-subset format tokens. The function signatures and format dialects differ. |
| Example | `parseDate("2023-06-15", "YYYY-MM-DD")` |
| Notes | Gap: Rust does not register `parseDate`; formulas using the celext v1 name will fail. The format token dialect also differs from Rust's ICU-subset. Porting requires registering an alias and normalizing the token dialect. |

### formatDate

| Field | Value |
|---|---|
| Name | `formatDate` |
| Aliases | none |
| Signature | `formatDate(t: timestamp, fmt: string) -> string` |
| Return type | string |
| Null/missing behavior | errors on unparseable input |
| Deterministic | yes |
| Python | Present (`helpers.py` line 65, registered line 437) |
| JS | Present (`helpers.ts` line 116); accepts ISO-8601 strings as `t` in addition to `Date` objects |
| Rust | Gap: `formatDate` is not registered. Rust registers `date_format` (builtins.rs line 503) using ICU-subset format tokens and accepting ISO-8601 date strings. |
| Example | `formatDate(date_val, "YYYY-MM-DD")` where `date_val="2023-06-15T00:00:00+00:00"` returns `"2023-06-15"` (fixture `01-formatDate.json`) |
| Notes | Same gap as `parseDate`: name and format token dialect differ in Rust. |

### parseDateTime

| Field | Value |
|---|---|
| Name | `parseDateTime` |
| Aliases | none |
| Signature | `parseDateTime(s: string, fmt: string) -> timestamp` |
| Return type | timestamp |
| Null/missing behavior | errors on null/missing input |
| Deterministic | yes (with offset-bearing input; requires `ctx.timezone` for offsetless) |
| Python | Present (registered line 441; reuses `_parse_date`) |
| JS | Present (`helpers.ts` line 120; reuses `parseDateWithFmt`) |
| Rust | Gap: `parseDateTime` not registered. Rust registers `date_parse_datetime` (builtins.rs line 444). The Rust version accepts 1 or 2 arguments; with 1 arg it tries RFC 3339 parse; with 2 args it tries the format string then falls back to `ctx.timezone` for offsetless. Python/JS always require 2 args. |
| Example | `parseDateTime("2024-01-15T10:30:00Z")` (Rust 1-arg form) vs `parseDateTime("2024-01-15 10:30:00", "YYYY-MM-DD HH:mm:ss")` |
| Notes | Gap on name. Rust's `date_parse_datetime` has richer behavior (1-arg RFC 3339, `ctx.timezone` fallback). The celext v1 2-arg form must still map through format-token translation. |

### formatDateTime

| Field | Value |
|---|---|
| Name | `formatDateTime` |
| Aliases | none |
| Signature | `formatDateTime(t: timestamp, fmt: string) -> string` |
| Return type | string |
| Null/missing behavior | errors on null/unparseable input |
| Deterministic | yes |
| Python | Present (registered line 445; reuses `_format_date`) |
| JS | Present (`helpers.ts` line 124; reuses `formatDateWithFmt`) |
| Rust | Gap: `formatDateTime` not registered. Rust registers `date_format` (builtins.rs line 503) which handles both date-only and datetime strings via the same function. |
| Example | TBD: same as `formatDate` but `fmt` includes time tokens |
| Notes | Same gap as `formatDate`. |

---

## 3. Phone

### normalizePhone

| Field | Value |
|---|---|
| Name | `normalizePhone` |
| Aliases | `phone_normalize` (Rust), `person_normalize_phone` (Rust alias) |
| Signature | `normalizePhone(s: string, country: string) -> string` |
| Return type | string (E.164 format, e.g. `"+254722000000"`) |
| Null/missing behavior | errors on empty input; errors on invalid number |
| Deterministic | yes (given same input and country) |
| Python | Present (`helpers.py` line 122, registered line 451). Uses `phonenumbers` library when available; falls back to heuristic with a small country-prefix table covering KE, TZ, UG, RW, ET, GH, NG, SN. |
| JS | Present (`helpers.ts` line 178). Heuristic-only; same country-prefix table as Python fallback. Does NOT use `phonenumbers`. |
| Rust | Present as `phone_normalize` and `person_normalize_phone` (builtins.rs lines 1235-1241). Delegates to `phone.rs` which uses the `phonenumber` crate (libphonenumber-compatible). Full E.164 validation including `pn.is_valid()` check (phone.rs line 29). |
| Example | `normalizePhone("0722000000", "KE")` returns `"+254722000000"` (fixture `14-normalizePhone.json`) |
| Notes | Gap: `normalizePhone` is not registered in Rust under that name. Rust callers must use `phone_normalize`. Behavior diverges: Rust uses libphonenumber (stricter validation, rejects invalid numbers); Python heuristic fallback and JS heuristic are more permissive. Rust also validates `pn.is_valid()`, which Python and JS do not. This is an intentional-diff candidate. |

---

## 4. Code systems

### lookupCode

| Field | Value |
|---|---|
| Name | `lookupCode` |
| Aliases | none |
| Signature | `lookupCode(value_set_id: string, code: string) -> string` |
| Return type | string (code label) |
| Null/missing behavior | throws `NotImplementedError` / runtime error unless registry is injected |
| Deterministic | yes (once registry is fixed) |
| Python | Stub (`helpers.py` line 397); raises `NotImplementedError`. Callers must inject via `REGISTERED_HELPERS['lookupCode']['fn']`. |
| JS | Stub (`helpers.ts` line 302); throws. |
| Rust | Gap: `lookupCode` is not registered. Rust has `code_label` (builtins.rs line 1036) and `code_map` (line 1000) which use the preloaded `CodeSystemRegistry`. The semantics differ: `code_label` takes `(system, code, locale)` and returns a label; `lookupCode` takes `(value_set_id, code)` and returns a label. |
| Example | `lookupCode("sex-administrative", "male")` returns `"Male"` (TBD: no fixture yet) |
| Notes | Gap: not registered in Rust under the celext v1 name. Must be ported as an alias or wrapper over `code_label`. Registry injection mechanism differs: Python injects via dict mutation; Rust uses a preloaded `CodeSystemRegistry` passed at compile time (spec §16.1). |

### mapCode

| Field | Value |
|---|---|
| Name | `mapCode` |
| Aliases | none |
| Signature | `mapCode(concept_map_id: string, code: string) -> string` |
| Return type | string (mapped target code id) |
| Null/missing behavior | throws `NotImplementedError` / runtime error unless registry is injected |
| Deterministic | yes (once registry is fixed) |
| Python | Stub (`helpers.py` line 413); raises `NotImplementedError`. |
| JS | Stub (`helpers.ts` line 308); throws. |
| Rust | Gap: `mapCode` is not registered. Rust has `code_map` (builtins.rs line 1000), `code_map_or_null` (line 1011), and `code_map_or_default` (line 1024). The Rust `code_map` takes `(system, value)` not `(concept_map_id, code)`. The naming convention and arity differ. |
| Example | `mapCode("sex-administrative-to-fhir", "male")` returns `"male"` (TBD) |
| Notes | Gap: not registered in Rust under the celext v1 name. Semantic mapping between `concept_map_id` and Rust's `code_system` convention is TBD and must be resolved before Phase 3. |

### iso3166

| Field | Value |
|---|---|
| Name | `iso3166` |
| Aliases | none |
| Signature | `iso3166(s: string) -> string` |
| Return type | string (ISO 3166-1 alpha-3 uppercase, e.g. `"KEN"`) |
| Null/missing behavior | throws `ValueError` / runtime error on unrecognized input |
| Deterministic | yes |
| Python | Present (`helpers.py` line 216, registered line 489). Falls back to `pycountry` when available; raises `ValueError` on unrecognized input. |
| JS | Present (`helpers.ts` line 236). No `pycountry` equivalent; throws on unrecognized input. Same alpha-2-to-alpha-3 table as Python. |
| Rust | Gap: `iso3166` is not registered in builtins.rs. Rust registers `address_normalize_country` (line 1367) which normalizes to alpha-2 (not alpha-3) using `CodeSystemRegistry::normalize_code`. The return type and behavior differ substantially. |
| Example | `iso3166("KE")` returns `"KEN"` (fixture `12-iso3166.json`) |
| Notes | Gap in Rust. The Python table covers 249 territories; JS covers the same set. Rust's `address_normalize_country` returns alpha-2, not alpha-3, and uses a different lookup path. Must be ported as a standalone function for celext v1 parity. |

### iso4217

| Field | Value |
|---|---|
| Name | `iso4217` |
| Aliases | none |
| Signature | `iso4217(s: string) -> string` |
| Return type | string (ISO 4217 alpha-3 uppercase, e.g. `"KES"`) |
| Null/missing behavior | throws `ValueError` / runtime error on unrecognized input |
| Deterministic | yes |
| Python | Present (`helpers.py` line 261, registered line 494). Falls back to `pycountry`; raises `ValueError`. Common-name table covers 10 currencies. |
| JS | Present (`helpers.ts` line 259). No `pycountry`; throws. Common-name lookup is absent in JS (only canonical codes accepted). |
| Rust | Gap: `iso4217` is not registered in builtins.rs. |
| Example | `iso4217("kes")` returns `"KES"` (fixture `15-iso4217.json`) |
| Notes | Gap in Rust. JS also lacks the common-name lookup that Python has (e.g. `"KENYAN SHILLING"` -> `"KES"`); this is a Python-only feature. Must be ported to Rust; common-name table is optional for v0.2 parity but should be noted. |

### iso639

| Field | Value |
|---|---|
| Name | `iso639` |
| Aliases | none |
| Signature | `iso639(s: string) -> string` |
| Return type | string (ISO 639-3 alpha-3 lowercase, e.g. `"eng"`) |
| Null/missing behavior | throws `ValueError` / runtime error on unrecognized input |
| Deterministic | yes |
| Python | Present (`helpers.py` line 319, registered line 499). Falls back to `pycountry`; raises `ValueError`. |
| JS | Present (`helpers.ts` line 291). No `pycountry`; throws. Same alpha-2-to-alpha-3 table as Python. |
| Rust | Gap: `iso639` is not registered in builtins.rs. |
| Example | `iso639("en")` returns `"eng"` (fixture `16-iso639.json`) |
| Notes | Gap in Rust. Must be ported. |

---

## 5. Regex

### regexMatch

| Field | Value |
|---|---|
| Name | `regexMatch` |
| Aliases | `text_matches` (Rust; different behavior: full-match vs search), `text_regex_replace` (different purpose) |
| Signature | `regexMatch(s: string, pattern: string) -> bool` |
| Return type | bool |
| Null/missing behavior | returns `false` on empty/null `s`; errors on invalid pattern |
| Deterministic | yes |
| Python | Present (`helpers.py` line 377, registered line 509). Uses `re.search` (partial match). |
| JS | Present (`helpers.ts` line 195). Uses `new RegExp(pattern).test(s)` (partial match). |
| Rust | Gap: `regexMatch` is not registered under that name. Rust registers `text_matches` (builtins.rs line 254) which uses `re.is_match` (equivalent to a partial search match). The name differs. |
| Example | `regexMatch("0712345678", "^[0-9]{10}$")` returns `true` (fixture `09-regexMatch.json`) |
| Notes | Gap: Rust uses `text_matches`, not `regexMatch`. Behavior appears equivalent (both partial-match), but `regexMatch` needs to be registered as an alias. Note: Python uses `re.search`, not `re.fullmatch`; the spec describes this as "RE2-compatible regex match" which means `re.search` semantics (partial). |

### regexExtract

| Field | Value |
|---|---|
| Name | `regexExtract` |
| Aliases | none |
| Signature | `regexExtract(s: string, pattern: string, group: int) -> string` |
| Return type | string (empty string if no match or group index out of range) |
| Null/missing behavior | returns `""` on no match; returns `""` on out-of-range group index |
| Deterministic | yes |
| Python | Present (`helpers.py` line 382, registered line 514). Returns `""` on `IndexError`. |
| JS | Present (`helpers.ts` line 199). Returns `m[group] ?? ""`. |
| Rust | Gap: `regexExtract` is not registered in builtins.rs under that name. |
| Example | `regexExtract("KEN123456", "([A-Z]{3})[0-9]+", 1)` returns `"KEN"` (fixture `17-regexExtract.json`) |
| Notes | Gap in Rust. Must be ported or an alias added to an existing regex extraction helper. |

---

## 6. Name helpers

### splitName

| Field | Value |
|---|---|
| Name | `splitName` |
| Aliases | none |
| Signature | `splitName(s: string, part: string) -> string` |
| Return type | string |
| Null/missing behavior | returns `""` for empty/blank input string; throws `ValueError`/error for invalid `part` |
| Deterministic | yes |
| Python | Present (`helpers.py` line 96, registered line 484). Splits on whitespace; `part` must be `"first"`, `"last"`, or `"middle"`. `last` returns `""` for single-token input. |
| JS | Present (`helpers.ts` line 148). Same logic; splits on `/\s+/`. `last` returns `""` for single-token. |
| Rust | Gap: `splitName` is not registered in builtins.rs. Rust has `name_parts` (builtins.rs line 368) which returns a map `{given, middle, family}`, and `name_initials` (line 416). Neither maps directly to `splitName`. |
| Example | `splitName("John Michael Doe", "first")` returns `"John"` (fixture `05-splitName-first.json`) |
| Notes | Gap in Rust. The Python/JS single-token behavior: `splitName("Alice", "last")` returns `""` because there is no second token. Must be ported. |

### joinName

| Field | Value |
|---|---|
| Name | `joinName` |
| Aliases | none |
| Signature | `joinName(parts: list<string>) -> string` |
| Return type | string |
| Null/missing behavior | filters out empty and whitespace-only parts before joining |
| Deterministic | yes |
| Python | Present (`helpers.py` line 117, registered line 504). Filters `p and p.strip()`. |
| JS | Present (`helpers.ts` line 162). Same filter: `p && p.trim()`. |
| Rust | Gap: `joinName` is not registered in builtins.rs. Rust has `name_full` (builtins.rs line 360) which takes two arguments `(given, family)`, not a list. |
| Example | `joinName(["Alice", "", "Smith"])` returns `"Alice Smith"` (fixture `08-joinName.json`) |
| Notes | Gap in Rust. `name_full` is a 2-argument function, not a list-based variadic join. Must be ported. |

---

## 7. FHIR / references

### fhirReference

| Field | Value |
|---|---|
| Name | `fhirReference` |
| Aliases | none |
| Signature | `fhirReference(type: string, id: string) -> string` |
| Return type | string (format `"{type}/{id}"`) |
| Null/missing behavior | errors on null/missing either argument (string concatenation; null would produce `"null/..."`) |
| Deterministic | yes |
| Python | Present (`helpers.py` line 91, registered line 461). Simple string interpolation. |
| JS | Present (`helpers.ts` line 144). Template literal. |
| Rust | Gap: `fhirReference` is not registered in builtins.rs. |
| Example | `fhirReference("Patient", "abc-123")` returns `"Patient/abc-123"` (fixture `04-fhirReference.json`) |
| Notes | Gap in Rust. Trivial to port. |

---

## 8. Struct / list constructors

### struct

| Field | Value |
|---|---|
| Name | `struct` |
| Aliases | none |
| Signature | `struct(key: string, value: any, ...) -> map` (variadic, even arg count) |
| Return type | map |
| Null/missing behavior | raises `ValueError` for odd arg count; raises `TypeError` for non-string keys |
| Deterministic | yes |
| Python | Present (`helpers.py` line 335, registered line 524). |
| JS | Gap: `struct` is not in the JS `REGISTERED_HELPERS` or `helpers.ts`. |
| Rust | Gap: `struct` is not registered in builtins.rs. |
| Example | TBD |
| Notes | Gap in JS and Rust. |

### listOf

| Field | Value |
|---|---|
| Name | `listOf` |
| Aliases | none |
| Signature | `listOf(item: any, ...) -> list` (variadic) |
| Return type | list |
| Null/missing behavior | returns `[]` with no arguments; passes through all argument values including null |
| Deterministic | yes |
| Python | Present (`helpers.py` line 357, registered line 529). |
| JS | Gap: `listOf` is not in JS. |
| Rust | Gap: `listOf` is not registered in builtins.rs. |
| Example | TBD |
| Notes | Gap in JS and Rust. |

---

## 9. Rust-only helpers (no celext v1 equivalent)

The Rust stdlib in `builtins.rs` defines a large set of helpers that are NOT in
the celext v1 Python/JS registry. These are v0.1 or Rust-native additions. They
are listed here for completeness but do not need celext v1 parity ports.

These namespaced groups exist in Rust only: `text_*`, `date_*`, `num_*`,
`list_*`, `map_*`, `code_*`, `id_*`, `person_*`, `phone_*`, `email_*`,
`geo_*`, `address_*`, `validate_*`, `json_*`, `privacy_*`.

The following are present only in Rust and not in celext v1:

- `present`, `missing`, `blank`, `default`, `require`, `null_if`, `null_if_blank`
- `type_string`, `type_int`, `type_float`, `type_bool`, `type_list`, `type_map`
- `type_is_string`, `type_is_number`, `type_is_bool`, `type_is_list`, `type_is_map`
- `text_trim`, `text_lower`, `text_upper`, `text_title`, `text_normalize_space`, `text_remove_accents`, `text_slug`, `text_replace`, `text_regex_replace`, `text_matches`, `text_split`, `text_join`, `text_left`, `text_right`, `text_substr`, `text_length`, `text_contains`, `text_starts_with`, `text_ends_with`
- `name_full`, `name_parts`, `name_initials`
- `date_parse`, `date_parse_datetime`, `date_format`, `date_today`, `date_age_on`, `date_years_between`, `date_days_between`, `date_add_days`, `date_add_months`, `date_start_of_month`, `date_end_of_month`, `date_is_valid`, `date_is_before`, `date_is_after`, `date_min`, `date_max`
- `num_round`, `num_floor`, `num_ceil`, `num_abs`, `num_min`, `num_max`, `num_clamp`, `num_is_valid`, `num_parse`, `num_percent`, `num_safe_divide`
- `list_compact`, `list_flatten`, `list_unique`, `list_sort`, `list_first`, `list_last`, `list_at`, `list_length`, `list_contains`, `list_join`, `list_filter_present`, `list_to_map`
- `map_get`, `map_has`, `map_pick`, `map_omit`, `map_merge`, `map_deep_merge`, `map_set`, `map_keys`, `map_values`, `map_entries`
- `code_map`, `code_map_or_null`, `code_map_or_default`, `code_label`, `code_exists`, `code_canonical`, `code_reverse_map`, `code_normalize`
- `id_make`, `id_uuid_v5`, `id_hash`, `id_slug`, `id_clean`, `id_is_valid`
- `person_age`, `person_is_minor`, `person_sex_or_gender`, `person_normalize_phone`
- `phone_normalize`, `phone_is_valid`, `phone_country_code`, `phone_mask`
- `email_normalize`, `email_is_valid`, `email_domain`
- `geo_point`, `geo_is_valid_lat`, `geo_is_valid_lon`, `geo_normalize_lat`, `geo_normalize_lon`, `geo_admin_code`
- `address_line`, `address_normalize_country`, `address_postal_code`
- `validate_required`, `validate_error`, `validate_warn`, `validate_matches`, `validate_in`, `validate_range`
- `json_parse`, `json_stringify`, `json_path`
- `privacy_mask`, `privacy_sha256`, `privacy_redact`

---

## 10. Gap summary

The table below summarizes helpers from the celext v1 Python registry that are
NOT registered under their celext v1 names in Rust. All must be ported or
aliased before claiming full v0.2 helper parity.

| celext v1 name | Python | JS | Rust | Gap |
|---|---|---|---|---|
| `parseDate` | Present | Present | Gap (Rust: `date_parse`, different format dialect) | Rust name + format token |
| `formatDate` | Present | Present | Gap (Rust: `date_format`) | Rust name + format token |
| `parseDateTime` | Present | Present | Gap (Rust: `date_parse_datetime`, different arity/dialect) | Rust name + format token |
| `formatDateTime` | Present | Present | Gap (Rust: `date_format`) | Rust name + format token |
| `normalizePhone` | Present | Present | Gap (Rust: `phone_normalize`) | Rust name; behavior diverges on validation |
| `lookupCode` | Stub | Stub | Gap (Rust: `code_label` with different arity) | Rust name + arity |
| `mapCode` | Stub | Stub | Gap (Rust: `code_map` with different convention) | Rust name + convention |
| `splitName` | Present | Present | Gap (Rust: `name_parts` returns map, not string part) | Rust name + return type |
| `joinName` | Present | Present | Gap (Rust: `name_full` takes 2 args, not a list) | Rust name + arity |
| `iso3166` | Present | Present | Gap | Not in Rust |
| `iso4217` | Present | Present | Gap | Not in Rust |
| `iso639` | Present | Present | Gap | Not in Rust |
| `fhirReference` | Present | Present | Gap | Not in Rust |
| `regexMatch` | Present | Present | Gap (Rust: `text_matches`) | Rust name |
| `regexExtract` | Present | Present | Gap | Not in Rust |
| `defaultIfEmpty` | Present | Present | Gap (Rust: `default`) | Rust name |
| `coalesceList` | Present | Gap | Gap | Not in JS or Rust |
| `struct` | Present | Gap | Gap | Not in JS or Rust |
| `listOf` | Present | Gap | Gap | Not in JS or Rust |

Helpers that ARE registered in Rust under the same name as celext v1:

| celext v1 name | Status |
|---|---|
| `coalesce` | Present in Rust (behavior note: Rust also skips whitespace-only strings) |
