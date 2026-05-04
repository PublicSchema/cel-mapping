//! AST-based path classifier for Missing-sentinel injection (spec §2.6).
//!
//! The sentinel injection strategy: only inject a Missing placeholder under a
//! dotted path if **every** occurrence of that path in the expression is a
//! direct argument to a missing-aware helper (e.g. `present`, `coalesce`,
//! `default`).  Paths that appear in comparison operators, arithmetic, or any
//! other strict context must NOT receive a sentinel — the CEL engine should
//! raise `NoSuchKey` as per the CEL spec.
//!
//! If a path appears in **both** a missing-aware and a strict context, the
//! strict context wins and no sentinel is injected.

use cel::common::ast::{CallExpr, Expr, IdedExpr, SelectExpr};
use std::collections::HashSet;

/// Helpers that treat a Missing sentinel as "absent" rather than a real value.
/// All other functions and operators are strict (they propagate errors on absent paths).
const MISSING_AWARE_HELPERS: &[&str] = &[
    "present",
    "missing",
    "blank",
    "coalesce",
    "default",
    "require",
    "null_if",
    "null_if_blank",
];

/// Classify the paths referenced in a compiled CEL expression.
///
/// Returns the set of dotted paths (as `"root.seg1.seg2"` strings) that appear
/// **exclusively** as direct arguments to missing-aware helpers.  Paths that
/// appear in any other context are excluded from the returned set.
///
/// The caller should inject a Missing sentinel only for the returned paths.
pub fn missing_aware_only_paths(expr: &IdedExpr) -> HashSet<String> {
    let (missing_aware, strict) = classify_all_paths(expr);
    missing_aware.difference(&strict).cloned().collect()
}

/// Classify all paths referenced in an expression into two sets:
/// - `missing_aware`: paths that appear as direct arguments to missing-aware helpers.
/// - `strict`: paths that appear in any strict context.
///
/// A path can appear in both sets (e.g., `present(source.a) && source.a == "x"`).
/// The caller decides the injection policy (typically: inject only if in `missing_aware`
/// but not in `strict`).
pub fn classify_all_paths(expr: &IdedExpr) -> (HashSet<String>, HashSet<String>) {
    let mut missing_aware: HashSet<String> = HashSet::new();
    let mut strict: HashSet<String> = HashSet::new();
    classify_expr(expr, false, &mut missing_aware, &mut strict);
    (missing_aware, strict)
}

/// Public re-export of `classify_expr` so `paths.rs` can drive per-node
/// classification when building multi-program injection sets.
pub fn classify_expr_pub(
    expr: &IdedExpr,
    in_missing_aware_arg: bool,
    missing_aware: &mut HashSet<String>,
    strict: &mut HashSet<String>,
) {
    classify_expr(expr, in_missing_aware_arg, missing_aware, strict);
}

/// Walk `expr`.
///
/// `in_missing_aware_arg`: we are currently evaluating an expression that is a
/// direct argument of a missing-aware helper — so any field selection here
/// should be recorded as missing-aware.
fn classify_expr(
    expr: &IdedExpr,
    in_missing_aware_arg: bool,
    missing_aware: &mut HashSet<String>,
    strict: &mut HashSet<String>,
) {
    match &expr.expr {
        Expr::Unspecified | Expr::Literal(_) | Expr::Ident(_) => {}

        Expr::Select(sel) => {
            // Collect the full dotted path rooted at an Ident.
            if let Some(path) = select_path(sel) {
                if in_missing_aware_arg {
                    missing_aware.insert(path);
                } else {
                    strict.insert(path);
                }
            }
            // Still recurse into the operand in the current mode to catch
            // any nested paths.
            classify_expr(&sel.operand, in_missing_aware_arg, missing_aware, strict);
        }

        Expr::Call(call) => {
            classify_call(call, in_missing_aware_arg, missing_aware, strict);
        }

        Expr::List(list) => {
            for elem in &list.elements {
                classify_expr(elem, in_missing_aware_arg, missing_aware, strict);
            }
        }

        Expr::Map(map) => {
            for entry in &map.entries {
                match &entry.expr {
                    cel::common::ast::EntryExpr::MapEntry(e) => {
                        classify_expr(&e.key, in_missing_aware_arg, missing_aware, strict);
                        classify_expr(&e.value, in_missing_aware_arg, missing_aware, strict);
                    }
                    cel::common::ast::EntryExpr::StructField(e) => {
                        classify_expr(&e.value, in_missing_aware_arg, missing_aware, strict);
                    }
                }
            }
        }

        Expr::Struct(s) => {
            for entry in &s.entries {
                match &entry.expr {
                    cel::common::ast::EntryExpr::StructField(e) => {
                        classify_expr(&e.value, in_missing_aware_arg, missing_aware, strict);
                    }
                    cel::common::ast::EntryExpr::MapEntry(e) => {
                        classify_expr(&e.key, in_missing_aware_arg, missing_aware, strict);
                        classify_expr(&e.value, in_missing_aware_arg, missing_aware, strict);
                    }
                }
            }
        }

        Expr::Comprehension(c) => {
            // The collection being iterated inherits the enclosing context: an
            // absent `source.items` inside `coalesce(source.items.filter(...))`
            // should still be treated as missing-aware for the iter range.
            classify_expr(&c.iter_range, in_missing_aware_arg, missing_aware, strict);
            // Per-iteration sub-expressions (filter predicates, map transformers,
            // accumulators) are strict per CEL spec: missing fields inside them
            // must raise NoSuchKey regardless of the enclosing helper.
            classify_expr(&c.accu_init, false, missing_aware, strict);
            classify_expr(&c.loop_cond, false, missing_aware, strict);
            classify_expr(&c.loop_step, false, missing_aware, strict);
            classify_expr(&c.result, false, missing_aware, strict);
        }
    }
}

fn classify_call(
    call: &CallExpr,
    in_missing_aware_arg: bool,
    missing_aware: &mut HashSet<String>,
    strict: &mut HashSet<String>,
) {
    let func = call.func_name.as_str();
    let is_missing_aware = MISSING_AWARE_HELPERS.contains(&func);

    // The call target (receiver in method-call form) is evaluated in strict context.
    if let Some(target) = &call.target {
        classify_expr(target, false, missing_aware, strict);
    }

    for arg in &call.args {
        if is_missing_aware {
            // Arguments to missing-aware helpers are evaluated in missing-aware mode.
            classify_expr(arg, true, missing_aware, strict);
        } else {
            // Arguments to all other functions/operators are strict — use the
            // enclosing mode.
            classify_expr(arg, in_missing_aware_arg, missing_aware, strict);
        }
    }
}

/// Collect the fully-dotted path string for a selection chain rooted at an
/// `Ident`, e.g. `source.given_name` → `"source.given_name"`.
/// Returns `None` for complex sub-expressions (index ops, computed keys, etc.)
/// where a simple path cannot be determined.
fn select_path(sel: &SelectExpr) -> Option<String> {
    let field = sel.field.clone();
    let parent = path_of(&sel.operand)?;
    Some(format!("{parent}.{field}"))
}

fn path_of(expr: &IdedExpr) -> Option<String> {
    match &expr.expr {
        Expr::Ident(name) => Some(name.clone()),
        Expr::Select(sel) => select_path(sel),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cel::Program;

    fn paths_for(expr: &str) -> HashSet<String> {
        let rewritten = crate::expr::rewrite_namespaced_calls(expr);
        let prog = Program::compile(&rewritten).expect("compile");
        missing_aware_only_paths(prog.expression())
    }

    #[test]
    fn present_arg_is_missing_aware() {
        let paths = paths_for("present(source.given_name)");
        assert!(paths.contains("source.given_name"), "{paths:?}");
    }

    #[test]
    fn comparison_is_strict() {
        let paths = paths_for("source.given_name != ''");
        assert!(paths.is_empty(), "expected empty, got {paths:?}");
    }

    #[test]
    fn comparison_is_strict_even_with_coalesce() {
        // present(source.a) uses it in missing-aware context;
        // source.a == "x" uses it in strict context → strict wins.
        let paths = paths_for("source.a == 'x' || present(source.a)");
        assert!(!paths.contains("source.a"), "strict wins: {paths:?}");
    }

    #[test]
    fn short_circuit_present_first() {
        // present(source.a) && source.a == "x"
        // present() arg is missing-aware; == is strict → strict wins.
        let paths = paths_for("present(source.a) && source.a == 'x'");
        assert!(!paths.contains("source.a"), "strict wins: {paths:?}");
    }

    #[test]
    fn coalesce_injects_all_args() {
        let paths = paths_for("coalesce(source.a, source.b, 'fallback')");
        assert!(paths.contains("source.a"), "{paths:?}");
        assert!(paths.contains("source.b"), "{paths:?}");
    }

    #[test]
    fn default_first_arg_is_missing_aware() {
        let paths = paths_for("default(source.a, 'fb')");
        assert!(paths.contains("source.a"), "{paths:?}");
    }

    #[test]
    fn null_if_first_arg_is_missing_aware() {
        let paths = paths_for("null_if(source.a, 'x')");
        assert!(paths.contains("source.a"), "{paths:?}");
    }

    #[test]
    fn comprehension_predicate_is_strict_under_missing_aware_parent() {
        // coalesce(...) is missing-aware. Its arg is a filter comprehension whose
        // predicate references source.threshold in a comparison. The predicate
        // is per-iteration strict CEL semantics — source.threshold must NOT be
        // recorded as missing-aware just because the outer call is.
        let paths = paths_for(
            "coalesce(source.items.filter(x, x.value > source.threshold), [])",
        );
        assert!(
            !paths.contains("source.threshold"),
            "filter predicate must be strict regardless of enclosing helper: {paths:?}"
        );
    }

    #[test]
    fn comprehension_iter_range_inherits_parent_mode() {
        // The collection being iterated is a direct sub-expression of the
        // missing-aware helper; an absent source.items should still be safe
        // to treat as empty for iteration.
        let paths = paths_for("coalesce(source.items.filter(x, x > 0), [])");
        assert!(
            paths.contains("source.items"),
            "iter range should inherit missing-aware mode from coalesce: {paths:?}"
        );
    }
}
