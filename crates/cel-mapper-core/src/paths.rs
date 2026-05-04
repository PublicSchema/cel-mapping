//! Extract `source.foo.bar`-style paths from CEL source for Missing injection.
//! Path discovery ignores string literals (same rules as [`crate::expr::rewrite_namespaced_calls`]).

use crate::cel_scan::for_each_code_segment;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Number, Value as JsonValue};

static ATTR_PATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(concat!(
        r"\b(source|root|ctx|vars|member|item|profile|target)\.",
        r"([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)"
    ))
    .expect("regex")
});

/// Paths whose root is one of `roots` (e.g. `item` / `member` for foreach rows).
pub fn filter_paths_by_roots(
    paths: &[(String, Vec<String>)],
    roots: &[&str],
) -> Vec<(String, Vec<String>)> {
    let hs: std::collections::HashSet<&str> = roots.iter().copied().collect();
    paths
        .iter()
        .filter(|(r, _)| hs.contains(r.as_str()))
        .cloned()
        .collect()
}

/// Wrap `source` / `root` / `ctx` / `vars` so [`augment_json_with_paths`] can inject Missing sentinels
/// under the same top-level keys CEL exposes (`source`, `root`, …).
pub fn build_binding_envelope(
    source: JsonValue,
    ctx: JsonValue,
    paths: &[(String, Vec<String>)],
    sentinel: &str,
) -> (JsonValue, JsonValue, JsonValue) {
    let mut m = Map::new();
    let root = source.clone();
    m.insert("source".into(), source);
    m.insert("root".into(), root);
    m.insert("ctx".into(), ctx);
    m.insert("vars".into(), JsonValue::Object(Map::new()));
    let mut env = JsonValue::Object(m);
    augment_json_with_paths(&mut env, paths, sentinel);
    let o = env.as_object().expect("binding envelope is object");
    (
        o.get("source").cloned().unwrap_or(JsonValue::Null),
        o.get("root").cloned().unwrap_or(JsonValue::Null),
        o.get("ctx").cloned().unwrap_or(JsonValue::Null),
    )
}

/// Collect dotted paths (root + segments) referenced after `source.`, `root.`, etc.
pub fn collect_dotted_paths(expressions: &[String]) -> Vec<(String, Vec<String>)> {
    collect_dotted_paths_with_roots(expressions, &[])
}

/// Collect dotted paths for the standard roots plus record-specific roots such as a custom
/// `foreach.as` binding.
pub fn collect_dotted_paths_with_roots(
    expressions: &[String],
    extra_roots: &[&str],
) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for expr in expressions {
        for_each_code_segment(expr, |code| {
            for cap in ATTR_PATH.captures_iter(code) {
                let root = cap.get(1).unwrap().as_str().to_string();
                let tail = cap.get(2).unwrap().as_str();
                let segs: Vec<String> = tail.split('.').map(|s| s.to_string()).collect();
                out.push((root, segs));
            }
            for root in extra_roots {
                collect_paths_for_root(code, root, &mut out);
            }
        });
    }
    out.sort();
    out.dedup();
    out
}

fn collect_paths_for_root(code: &str, root: &str, out: &mut Vec<(String, Vec<String>)>) {
    if root.is_empty() || matches!(root, "source" | "root" | "ctx" | "vars" | "member" | "item") {
        return;
    }
    let Ok(re) = Regex::new(&format!(
        r"\b{}\.(?P<tail>[a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)",
        regex::escape(root)
    )) else {
        return;
    };
    for cap in re.captures_iter(code) {
        let Some(tail) = cap.name("tail").map(|m| m.as_str()) else {
            continue;
        };
        out.push((
            root.to_string(),
            tail.split('.').map(|s| s.to_string()).collect(),
        ));
    }
}

/// A single dotted binding from `expr` for diagnostics (prefers `source.*`, else first path).
pub fn primary_binding_hint(expr: &str) -> Option<String> {
    let paths = collect_dotted_paths(&[expr.to_string()]);
    let pick = paths
        .iter()
        .find(|(root, segs)| root == "source" && !segs.is_empty())
        .or_else(|| paths.iter().find(|(_, segs)| !segs.is_empty()))
        .or_else(|| paths.first());
    let (root, segs) = pick?;
    if segs.is_empty() {
        Some(root.clone())
    } else {
        Some(format!("{root}.{}", segs.join(".")))
    }
}

/// Ensure each path exists under the given JSON object root key (`source` / `root` / …).
pub fn augment_json_with_paths(
    data: &mut JsonValue,
    paths: &[(String, Vec<String>)],
    sentinel: &str,
) {
    let obj = match data {
        JsonValue::Object(m) => m,
        _ => return,
    };
    for (root, segs) in paths {
        if segs.is_empty() {
            continue;
        }
        let Some(root_val) = obj.get_mut(root) else {
            continue;
        };
        ensure_path(root_val, segs, sentinel);
    }
}

fn ensure_path(cur: &mut JsonValue, segs: &[String], sentinel: &str) {
    if segs.is_empty() {
        return;
    }
    let head = &segs[0];
    let rest = &segs[1..];
    if rest.is_empty() {
        if let JsonValue::Object(m) = cur {
            if !m.contains_key(head) {
                m.insert(head.clone(), JsonValue::String(sentinel.to_string()));
            }
        }
        return;
    }
    if let JsonValue::Object(m) = cur {
        let entry = m
            .entry(head.clone())
            .or_insert_with(|| JsonValue::Object(Map::new()));
        if entry.is_null() {
            *entry = JsonValue::Object(Map::new());
        }
        if let JsonValue::Object(_) = entry {
            ensure_path(entry, rest, sentinel);
        }
    }
}

/// Clone the loop element under each row-scoped root (`item`, `member`, `as` name), inject Missing, then return the augmented object for `as_name` (fallback: `item`, then first root).
pub fn augment_loop_element(
    item: &JsonValue,
    row_paths: &[(String, Vec<String>)],
    as_name: &str,
    sentinel: &str,
) -> JsonValue {
    if row_paths.is_empty() {
        return item.clone();
    }
    let base = match item {
        JsonValue::Object(_) => item.clone(),
        JsonValue::Null => JsonValue::Object(Map::new()),
        _ => return item.clone(),
    };
    let mut roots: Vec<String> = row_paths.iter().map(|(r, _)| r.clone()).collect();
    roots.sort();
    roots.dedup();
    if roots.is_empty() {
        return item.clone();
    }
    let mut w = Map::new();
    for r in &roots {
        w.insert(r.clone(), base.clone());
    }
    let mut wrap = JsonValue::Object(w);
    augment_json_with_paths(&mut wrap, row_paths, sentinel);
    let primary = if roots.iter().any(|r| r == as_name) {
        as_name.to_string()
    } else if roots.iter().any(|r| r == "item") {
        "item".into()
    } else {
        roots[0].clone()
    };
    wrap.get(&primary).cloned().unwrap_or(base)
}

/// Serialize numbers preserving int vs float where possible (spec §3.2).
pub fn json_number_f64(n: f64) -> JsonValue {
    if n.is_finite() && n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        JsonValue::Number(Number::from(n as i64))
    } else {
        match Number::from_f64(n) {
            Some(num) => JsonValue::Number(num),
            None => JsonValue::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_binding_hint_prefers_source() {
        let e = r#"present(source.a) && ctx.today == "x""#;
        assert_eq!(primary_binding_hint(e).as_deref(), Some("source.a"));
    }

    #[test]
    fn collect_dotted_paths_ignores_paths_inside_string_literals() {
        let exprs = vec![r#"present(source.keep) && "source.ghost.path".size() > 0"#.to_string()];
        let p = collect_dotted_paths(&exprs);
        assert!(
            p.iter()
                .any(|(r, s)| r == "source" && s == &vec!["keep".to_string()]),
            "expected source.keep, got {p:?}"
        );
        assert!(
            !p.iter()
                .any(|(r, s)| r == "source" && s == &vec!["ghost".to_string(), "path".to_string()]),
            "must not pick up source.ghost from inside a string literal: {p:?}"
        );
    }
}
