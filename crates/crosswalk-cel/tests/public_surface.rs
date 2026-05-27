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
