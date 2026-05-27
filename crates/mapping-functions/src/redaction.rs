pub fn mask(input: &str, visible_last: usize) -> String {
    if input.len() <= visible_last {
        return "*".repeat(input.len());
    }
    "*".repeat(input.len().saturating_sub(visible_last)) + &input[input.len() - visible_last..]
}

pub fn redact() -> String {
    "[REDACTED]".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_without_error_leaking_input() {
        assert_eq!(mask("123456789", 4), "*****6789");
        assert_eq!(mask("123", 4), "***");
        assert_eq!(redact(), "[REDACTED]");
    }
}
