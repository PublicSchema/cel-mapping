use crosswalk_cel::SecurityLimits;
use crosswalk_functions::codes::CodeSystemRegistry;
use crosswalk_publicschema::{
    compile_publicschema_mapping, evaluate_publicschema_mapping, CompiledPublicSchemaMapping,
    PrivacyMode, PublicSchemaCompileOptions, PublicSchemaDirection, PublicSchemaEvaluateOptions,
    PublicSchemaEvaluationInput, PublicSchemaTransformOutput,
};
use serde_json::json;

fn compile(mapping: &str, limits: &SecurityLimits) -> CompiledPublicSchemaMapping {
    compile_publicschema_mapping(
        mapping,
        limits,
        CodeSystemRegistry::new(),
        None,
        PublicSchemaCompileOptions::default(),
    )
    .unwrap()
}

fn evaluate(
    compiled: &CompiledPublicSchemaMapping,
    source: serde_json::Value,
    context: serde_json::Value,
) -> PublicSchemaTransformOutput {
    evaluate_publicschema_mapping(
        compiled,
        PublicSchemaEvaluationInput {
            source,
            context,
            options: PublicSchemaEvaluateOptions {
                direction: PublicSchemaDirection::ToTarget,
                errors_mode: Some("collect".into()),
                privacy: PrivacyMode::Authoring,
            },
        },
    )
}

#[test]
fn publicschema_crate_compiles_and_evaluates_without_core_dependency() {
    let mapping = r#"
version: "0.2"
property_mappings:
  - source: /name
    target: /display
    formula: text.upper(source)
"#;
    let compiled = compile(mapping, &SecurityLimits::default());
    let output = evaluate(&compiled, json!({"name": "ada"}), json!({}));

    assert!(output.ok, "{:?}", output.errors);
    assert_eq!(output.output, json!({"display": "ADA"}));
}

#[test]
fn direct_publicschema_date_today_does_not_use_ctx_today_as_helper_fallback() {
    let compiled = compile(
        r#"
version: "0.2"
property_mappings:
  - source: /name
    target: /today
    formula: date_today()
"#,
        &SecurityLimits::default(),
    );

    let output = evaluate(
        &compiled,
        json!({"name": "ada"}),
        json!({"today": "2099-01-02"}),
    );

    assert!(!output.ok);
    assert_eq!(output.output, json!({}));
    assert_eq!(output.log[0].status, "formula_error");
    assert!(
        output.errors[0].message.contains("today"),
        "{:?}",
        output.errors
    );
}

#[test]
fn direct_publicschema_offsetless_datetime_does_not_use_ctx_timezone_as_helper_fallback() {
    let compiled = compile(
        r#"
version: "0.2"
property_mappings:
  - source: /name
    target: /dt
    formula: date_parse_datetime("2024-01-15 10:30:00", "yyyy-MM-dd HH:mm:ss")
"#,
        &SecurityLimits::default(),
    );

    let output = evaluate(
        &compiled,
        json!({"name": "ada"}),
        json!({"timezone": "Asia/Bangkok"}),
    );

    assert!(!output.ok);
    assert_eq!(output.output, json!({}));
    assert_eq!(output.log[0].status, "formula_error");
    assert!(
        output.errors[0].message.contains("timezone"),
        "{:?}",
        output.errors
    );
}

#[test]
fn direct_publicschema_evaluate_does_not_install_function_budget_guard() {
    let limits = SecurityLimits {
        max_string_bytes: 1,
        ..Default::default()
    };
    let compiled = compile(
        r#"
version: "0.2"
property_mappings:
  - source: /name
    target: /expanded
    formula: text_replace("aa", "a", "bbbb")
"#,
        &limits,
    );

    let output = evaluate(&compiled, json!({"name": "ada"}), json!({}));

    assert!(output.ok, "{:?}", output.errors);
    assert_eq!(output.output, json!({"expanded": "bbbbbbbb"}));
}
