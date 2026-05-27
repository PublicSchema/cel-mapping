use crate::FunctionError;
use serde_json::Value;

pub fn parse_json(input: &str) -> Result<Value, FunctionError> {
    serde_json::from_str(input).map_err(|err| FunctionError::new("JSON_PARSE", err.to_string()))
}

pub fn stringify_json(value: &Value) -> Result<String, FunctionError> {
    serde_json::to_string(value)
        .map_err(|err| FunctionError::new("JSON_STRINGIFY", err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_helpers_round_trip_compactly() {
        let parsed = parse_json(r#"{"b":2}"#).unwrap();
        assert_eq!(parsed, json!({"b": 2}));
        assert_eq!(stringify_json(&parsed).unwrap(), r#"{"b":2}"#);
    }
}
