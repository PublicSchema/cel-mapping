//! International phone handling (spec §7.12) via [`phonenumber`] (libphonenumber-compatible).

use super::helpers::value_as_str;
use crate::eval_ctx::eval_ctx_get;
use crate::missing::is_missing;
use cel::Value;

fn region_hint(country: &Value) -> Option<String> {
    if matches!(country, Value::Null) || is_missing(country) {
        eval_ctx_get(&["country"]).and_then(|j| j.as_str().map(|s| s.trim().to_string()))
    } else {
        value_as_str(country).ok().map(|s| s.trim().to_string())
    }
}

pub fn normalize_e164(input: &str, country: &Value) -> Result<String, String> {
    mapping_functions::phone::normalize_phone_e164(input, region_hint(country).as_deref())
        .map_err(|err| err.message)
}

pub fn is_valid(input: &str, country: &Value) -> bool {
    mapping_functions::phone::is_valid_phone(input, region_hint(country).as_deref())
}

/// ITU-T country calling code digits only (e.g. `"1"`, `"44"`), or error if not parseable.
pub fn country_calling_code(input: &str, country: &Value) -> Result<String, String> {
    mapping_functions::phone::country_calling_code(input, region_hint(country).as_deref())
        .map_err(|err| err.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cel::Value;

    #[test]
    fn normalize_us_national() {
        let out = normalize_e164(
            "(650) 253-0000",
            &Value::String(std::sync::Arc::new("US".into())),
        )
        .unwrap();
        assert_eq!(out, "+16502530000");
    }

    #[test]
    fn country_code_from_e164() {
        let cc = country_calling_code("+442071838750", &Value::Null).unwrap();
        assert_eq!(cc, "44");
    }
}
