use crosswalk_core::{
    evaluate_cel_expression_with_input, preview_cel_expression_with_input, MappingRuntime,
    RuntimeOptions, SecurityLimits, StandaloneEvalError, StandaloneExpressionInput,
};
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeMap;
use std::sync::Arc;

fn input(
    bindings: impl IntoIterator<Item = (&'static str, JsonValue)>,
) -> StandaloneExpressionInput {
    StandaloneExpressionInput::new(
        bindings
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

#[test]
fn runtime_evaluates_arbitrary_root_bindings() {
    let rt = MappingRuntime::new(RuntimeOptions::default());
    let result = rt
        .evaluate_cel_expression_with_input(
            r#"patient.name + " @ " + encounter.id"#,
            input([
                ("patient", json!({ "name": "Asha" })),
                ("encounter", json!({ "id": "E-42" })),
            ]),
        )
        .unwrap();

    assert_eq!(result, json!("Asha @ E-42"));
}

#[test]
fn free_function_evaluates_arbitrary_root_bindings() {
    let result = evaluate_cel_expression_with_input(
        "claim.total + payment.amount",
        input([
            ("claim", json!({ "total": 7 })),
            ("payment", json!({ "amount": 5 })),
        ]),
        &SecurityLimits::default(),
        Arc::new(Default::default()),
    )
    .unwrap();

    assert_eq!(result, json!(12));
}

#[test]
fn missing_aware_helpers_work_for_arbitrary_roots() {
    let rt = MappingRuntime::new(RuntimeOptions::default());

    assert_eq!(
        rt.evaluate_cel_expression_with_input(
            "present(patient.middle_name)",
            input([("patient", json!({ "given_name": "Asha" }))]),
        )
        .unwrap(),
        json!(false)
    );
    assert_eq!(
        rt.evaluate_cel_expression_with_input(
            r#"default(patient.middle_name, "N/A")"#,
            input([("patient", json!({ "given_name": "Asha" }))]),
        )
        .unwrap(),
        json!("N/A")
    );
}

#[test]
fn strict_access_for_arbitrary_root_missing_field_still_errors() {
    let rt = MappingRuntime::new(RuntimeOptions::default());
    let result = rt.evaluate_cel_expression_with_input(
        r#"patient.middle_name == "x""#,
        input([("patient", json!({ "given_name": "Asha" }))]),
    );

    assert!(
        result.is_err(),
        "strict missing access should not receive a Missing sentinel: {result:?}"
    );
}

#[test]
fn preview_evaluates_arbitrary_root_bindings() {
    let rt = MappingRuntime::new(RuntimeOptions::default());
    let preview = rt.preview_cel_expression_with_input(
        r#"default(patient.middle_name, "N/A")"#,
        input([("patient", json!({ "given_name": "Asha" }))]),
    );

    assert!(preview.is_ok(), "{:?}", preview.issues);
    assert_eq!(preview.value, Some(json!("N/A")));
}

#[test]
fn runtime_generic_input_preserves_ctx_defaults() {
    let rt = MappingRuntime::new(RuntimeOptions {
        timezone: Some("Asia/Bangkok".to_string()),
        ..Default::default()
    });

    let result = rt
        .evaluate_cel_expression_with_input("ctx.timezone", input([("patient", json!({}))]))
        .unwrap();

    assert_eq!(result, json!("Asia/Bangkok"));
}

#[test]
fn invalid_binding_names_are_rejected_for_evaluate_and_preview() {
    let rt = MappingRuntime::new(RuntimeOptions::default());
    let mut root_bindings = BTreeMap::new();
    root_bindings.insert("bad-name".to_string(), json!({}));
    let input = StandaloneExpressionInput::new(root_bindings);

    let result = rt.evaluate_cel_expression_with_input("1", input.clone());
    assert!(matches!(
        result,
        Err(StandaloneEvalError::InvalidBindingName { name, .. }) if name == "bad-name"
    ));

    let preview = preview_cel_expression_with_input(
        "1",
        input,
        &SecurityLimits::default(),
        Arc::new(Default::default()),
    );
    assert!(!preview.is_ok());
    assert!(preview.issues[0]
        .message
        .contains("invalid root binding name `bad-name`"));
}
