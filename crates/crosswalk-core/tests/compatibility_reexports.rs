use crosswalk_core::{
    CodeEntry, CodeSystemRegistry, CompileError, CompiledCel, ErrorCode, ErrorMode,
    ExpressionPreviewResult, PrivacyMode, PublicSchemaCompileOptions, PublicSchemaDirection,
    PublicSchemaEvaluateOptions, PublicSchemaEvaluationInput, SecurityLimits,
    StandaloneExpressionInput,
};
use serde_json::json;

fn accepts_publicschema_options(_: crosswalk_publicschema::PublicSchemaCompileOptions) {}
fn accepts_publicschema_direction(_: crosswalk_publicschema::PublicSchemaDirection) {}
fn accepts_publicschema_eval_input(_: crosswalk_publicschema::PublicSchemaEvaluationInput) {}
fn accepts_security_limits(_: crosswalk_cel::SecurityLimits) {}
fn accepts_standalone_input(_: crosswalk_cel::StandaloneExpressionInput) {}
fn accepts_error_code(_: crosswalk_cel::ErrorCode) {}
fn accepts_code_registry(_: crosswalk_functions::codes::CodeSystemRegistry) {}

#[test]
fn moved_public_types_remain_assignable_from_core_paths() {
    accepts_publicschema_options(PublicSchemaCompileOptions::default());
    accepts_publicschema_direction(PublicSchemaDirection::ToTarget);
    accepts_publicschema_eval_input(PublicSchemaEvaluationInput {
        source: json!({}),
        context: json!({}),
        options: PublicSchemaEvaluateOptions {
            direction: PublicSchemaDirection::FromTarget,
            privacy: PrivacyMode::Authoring,
            ..Default::default()
        },
    });
    accepts_security_limits(SecurityLimits::default());
    accepts_standalone_input(StandaloneExpressionInput::default());
    accepts_error_code(ErrorCode::ValidationError);
    accepts_code_registry(CodeSystemRegistry::new());
}

#[test]
fn moved_public_types_preserve_serde_shapes() {
    assert_eq!(
        serde_json::to_value(PublicSchemaDirection::FromTarget).unwrap(),
        json!("from_target")
    );
    assert_eq!(
        serde_json::to_value(PrivacyMode::Authoring).unwrap(),
        json!("authoring")
    );
    assert_eq!(
        serde_json::to_value(ErrorCode::MissingRequiredValue).unwrap(),
        json!("MISSING_REQUIRED_VALUE")
    );
}

#[test]
fn moved_compile_and_preview_types_are_importable_from_core() {
    let _compiled_cel: Option<CompiledCel> = None;
    let _compile_error: Option<CompileError> = None;
    let preview = ExpressionPreviewResult::success("1".into(), "1".into(), json!(1));
    assert!(preview.is_ok());
    assert!(matches!(
        ErrorMode::parse(Some("collect")),
        ErrorMode::Collect
    ));
}

#[test]
fn code_entry_shape_is_compatible() {
    let entry = CodeEntry {
        id: "x".into(),
        label: Default::default(),
        aliases: vec!["alias".into()],
        extra: Default::default(),
    };
    assert_eq!(entry.id, "x");
}
