use cel_evaluator::{ErrorCode, SecurityLimits};
use mapping_functions::codes::CodeSystemRegistry;
use publicschema_mapping::{
    compile_publicschema_mapping, evaluate_publicschema_mapping, CompiledPublicSchemaMapping,
    PrivacyMode, PublicSchemaDirection, PublicSchemaEvaluateOptions, PublicSchemaEvaluationInput,
    PublicSchemaTransformOutput,
};
use serde_json::json;

#[derive(Default)]
struct RuntimeOptions;

struct MappingRuntime {
    limits: SecurityLimits,
    codes: CodeSystemRegistry,
}

impl MappingRuntime {
    fn new(_: RuntimeOptions) -> Self {
        Self {
            limits: SecurityLimits::default(),
            codes: CodeSystemRegistry::new(),
        }
    }

    fn compile_publicschema_mapping(
        &self,
        mapping: &str,
        options: publicschema_mapping::PublicSchemaCompileOptions,
    ) -> Result<CompiledPublicSchemaMapping, cel_evaluator::CompileError> {
        compile_publicschema_mapping(mapping, &self.limits, self.codes.clone(), None, options)
    }

    fn evaluate_publicschema_mapping(
        &self,
        mapping: &CompiledPublicSchemaMapping,
        input: PublicSchemaEvaluationInput,
    ) -> PublicSchemaTransformOutput {
        evaluate_publicschema_mapping(mapping, input)
    }
}

fn eval(mapping: &str, source: serde_json::Value) -> PublicSchemaTransformOutput {
    let rt = MappingRuntime::new(RuntimeOptions);
    let compiled = rt
        .compile_publicschema_mapping(mapping, Default::default())
        .unwrap();
    rt.evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source,
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                privacy: PrivacyMode::Authoring,
                ..Default::default()
            },
        },
    )
}

#[test]
fn identity_copy_uses_json_pointer_source_and_target() {
    let out = eval(
        r#"
version: "0.2"
id: demo
property_mappings:
  - source: /person/name
    target: /name/full
"#,
        json!({"person": {"name": "Ada"}}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({"name": {"full": "Ada"}}));
    assert_eq!(out.log[0].status, "applied");
    assert_eq!(out.log[0].resolved_input, Some(json!("Ada")));
}

#[test]
fn explicit_source_formula_is_identity() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /gender
    target: /sex
    formula:
      to_target:
        expression: source
"#,
        json!({"gender": "female"}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({"sex": "female"}));
}

#[test]
fn value_mappings_apply_source_to_target_crosswalk() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - rule_id: sex
    source: /sex
    target: /sex
    formula: source
    quality: close
    value_mappings:
      - source_value: M
        target_value: male
        quality: exact
      - source_value: F
        target_value: female
        quality: exact
"#,
        json!({"sex": "M"}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({"sex": "male"}));
    assert_eq!(out.log[0].rule_id.as_deref(), Some("sex"));
    assert_eq!(out.log[0].quality.as_deref(), Some("close"));
    assert_eq!(out.log[0].resolved_output, Some(json!("male")));
}

#[test]
fn value_mappings_apply_target_to_source_crosswalk() {
    let rt = MappingRuntime::new(RuntimeOptions);
    let compiled = rt
        .compile_publicschema_mapping(
            r#"
version: "0.2"
property_mappings:
  - source: /sex
    target: /sex
    value_mappings:
      - source_value: M
        target_value: male
      - source_value: F
        target_value: female
"#,
            Default::default(),
        )
        .unwrap();
    let out = rt.evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source: json!({"sex": "female"}),
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                direction: PublicSchemaDirection::FromTarget,
                privacy: PrivacyMode::Authoring,
                ..Default::default()
            },
        },
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({"sex": "F"}));
}

#[test]
fn value_mappings_reverse_ambiguous_crosswalk_fails_closed() {
    let rt = MappingRuntime::new(RuntimeOptions);
    let compiled = rt
        .compile_publicschema_mapping(
            r#"
version: "0.2"
property_mappings:
  - source: /local_code
    target: /canonical
    value_mappings:
      - source_value: A
        target_value: shared
      - source_value: B
        target_value: shared
"#,
            Default::default(),
        )
        .unwrap();
    let out = rt.evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source: json!({"canonical": "shared"}),
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                direction: PublicSchemaDirection::FromTarget,
                errors_mode: Some("collect".into()),
                privacy: PrivacyMode::Authoring,
            },
        },
    );
    assert!(!out.ok);
    assert_eq!(out.output, json!({}));
    assert_eq!(out.log[0].status, "value_unmapped");
    assert!(
        out.log[0].issues[0]
            .message
            .contains("ambiguous reverse value_mapping"),
        "{:?}",
        out.log[0].issues
    );
    assert_eq!(out.errors[0].code, ErrorCode::ValidationError);
    assert!(
        out.errors[0]
            .message
            .contains("ambiguous reverse value_mapping"),
        "{:?}",
        out.errors
    );
}

#[test]
fn unmapped_value_mapping_value_fails_closed() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /sex
    target: /sex
    value_mappings:
      - source_value: M
        target_value: male
"#,
        json!({"sex": "U"}),
    );
    assert!(!out.ok);
    assert_eq!(out.output, json!({}));
    assert_eq!(out.log[0].status, "value_unmapped");
}

#[test]
fn formula_resolves_source_root_and_target_alias_in_to_target_direction() {
    // Spec §6.1: ToTarget (external→profile) binds `target` to root; `profile` is absent.
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /given
    target: /display
    formula:
      to_target:
        expression: 'source + " " + root.family + " " + target.family'
"#,
        json!({"given": "Ada", "family": "Lovelace"}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({"display": "Ada Lovelace Lovelace"}));
}

#[test]
fn formula_resolves_profile_alias_in_from_target_direction() {
    // Spec §6.1: FromTarget (profile→external) binds `profile` to root; `target` is absent.
    let rt = MappingRuntime::new(RuntimeOptions);
    let compiled = rt
        .compile_publicschema_mapping(
            r#"
version: "0.2"
property_mappings:
  - source: /external_path
    target: /family
    formula:
      from_target:
        expression: 'profile.family'
"#,
            Default::default(),
        )
        .unwrap();
    let out = rt.evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source: json!({"family": "Lovelace"}),
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                direction: PublicSchemaDirection::FromTarget,
                privacy: PrivacyMode::Authoring,
                ..Default::default()
            },
        },
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({"external_path": "Lovelace"}));
}

#[test]
fn missing_optional_is_defaulted() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /missing
    target: /x
"#,
        json!({}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({}));
    // Spec §5.2: missing optional source → status "defaulted" (spec vocabulary: applied|defaulted|skipped).
    assert_eq!(out.log[0].status, "defaulted");
}

#[test]
fn missing_required_fails_strict() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /missing
    target: /x
    required: true
"#,
        json!({}),
    );
    assert!(!out.ok);
    assert_eq!(out.errors[0].code, ErrorCode::MissingRequiredValue);
    // Spec §8.3: missing required source → status "missing".
    assert_eq!(out.log[0].status, "missing");
}

#[test]
fn wrong_direction_does_not_identity_fallback() {
    let rt = MappingRuntime::new(RuntimeOptions);
    let compiled = rt
        .compile_publicschema_mapping(
            r#"
version: "0.2"
property_mappings:
  - source: /a
    target: /b
    formula:
      to_target:
        expression: "source + 1"
"#,
            Default::default(),
        )
        .unwrap();
    let out = rt.evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source: json!({"b": 2}),
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                direction: PublicSchemaDirection::FromTarget,
                errors_mode: Some("collect".into()),
                privacy: PrivacyMode::Authoring,
            },
        },
    );
    assert!(!out.ok);
    assert_eq!(out.output, json!({}));
    // Spec §8.3: a non-identity formula that has no expression for the executing
    // direction is a formula_error (identity fallback is forbidden by §5.3 / §8.2).
    assert_eq!(out.log[0].status, "formula_error");
}

#[test]
fn duplicate_target_last_write_wins_with_warning() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /a
    target: /x
  - source: /b
    target: /x
"#,
        json!({"a": 1, "b": 2}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output, json!({"x": 2}));
    assert!(
        out.warnings
            .iter()
            .any(|w| w.message.contains("last write wins")),
        "{:?}",
        out.warnings
    );
}

#[test]
fn numeric_pointer_writes_create_array_padding() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /name
    target: /items/3/name
"#,
        json!({"name": "Ada"}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.output["items"][0], serde_json::Value::Null);
    assert_eq!(out.output["items"][3]["name"], json!("Ada"));
}

#[test]
fn rule_id_is_propagated_into_log() {
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - id: given-name
    source: /given
    target: /given_name
    quality: exact
"#,
        json!({"given": "Ada"}),
    );
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    assert_eq!(out.log[0].rule_id.as_deref(), Some("given-name"));
    assert_eq!(out.log[0].quality.as_deref(), Some("exact"));
    // Spec §9 example field names.
    assert_eq!(out.log[0].source_path, "/given");
    assert_eq!(out.log[0].target_path, "/given_name");
}

#[test]
fn write_error_emits_log_entry_with_status() {
    // Writing /a/b when /a is a string is a write error.
    let rt = MappingRuntime::new(RuntimeOptions);
    let compiled = rt
        .compile_publicschema_mapping(
            r#"
version: "0.2"
property_mappings:
  - id: r1
    source: /seed
    target: /a
  - id: r2
    source: /seed
    target: /a/b
"#,
            Default::default(),
        )
        .unwrap();
    let out = rt.evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source: json!({"seed": "x"}),
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                errors_mode: Some("collect".into()),
                privacy: PrivacyMode::Authoring,
                ..Default::default()
            },
        },
    );
    assert!(!out.ok);
    let r2 = out
        .log
        .iter()
        .find(|e| e.rule_id.as_deref() == Some("r2"))
        .expect("write_error log entry must exist for r2");
    assert_eq!(r2.status, "write_error");
    assert!(!r2.issues.is_empty(), "write_error log carries issues");
}

#[test]
fn canonical_hash_is_stable_across_key_order_and_uses_spec_label() {
    let rt = MappingRuntime::new(RuntimeOptions);
    let a = rt
        .compile_publicschema_mapping(
            r#"
version: "0.2"
id: demo
source: src
target: tgt
property_mappings:
  - id: r1
    source: /a
    target: /a
"#,
            Default::default(),
        )
        .unwrap();
    // Same logical document, different key order in YAML.
    let b = rt
        .compile_publicschema_mapping(
            r#"
target: tgt
source: src
id: demo
version: "0.2"
property_mappings:
  - id: r1
    target: /a
    source: /a
"#,
            Default::default(),
        )
        .unwrap();
    assert_eq!(a.meta.deterministic_hash, b.meta.deterministic_hash);
    assert_eq!(a.meta.hash_status, "canonical");
    assert_eq!(a.meta.deterministic_hash.len(), 64); // sha256 hex
}

#[test]
fn profile_alias_is_absent_in_to_target_direction() {
    // Per spec §6.1, in ToTarget direction `profile` MUST be absent.
    // Referencing it should fail-closed (formula_error), not silently identity-copy.
    let rt = MappingRuntime::new(RuntimeOptions);
    let compiled = rt
        .compile_publicschema_mapping(
            r#"
version: "0.2"
property_mappings:
  - source: /given
    target: /display
    formula:
      to_target:
        expression: 'profile.family'
"#,
            Default::default(),
        )
        .unwrap();
    let out = rt.evaluate_publicschema_mapping(
        &compiled,
        PublicSchemaEvaluationInput {
            source: json!({"given": "Ada", "family": "Lovelace"}),
            context: json!({}),
            options: PublicSchemaEvaluateOptions {
                errors_mode: Some("collect".into()),
                privacy: PrivacyMode::Authoring,
                ..Default::default()
            },
        },
    );
    assert!(
        !out.ok,
        "profile alias must not be defined in ToTarget direction"
    );
    assert_eq!(out.log[0].status, "formula_error");
}

#[test]
fn malformed_pointer_escape_is_rejected() {
    // RFC 6901 §3: a `~` not followed by 0 or 1 is invalid.
    let out = eval(
        r#"
version: "0.2"
property_mappings:
  - source: /weird~2segment
    target: /x
"#,
        json!({"weird~2segment": "v"}),
    );
    // Pointer with invalid escape resolves to None (missing), so the optional rule
    // is defaulted. We do not silently treat ~2 as a literal.
    assert_eq!(out.log[0].status, "defaulted");
}
