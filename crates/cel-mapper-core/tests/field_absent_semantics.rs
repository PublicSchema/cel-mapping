//! Task #16: field-absent comparison semantics.
//!
//! When a source field is missing, comparison operators must error (per CEL spec
//! and cel-js / celpy behaviour) rather than silently returning true/false via
//! the Missing sentinel.
//!
//! Missing-aware helpers (`present`, `coalesce`, `default`, …) must still work.

use cel_mapper_core::{EvaluationInput, MappingRuntime, RuntimeOptions};
use serde_json::{json, Value as JsonValue};

fn rt() -> MappingRuntime {
    MappingRuntime::new(RuntimeOptions::default())
}

fn eval_with_source(
    expr: &str,
    source: JsonValue,
) -> Result<JsonValue, cel_mapper_core::StandaloneEvalError> {
    rt().evaluate_cel_expression(
        expr,
        EvaluationInput {
            source,
            context: json!({}),
        },
    )
}

fn eval_empty(expr: &str) -> Result<JsonValue, cel_mapper_core::StandaloneEvalError> {
    eval_with_source(expr, json!({}))
}

// ---------------------------------------------------------------------------
// Comparison operators must error when source field is missing
// ---------------------------------------------------------------------------

#[test]
fn not_equals_missing_field_errors() {
    let result = eval_empty(r#"source.given_name != """#);
    assert!(
        result.is_err(),
        "expected Err for missing field in != comparison, got: {result:?}"
    );
}

#[test]
fn equals_missing_field_errors() {
    let result = eval_empty(r#"source.a == "x""#);
    assert!(
        result.is_err(),
        "expected Err for missing field in == comparison, got: {result:?}"
    );
}

#[test]
fn less_than_missing_field_errors() {
    let result = eval_empty("source.a < 5");
    assert!(
        result.is_err(),
        "expected Err for missing field in < comparison, got: {result:?}"
    );
}

#[test]
fn greater_than_missing_field_errors() {
    let result = eval_empty("source.a > 5");
    assert!(
        result.is_err(),
        "expected Err for missing field in > comparison, got: {result:?}"
    );
}

#[test]
fn less_than_or_equal_missing_field_errors() {
    let result = eval_empty("source.a <= 5");
    assert!(
        result.is_err(),
        "expected Err for missing field in <= comparison, got: {result:?}"
    );
}

#[test]
fn greater_than_or_equal_missing_field_errors() {
    let result = eval_empty("source.a >= 5");
    assert!(
        result.is_err(),
        "expected Err for missing field in >= comparison, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Missing-aware helpers must still work
// ---------------------------------------------------------------------------

#[test]
fn present_missing_field_returns_false() {
    let result = eval_empty("present(source.given_name)").unwrap();
    assert_eq!(
        result,
        json!(false),
        "present() on missing field should return false"
    );
}

#[test]
fn coalesce_missing_field_returns_fallback() {
    let result = eval_empty(r#"coalesce(source.a, "fallback")"#).unwrap();
    assert_eq!(
        result,
        json!("fallback"),
        "coalesce() should return fallback for missing field"
    );
}

#[test]
fn default_missing_field_returns_fallback() {
    let result = eval_empty(r#"default(source.a, "fallback")"#).unwrap();
    assert_eq!(
        result,
        json!("fallback"),
        "default() should return fallback for missing field"
    );
}

#[test]
fn blank_missing_field_returns_true() {
    let result = eval_empty("blank(source.a)").unwrap();
    assert_eq!(
        result,
        json!(true),
        "blank() should return true for missing field"
    );
}

#[test]
fn missing_helper_missing_field_returns_true() {
    let result = eval_empty("missing(source.a)").unwrap();
    assert_eq!(
        result,
        json!(true),
        "missing() should return true for missing field"
    );
}

#[test]
fn null_if_missing_field_returns_null() {
    // null_if is in MISSING_AWARE_HELPERS so the sentinel is injected for the
    // first arg; null_if_impl must treat the sentinel as null rather than
    // propagating it as a 30+ char string. Otherwise the output would carry
    // the sentinel as a real value.
    let result = eval_empty(r#"null_if(source.a, "x")"#).unwrap();
    assert_eq!(
        result,
        json!(null),
        "null_if() on missing first arg must return null, not the sentinel"
    );
}

// ---------------------------------------------------------------------------
// Mixed expressions
// ---------------------------------------------------------------------------

#[test]
fn present_mixed_with_strict_errors() {
    // source.a appears in both a missing-aware context (present()) and a strict
    // context (== comparison). Strict wins: no sentinel injected. present() itself
    // raises NoSuchKey before the && can short-circuit. Result: Err.
    //
    // Invariant authors should write `!present(source.a) || source.a == "x"` to
    // correctly handle missing fields — this guards the strict access with present().
    let result = eval_empty(r#"present(source.a) && source.a == "x""#);
    assert!(
        result.is_err(),
        "mixed strict/missing-aware: strict wins, no sentinel injected: {result:?}"
    );
}

#[test]
fn strict_access_first_then_present_errors() {
    // source.a == "x" is evaluated first and errors; || never reaches present().
    let result = eval_empty(r#"source.a == "x" || present(source.a)"#);
    assert!(
        result.is_err(),
        "first clause errors before short-circuit can help: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Fields present in source: comparisons must work normally
// ---------------------------------------------------------------------------

#[test]
fn comparison_present_field_works() {
    let result =
        eval_with_source(r#"source.given_name != """#, json!({"given_name": "Alice"})).unwrap();
    assert_eq!(result, json!(true));
}

#[test]
fn comparison_empty_string_present_field_works() {
    let result = eval_with_source(r#"source.given_name != """#, json!({"given_name": ""})).unwrap();
    assert_eq!(result, json!(false));
}
