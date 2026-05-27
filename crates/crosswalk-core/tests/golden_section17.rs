//! Golden-style integration test for `spec.md` §17 example mapping + fixtures.

use crosswalk_core::runtime::{EvaluationInput, MappingRuntime, RuntimeOptions};
use serde_json::{json, Value};

fn load_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn section17_example_end_to_end() {
    let yaml = load_fixture("spec_section17_mapping.yaml");
    let source: Value = serde_json::from_str(&load_fixture("spec_section17_source.json")).unwrap();
    let ctx: Value = serde_json::from_str(&load_fixture("spec_section17_ctx.json")).unwrap();

    let rt = MappingRuntime::new(RuntimeOptions::default());
    let compiled = rt.compile_mapping(&yaml).unwrap();
    let out = rt.evaluate(
        &compiled,
        EvaluationInput {
            source,
            context: ctx,
        },
    );

    assert!(
        out.errors.is_empty(),
        "unexpected mapping errors: {:?}",
        out.errors
    );

    let households = out.records.get("households").expect("households record");
    assert_eq!(households.len(), 1);
    let h = &households[0];
    assert_eq!(h["external_id"], json!("h1"));
    assert_eq!(h["village"], json!("oak lane"));
    assert!(
        h["id"].as_str().unwrap().starts_with("household_"),
        "id: {:?}",
        h["id"]
    );
    let submitted = h["submitted_at"].as_str().unwrap();
    assert!(
        submitted.contains("2024-01-15"),
        "submitted_at: {submitted}"
    );

    let people = out.records.get("people").expect("people record");
    assert_eq!(people.len(), 1);
    let p = &people[0];
    assert_eq!(p["name"], json!("John Doe"));
    assert_eq!(p["birth_date"], json!("1990-05-20"));
    assert_eq!(p["age"], json!(34));
    assert_eq!(p["gender"], json!("canonical.gender.male"));
    assert_eq!(p["phone"], json!("+16502530000"));
    assert_eq!(p["is_minor"], json!(false));
    assert!(
        p["household_id"].as_str().unwrap() == h["id"].as_str().unwrap(),
        "household_id should match household row id"
    );
    assert!(p["id"].as_str().unwrap().starts_with("person_"));
}

#[test]
fn budget_rejects_oversized_json_input_via_source_field() {
    let mut rt = MappingRuntime::new(RuntimeOptions::default());
    rt.limits.max_string_bytes = 64;

    let yaml = r#"
version: "0.1"
name: t
records:
  r:
    fields:
      x: 'json_parse(source.raw)'
"#;
    let m = rt.compile_mapping(yaml).unwrap();
    let big = "y".repeat(80);
    let raw = json!({ "k": big }).to_string();
    assert!(raw.len() > 64, "fixture string len {}", raw.len());
    let out = rt.evaluate(
        &m,
        EvaluationInput {
            source: json!({ "raw": raw }),
            context: json!({}),
        },
    );
    assert!(
        !out.errors.is_empty(),
        "expected json_parse to hit string budget: {:?}",
        out.errors
    );
}

#[test]
fn budget_rejects_list_transform_over_limit() {
    let mut rt = MappingRuntime::new(RuntimeOptions::default());
    rt.limits.max_list_len = 4;

    // `list_compact` enforces max_list_len on list **input** to O(n) transforms; `list_length` does not.
    let yaml = r#"
version: "0.1"
name: t
records:
  r:
    fields:
      x: "list_compact(source.items)"
"#;
    let m = rt.compile_mapping(yaml).unwrap();
    let out = rt.evaluate(
        &m,
        EvaluationInput {
            source: json!({ "items": [1, 2, 3, 4, 5, 6] }),
            context: json!({}),
        },
    );
    assert!(
        !out.errors.is_empty(),
        "expected list_compact budget error: {:?}",
        out.errors
    );
}
