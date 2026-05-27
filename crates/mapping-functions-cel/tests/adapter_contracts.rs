use cel::Program;
use mapping_functions_cel::{FunctionRegistry, FunctionRequestContext};
use serde_json::json;
use std::{collections::BTreeSet, sync::Arc};

fn eval(expr: &str, request: FunctionRequestContext) -> serde_json::Value {
    let mut ctx = cel::Context::default();
    mapping_functions_cel::register_stdlib(&mut ctx, Arc::new(FunctionRegistry::new()));
    mapping_functions_cel::eval_ctx::set_eval_ctx(request);
    let program = Program::compile(expr).unwrap();
    let value = program.execute(&ctx).unwrap();
    value.json().unwrap()
}

#[test]
fn regex_extract_no_match_is_empty_string_in_cel_adapter() {
    assert_eq!(
        mapping_functions::text::regex_extract("hello", r"\d+", 0).unwrap(),
        None
    );
    assert_eq!(
        eval(
            r#"text_regex_extract("hello", "\\d+", 0)"#,
            FunctionRequestContext::default()
        ),
        json!("")
    );
}

#[test]
fn adapter_resolves_today_timezone_and_country_fallbacks() {
    let request = FunctionRequestContext {
        country: Some("US".into()),
        timezone: Some("Asia/Bangkok".into()),
        today: Some("2026-05-27".into()),
    };

    assert_eq!(eval("date_today()", request.clone()), json!("2026-05-27"));
    assert_eq!(
        eval(
            r#"date_parse_datetime("2024-01-15 10:30:00", "yyyy-MM-dd HH:mm:ss")"#,
            request.clone()
        ),
        json!("2024-01-15T10:30:00+07:00")
    );
    assert_eq!(
        eval(r#"phone_normalize("(650) 253-0000", null)"#, request),
        json!("+16502530000")
    );
}

#[test]
fn helper_metadata_covers_registered_public_helpers_without_duplicates() {
    let metadata = mapping_functions_cel::helper_metadata();
    let names: BTreeSet<_> = metadata.iter().map(|helper| helper.name).collect();

    assert_eq!(
        metadata.len(),
        names.len(),
        "duplicate helper metadata names"
    );
    for expected in [
        "present",
        "text_lower",
        "text_regex_extract",
        "date_is_valid",
        "list_of",
        "coalesce_list",
        "map_of",
        "id_uuid_v5",
        "privacy_sha256",
        "fhir_reference",
        "json_path",
    ] {
        assert!(names.contains(expected), "missing metadata for {expected}");
    }
}

#[test]
fn date_is_valid_is_advertised_and_registered() {
    assert_eq!(
        eval(
            r#"date_is_valid("2026-05-27")"#,
            FunctionRequestContext::default()
        ),
        json!(true)
    );
    assert_eq!(
        eval(
            r#"date_is_valid("not-a-date")"#,
            FunctionRequestContext::default()
        ),
        json!(false)
    );
}
