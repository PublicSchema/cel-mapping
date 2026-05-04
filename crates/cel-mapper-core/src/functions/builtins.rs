//! Host functions (spec §7) registered under `namespace_function` names (see `expr::rewrite_namespaced_calls`).

use super::helpers::{
    blank_value, coalesce_impl, default_impl, missing_value_bool, null_if_blank_impl, null_if_impl,
    present_value, require_present, value_as_str,
};
use crate::budget;
use crate::code_system::CodeSystemRegistry;
use crate::eval_ctx::{eval_ctx_get, push_warning};
use crate::missing::{is_missing, missing_value};
use crate::output::{cel_to_json, deep_merge_json, merge_json_objects};
use cel::extractors::Arguments;
use cel::{Context, ExecutionError, Value};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use regex::Regex;
use serde_json::{Map, Value as JsonValue};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;

fn err_fn(name: &str, m: impl ToString) -> ExecutionError {
    ExecutionError::function_error(name, m)
}

fn str_from(v: &Value) -> Result<String, ExecutionError> {
    value_as_str(v).map_err(|e| err_fn("str", e))
}

fn icuish_to_chrono(pat: &str) -> String {
    pat.replace("XXX", "%:z")
        .replace("yyyy", "%Y")
        .replace("MM", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
}

pub fn register_stdlib(ctx: &mut Context, codes: Arc<CodeSystemRegistry>) {
    // --- 7.1 presence ---
    ctx.add_function("present", |v: Value| present_value(&v));
    ctx.add_function("missing", |v: Value| missing_value_bool(&v));
    ctx.add_function("blank", |v: Value| blank_value(&v));
    ctx.add_function(
        "coalesce",
        |Arguments(args): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(args.len())?;
            Ok(coalesce_impl(&args))
        },
    );
    ctx.add_function(
        "default",
        |a: Value, b: Value| -> Result<Value, ExecutionError> { Ok(default_impl(&a, &b)) },
    );
    ctx.add_function(
        "require",
        |v: Value, msg: Value| -> Result<Value, ExecutionError> {
            let m = if matches!(msg, Value::Null) || is_missing(&msg) {
                None
            } else {
                let s = str_from(&msg)?;
                (!s.is_empty()).then_some(s)
            };
            require_present(&v, m).map_err(|e| err_fn("require", e))
        },
    );
    ctx.add_function(
        "null_if",
        |a: Value, b: Value| -> Result<Value, ExecutionError> { Ok(null_if_impl(&a, &b)) },
    );
    ctx.add_function(
        "null_if_blank",
        |v: Value| -> Result<Value, ExecutionError> { Ok(null_if_blank_impl(&v)) },
    );

    // --- 7.2 type ---
    ctx.add_function("type_string", |v: Value| -> Result<Value, ExecutionError> {
        Ok(Value::String(Arc::new(str_from(&v)?)))
    });
    ctx.add_function("type_int", |v: Value| -> Result<Value, ExecutionError> {
        if is_missing(&v) || matches!(v, Value::Null) {
            return Err(err_fn("type_int", "cannot parse null/missing as int"));
        }
        match v {
            Value::Int(i) => Ok(Value::Int(i)),
            Value::UInt(u) => i64::try_from(u)
                .map(Value::Int)
                .map_err(|_| err_fn("type_int", "integer overflow")),
            Value::Float(f) => {
                const I64_MAX_PLUS_ONE_AS_F64: f64 = 9_223_372_036_854_775_808.0;
                if !f.is_finite() {
                    return Err(err_fn("type_int", "non-finite float"));
                }
                if f.trunc() != f {
                    return Err(err_fn("type_int", "float must be integral"));
                }
                if f < i64::MIN as f64 || f >= I64_MAX_PLUS_ONE_AS_F64 {
                    return Err(err_fn("type_int", "integer overflow"));
                }
                Ok(Value::Int(f as i64))
            }
            Value::String(s) => s
                .trim()
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|e| err_fn("type_int", e)),
            Value::Bool(b) => Ok(Value::Int(if b { 1 } else { 0 })),
            _ => Err(err_fn("type_int", "unsupported type")),
        }
    });
    ctx.add_function("type_float", |v: Value| -> Result<Value, ExecutionError> {
        if is_missing(&v) || matches!(v, Value::Null) {
            return Err(err_fn("type_float", "cannot parse null/missing"));
        }
        match v {
            Value::Float(f) => Ok(Value::Float(f)),
            Value::Int(i) => Ok(Value::Float(i as f64)),
            Value::UInt(u) => Ok(Value::Float(u as f64)),
            Value::String(s) => s
                .trim()
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|e| err_fn("type_float", e)),
            _ => Err(err_fn("type_float", "unsupported type")),
        }
    });
    ctx.add_function("type_bool", |v: Value| -> Result<Value, ExecutionError> {
        if is_missing(&v) || matches!(v, Value::Null) {
            return Err(err_fn("type_bool", "cannot parse null/missing"));
        }
        match v {
            Value::Bool(b) => Ok(Value::Bool(b)),
            Value::Int(i) => Ok(Value::Bool(i != 0)),
            Value::UInt(u) => Ok(Value::Bool(u != 0)),
            Value::String(s) => {
                let t = s.trim().to_ascii_lowercase();
                Ok(Value::Bool(match t.as_str() {
                    "true" | "yes" | "y" | "1" => true,
                    "false" | "no" | "n" | "0" => false,
                    _ => return Err(err_fn("type_bool", "invalid bool string")),
                }))
            }
            _ => Err(err_fn("type_bool", "unsupported type")),
        }
    });
    ctx.add_function("type_list", |v: Value| -> Result<Value, ExecutionError> {
        if is_missing(&v) || matches!(v, Value::Null) {
            return Ok(Value::List(Arc::new(vec![])));
        }
        Ok(match v {
            Value::List(l) => Value::List(l.clone()),
            other => Value::List(Arc::new(vec![other.clone()])),
        })
    });
    ctx.add_function("type_map", |v: Value| -> Result<Value, ExecutionError> {
        match v {
            Value::Map(m) => Ok(Value::Map(m.clone())),
            _ => Err(err_fn("type_map", "not a map")),
        }
    });
    ctx.add_function("type_is_string", |v: Value| matches!(v, Value::String(_)));
    ctx.add_function("type_is_number", |v: Value| {
        matches!(v, Value::Int(_) | Value::UInt(_) | Value::Float(_))
    });
    ctx.add_function("type_is_bool", |v: Value| matches!(v, Value::Bool(_)));
    ctx.add_function("type_is_list", |v: Value| matches!(v, Value::List(_)));
    ctx.add_function("type_is_map", |v: Value| matches!(v, Value::Map(_)));

    // --- 7.3 text ---
    ctx.add_function("text_trim", |v: Value| -> Result<Value, ExecutionError> {
        Ok(Value::String(Arc::new(str_from(&v)?.trim().to_string())))
    });
    ctx.add_function("text_lower", |v: Value| -> Result<Value, ExecutionError> {
        Ok(Value::String(Arc::new(str_from(&v)?.to_ascii_lowercase())))
    });
    ctx.add_function("text_upper", |v: Value| -> Result<Value, ExecutionError> {
        Ok(Value::String(Arc::new(str_from(&v)?.to_ascii_uppercase())))
    });
    ctx.add_function("text_title", |v: Value| -> Result<Value, ExecutionError> {
        let s = str_from(&v)?;
        let t: String = s
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        Ok(Value::String(Arc::new(t)))
    });
    ctx.add_function(
        "text_normalize_space",
        |v: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            Ok(Value::String(Arc::new(
                s.split_whitespace().collect::<Vec<_>>().join(" "),
            )))
        },
    );
    ctx.add_function(
        "text_remove_accents",
        |v: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let d: String = s
                .nfd()
                .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
                .collect();
            Ok(Value::String(Arc::new(d)))
        },
    );
    ctx.add_function("text_slug", |v: Value| -> Result<Value, ExecutionError> {
        let s = str_from(&v)?.to_ascii_lowercase();
        let slug: String = s
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        Ok(Value::String(Arc::new(slug)))
    });
    ctx.add_function(
        "text_replace",
        |v: Value, from: Value, to: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let a = str_from(&from)?;
            let b = str_from(&to)?;
            let out = s.replace(&a, &b);
            budget::enforce_max_string_bytes(out.len())?;
            Ok(Value::String(Arc::new(out)))
        },
    );
    ctx.add_function(
        "text_regex_replace",
        |v: Value, pat: Value, rep: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let p = str_from(&pat)?;
            let r = str_from(&rep)?;
            let re = Regex::new(&p).map_err(|e| err_fn("text_regex_replace", e))?;
            let out = re.replace_all(&s, r.as_str()).to_string();
            budget::enforce_max_string_bytes(out.len())?;
            Ok(Value::String(Arc::new(out)))
        },
    );
    ctx.add_function(
        "text_matches",
        |v: Value, pat: Value| -> Result<bool, ExecutionError> {
            let s = str_from(&v)?;
            let p = str_from(&pat)?;
            let re = Regex::new(&p).map_err(|e| err_fn("text_matches", e))?;
            Ok(re.is_match(&s))
        },
    );
    ctx.add_function(
        "text_split",
        |v: Value, sep: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let d = str_from(&sep)?;
            let parts: Vec<Value> = s
                .split(&d)
                .map(|x| Value::String(Arc::new(x.to_string())))
                .collect();
            budget::enforce_max_list_len(parts.len())?;
            Ok(Value::List(Arc::new(parts)))
        },
    );
    ctx.add_function(
        "text_join",
        |parts: Value, sep: Value| -> Result<Value, ExecutionError> {
            let list = match &parts {
                Value::List(l) => l.as_ref(),
                _ => return Err(err_fn("text_join", "expected list")),
            };
            budget::enforce_max_list_len(list.len())?;
            let d = str_from(&sep)?;
            let mut out = String::new();
            for (i, x) in list.iter().enumerate() {
                if i > 0 {
                    out.push_str(&d);
                }
                out.push_str(&str_from(x)?);
            }
            budget::enforce_max_string_bytes(out.len())?;
            Ok(Value::String(Arc::new(out)))
        },
    );
    ctx.add_function(
        "text_left",
        |v: Value, n: i64| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let n = usize::try_from(n.max(0)).unwrap_or(0);
            Ok(Value::String(Arc::new(s.chars().take(n).collect())))
        },
    );
    ctx.add_function(
        "text_right",
        |v: Value, n: i64| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let n = usize::try_from(n.max(0)).unwrap_or(0);
            let len = s.chars().count();
            let skip = len.saturating_sub(n);
            Ok(Value::String(Arc::new(s.chars().skip(skip).collect())))
        },
    );
    ctx.add_function(
        "text_substr",
        |v: Value, start: i64, len: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let st = usize::try_from(start.max(0)).unwrap_or(0);
            let chs: Vec<char> = s.chars().collect();
            let slice = if matches!(len, Value::Null) || is_missing(&len) {
                chs.get(st..).unwrap_or(&[])
            } else {
                let l = match len {
                    Value::Int(i) => i,
                    Value::UInt(u) => u as i64,
                    Value::Float(f) => f as i64,
                    _ => str_from(&len)?
                        .parse()
                        .map_err(|e| err_fn("text_substr", e))?,
                };
                let le = usize::try_from(l.max(0)).unwrap_or(0);
                chs.get(st..(st + le).min(chs.len())).unwrap_or(&[])
            };
            Ok(Value::String(Arc::new(slice.iter().collect())))
        },
    );
    ctx.add_function("text_length", |v: Value| -> Result<i64, ExecutionError> {
        Ok(str_from(&v)?.chars().count() as i64)
    });
    ctx.add_function(
        "text_contains",
        |v: Value, n: Value| -> Result<bool, ExecutionError> {
            Ok(str_from(&v)?.contains(&str_from(&n)?))
        },
    );
    ctx.add_function(
        "text_starts_with",
        |v: Value, p: Value| -> Result<bool, ExecutionError> {
            Ok(str_from(&v)?.starts_with(&str_from(&p)?))
        },
    );
    ctx.add_function(
        "text_ends_with",
        |v: Value, p: Value| -> Result<bool, ExecutionError> {
            Ok(str_from(&v)?.ends_with(&str_from(&p)?))
        },
    );
    ctx.add_function(
        "text_regex_extract",
        |v: Value, pat: Value, group: i64| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let p = str_from(&pat)?;
            let re = Regex::new(&p).map_err(|e| err_fn("text_regex_extract", e))?;
            let g = usize::try_from(group.max(0)).unwrap_or(0);
            match re.captures(&s) {
                None => Ok(Value::String(Arc::new(String::new()))),
                Some(caps) => {
                    let extracted = caps
                        .get(g)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                    Ok(Value::String(Arc::new(extracted)))
                }
            }
        },
    );

    // --- 7.4 name ---
    ctx.add_function(
        "name_full",
        |given: Value, family: Value| -> Result<Value, ExecutionError> {
            let g = str_from(&given)?.trim().to_string();
            let f = str_from(&family)?.trim().to_string();
            let full = format!("{g} {f}").trim().to_string();
            Ok(Value::String(Arc::new(full)))
        },
    );
    ctx.add_function("name_parts", |v: Value| -> Result<Value, ExecutionError> {
        use cel::objects::{Key, Map};
        use std::collections::HashMap;
        let s = str_from(&v)?;
        let parts: Vec<&str> = s.split_whitespace().collect();
        let mut hm: HashMap<Key, Value> = HashMap::from([
            (Key::String(Arc::new("given".into())), Value::Null),
            (Key::String(Arc::new("middle".into())), Value::Null),
            (Key::String(Arc::new("family".into())), Value::Null),
        ]);
        match parts.len() {
            0 => {}
            1 => {
                hm.insert(
                    Key::String(Arc::new("given".into())),
                    Value::String(Arc::new(parts[0].to_string())),
                );
            }
            2 => {
                hm.insert(
                    Key::String(Arc::new("given".into())),
                    Value::String(Arc::new(parts[0].to_string())),
                );
                hm.insert(
                    Key::String(Arc::new("family".into())),
                    Value::String(Arc::new(parts[1].to_string())),
                );
            }
            _ => {
                hm.insert(
                    Key::String(Arc::new("given".into())),
                    Value::String(Arc::new(parts[0].to_string())),
                );
                hm.insert(
                    Key::String(Arc::new("family".into())),
                    Value::String(Arc::new(parts[parts.len() - 1].to_string())),
                );
                let mid = parts[1..parts.len() - 1].join(" ");
                if !mid.is_empty() {
                    hm.insert(
                        Key::String(Arc::new("middle".into())),
                        Value::String(Arc::new(mid)),
                    );
                }
            }
        }
        Ok(Value::Map(Map { map: Arc::new(hm) }))
    });
    ctx.add_function(
        "name_initials",
        |v: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let ini: String = s
                .split_whitespace()
                .filter_map(|w| w.chars().next())
                .collect();
            Ok(Value::String(Arc::new(ini)))
        },
    );

    // --- 7.5 date (chrono; ICU subset via pattern translation) ---
    ctx.add_function(
        "date_parse",
        |v: Value, fmt: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let pat = if matches!(fmt, Value::Null) || is_missing(&fmt) {
                "yyyy-MM-dd".into()
            } else {
                str_from(&fmt)?
            };
            let ch = icuish_to_chrono(&pat);
            let nd = NaiveDate::parse_from_str(&s, &ch).map_err(|e| err_fn("date_parse", e))?;
            Ok(Value::String(Arc::new(nd.format("%Y-%m-%d").to_string())))
        },
    );
    ctx.add_function(
        "date_parse_datetime",
        |Arguments(args): Arguments| -> Result<Value, ExecutionError> {
            let (v, fmt) = match args.len() {
                1 => (args[0].clone(), Value::Null),
                2 => (args[0].clone(), args[1].clone()),
                n => {
                    return Err(err_fn(
                        "date_parse_datetime",
                        format!("expected 1 or 2 arguments, got {n}"),
                    ))
                }
            };
            let s = str_from(&v)?;
            if !matches!(fmt, Value::Null) && !is_missing(&fmt) {
                let ch = icuish_to_chrono(&str_from(&fmt)?);
                if let Ok(dt) = chrono::DateTime::parse_from_str(&s, &ch) {
                    return Ok(Value::String(Arc::new(dt.to_rfc3339())));
                }
                let tzs = eval_ctx_get(&["timezone"])
                    .and_then(|x| x.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| {
                        err_fn(
                            "date_parse_datetime",
                            "ctx.timezone required for offset-less datetime",
                        )
                    })?;
                let tz = Tz::from_str(&tzs)
                    .map_err(|_| err_fn("date_parse_datetime", "invalid ctx.timezone"))?;
                let ndt = NaiveDateTime::parse_from_str(&s, &ch)
                    .map_err(|e| err_fn("date_parse_datetime", e))?;
                let dt = tz
                    .from_local_datetime(&ndt)
                    .single()
                    .ok_or_else(|| err_fn("date_parse_datetime", "ambiguous local"))?;
                return Ok(Value::String(Arc::new(dt.to_rfc3339())));
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                return Ok(Value::String(Arc::new(dt.to_rfc3339())));
            }
            let tzs = eval_ctx_get(&["timezone"])
                .and_then(|x| x.as_str().map(|s| s.to_string()))
                .ok_or_else(|| {
                    err_fn(
                        "date_parse_datetime",
                        "ctx.timezone required for offset-less datetime",
                    )
                })?;
            let tz = Tz::from_str(&tzs)
                .map_err(|_| err_fn("date_parse_datetime", "invalid ctx.timezone"))?;
            let ndt = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S"))
                .map_err(|e| err_fn("date_parse_datetime", e))?;
            let dt = tz
                .from_local_datetime(&ndt)
                .single()
                .ok_or_else(|| err_fn("date_parse_datetime", "ambiguous"))?;
            Ok(Value::String(Arc::new(dt.to_rfc3339())))
        },
    );
    ctx.add_function(
        "date_format",
        |v: Value, fmt: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let pat = str_from(&fmt)?;
            let ch = icuish_to_chrono(&pat);
            if let Ok(nd) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                return Ok(Value::String(Arc::new(nd.format(&ch).to_string())));
            }
            let dt =
                chrono::DateTime::parse_from_rfc3339(&s).map_err(|e| err_fn("date_format", e))?;
            Ok(Value::String(Arc::new(dt.format(&ch).to_string())))
        },
    );
    ctx.add_function("date_today", || -> Result<Value, ExecutionError> {
        let t = eval_ctx_get(&["today"])
            .and_then(|x| x.as_str().map(|s| s.to_string()))
            .ok_or_else(|| err_fn("date_today", "ctx.today not set"))?;
        Ok(Value::String(Arc::new(t)))
    });
    ctx.add_function(
        "date_age_on",
        |birth: Value, refd: Value| -> Result<i64, ExecutionError> {
            let b = NaiveDate::parse_from_str(&str_from(&birth)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_age_on", e))?;
            let r = NaiveDate::parse_from_str(&str_from(&refd)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_age_on", e))?;
            let age = r.year()
                - b.year()
                - if r.month() < b.month() || (r.month() == b.month() && r.day() < b.day()) {
                    1
                } else {
                    0
                };
            Ok(age as i64)
        },
    );
    ctx.add_function(
        "date_years_between",
        |a: Value, b: Value| -> Result<i64, ExecutionError> { years_between_dates(&a, &b) },
    );
    ctx.add_function(
        "date_days_between",
        |a: Value, b: Value| -> Result<i64, ExecutionError> {
            let da = NaiveDate::parse_from_str(&str_from(&a)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_days_between", e))?;
            let db = NaiveDate::parse_from_str(&str_from(&b)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_days_between", e))?;
            Ok(db.signed_duration_since(da).num_days())
        },
    );
    ctx.add_function(
        "date_add_days",
        |d: Value, days: i64| -> Result<Value, ExecutionError> {
            let nd = NaiveDate::parse_from_str(&str_from(&d)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_add_days", e))?;
            let out = nd
                .checked_add_signed(Duration::days(days))
                .ok_or_else(|| err_fn("date_add_days", "overflow"))?;
            Ok(Value::String(Arc::new(out.format("%Y-%m-%d").to_string())))
        },
    );
    ctx.add_function(
        "date_add_months",
        |d: Value, months: i64| -> Result<Value, ExecutionError> {
            let nd = NaiveDate::parse_from_str(&str_from(&d)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_add_months", e))?;
            let out =
                add_months_safe(nd, months).ok_or_else(|| err_fn("date_add_months", "overflow"))?;
            Ok(Value::String(Arc::new(out.format("%Y-%m-%d").to_string())))
        },
    );
    ctx.add_function(
        "date_start_of_month",
        |d: Value| -> Result<Value, ExecutionError> {
            let nd = NaiveDate::parse_from_str(&str_from(&d)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_start_of_month", e))?;
            let out = nd.with_day(1).unwrap();
            Ok(Value::String(Arc::new(out.format("%Y-%m-%d").to_string())))
        },
    );
    ctx.add_function(
        "date_end_of_month",
        |d: Value| -> Result<Value, ExecutionError> {
            let nd = NaiveDate::parse_from_str(&str_from(&d)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_end_of_month", e))?;
            let (ny, nm) = if nd.month() == 12 {
                (nd.year() + 1, 1)
            } else {
                (nd.year(), nd.month() + 1)
            };
            let first_next = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
            let last = first_next.pred_opt().unwrap();
            Ok(Value::String(Arc::new(last.format("%Y-%m-%d").to_string())))
        },
    );
    ctx.add_function(
        "date_is_valid",
        |v: Value, fmt: Value| -> Result<bool, ExecutionError> {
            let s = str_from(&v)?;
            if !matches!(fmt, Value::Null) && !is_missing(&fmt) {
                let ch = icuish_to_chrono(&str_from(&fmt)?);
                return Ok(NaiveDate::parse_from_str(&s, &ch).is_ok());
            }
            Ok(NaiveDate::parse_from_str(&s, "%Y-%m-%d").is_ok())
        },
    );
    ctx.add_function(
        "date_is_before",
        |a: Value, b: Value| -> Result<bool, ExecutionError> {
            let da = NaiveDate::parse_from_str(&str_from(&a)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_is_before", e))?;
            let db = NaiveDate::parse_from_str(&str_from(&b)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_is_before", e))?;
            Ok(da < db)
        },
    );
    ctx.add_function(
        "date_is_after",
        |a: Value, b: Value| -> Result<bool, ExecutionError> {
            let da = NaiveDate::parse_from_str(&str_from(&a)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_is_after", e))?;
            let db = NaiveDate::parse_from_str(&str_from(&b)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_is_after", e))?;
            Ok(da > db)
        },
    );
    ctx.add_function(
        "date_min",
        |a: Value, b: Value| -> Result<Value, ExecutionError> {
            let da = NaiveDate::parse_from_str(&str_from(&a)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_min", e))?;
            let db = NaiveDate::parse_from_str(&str_from(&b)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_min", e))?;
            Ok(Value::String(Arc::new(if da <= db {
                da.format("%Y-%m-%d").to_string()
            } else {
                db.format("%Y-%m-%d").to_string()
            })))
        },
    );
    ctx.add_function(
        "date_max",
        |a: Value, b: Value| -> Result<Value, ExecutionError> {
            let da = NaiveDate::parse_from_str(&str_from(&a)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_max", e))?;
            let db = NaiveDate::parse_from_str(&str_from(&b)?, "%Y-%m-%d")
                .map_err(|e| err_fn("date_max", e))?;
            Ok(Value::String(Arc::new(if da >= db {
                da.format("%Y-%m-%d").to_string()
            } else {
                db.format("%Y-%m-%d").to_string()
            })))
        },
    );

    // --- 7.6 num ---
    ctx.add_function(
        "num_round",
        |v: Value, digits: Value| -> Result<Value, ExecutionError> {
            let x = num_f64(&v)?;
            let d = if matches!(digits, Value::Null) || is_missing(&digits) {
                0i64
            } else {
                match digits {
                    Value::Int(i) => i,
                    Value::UInt(u) => u as i64,
                    Value::Float(f) => f as i64,
                    _ => str_from(&digits)?
                        .parse()
                        .map_err(|e| err_fn("num_round", e))?,
                }
            }
            .clamp(0, 18) as i32;
            let m = 10f64.powi(d);
            Ok(Value::Float((x * m).round() / m))
        },
    );
    ctx.add_function("num_floor", |v: Value| -> Result<i64, ExecutionError> {
        Ok(num_f64(&v)?.floor() as i64)
    });
    ctx.add_function("num_ceil", |v: Value| -> Result<i64, ExecutionError> {
        Ok(num_f64(&v)?.ceil() as i64)
    });
    ctx.add_function("num_abs", |v: Value| -> Result<Value, ExecutionError> {
        Ok(Value::Float(num_f64(&v)?.abs()))
    });
    ctx.add_function(
        "num_min",
        |Arguments(args): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(args.len())?;
            let mut m = f64::INFINITY;
            for a in args.iter() {
                m = m.min(num_f64(a)?);
            }
            Ok(Value::Float(m))
        },
    );
    ctx.add_function(
        "num_max",
        |Arguments(args): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(args.len())?;
            let mut m = f64::NEG_INFINITY;
            for a in args.iter() {
                m = m.max(num_f64(a)?);
            }
            Ok(Value::Float(m))
        },
    );
    ctx.add_function(
        "num_clamp",
        |v: Value, lo: Value, hi: Value| -> Result<Value, ExecutionError> {
            let x = num_f64(&v)?;
            let a = num_f64(&lo)?;
            let b = num_f64(&hi)?;
            Ok(Value::Float(x.clamp(a, b)))
        },
    );
    ctx.add_function("num_is_valid", |v: Value| -> bool { num_f64(&v).is_ok() });
    ctx.add_function(
        "num_parse",
        |v: Value, _locale: Value| -> Result<Value, ExecutionError> {
            num_f64(&v).map(Value::Float)
        },
    );
    ctx.add_function(
        "num_percent",
        |part: Value, total: Value| -> Result<Value, ExecutionError> {
            let p = num_f64(&part)?;
            let t = num_f64(&total)?;
            if t == 0.0 {
                return Err(err_fn("num_percent", "total is zero"));
            }
            Ok(Value::Float(100.0 * p / t))
        },
    );
    ctx.add_function(
        "num_safe_divide",
        |num: Value, den: Value, fb: Value| -> Result<Value, ExecutionError> {
            let n = num_f64(&num).unwrap_or(0.0);
            let d = num_f64(&den).unwrap_or(0.0);
            if d == 0.0 {
                return Ok(if matches!(fb, Value::Null) || is_missing(&fb) {
                    Value::Float(0.0)
                } else {
                    fb
                });
            }
            Ok(Value::Float(n / d))
        },
    );

    // --- 7.7 list ---
    ctx.add_function(
        "list_compact",
        |v: Value| -> Result<Value, ExecutionError> {
            let l = list_ref(&v)?;
            budget::enforce_max_list_len(l.len())?;
            let out: Vec<Value> = l.iter().filter(|x| present_value(x)).cloned().collect();
            budget::enforce_max_list_len(out.len())?;
            Ok(Value::List(Arc::new(out)))
        },
    );
    ctx.add_function(
        "list_flatten",
        |v: Value| -> Result<Value, ExecutionError> {
            let l = list_ref(&v)?;
            budget::enforce_max_list_len(l.len())?;
            let mut out = Vec::new();
            flatten_rec(l, &mut out);
            budget::enforce_max_list_len(out.len())?;
            Ok(Value::List(Arc::new(out)))
        },
    );
    ctx.add_function("list_unique", |v: Value| -> Result<Value, ExecutionError> {
        let l = list_ref(&v)?;
        budget::enforce_max_list_len(l.len())?;
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for x in l.iter().cloned() {
            let key = format!("{:?}", x);
            if seen.insert(key) {
                out.push(x);
            }
        }
        budget::enforce_max_list_len(out.len())?;
        Ok(Value::List(Arc::new(out)))
    });
    ctx.add_function("list_sort", |v: Value| -> Result<Value, ExecutionError> {
        let l = list_ref(&v)?;
        budget::enforce_max_list_len(l.len())?;
        let mut l = l.to_vec();
        l.sort_by(|a, b| partial_cmp_vals(a, b).unwrap_or(Ordering::Equal));
        Ok(Value::List(Arc::new(l)))
    });
    ctx.add_function("list_first", |v: Value| -> Result<Value, ExecutionError> {
        let l = list_ref(&v)?;
        Ok(l.first().cloned().unwrap_or(Value::Null))
    });
    ctx.add_function("list_last", |v: Value| -> Result<Value, ExecutionError> {
        let l = list_ref(&v)?;
        Ok(l.last().cloned().unwrap_or(Value::Null))
    });
    ctx.add_function(
        "list_at",
        |v: Value, idx: i64, fb: Value| -> Result<Value, ExecutionError> {
            let l = list_ref(&v)?;
            let i = usize::try_from(idx).ok();
            Ok(i.and_then(|i| l.get(i).cloned()).unwrap_or_else(|| {
                if matches!(fb, Value::Null) || is_missing(&fb) {
                    Value::Null
                } else {
                    fb
                }
            }))
        },
    );
    ctx.add_function("list_length", |v: Value| -> Result<i64, ExecutionError> {
        Ok(list_ref(&v)?.len() as i64)
    });
    ctx.add_function(
        "list_contains",
        |v: Value, x: Value| -> Result<bool, ExecutionError> { Ok(list_ref(&v)?.contains(&x)) },
    );
    ctx.add_function(
        "list_join",
        |v: Value, sep: Value| -> Result<Value, ExecutionError> {
            let l = list_ref(&v)?;
            budget::enforce_max_list_len(l.len())?;
            let d = str_from(&sep)?;
            let mut out = String::new();
            for (i, x) in l.iter().enumerate() {
                if i > 0 {
                    out.push_str(&d);
                }
                out.push_str(&str_from(x)?);
            }
            budget::enforce_max_string_bytes(out.len())?;
            Ok(Value::String(Arc::new(out)))
        },
    );
    ctx.add_function(
        "list_filter_present",
        |v: Value| -> Result<Value, ExecutionError> {
            let l = list_ref(&v)?;
            budget::enforce_max_list_len(l.len())?;
            let out: Vec<Value> = l.iter().filter(|x| present_value(x)).cloned().collect();
            budget::enforce_max_list_len(out.len())?;
            Ok(Value::List(Arc::new(out)))
        },
    );
    ctx.add_function(
        "list_of",
        |Arguments(args): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(args.len())?;
            Ok(Value::List(Arc::new(args.to_vec())))
        },
    );
    ctx.add_function(
        "coalesce_list",
        |Arguments(args): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(args.len())?;
            for a in args.iter() {
                if let Value::List(l) = a {
                    if !l.is_empty() {
                        return Ok(a.clone());
                    }
                }
            }
            Ok(Value::List(Arc::new(vec![])))
        },
    );
    ctx.add_function(
        "list_to_map",
        |v: Value, key_field: Value| -> Result<Value, ExecutionError> {
            let l = list_ref(&v)?;
            budget::enforce_max_list_len(l.len())?;
            let l = l.to_vec();
            let kf = str_from(&key_field)?;
            let mut hm = std::collections::HashMap::new();
            for item in l {
                let key = map_lookup_value(&item, &kf)
                    .map(|x| str_from(&x))
                    .transpose()?
                    .unwrap_or_default();
                hm.insert(cel::objects::Key::String(Arc::new(key)), item);
            }
            Ok(Value::Map(cel::objects::Map { map: Arc::new(hm) }))
        },
    );

    // --- 7.8 map ---
    ctx.add_function(
        "map_get",
        |obj: Value, path: Value, fb: Value| -> Result<Value, ExecutionError> {
            let p = str_from(&path)?;
            Ok(match map_lookup_value(&obj, &p) {
                Some(v) => v,
                None => {
                    if matches!(fb, Value::Null) || is_missing(&fb) {
                        missing_value()
                    } else {
                        fb
                    }
                }
            })
        },
    );
    ctx.add_function(
        "map_has",
        |obj: Value, path: Value| -> Result<bool, ExecutionError> {
            let p = str_from(&path)?;
            Ok(map_path_exists(&obj, &p))
        },
    );
    ctx.add_function(
        "map_pick",
        |obj: Value, keys: Value| -> Result<Value, ExecutionError> {
            let m = as_map(&obj)?;
            let ks = list_ref(&keys)?;
            budget::enforce_max_list_len(ks.len())?;
            let mut out = std::collections::HashMap::new();
            for k in ks.iter() {
                let key = str_from(k)?;
                if let Some(v) = m.map.get(&cel::objects::Key::String(Arc::new(key.clone()))) {
                    out.insert(cel::objects::Key::String(Arc::new(key)), v.clone());
                }
            }
            Ok(Value::Map(cel::objects::Map { map: Arc::new(out) }))
        },
    );
    ctx.add_function(
        "map_omit",
        |obj: Value, keys: Value| -> Result<Value, ExecutionError> {
            let m = as_map(&obj)?;
            let kref = list_ref(&keys)?;
            budget::enforce_max_list_len(kref.len())?;
            let ks: std::collections::HashSet<String> =
                kref.iter().map(str_from).collect::<Result<_, _>>()?;
            let mut out = (*m.map).clone();
            out.retain(|k, _| {
                let s = k.to_string();
                !ks.contains(&s)
            });
            Ok(Value::Map(cel::objects::Map { map: Arc::new(out) }))
        },
    );
    ctx.add_function(
        "map_merge",
        |a: Value, b: Value| -> Result<Value, ExecutionError> {
            let ja = cel_to_json(&a).map_err(|e| err_fn("map_merge", e))?;
            let jb = cel_to_json(&b).map_err(|e| err_fn("map_merge", e))?;
            let merged = merge_json_objects(ja, jb);
            cel::to_value(merged).map_err(|e| err_fn("map_merge", e))
        },
    );
    ctx.add_function(
        "map_deep_merge",
        |a: Value, b: Value| -> Result<Value, ExecutionError> {
            let ja = cel_to_json(&a).map_err(|e| err_fn("map_deep_merge", e))?;
            let jb = cel_to_json(&b).map_err(|e| err_fn("map_deep_merge", e))?;
            let merged = deep_merge_json(ja, jb);
            cel::to_value(merged).map_err(|e| err_fn("map_deep_merge", e))
        },
    );
    ctx.add_function(
        "map_set",
        |obj: Value, path: Value, val: Value| -> Result<Value, ExecutionError> {
            let mut j = cel_to_json(&obj).map_err(|e| err_fn("map_set", e))?;
            let p = str_from(&path)?;
            let mut vj = cel_to_json(&val).map_err(|e| err_fn("map_set", e))?;
            set_json_path(&mut j, &p, &mut vj)?;
            cel::to_value(j).map_err(|e| err_fn("map_set", e))
        },
    );
    ctx.add_function("map_keys", |obj: Value| -> Result<Value, ExecutionError> {
        let m = as_map(&obj)?;
        let keys: Vec<Value> = m
            .map
            .keys()
            .map(|k| Value::String(Arc::new(k.to_string())))
            .collect();
        budget::enforce_max_list_len(keys.len())?;
        Ok(Value::List(Arc::new(keys)))
    });
    ctx.add_function(
        "map_values",
        |obj: Value| -> Result<Value, ExecutionError> {
            let m = as_map(&obj)?;
            let vals: Vec<Value> = m.map.values().cloned().collect();
            budget::enforce_max_list_len(vals.len())?;
            Ok(Value::List(Arc::new(vals)))
        },
    );
    ctx.add_function(
        "map_entries",
        |obj: Value| -> Result<Value, ExecutionError> {
            let m = as_map(&obj)?;
            let mut entries = Vec::new();
            for (k, v) in m.map.iter() {
                let mut e = std::collections::HashMap::new();
                e.insert(
                    cel::objects::Key::String(Arc::new("key".into())),
                    Value::String(Arc::new(k.to_string())),
                );
                e.insert(
                    cel::objects::Key::String(Arc::new("value".into())),
                    v.clone(),
                );
                entries.push(Value::Map(cel::objects::Map { map: Arc::new(e) }));
            }
            budget::enforce_max_list_len(entries.len())?;
            Ok(Value::List(Arc::new(entries)))
        },
    );

    ctx.add_function(
        "map_of",
        |Arguments(args): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(args.len())?;
            if args.len() % 2 != 0 {
                return Err(err_fn(
                    "map_of",
                    format!(
                        "expected an even number of arguments, got {}",
                        args.len()
                    ),
                ));
            }
            let mut out = std::collections::HashMap::new();
            for i in (0..args.len()).step_by(2) {
                let key = match &args[i] {
                    Value::String(s) => s.as_ref().clone(),
                    _ => {
                        return Err(err_fn(
                            "map_of",
                            format!("key at position {i} must be a string"),
                        ))
                    }
                };
                out.insert(
                    cel::objects::Key::String(Arc::new(key)),
                    args[i + 1].clone(),
                );
            }
            Ok(Value::Map(cel::objects::Map { map: Arc::new(out) }))
        },
    );

    // --- 7.9 code ---
    let creg = Arc::clone(&codes);
    ctx.add_function(
        "code_map",
        move |sys: Value, val: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&sys)?;
            let v = str_from(&val)?;
            let e = creg
                .map(&s, &v)
                .ok_or_else(|| err_fn("code_map", "unknown code"))?;
            Ok(Value::String(Arc::new(e.id.clone())))
        },
    );
    let creg2 = Arc::clone(&codes);
    ctx.add_function(
        "code_map_or_null",
        move |sys: Value, val: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&sys)?;
            let v = str_from(&val)?;
            Ok(match creg2.map(&s, &v) {
                Some(e) => Value::String(Arc::new(e.id.clone())),
                None => Value::Null,
            })
        },
    );
    let creg3 = Arc::clone(&codes);
    ctx.add_function(
        "code_map_or_default",
        move |sys: Value, val: Value, fb: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&sys)?;
            let v = str_from(&val)?;
            Ok(match creg3.map(&s, &v) {
                Some(e) => Value::String(Arc::new(e.id.clone())),
                None => fb,
            })
        },
    );
    let creg4 = Arc::clone(&codes);
    ctx.add_function(
        "code_label",
        move |sys: Value, code: Value, loc: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&sys)?;
            let c = str_from(&code)?;
            let locale = if matches!(loc, Value::Null) || is_missing(&loc) {
                "en".to_string()
            } else {
                str_from(&loc)?
            };
            let e = creg4
                .entry_by_canonical(&s, &c)
                .or_else(|| creg4.map(&s, &c))
                .ok_or_else(|| err_fn("code_label", "unknown code"))?;
            let lbl = e
                .label
                .get(&locale)
                .or_else(|| e.label.values().next())
                .cloned()
                .unwrap_or_else(|| e.id.clone());
            Ok(Value::String(Arc::new(lbl)))
        },
    );
    let creg5 = Arc::clone(&codes);
    ctx.add_function(
        "code_exists",
        move |sys: Value, val: Value| -> Result<bool, ExecutionError> {
            let s = str_from(&sys)?;
            let v = str_from(&val)?;
            Ok(creg5.exists(&s, &v))
        },
    );
    let creg6 = Arc::clone(&codes);
    ctx.add_function(
        "code_canonical",
        move |sys: Value, val: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&sys)?;
            let v = str_from(&val)?;
            let e = creg6
                .map(&s, &v)
                .ok_or_else(|| err_fn("code_canonical", "unknown code"))?;
            let mut m = std::collections::HashMap::new();
            m.insert(
                cel::objects::Key::String(Arc::new("id".into())),
                Value::String(Arc::new(e.id.clone())),
            );
            let lbl = e
                .label
                .get("en")
                .or_else(|| e.label.values().next())
                .cloned()
                .unwrap_or_default();
            m.insert(
                cel::objects::Key::String(Arc::new("label".into())),
                Value::String(Arc::new(lbl)),
            );
            m.insert(
                cel::objects::Key::String(Arc::new("system".into())),
                Value::String(Arc::new(s)),
            );
            for (k, vj) in &e.extra {
                if let Ok(cv) = cel::to_value(vj.clone()) {
                    m.insert(cel::objects::Key::String(Arc::new(k.clone())), cv);
                }
            }
            Ok(Value::Map(cel::objects::Map { map: Arc::new(m) }))
        },
    );
    let creg7 = Arc::clone(&codes);
    ctx.add_function(
        "code_reverse_map",
        move |sys: Value, canon: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&sys)?;
            let c = str_from(&canon)?;
            Ok(match creg7.reverse_map(&s, &c) {
                Some(x) => Value::String(Arc::new(x)),
                None => Value::Null,
            })
        },
    );
    ctx.add_function(
        "code_normalize",
        |v: Value| -> Result<Value, ExecutionError> {
            Ok(Value::String(Arc::new(CodeSystemRegistry::normalize_code(
                &str_from(&v)?,
            ))))
        },
    );

    // --- 7.10 id ---
    ctx.add_function(
        "id_make",
        |prefix: Value, Arguments(parts): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(parts.len().saturating_add(32))?;
            let pfx = str_from(&prefix)?;
            let mut arr = vec![JsonValue::String(pfx.clone())];
            for x in parts.iter() {
                arr.push(cel_to_json(x).map_err(|e| err_fn("id_make", e))?);
            }
            let sorted = sort_json(JsonValue::Array(arr));
            let payload = serde_json::to_string(&sorted).map_err(|e| err_fn("id_make", e))?;
            let mut h = Sha256::new();
            h.update(payload.as_bytes());
            let hex = hex::encode(h.finalize());
            Ok(Value::String(Arc::new(format!("{pfx}_{}", &hex[..32]))))
        },
    );
    ctx.add_function(
        "id_uuid_v5",
        |ns: Value, val: Value| -> Result<Value, ExecutionError> {
            let n = str_from(&ns)?;
            let v = str_from(&val)?;
            let u = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, format!("{n}:{v}").as_bytes());
            Ok(Value::String(Arc::new(u.to_string())))
        },
    );
    ctx.add_function(
        "id_hash",
        |v: Value, algo: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let a = if matches!(algo, Value::Null) || is_missing(&algo) {
                "sha256".to_string()
            } else {
                str_from(&algo)?
            };
            if a != "sha256" {
                return Err(err_fn("id_hash", "only sha256 supported"));
            }
            let mut h = Sha256::new();
            h.update(s.as_bytes());
            Ok(Value::String(Arc::new(hex::encode(h.finalize()))))
        },
    );
    ctx.add_function(
        "id_slug",
        |pfx: Value, val: Value| -> Result<Value, ExecutionError> {
            let p = str_from(&pfx)?;
            let s = str_from(&val)?.to_ascii_lowercase();
            let slug: String = s
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
                .join("-");
            Ok(Value::String(Arc::new(format!("{p}_{slug}"))))
        },
    );
    ctx.add_function("id_clean", |v: Value| -> Result<Value, ExecutionError> {
        let s = str_from(&v)?;
        let out: String = s
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        Ok(Value::String(Arc::new(out)))
    });
    ctx.add_function(
        "id_is_valid",
        |v: Value, pat: Value| -> Result<bool, ExecutionError> {
            let s = str_from(&v)?;
            if !matches!(pat, Value::Null) && !is_missing(&pat) {
                let re = Regex::new(&str_from(&pat)?).map_err(|e| err_fn("id_is_valid", e))?;
                return Ok(re.is_match(&s));
            }
            Ok(!s.is_empty())
        },
    );

    // --- 7.11 person ---
    ctx.add_function("person_age", |a: Value, b: Value| date_age_on_impl(a, b));
    ctx.add_function(
        "person_is_minor",
        |bd: Value, rd: Value, th: Value| -> Result<bool, ExecutionError> {
            let age = date_age_on_impl(bd, rd)?;
            let t = if matches!(th, Value::Null) || is_missing(&th) {
                18i64
            } else {
                num_f64(&th)? as i64
            };
            Ok(age < t)
        },
    );
    let creg8 = Arc::clone(&codes);
    ctx.add_function(
        "person_sex_or_gender",
        move |v: Value, sys: Value| -> Result<Value, ExecutionError> {
            let s = if matches!(sys, Value::Null) || is_missing(&sys) {
                return Err(err_fn("person_sex_or_gender", "system required"));
            } else {
                str_from(&sys)?
            };
            let val = str_from(&v)?;
            let e = creg8
                .map(&s, &val)
                .ok_or_else(|| err_fn("person_sex_or_gender", "unmapped"))?;
            Ok(Value::String(Arc::new(e.id.clone())))
        },
    );
    ctx.add_function("person_normalize_phone", |v: Value, country: Value| {
        phone_normalize_impl(v, country)
    });

    // --- 7.12 phone ---
    ctx.add_function("phone_normalize", |v: Value, country: Value| {
        phone_normalize_impl(v, country)
    });
    ctx.add_function("phone_is_valid", |v: Value, country: Value| -> bool {
        let Ok(s) = str_from(&v) else {
            return false;
        };
        crate::functions::phone::is_valid(&s, &country)
    });
    ctx.add_function(
        "phone_country_code",
        |v: Value, country: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            match crate::functions::phone::country_calling_code(&s, &country) {
                Ok(cc) => Ok(Value::String(Arc::new(cc))),
                Err(m) => Err(err_fn("phone_country_code", m)),
            }
        },
    );
    ctx.add_function("phone_mask", |v: Value| -> Result<Value, ExecutionError> {
        let s = str_from(&v)?;
        if s.len() <= 4 {
            return Ok(Value::String(Arc::new("*".repeat(s.len()))));
        }
        let vis = 4;
        let masked = "*".repeat(s.len().saturating_sub(vis)) + &s[s.len() - vis..];
        Ok(Value::String(Arc::new(masked)))
    });

    // --- 7.13 email ---
    ctx.add_function(
        "email_normalize",
        |v: Value| -> Result<Value, ExecutionError> {
            Ok(Value::String(Arc::new(
                str_from(&v)?.trim().to_ascii_lowercase(),
            )))
        },
    );
    ctx.add_function("email_is_valid", |v: Value| -> bool {
        let Ok(s) = str_from(&v) else {
            return false;
        };
        s.contains('@') && !s.starts_with('@') && !s.ends_with('@')
    });
    ctx.add_function(
        "email_domain",
        |v: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let d = s.split('@').nth(1).unwrap_or("").to_string();
            Ok(Value::String(Arc::new(d)))
        },
    );

    // --- 7.14 geo ---
    ctx.add_function(
        "geo_point",
        |lat: Value, lon: Value| -> Result<Value, ExecutionError> {
            let la = num_f64(&lat)?;
            let lo = num_f64(&lon)?;
            let mut m = std::collections::HashMap::new();
            m.insert(
                cel::objects::Key::String(Arc::new("type".into())),
                Value::String(Arc::new("Point".into())),
            );
            m.insert(
                cel::objects::Key::String(Arc::new("coordinates".into())),
                Value::List(Arc::new(vec![Value::Float(lo), Value::Float(la)])),
            );
            Ok(Value::Map(cel::objects::Map { map: Arc::new(m) }))
        },
    );
    ctx.add_function("geo_is_valid_lat", |v: Value| -> bool {
        num_f64(&v)
            .map(|la| (-90.0..=90.0).contains(&la))
            .unwrap_or(false)
    });
    ctx.add_function("geo_is_valid_lon", |v: Value| -> bool {
        num_f64(&v)
            .map(|lo| (-180.0..=180.0).contains(&lo))
            .unwrap_or(false)
    });
    ctx.add_function(
        "geo_normalize_lat",
        |v: Value| -> Result<Value, ExecutionError> {
            num_f64(&v).map(|x| Value::Float(x.clamp(-90.0, 90.0)))
        },
    );
    ctx.add_function(
        "geo_normalize_lon",
        |v: Value| -> Result<Value, ExecutionError> {
            num_f64(&v).map(|x| Value::Float(x.clamp(-180.0, 180.0)))
        },
    );
    let creg9 = Arc::clone(&codes);
    ctx.add_function(
        "geo_admin_code",
        move |v: Value, sys: Value| -> Result<Value, ExecutionError> {
            let s = if matches!(sys, Value::Null) || is_missing(&sys) {
                return Err(err_fn("geo_admin_code", "system required"));
            } else {
                str_from(&sys)?
            };
            let val = str_from(&v)?;
            let e = creg9
                .map(&s, &val)
                .ok_or_else(|| err_fn("geo_admin_code", "unmapped"))?;
            Ok(Value::String(Arc::new(e.id.clone())))
        },
    );

    // --- 7.15 address ---
    ctx.add_function(
        "address_line",
        |Arguments(parts): Arguments| -> Result<Value, ExecutionError> {
            budget::enforce_max_list_len(parts.len())?;
            let mut out = Vec::new();
            for p in parts.iter() {
                let s = str_from(p)?.trim().to_string();
                if !s.is_empty() {
                    out.push(s);
                }
            }
            let joined = out.join(", ");
            budget::enforce_max_string_bytes(joined.len())?;
            Ok(Value::String(Arc::new(joined)))
        },
    );
    ctx.add_function(
        "address_normalize_country",
        |v: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?.trim().to_ascii_uppercase();
            if s.len() == 2 {
                Ok(Value::String(Arc::new(s)))
            } else {
                Ok(Value::String(Arc::new(CodeSystemRegistry::normalize_code(
                    &s,
                ))))
            }
        },
    );
    ctx.add_function(
        "address_postal_code",
        |v: Value| -> Result<Value, ExecutionError> {
            Ok(Value::String(Arc::new(str_from(&v)?.trim().to_string())))
        },
    );

    // --- 7.16 validate ---
    ctx.add_function(
        "validate_required",
        |v: Value, msg: Value| -> Result<Value, ExecutionError> {
            let m = if matches!(msg, Value::Null) || is_missing(&msg) {
                None
            } else {
                let s = str_from(&msg)?;
                (!s.is_empty()).then_some(s)
            };
            require_present(&v, m).map_err(|e| err_fn("validate_required", e))
        },
    );
    ctx.add_function(
        "validate_error",
        |msg: Value, code: Value| -> Result<Value, ExecutionError> {
            let c = if matches!(code, Value::Null) || is_missing(&code) {
                String::new()
            } else {
                str_from(&code)?
            };
            Err(err_fn(
                "validate_error",
                format!("{} [{}]", str_from(&msg)?, c),
            ))
        },
    );
    ctx.add_function(
        "validate_warn",
        |cond: bool, msg: Value, code: Value| -> Result<bool, ExecutionError> {
            if !cond {
                let c = if matches!(code, Value::Null) || is_missing(&code) {
                    String::new()
                } else {
                    str_from(&code)?
                };
                push_warning(format!("{} [{}]", str_from(&msg)?, c));
            }
            Ok(true)
        },
    );
    ctx.add_function(
        "validate_matches",
        |v: Value, pat: Value, msg: Value| -> Result<bool, ExecutionError> {
            let s = str_from(&v)?;
            let re = Regex::new(&str_from(&pat)?).map_err(|e| err_fn("validate_matches", e))?;
            if !re.is_match(&s) {
                let m = if matches!(msg, Value::Null) || is_missing(&msg) {
                    "pattern mismatch".into()
                } else {
                    str_from(&msg)?
                };
                return Err(err_fn("validate_matches", m));
            }
            Ok(true)
        },
    );
    ctx.add_function(
        "validate_in",
        |v: Value, allowed: Value, msg: Value| -> Result<bool, ExecutionError> {
            let list = list_ref(&allowed)?;
            budget::enforce_max_list_len(list.len())?;
            if !list.iter().any(|x| x == &v) {
                let m = if matches!(msg, Value::Null) || is_missing(&msg) {
                    "not in allowed set".into()
                } else {
                    str_from(&msg)?
                };
                return Err(err_fn("validate_in", m));
            }
            Ok(true)
        },
    );
    ctx.add_function(
        "validate_range",
        |v: Value, lo: Value, hi: Value, msg: Value| -> Result<bool, ExecutionError> {
            let x = num_f64(&v)?;
            let a = num_f64(&lo)?;
            let b = num_f64(&hi)?;
            if !(x >= a && x <= b) {
                let m = if matches!(msg, Value::Null) || is_missing(&msg) {
                    "out of range".into()
                } else {
                    str_from(&msg)?
                };
                return Err(err_fn("validate_range", m));
            }
            Ok(true)
        },
    );

    // --- 7.17 json ---
    ctx.add_function("json_parse", |v: Value| -> Result<Value, ExecutionError> {
        let s = str_from(&v)?;
        budget::enforce_max_string_bytes(s.len())?;
        let j: JsonValue = serde_json::from_str(&s).map_err(|e| err_fn("json_parse", e))?;
        cel::to_value(j).map_err(|e| err_fn("json_parse", e))
    });
    ctx.add_function(
        "json_stringify",
        |v: Value| -> Result<Value, ExecutionError> {
            let j = cel_to_json(&v).map_err(|e| err_fn("json_stringify", e))?;
            let s = serde_json::to_string(&j).map_err(|e| err_fn("json_stringify", e))?;
            budget::enforce_max_string_bytes(s.len())?;
            Ok(Value::String(Arc::new(s)))
        },
    );
    ctx.add_function(
        "json_path",
        |v: Value, path: Value, fb: Value| -> Result<Value, ExecutionError> {
            let p = str_from(&path)?;
            let tail = p.strip_prefix("$.").unwrap_or(&p);
            Ok(map_lookup_value(&v, tail).unwrap_or_else(|| {
                if matches!(fb, Value::Null) || is_missing(&fb) {
                    Value::Null
                } else {
                    fb.clone()
                }
            }))
        },
    );

    // --- 7.18 fhir ---
    ctx.add_function(
        "fhir_reference",
        |resource_type: Value, resource_id: Value| -> Result<Value, ExecutionError> {
            let t = str_from(&resource_type)?;
            let id = str_from(&resource_id)?;
            Ok(Value::String(Arc::new(format!("{t}/{id}"))))
        },
    );

    // --- 7.19 privacy ---
    ctx.add_function(
        "privacy_mask",
        |v: Value, visible_last: Value| -> Result<Value, ExecutionError> {
            let s = str_from(&v)?;
            let n = if matches!(visible_last, Value::Null) || is_missing(&visible_last) {
                4i64
            } else {
                match visible_last {
                    Value::Int(i) => i,
                    Value::UInt(u) => u as i64,
                    Value::Float(f) => f as i64,
                    _ => str_from(&visible_last)?.parse().unwrap_or(4),
                }
            }
            .max(0) as usize;
            if s.len() <= n {
                return Ok(Value::String(Arc::new("*".repeat(s.len()))));
            }
            let masked = "*".repeat(s.len().saturating_sub(n)) + &s[s.len() - n..];
            Ok(Value::String(Arc::new(masked)))
        },
    );
    ctx.add_function(
        "privacy_sha256",
        |v: Value, salt: Value| -> Result<Value, ExecutionError> {
            let mut h = Sha256::new();
            if !matches!(salt, Value::Null) && !is_missing(&salt) {
                h.update(str_from(&salt)?.as_bytes());
            }
            h.update(str_from(&v)?.as_bytes());
            Ok(Value::String(Arc::new(hex::encode(h.finalize()))))
        },
    );
    ctx.add_function(
        "privacy_redact",
        |_v: Value| -> Result<Value, ExecutionError> {
            Ok(Value::String(Arc::new("[REDACTED]".into())))
        },
    );
}

fn date_age_on_impl(a: Value, b: Value) -> Result<i64, ExecutionError> {
    let birth = NaiveDate::parse_from_str(&str_from(&a)?, "%Y-%m-%d")
        .map_err(|e| err_fn("date_age_on", e))?;
    let r = NaiveDate::parse_from_str(&str_from(&b)?, "%Y-%m-%d")
        .map_err(|e| err_fn("date_age_on", e))?;
    let age = r.year()
        - birth.year()
        - if r.month() < birth.month() || (r.month() == birth.month() && r.day() < birth.day()) {
            1
        } else {
            0
        };
    Ok(age as i64)
}

fn add_months_safe(d: NaiveDate, months: i64) -> Option<NaiveDate> {
    let total = d.month0() as i64 + months;
    let year = d.year() as i64 + total.div_euclid(12);
    let month0 = total.rem_euclid(12) as u32;
    let day = d.day().min(last_day_of_month(year as i32, month0 + 1));
    NaiveDate::from_ymd_opt(year as i32, month0 + 1, day)
}

fn last_day_of_month(y: i32, m: u32) -> u32 {
    (1..=31)
        .rev()
        .filter_map(|d| NaiveDate::from_ymd_opt(y, m, d))
        .next()
        .map(|d| d.day())
        .unwrap_or(28)
}

fn num_f64(v: &Value) -> Result<f64, ExecutionError> {
    if is_missing(v) || matches!(v, Value::Null) {
        return Err(err_fn("num", "null/missing"));
    }
    match v {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        Value::UInt(u) => Ok(*u as f64),
        Value::String(s) => s.trim().parse().map_err(|e| err_fn("num", e)),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        _ => Err(err_fn("num", "unsupported")),
    }
}

/// Borrow a CEL list slice (no list-length budget; use for read-only access).
fn list_ref(v: &Value) -> Result<&[Value], ExecutionError> {
    match v {
        Value::List(l) => Ok(l.as_ref()),
        _ => Err(err_fn("list", "expected list")),
    }
}

fn flatten_rec(items: &[Value], out: &mut Vec<Value>) {
    for x in items {
        match x {
            Value::List(inner) => flatten_rec(inner.as_ref(), out),
            _ => out.push(x.clone()),
        }
    }
}

fn partial_cmp_vals(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.partial_cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::String(x), Value::String(y)) => x.partial_cmp(y),
        _ => None,
    }
}

fn as_map(v: &Value) -> Result<&cel::objects::Map, ExecutionError> {
    match v {
        Value::Map(m) => Ok(m),
        _ => Err(err_fn("map", "expected map")),
    }
}

fn map_path_exists(obj: &Value, path: &str) -> bool {
    let mut cur = obj;
    for seg in path.split('.') {
        match cur {
            Value::Map(m) => {
                let k = cel::objects::Key::String(Arc::new(seg.to_string()));
                match m.map.get(&k) {
                    Some(next) => cur = next,
                    None => return false,
                }
            }
            _ => return false,
        }
    }
    true
}

fn map_lookup_value(obj: &Value, path: &str) -> Option<Value> {
    let mut cur = obj;
    for seg in path.split('.') {
        match cur {
            Value::Map(m) => {
                let k = cel::objects::Key::String(Arc::new(seg.to_string()));
                match m.map.get(&k) {
                    Some(next) => cur = next,
                    None => return None,
                }
            }
            _ => return None,
        }
    }
    Some(cur.clone())
}

fn years_between_dates(a: &Value, b: &Value) -> Result<i64, ExecutionError> {
    let da = NaiveDate::parse_from_str(&str_from(a)?, "%Y-%m-%d")
        .map_err(|e| err_fn("date_years_between", e))?;
    let db = NaiveDate::parse_from_str(&str_from(b)?, "%Y-%m-%d")
        .map_err(|e| err_fn("date_years_between", e))?;
    let (start, end) = if da <= db { (da, db) } else { (db, da) };
    let mut y = end.year() - start.year();
    if (end.month(), end.day()) < (start.month(), start.day()) {
        y -= 1;
    }
    Ok(y as i64)
}

fn set_json_path(
    root: &mut JsonValue,
    path: &str,
    val: &mut JsonValue,
) -> Result<(), ExecutionError> {
    let segs: Vec<&str> = path.split('.').collect();
    if segs.is_empty() {
        return Ok(());
    }
    if !root.is_object() {
        *root = JsonValue::Object(Map::new());
    }
    let last = *segs.last().unwrap();
    let mut cur = root;
    for s in &segs[..segs.len() - 1] {
        if !cur.is_object() {
            *cur = JsonValue::Object(Map::new());
        }
        let o = cur.as_object_mut().unwrap();
        let e = o
            .entry(s.to_string())
            .or_insert_with(|| JsonValue::Object(Map::new()));
        cur = e;
    }
    if let JsonValue::Object(o) = cur {
        o.insert(last.to_string(), val.clone());
    }
    Ok(())
}

fn sort_json(v: JsonValue) -> JsonValue {
    match v {
        JsonValue::Object(m) => {
            let sorted: BTreeMap<_, _> = m.into_iter().map(|(k, v)| (k, sort_json(v))).collect();
            JsonValue::Object(sorted.into_iter().collect())
        }
        JsonValue::Array(a) => JsonValue::Array(a.into_iter().map(sort_json).collect()),
        o => o,
    }
}

fn phone_normalize_impl(v: Value, country: Value) -> Result<Value, ExecutionError> {
    let s = str_from(&v)?;
    match crate::functions::phone::normalize_e164(&s, &country) {
        Ok(out) => Ok(Value::String(Arc::new(out))),
        Err(m) => Err(err_fn("phone_normalize", m)),
    }
}
