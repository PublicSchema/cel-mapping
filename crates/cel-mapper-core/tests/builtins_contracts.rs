use cel_mapper_core::{EvaluationInput, MappingRuntime, RuntimeOptions, StandaloneEvalError};
use serde_json::{json, Value as JsonValue};

fn empty_input() -> EvaluationInput {
    EvaluationInput {
        source: json!({}),
        context: json!({}),
    }
}

fn eval(expr: &str, rt: &MappingRuntime) -> Result<JsonValue, StandaloneEvalError> {
    rt.evaluate_cel_expression(expr, empty_input())
}

#[test]
fn type_int_rejects_non_integral_float() {
    let rt = MappingRuntime::new(RuntimeOptions::default());
    let err = eval("type_int(1.5)", &rt).unwrap_err();
    assert!(
        err.to_string().contains("integral"),
        "expected integral error, got {err}"
    );
}

#[test]
fn type_int_rejects_uint_above_i64_max() {
    let rt = MappingRuntime::new(RuntimeOptions::default());
    let err = eval("type_int(9223372036854775808u)", &rt).unwrap_err();
    assert!(
        err.to_string().contains("overflow"),
        "expected overflow error, got {err}"
    );
}

#[test]
fn date_parse_datetime_explicit_format_uses_runtime_timezone_for_offsetless_input() {
    let rt = MappingRuntime::new(RuntimeOptions {
        timezone: Some("Asia/Bangkok".into()),
        ..Default::default()
    });
    let out = eval(
        "date.parse_datetime('2024-01-15 10:30:00', 'yyyy-MM-dd HH:mm:ss')",
        &rt,
    )
    .unwrap();
    assert_eq!(out, json!("2024-01-15T10:30:00+07:00"));
}

#[test]
fn date_parse_datetime_explicit_offset_format_uses_provided_offset() {
    let rt = MappingRuntime::new(RuntimeOptions {
        timezone: Some("Asia/Bangkok".into()),
        ..Default::default()
    });
    let out = eval(
        "date.parse_datetime('2024-01-15 10:30:00+05:30', 'yyyy-MM-dd HH:mm:ssXXX')",
        &rt,
    )
    .unwrap();
    assert_eq!(out, json!("2024-01-15T10:30:00+05:30"));
}
