use crosswalk_core::{EvaluationInput, MappingRuntime, RuntimeOptions, StandaloneEvalError};
use serde_json::{json, Value as JsonValue};

fn rt() -> MappingRuntime {
    MappingRuntime::new(RuntimeOptions::default())
}

fn eval(expr: &str) -> Result<JsonValue, StandaloneEvalError> {
    let rt = rt();
    rt.evaluate_cel_expression(
        expr,
        EvaluationInput {
            source: json!({}),
            context: json!({}),
        },
    )
}

// ---------------------------------------------------------------------------
// text_regex_extract
// ---------------------------------------------------------------------------

#[test]
fn text_regex_extract_group_zero_full_match() {
    let out = eval(r#"text_regex_extract("hello world", "w\\w+", 0)"#).unwrap();
    assert_eq!(out, json!("world"));
}

#[test]
fn text_regex_extract_named_group_by_index() {
    let out = eval(r#"text_regex_extract("2024-05-01", "(\\d{4})-(\\d{2})-(\\d{2})", 2)"#).unwrap();
    assert_eq!(out, json!("05"));
}

#[test]
fn text_regex_extract_no_match_returns_empty_string() {
    let out = eval(r#"text_regex_extract("hello", "\\d+", 0)"#).unwrap();
    assert_eq!(out, json!(""));
}

#[test]
fn text_regex_extract_group_out_of_range_returns_empty_string() {
    // Pattern has 1 group; requesting group 5
    let out = eval(r#"text_regex_extract("hello", "(hello)", 5)"#).unwrap();
    assert_eq!(out, json!(""));
}

#[test]
fn text_regex_extract_empty_input_returns_empty_string() {
    let out = eval(r#"text_regex_extract("", ".*", 0)"#).unwrap();
    assert_eq!(out, json!(""));
}

// ---------------------------------------------------------------------------
// fhir_reference
// ---------------------------------------------------------------------------

#[test]
fn fhir_reference_builds_slash_path() {
    let out = eval(r#"fhir_reference("Patient", "123")"#).unwrap();
    assert_eq!(out, json!("Patient/123"));
}

#[test]
fn fhir_reference_arbitrary_resource_types() {
    let out = eval(r#"fhir_reference("Observation", "abc-def")"#).unwrap();
    assert_eq!(out, json!("Observation/abc-def"));
}

#[test]
fn fhir_reference_empty_id_produces_trailing_slash() {
    let out = eval(r#"fhir_reference("Patient", "")"#).unwrap();
    assert_eq!(out, json!("Patient/"));
}

// ---------------------------------------------------------------------------
// coalesce_list
// ---------------------------------------------------------------------------

#[test]
fn coalesce_list_returns_first_non_empty() {
    let out = eval(r#"coalesce_list([], ["a", "b"], ["c"])"#).unwrap();
    assert_eq!(out, json!(["a", "b"]));
}

#[test]
fn coalesce_list_all_empty_returns_empty() {
    let out = eval(r#"coalesce_list([], [])"#).unwrap();
    assert_eq!(out, json!([]));
}

#[test]
fn coalesce_list_no_args_returns_empty() {
    let out = eval(r#"coalesce_list()"#).unwrap();
    assert_eq!(out, json!([]));
}

#[test]
fn coalesce_list_skips_null() {
    // Null is not a list so it is skipped; the non-empty list wins
    let out = eval(r#"coalesce_list(null, ["x"])"#).unwrap();
    assert_eq!(out, json!(["x"]));
}

#[test]
fn coalesce_list_first_non_empty_wins_even_with_later_longer_list() {
    let out = eval(r#"coalesce_list(["a"], ["b", "c", "d"])"#).unwrap();
    assert_eq!(out, json!(["a"]));
}

// ---------------------------------------------------------------------------
// map_of
// ---------------------------------------------------------------------------

#[test]
fn map_of_two_pairs() {
    let out = eval(r#"map_of("a", 1, "b", 2)"#).unwrap();
    // JSON objects are order-independent
    assert_eq!(out["a"], json!(1));
    assert_eq!(out["b"], json!(2));
}

#[test]
fn map_of_no_args_returns_empty_map() {
    let out = eval(r#"map_of()"#).unwrap();
    assert!(out.as_object().unwrap().is_empty());
}

#[test]
fn map_of_odd_arg_count_errors() {
    let err = eval(r#"map_of("a", 1, "b")"#).unwrap_err();
    assert!(
        err.to_string().contains("even"),
        "expected even-args error, got: {err}"
    );
}

#[test]
fn map_of_non_string_key_errors() {
    let err = eval(r#"map_of(1, "v")"#).unwrap_err();
    assert!(
        err.to_string().contains("string"),
        "expected string-key error, got: {err}"
    );
}

#[test]
fn map_of_values_can_be_any_type() {
    let out = eval(r#"map_of("s", "hello", "n", 42, "b", true)"#).unwrap();
    assert_eq!(out["s"], json!("hello"));
    assert_eq!(out["n"], json!(42));
    assert_eq!(out["b"], json!(true));
}

// ---------------------------------------------------------------------------
// list_of
// ---------------------------------------------------------------------------

#[test]
fn list_of_no_args_returns_empty_list() {
    let out = eval(r#"list_of()"#).unwrap();
    assert_eq!(out, json!([]));
}

#[test]
fn list_of_single_item() {
    let out = eval(r#"list_of("hello")"#).unwrap();
    assert_eq!(out, json!(["hello"]));
}

#[test]
fn list_of_mixed_types() {
    let out = eval(r#"list_of(1, "two", true)"#).unwrap();
    assert_eq!(out, json!([1, "two", true]));
}

#[test]
fn list_of_passes_null_through() {
    let out = eval(r#"list_of(null, "after")"#).unwrap();
    // null should be present in the list
    assert_eq!(out.as_array().unwrap().len(), 2);
    assert!(out[0].is_null());
    assert_eq!(out[1], json!("after"));
}
