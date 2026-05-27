//! Rewrite `text.trim` → `text_trim` so identifiers are valid CEL function names (spec §6).
//! Rewriting is **string-literal aware**: patterns inside `"..."` / `'...'` are not touched.

use once_cell::sync::Lazy;
use regex::Regex;

static NS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(concat!(
        r"\b(text|type|date|num|list|map|code|id|name|person|phone|email|geo|address|validate|json|privacy)",
        r"\.([a-zA-Z_][a-zA-Z0-9_]*)"
    ))
    .expect("regex")
});

fn rewrite_segment(code: &str) -> String {
    NS.replace_all(code, "${1}_${2}").to_string()
}

pub fn rewrite_namespaced_calls(expr: &str) -> String {
    let mut out = String::with_capacity(expr.len() + 8);
    let mut rest = expr;
    while !rest.is_empty() {
        let first_quote = rest
            .char_indices()
            .find(|(_, c)| *c == '"' || *c == '\'')
            .map(|(i, _)| i);
        let (code, after_code) = match first_quote {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        out.push_str(&rewrite_segment(code));
        if after_code.is_empty() {
            break;
        }
        if let Some((lit, tail)) = crate::cel_scan::consume_quoted_string(after_code) {
            out.push_str(lit);
            rest = tail;
        } else {
            out.push_str(after_code);
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_skips_double_quoted_literals() {
        let e = r#"map.get(source, "person.name")"#;
        assert_eq!(
            rewrite_namespaced_calls(e),
            r#"map_get(source, "person.name")"#
        );
    }

    #[test]
    fn rewrite_still_applies_in_code() {
        assert_eq!(rewrite_namespaced_calls("text.trim(x)"), "text_trim(x)");
    }
}
