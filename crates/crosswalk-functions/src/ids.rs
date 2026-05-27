use sha2::{Digest, Sha256};

pub fn stable_hash_sha256(input: &str, salt: Option<&str>) -> String {
    let mut hash = Sha256::new();
    if let Some(salt) = salt {
        hash.update(salt.as_bytes());
    }
    hash.update(input.as_bytes());
    hex::encode(hash.finalize())
}

pub fn prefixed_slug(prefix: &str, input: &str) -> String {
    format!("{prefix}_{}", crate::text::slug(input))
}

pub fn clean_id(input: &str) -> String {
    input
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_deterministic() {
        assert_eq!(
            stable_hash_sha256("abc", None),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(prefixed_slug("ps", "Hello World!"), "ps_hello-world");
        assert_eq!(clean_id("a b_c-d!"), "ab_c-d");
    }
}
