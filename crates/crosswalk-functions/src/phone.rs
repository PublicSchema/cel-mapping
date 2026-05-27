use crate::FunctionError;
use phonenumber::{country::Id, Mode};
use std::str::FromStr;

fn region_hint(country_hint: Option<&str>) -> Option<Id> {
    country_hint.and_then(|country| Id::from_str(country.trim()).ok())
}

pub fn normalize_phone_e164(
    input: &str,
    country_hint: Option<&str>,
) -> Result<String, FunctionError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(FunctionError::new("PHONE_EMPTY", "empty phone number"));
    }
    let parsed = phonenumber::parse(region_hint(country_hint), trimmed)
        .map_err(|err| FunctionError::new("PHONE_PARSE", err.to_string()))?;
    if !parsed.is_valid() {
        return Err(FunctionError::new("PHONE_INVALID", "invalid phone number"));
    }
    Ok(parsed.format().mode(Mode::E164).to_string())
}

pub fn is_valid_phone(input: &str, country_hint: Option<&str>) -> bool {
    phonenumber::parse(region_hint(country_hint), input.trim())
        .map(|parsed| parsed.is_valid())
        .unwrap_or(false)
}

pub fn country_calling_code(
    input: &str,
    country_hint: Option<&str>,
) -> Result<String, FunctionError> {
    let parsed = phonenumber::parse(region_hint(country_hint), input.trim())
        .map_err(|err| FunctionError::new("PHONE_PARSE", err.to_string()))?;
    Ok(parsed.code().value().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_helpers_accept_explicit_country_hint() {
        assert_eq!(
            normalize_phone_e164("(650) 253-0000", Some("US")).unwrap(),
            "+16502530000"
        );
        assert_eq!(country_calling_code("+442071838750", None).unwrap(), "44");
        assert!(is_valid_phone("(650) 253-0000", Some("US")));
    }

    #[test]
    fn phone_errors_have_stable_codes() {
        assert_eq!(
            normalize_phone_e164("", Some("US")).unwrap_err().code,
            "PHONE_EMPTY"
        );
    }
}
