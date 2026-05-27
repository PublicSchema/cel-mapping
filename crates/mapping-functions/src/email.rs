pub fn normalize_email(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

pub fn email_domain(input: &str) -> Option<String> {
    input.split('@').nth(1).map(ToString::to_string)
}

pub fn is_valid_email(input: &str) -> bool {
    input.contains('@') && !input.starts_with('@') && !input.ends_with('@')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_helpers_match_current_behavior() {
        assert_eq!(normalize_email(" USER@Example.ORG "), "user@example.org");
        assert_eq!(
            email_domain("user@example.org").as_deref(),
            Some("example.org")
        );
        assert_eq!(email_domain("not-an-email"), None);
        assert!(is_valid_email("a@b"));
        assert!(!is_valid_email("@b"));
    }
}
