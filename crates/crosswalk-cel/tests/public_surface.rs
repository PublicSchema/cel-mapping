fn crate_root_public_use_statements() -> Vec<String> {
    let source = include_str!("../src/lib.rs");
    let mut statements = Vec::new();
    let mut current = None::<String>;

    for line in source.lines() {
        let line = line.split("//").next().unwrap_or("").trim();

        if let Some(statement) = &mut current {
            statement.push(' ');
            statement.push_str(line);
            if line.ends_with(';') {
                statements.push(statement.trim().to_string());
                current = None;
            }
        } else if line.starts_with("pub use ") {
            if line.ends_with(';') {
                statements.push(line.to_string());
            } else {
                current = Some(line.to_string());
            }
        }
    }

    statements
}

#[test]
fn crate_root_does_not_re_export_private_cel_runtime_helpers() {
    let public_uses = crate_root_public_use_statements();

    for forbidden_name in ["CelValue", "run_program", "json_to_cel"] {
        assert!(
            public_uses
                .iter()
                .all(|statement| !statement.contains(forbidden_name)),
            "crosswalk-cel crate root must not publicly re-export `{forbidden_name}`"
        );
    }

    assert!(
        !public_uses
            .iter()
            .any(|statement| statement == "pub use evaluator::*;"),
        "crosswalk-cel crate root must not publicly re-export evaluator helpers wholesale"
    );
}

#[test]
fn crate_root_does_not_re_export_dependency_crates_wholesale() {
    let public_uses = crate_root_public_use_statements();

    for forbidden_statement in [
        "pub use crosswalk_functions;",
        "pub use crosswalk_functions_cel;",
    ] {
        assert!(
            !public_uses
                .iter()
                .any(|statement| statement == forbidden_statement),
            "crosswalk-cel crate root must not publicly re-export `{forbidden_statement}`"
        );
    }
}

#[test]
fn compiled_expression_does_not_expose_engine_program_field() {
    let compiled = include_str!("../src/compiled.rs");

    assert!(
        !compiled.contains("pub program"),
        "CompiledCel must not expose the upstream cel::Program field"
    );
}

#[test]
fn public_modules_do_not_expose_upstream_cel_value_or_program_helpers() {
    for (path, forbidden) in [
        (
            "../src/paths.rs",
            "pub fn collect_missing_aware_injection_paths(",
        ),
        ("../src/ast_paths.rs", "pub fn classify_all_paths"),
        ("../src/output.rs", "pub fn cel_to_json"),
        ("../src/missing.rs", "pub fn missing_value"),
        ("../src/missing.rs", "pub fn is_missing"),
    ] {
        let source = include_str!("../src/lib.rs");
        let module_source = match path {
            "../src/paths.rs" => include_str!("../src/paths.rs"),
            "../src/ast_paths.rs" => include_str!("../src/ast_paths.rs"),
            "../src/output.rs" => include_str!("../src/output.rs"),
            "../src/missing.rs" => include_str!("../src/missing.rs"),
            _ => source,
        };

        assert!(
            !module_source.contains(forbidden),
            "{path} must not expose `{forbidden}`"
        );
    }
}
