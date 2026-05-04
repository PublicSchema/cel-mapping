use crate::missing::is_missing;
use cel::Value;

pub fn is_blank_string(s: &str) -> bool {
    s.chars().all(|c| c.is_whitespace())
}

pub fn present_value(v: &Value) -> bool {
    if is_missing(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::String(s) => !s.is_empty() && !is_blank_string(s),
        Value::List(l) => !l.is_empty(),
        Value::Map(m) => !m.map.is_empty(),
        _ => true,
    }
}

pub fn blank_value(v: &Value) -> bool {
    if is_missing(v) || matches!(v, Value::Null) {
        return true;
    }
    if let Value::String(s) = v {
        return s.is_empty() || is_blank_string(s);
    }
    false
}

pub fn missing_value_bool(v: &Value) -> bool {
    is_missing(v)
}

pub fn value_as_str(v: &Value) -> Result<String, String> {
    if is_missing(v) {
        return Ok(String::new());
    }
    match v {
        Value::String(s) => Ok(s.to_string()),
        Value::Int(i) => Ok(i.to_string()),
        Value::UInt(u) => Ok(u.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err("expected scalar".into()),
    }
}

pub fn require_present(v: &Value, msg: Option<String>) -> Result<Value, String> {
    if present_value(v) {
        Ok(v.clone())
    } else {
        Err(msg.unwrap_or_else(|| "required value missing".into()))
    }
}

pub fn null_if_impl(v: &Value, m: &Value) -> Value {
    if v == m {
        Value::Null
    } else {
        v.clone()
    }
}

pub fn null_if_blank_impl(v: &Value) -> Value {
    if blank_value(v) {
        Value::Null
    } else {
        v.clone()
    }
}

pub fn default_impl(v: &Value, fb: &Value) -> Value {
    if blank_value(v) {
        fb.clone()
    } else {
        v.clone()
    }
}

pub fn coalesce_impl(args: &[Value]) -> Value {
    for a in args {
        if present_value(a) {
            return a.clone();
        }
    }
    Value::Null
}
