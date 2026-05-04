//! International phone handling (spec §7.12) via [`phonenumber`] (libphonenumber-compatible).

use super::helpers::value_as_str;
use crate::eval_ctx::eval_ctx_get;
use crate::missing::is_missing;
use cel::Value;
use phonenumber::{country::Id, Mode};
use std::str::FromStr;

fn region_hint(country: &Value) -> Option<Id> {
    if matches!(country, Value::Null) || is_missing(country) {
        eval_ctx_get(&["country"])
            .and_then(|j| j.as_str().map(|s| s.trim().to_string()))
            .and_then(|s| Id::from_str(s.trim()).ok())
    } else {
        value_as_str(country)
            .ok()
            .and_then(|s| Id::from_str(s.trim()).ok())
    }
}

pub fn normalize_e164(input: &str, country: &Value) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("empty phone number".into());
    }
    let region = region_hint(country);
    let pn = phonenumber::parse(region, trimmed).map_err(|e| e.to_string())?;
    if !pn.is_valid() {
        return Err("invalid phone number".into());
    }
    Ok(pn.format().mode(Mode::E164).to_string())
}

pub fn is_valid(input: &str, country: &Value) -> bool {
    let region = region_hint(country);
    phonenumber::parse(region, input.trim())
        .map(|pn| pn.is_valid())
        .unwrap_or(false)
}

/// ITU-T country calling code digits only (e.g. `"1"`, `"44"`), or error if not parseable.
pub fn country_calling_code(input: &str, country: &Value) -> Result<String, String> {
    let region = region_hint(country);
    let pn = phonenumber::parse(region, input.trim()).map_err(|e| e.to_string())?;
    Ok(pn.code().value().to_string())
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
