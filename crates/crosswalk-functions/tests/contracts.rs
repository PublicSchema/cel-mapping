use crosswalk_functions::{email, ids, text};

#[test]
fn default_feature_helpers_are_available() {
    assert_eq!(
        text::regex_extract("abc123", r"(\d+)", 1)
            .unwrap()
            .as_deref(),
        Some("123")
    );
    assert_eq!(email::normalize_email(" A@B "), "a@b");
    assert_eq!(
        ids::stable_hash_sha256("abc", None),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
