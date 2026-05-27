// SPDX-License-Identifier: Apache-2.0
use crosswalk_core::{EvaluationInput, MappingRuntime, RuntimeOptions, StandaloneEvalError};
use serde_json::{json, Value as JsonValue};

fn rt() -> MappingRuntime {
    MappingRuntime::new(RuntimeOptions::default())
}

fn eval(expr: &str) -> Result<JsonValue, StandaloneEvalError> {
    rt().evaluate_cel_expression(
        expr,
        EvaluationInput {
            source: json!({}),
            context: json!({}),
        },
    )
}

// ---------------------------------------------------------------------------
// code_normalize(system, value) — 2-arg form
// ---------------------------------------------------------------------------

// iso3166-alpha3
#[test]
fn code_normalize_iso3166_alpha2_to_alpha3() {
    assert_eq!(
        eval(r#"code_normalize("iso3166-alpha3", "KE")"#).unwrap(),
        json!("KEN")
    );
}

#[test]
fn code_normalize_iso3166_us() {
    assert_eq!(
        eval(r#"code_normalize("iso3166-alpha3", "US")"#).unwrap(),
        json!("USA")
    );
}

#[test]
fn code_normalize_iso3166_already_alpha3_identity() {
    assert_eq!(
        eval(r#"code_normalize("iso3166-alpha3", "KEN")"#).unwrap(),
        json!("KEN")
    );
}

#[test]
fn code_normalize_iso3166_lowercase_input() {
    assert_eq!(
        eval(r#"code_normalize("iso3166-alpha3", "ke")"#).unwrap(),
        json!("KEN")
    );
}

#[test]
fn code_normalize_iso3166_unknown_code_errors() {
    let err = eval(r#"code_normalize("iso3166-alpha3", "ZZ")"#).unwrap_err();
    assert!(
        err.to_string().contains("unknown code") || err.to_string().contains("ZZ"),
        "expected unknown-code error, got: {err}"
    );
}

// iso4217
#[test]
fn code_normalize_iso4217_uppercase_passthrough() {
    assert_eq!(
        eval(r#"code_normalize("iso4217", "USD")"#).unwrap(),
        json!("USD")
    );
}

#[test]
fn code_normalize_iso4217_lowercase_normalizes() {
    assert_eq!(
        eval(r#"code_normalize("iso4217", "eur")"#).unwrap(),
        json!("EUR")
    );
}

#[test]
fn code_normalize_iso4217_kes() {
    assert_eq!(
        eval(r#"code_normalize("iso4217", "KES")"#).unwrap(),
        json!("KES")
    );
}

#[test]
fn code_normalize_iso4217_unknown_errors() {
    let err = eval(r#"code_normalize("iso4217", "ZZZ")"#).unwrap_err();
    assert!(
        err.to_string().contains("unknown code") || err.to_string().contains("ZZZ"),
        "expected unknown-code error, got: {err}"
    );
}

// iso639-3
#[test]
fn code_normalize_iso639_alpha2_to_alpha3() {
    assert_eq!(
        eval(r#"code_normalize("iso639-3", "en")"#).unwrap(),
        json!("eng")
    );
}

#[test]
fn code_normalize_iso639_sw_to_swa() {
    assert_eq!(
        eval(r#"code_normalize("iso639-3", "sw")"#).unwrap(),
        json!("swa")
    );
}

#[test]
fn code_normalize_iso639_already_alpha3_identity() {
    assert_eq!(
        eval(r#"code_normalize("iso639-3", "eng")"#).unwrap(),
        json!("eng")
    );
}

#[test]
fn code_normalize_iso639_unknown_errors() {
    let err = eval(r#"code_normalize("iso639-3", "zz")"#).unwrap_err();
    assert!(
        err.to_string().contains("unknown code") || err.to_string().contains("zz"),
        "expected unknown-code error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// code_normalize(value) — 1-arg form unchanged
// ---------------------------------------------------------------------------

#[test]
fn code_normalize_one_arg_trims_and_lowercases() {
    assert_eq!(
        eval(r#"code_normalize("  Hello World  ")"#).unwrap(),
        json!("hello world")
    );
}

#[test]
fn code_normalize_one_arg_already_lowercase() {
    assert_eq!(eval(r#"code_normalize("abc")"#).unwrap(), json!("abc"));
}

// ---------------------------------------------------------------------------
// Unknown system — fail closed, not silent wrong result
// ---------------------------------------------------------------------------

#[test]
fn code_normalize_unknown_system_errors() {
    let err = eval(r#"code_normalize("no-such-system", "KE")"#).unwrap_err();
    assert!(
        err.to_string().contains("unknown") || err.to_string().contains("no-such-system"),
        "expected unknown-system error, got: {err}"
    );
}
