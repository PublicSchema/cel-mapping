//! Convert CEL [`Value`] to JSON output, stripping internal Missing sentinel.
//!
//! Integers are limited to the JSON / JavaScript **safe integer** range so WASM/JS consumers
//! do not silently lose precision (spec §3.2 interop).

use crate::missing::MISSING_STR;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use cel::Value;
use serde_json::{Map, Number, Value as JsonValue};

/// `2^53 - 1` — largest integer exactly representable as a JS `Number`.
pub const JSON_SAFE_INT_MAX: i64 = 9007199254740991;
/// `-(2^53 - 1)`.
pub const JSON_SAFE_INT_MIN: i64 = -9007199254740991;

fn json_safe_i64(i: i64) -> Result<JsonValue, String> {
    if !(JSON_SAFE_INT_MIN..=JSON_SAFE_INT_MAX).contains(&i) {
        return Err(format!(
            "integer {i} is outside JSON safe range [{JSON_SAFE_INT_MIN}, {JSON_SAFE_INT_MAX}]"
        ));
    }
    Ok(JsonValue::Number(Number::from(i)))
}

fn json_safe_u64(u: u64) -> Result<JsonValue, String> {
    if u > JSON_SAFE_INT_MAX as u64 {
        return Err(format!(
            "unsigned integer {u} is outside JSON safe range (max {JSON_SAFE_INT_MAX})"
        ));
    }
    Ok(JsonValue::Number(Number::from(u as i64)))
}

pub fn cel_to_json(v: &Value) -> Result<JsonValue, String> {
    match v {
        Value::Null => Ok(JsonValue::Null),
        Value::Bool(b) => Ok(JsonValue::Bool(*b)),
        Value::Int(i) => json_safe_i64(*i),
        Value::UInt(u) => json_safe_u64(*u),
        Value::Float(f) => {
            if !f.is_finite() {
                return Err("non-finite float".into());
            }
            Number::from_f64(*f)
                .map(JsonValue::Number)
                .ok_or_else(|| "float not representable as JSON number".into())
        }
        Value::String(s) => {
            if s.as_str() == MISSING_STR {
                Ok(JsonValue::Null)
            } else {
                Ok(JsonValue::String(s.to_string()))
            }
        }
        Value::Bytes(b) => Ok(JsonValue::String(STANDARD.encode(b.as_slice()))),
        Value::List(l) => Ok(JsonValue::Array(
            l.iter().map(cel_to_json).collect::<Result<Vec<_>, _>>()?,
        )),
        Value::Map(m) => {
            let mut obj = Map::new();
            for (k, val) in m.map.iter() {
                obj.insert(k.to_string(), cel_to_json(val)?);
            }
            Ok(JsonValue::Object(obj))
        }
        Value::Timestamp(_) | Value::Duration(_) => v.json().map_err(|e| e.to_string()),
        Value::Function(_, _) => Err("cannot serialize function".into()),
        Value::Opaque(_) => v.json().map_err(|e| e.to_string()),
    }
}

pub fn omit_null_keys(mut v: JsonValue) -> JsonValue {
    if let JsonValue::Object(m) = &mut v {
        m.retain(|_, val| !val.is_null());
    }
    v
}

pub fn merge_json_objects(a: JsonValue, b: JsonValue) -> JsonValue {
    let mut am = match a {
        JsonValue::Object(m) => m,
        _ => Map::new(),
    };
    let bm = match b {
        JsonValue::Object(m) => m,
        _ => return JsonValue::Object(am),
    };
    for (k, v) in bm {
        am.insert(k, v);
    }
    JsonValue::Object(am)
}

pub fn deep_merge_json(a: JsonValue, b: JsonValue) -> JsonValue {
    use JsonValue::Object;
    match (a, b) {
        (Object(mut am), Object(bm)) => {
            for (k, bv) in bm {
                let entry = am.entry(k.clone()).or_insert(JsonValue::Null);
                *entry = match (&*entry, &bv) {
                    (Object(_), Object(_)) => deep_merge_json(entry.clone(), bv),
                    _ => bv,
                };
            }
            Object(am)
        }
        (_, b) => b,
    }
}
