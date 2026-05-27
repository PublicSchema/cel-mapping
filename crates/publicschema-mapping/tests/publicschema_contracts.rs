use cel_evaluator::SecurityLimits;
use mapping_functions::codes::CodeSystemRegistry;
use publicschema_mapping::{
    compile_publicschema_mapping, evaluate_publicschema_mapping, PrivacyMode,
    PublicSchemaCompileOptions, PublicSchemaDirection, PublicSchemaEvaluateOptions,
    PublicSchemaEvaluationInput,
};
use serde_json::json;

#[test]
fn publicschema_crate_compiles_and_evaluates_without_core_dependency() {
    let mapping = r#"
version: "0.2"
property_mappings:
  - source: /name
    target: /display
    formula: text.upper(source)
"#;
    let compiled = compile_publicschema_mapping(
        mapping,
        &SecurityLimits::default(),
        CodeSystemRegistry::new(),
        None,
        PublicSchemaCompileOptions::default(),
    )
    .unwrap();

    let output = evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source: json!({"name": "ada"}),
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                direction: PublicSchemaDirection::ToTarget,
                errors_mode: None,
                privacy: PrivacyMode::Authoring,
            },
        },
    );

    assert!(output.ok, "{:?}", output.errors);
    assert_eq!(output.output, json!({"display": "ADA"}));
}
