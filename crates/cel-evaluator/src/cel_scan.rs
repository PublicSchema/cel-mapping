//! Skip CEL string literals when scanning expression text (path extraction, etc.).

/// Skip a quoted segment starting at `rest[0]` (must be `'` or `"`).
/// Returns `(literal_including_quotes, rest_after)`.
pub(crate) fn consume_quoted_string(rest: &str) -> Option<(&str, &str)> {
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let qlen = quote.len_utf8();
    let mut escaped = false;
    for (i, c) in rest[qlen..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == quote {
            let out_end = qlen + i + c.len_utf8();
            return Some((&rest[..out_end], &rest[out_end..]));
        }
    }
    Some((rest, ""))
}

/// Invoke `f` on each maximal substring outside of `"..."` / `'...'` literals.
pub(crate) fn for_each_code_segment(expr: &str, mut f: impl FnMut(&str)) {
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
        if !code.is_empty() {
            f(code);
        }
        if after_code.is_empty() {
            break;
        }
        if let Some((_lit, tail)) = consume_quoted_string(after_code) {
            rest = tail;
        } else {
            f(after_code);
            break;
        }
    }
}
