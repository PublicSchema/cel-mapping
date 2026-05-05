//! Fixture-driven test harness for the PublicSchema v0.2 golden corpus.
//!
//! Reads every `.json` file under `tests/fixtures/publicschema-parity/`, compiles and
//! evaluates each mapping, then asserts the relaxed expected shape:
//!   - `ok` must match
//!   - `output` must equal the expected JSON object (deep equality)
//!   - `errors_count` must match the number of errors in the result
//!   - `warnings_count` must match the number of warnings in the result
//!   - `log_statuses` must match the sequence of status strings in `log`
//!
//! We use a relaxed shape (counts and status sequences rather than full structural
//! equality on errors/warnings) because error message wording may evolve without
//! breaking the behavioral contract.

use cel_mapper_core::{
    MappingRuntime, PrivacyMode, PublicSchemaDirection, PublicSchemaEvaluateOptions,
    PublicSchemaEvaluationInput, RuntimeOptions,
};
use serde_json::Value as JsonValue;
use std::fs;

/// Loaded from each fixture file.
#[derive(Debug)]
struct Fixture {
    name: String,
    mapping_json: String,
    direction: PublicSchemaDirection,
    source_data: JsonValue,
    ctx: JsonValue,
    options: FixtureOptions,
    expected: ExpectedShape,
}

#[derive(Debug)]
struct FixtureOptions {
    privacy: PrivacyMode,
    errors_mode: Option<String>,
}

#[derive(Debug)]
struct ExpectedShape {
    ok: bool,
    output: JsonValue,
    errors_count: usize,
    warnings_count: usize,
    log_statuses: Vec<String>,
}

fn load_fixtures() -> Vec<Fixture> {
    let dir = format!(
        "{}/tests/fixtures/publicschema-parity",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut fixtures = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read fixture dir {dir}: {e}"))
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x == "json")
                .unwrap_or(false)
        })
        .collect();
    // Sort for stable test ordering.
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display()));
        let v: JsonValue = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()));

        let name = v["name"]
            .as_str()
            .unwrap_or_else(|| panic!("missing 'name' in {}", path.display()))
            .to_string();

        // The mapping object is serialized back to JSON so the runtime's
        // JSON-detection path (leading `{`) picks the right parser.
        let mapping_json = serde_json::to_string(&v["mapping"])
            .unwrap_or_else(|e| panic!("cannot serialize mapping in {name}: {e}"));

        let direction = match v["direction"].as_str().unwrap_or("to_target") {
            "from_target" => PublicSchemaDirection::FromTarget,
            _ => PublicSchemaDirection::ToTarget,
        };

        let source_data = v["source_data"].clone();
        let ctx = v["ctx"].clone();

        let privacy = match v["options"]["privacy"].as_str().unwrap_or("production") {
            "authoring" => PrivacyMode::Authoring,
            "debug" => PrivacyMode::Debug,
            _ => PrivacyMode::Production,
        };
        let errors_mode = v["options"]["errors_mode"].as_str().map(|s| s.to_string());

        let expected_ok = v["expected"]["ok"]
            .as_bool()
            .unwrap_or_else(|| panic!("missing expected.ok in {name}"));
        let expected_output = v["expected"]["output"].clone();
        let errors_count = v["expected"]["errors_count"]
            .as_u64()
            .unwrap_or_else(|| panic!("missing expected.errors_count in {name}"))
            as usize;
        let warnings_count = v["expected"]["warnings_count"]
            .as_u64()
            .unwrap_or_else(|| panic!("missing expected.warnings_count in {name}"))
            as usize;
        let log_statuses: Vec<String> = v["expected"]["log_statuses"]
            .as_array()
            .unwrap_or_else(|| panic!("missing expected.log_statuses in {name}"))
            .iter()
            .map(|s| {
                s.as_str()
                    .unwrap_or_else(|| panic!("non-string in log_statuses for {name}"))
                    .to_string()
            })
            .collect();

        fixtures.push(Fixture {
            name,
            mapping_json,
            direction,
            source_data,
            ctx,
            options: FixtureOptions {
                privacy,
                errors_mode,
            },
            expected: ExpectedShape {
                ok: expected_ok,
                output: expected_output,
                errors_count,
                warnings_count,
                log_statuses,
            },
        });
    }
    fixtures
}

#[test]
fn run_all_fixtures() {
    let fixtures = load_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found in tests/fixtures/publicschema-parity/"
    );

    let rt = MappingRuntime::new(RuntimeOptions::default());
    let mut failures: Vec<String> = Vec::new();

    for fixture in &fixtures {
        let compiled =
            match rt.compile_publicschema_mapping(&fixture.mapping_json, Default::default()) {
                Ok(c) => c,
                Err(e) => {
                    failures.push(format!("[{}] compile failed: {e}", fixture.name));
                    continue;
                }
            };

        let result = rt.evaluate_publicschema_mapping(
            &compiled,
            PublicSchemaEvaluationInput {
                source: fixture.source_data.clone(),
                context: fixture.ctx.clone(),
                options: PublicSchemaEvaluateOptions {
                    direction: fixture.direction,
                    errors_mode: fixture.options.errors_mode.clone(),
                    privacy: fixture.options.privacy,
                },
            },
        );

        let mut fixture_failures: Vec<String> = Vec::new();

        if result.ok != fixture.expected.ok {
            fixture_failures.push(format!(
                "ok: got {}, want {}",
                result.ok, fixture.expected.ok
            ));
        }

        if result.output != fixture.expected.output {
            fixture_failures.push(format!(
                "output mismatch:\n  got:  {}\n  want: {}",
                serde_json::to_string_pretty(&result.output).unwrap_or_default(),
                serde_json::to_string_pretty(&fixture.expected.output).unwrap_or_default()
            ));
        }

        if result.errors.len() != fixture.expected.errors_count {
            fixture_failures.push(format!(
                "errors_count: got {}, want {}  (errors: {:?})",
                result.errors.len(),
                fixture.expected.errors_count,
                result.errors
            ));
        }

        if result.warnings.len() != fixture.expected.warnings_count {
            fixture_failures.push(format!(
                "warnings_count: got {}, want {}  (warnings: {:?})",
                result.warnings.len(),
                fixture.expected.warnings_count,
                result.warnings
            ));
        }

        let got_statuses: Vec<String> = result.log.iter().map(|e| e.status.clone()).collect();
        if got_statuses != fixture.expected.log_statuses {
            fixture_failures.push(format!(
                "log_statuses: got {:?}, want {:?}",
                got_statuses, fixture.expected.log_statuses
            ));
        }

        if !fixture_failures.is_empty() {
            failures.push(format!(
                "[{}]\n  {}",
                fixture.name,
                fixture_failures.join("\n  ")
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} fixture(s) failed:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}
