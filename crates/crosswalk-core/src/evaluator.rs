//! Mapping evaluation (CEL programs, record/field loop).
#![allow(clippy::result_large_err, clippy::too_many_arguments)]

use crate::compiled::CompiledCel;
use crate::compiled::{CompiledMapping, CompiledRecord, ErrorMode};
use crate::errors::{ErrorCode, ExpressionPreviewResult, MappingError, StandaloneEvalError};
use crate::functions::register_stdlib;
use crate::mapping::FieldYaml;
use crate::missing::{is_missing, MISSING_STR};
use crate::output::{cel_to_json, omit_null_keys};
use crate::paths::{
    augment_json_with_paths, augment_loop_element, build_binding_envelope,
    collect_dotted_paths_with_roots, collect_missing_aware_injection_paths, filter_paths_by_roots,
};
use cel::{Context, ExecutionError, Value};
use serde_json::{Map, Value as JsonValue};
use std::collections::BTreeMap;
use std::sync::Arc;

pub use crosswalk_cel::StandaloneExpressionInput;

pub fn evaluate_mapping(
    mapping: &CompiledMapping,
    source: JsonValue,
    ctx: JsonValue,
) -> crate::runtime::MappingOutput {
    let mut records: BTreeMap<String, Vec<JsonValue>> = BTreeMap::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mode = mapping.error_mode;
    let codes = Arc::clone(&mapping.code_systems);

    let paths = collect_mapping_paths(mapping);
    let (src_json, root_json, ctx_json) = build_binding_envelope(source, ctx, &paths, MISSING_STR);
    let source_val = json_to_cel(&src_json);
    let root_val = json_to_cel(&root_json);
    let ctx_val = json_to_cel(&ctx_json);

    for v in &mapping.validations {
        match run_program(
            &v.cel,
            &source_val,
            &root_val,
            &ctx_val,
            &empty_vars(),
            None,
            None,
            None,
            &[],
            &codes,
        ) {
            Ok(val) if truthy(&val) => {}
            Ok(_) => {
                let err = MappingError::error(
                    ErrorCode::ValidationError,
                    v.message.clone(),
                    Some(v.path.clone()),
                    Some(v.cel.source.clone()),
                );
                let mut err = err;
                err.source_path = crate::paths::primary_binding_hint(&v.cel.source);
                if matches!(mode, ErrorMode::Lenient) {
                    warnings.push(err.warning());
                } else {
                    errors.push(err);
                }
            }
            Err(e) => {
                let err = exec_validation_err(&v.path, &v.cel.source, &v.message, e);
                if matches!(mode, ErrorMode::Lenient) {
                    warnings.push(err.warning());
                } else {
                    errors.push(err);
                }
            }
        }
    }

    if matches!(mode, ErrorMode::Strict) && !errors.is_empty() {
        return crate::runtime::MappingOutput {
            records,
            warnings,
            errors,
        };
    }

    for rec in &mapping.records {
        match eval_record(
            rec,
            &source_val,
            &root_val,
            &ctx_val,
            &codes,
            mode,
            &mut errors,
            &mut warnings,
            &paths,
        ) {
            Ok(rows) => {
                records.insert(rec.name.clone(), rows);
            }
            Err(e) => {
                errors.push(e);
                if matches!(mode, ErrorMode::Strict) {
                    break;
                }
            }
        }
    }

    crate::runtime::MappingOutput {
        records,
        warnings,
        errors,
    }
}

fn collect_mapping_paths(mapping: &CompiledMapping) -> Vec<(String, Vec<String>)> {
    // Build a flat list of all compiled programs in this mapping so we can
    // perform AST-based classification (missing-aware vs strict context).
    let mut programs: Vec<&cel::Program> = Vec::new();
    for v in &mapping.validations {
        programs.push(&v.cel.program);
    }
    for rec in &mapping.records {
        if let Some(em) = &rec.emit {
            programs.push(&em.program);
        }
        if let Some(fx) = &rec.foreach {
            programs.push(&fx.program);
        }
        if let Some(w) = &rec.when {
            programs.push(&w.program);
        }
        for (_, cel) in &rec.vars {
            programs.push(&cel.program);
        }
        for field in &rec.fields {
            programs.push(&field.cel.program);
        }
    }
    let mut paths = collect_missing_aware_injection_paths(&programs);

    // For custom `foreach.as` bindings (non-standard root names like a user-defined
    // alias), AST classification cannot infer the root from the standard set.
    // Fall back to regex-based collection for those extra roots only.
    for rec in &mapping.records {
        let Some(as_name) = rec.r#as.as_deref() else {
            continue;
        };
        if matches!(as_name, "item" | "member") {
            continue;
        }
        let mut row_exprs = Vec::new();
        if let Some(when) = &rec.when {
            row_exprs.push(when.source.clone());
        }
        for (_, cel) in &rec.vars {
            row_exprs.push(cel.source.clone());
        }
        for field in &rec.fields {
            row_exprs.push(field.cel.source.clone());
        }
        paths.extend(collect_dotted_paths_with_roots(&row_exprs, &[as_name]));
    }
    paths.sort();
    paths.dedup();
    paths
}

/// `on_error` when an expression fails (spec §5.4): required is always `fail`; optional defaults
/// to `fail` in strict/collect and `null` in lenient.
fn field_error_policy(mode: ErrorMode, required: bool, explicit: Option<&str>) -> String {
    if required {
        return "fail".into();
    }
    let d = match mode {
        ErrorMode::Lenient => "null",
        ErrorMode::Strict | ErrorMode::Collect => "fail",
    };
    match explicit {
        None => d.into(),
        Some(s @ ("fail" | "null" | "omit" | "default")) => s.into(),
        Some(_) => d.into(),
    }
}

fn empty_vars() -> Value {
    Value::Map(cel::objects::Map {
        map: Arc::new(std::collections::HashMap::new()),
    })
}

fn json_object_to_cel_map(j: &JsonValue) -> std::collections::HashMap<cel::objects::Key, Value> {
    let JsonValue::Object(m) = j else {
        return std::collections::HashMap::new();
    };
    m.iter()
        .map(|(k, v)| {
            (
                cel::objects::Key::String(Arc::new(k.clone())),
                json_to_cel(v),
            )
        })
        .collect()
}

fn eval_record(
    rec: &CompiledRecord,
    source: &Value,
    root: &Value,
    ctx: &Value,
    codes: &Arc<crate::code_system::CodeSystemRegistry>,
    mode: ErrorMode,
    collected: &mut Vec<MappingError>,
    warnings: &mut Vec<MappingError>,
    all_paths: &[(String, Vec<String>)],
) -> Result<Vec<JsonValue>, MappingError> {
    let mut out = Vec::new();

    if let Some(fx) = &rec.foreach {
        let list_v = match run_program(
            fx,
            source,
            root,
            ctx,
            &empty_vars(),
            None,
            None,
            None,
            &[],
            codes,
        ) {
            Ok(v) => v,
            Err(e) => {
                let err = exec_mapping_err_path(&rec.name, "foreach", e, "foreach", &fx.source);
                return match mode {
                    ErrorMode::Strict => Err(err),
                    ErrorMode::Lenient => {
                        warnings.push(err.warning());
                        Ok(vec![])
                    }
                    ErrorMode::Collect => {
                        collected.push(err);
                        Ok(vec![])
                    }
                };
            }
        };
        let items = match list_v {
            Value::List(l) => l.as_ref().clone(),
            _ => {
                return Err(MappingError::error(
                    ErrorCode::TypeError,
                    "foreach must evaluate to a list",
                    Some(format!("records.{}", rec.name)),
                    None,
                ));
            }
        };
        let as_name = rec.r#as.clone().unwrap_or_else(|| "item".into());
        let mut row_roots = vec!["item", "member"];
        if as_name != "item" && as_name != "member" {
            row_roots.push(as_name.as_str());
        }
        let row_paths = filter_paths_by_roots(all_paths, &row_roots);
        for (index, item) in items.into_iter().enumerate() {
            let item_json = match cel_to_json(&item) {
                Ok(j) => j,
                Err(_) => JsonValue::Object(Map::new()),
            };
            let aug_json =
                augment_loop_element(&item_json, &row_paths, as_name.as_str(), MISSING_STR);
            let aug_item = json_to_cel(&aug_json);
            if let Some(w) = &rec.when {
                let wv = match run_program(
                    w,
                    source,
                    root,
                    ctx,
                    &empty_vars(),
                    Some(&aug_item),
                    Some(index as i64),
                    Some(as_name.as_str()),
                    &[],
                    codes,
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = exec_mapping_err_path(&rec.name, "when", e, "when", &w.source)
                            .with_record(rec.name.clone(), Some(index));
                        match mode {
                            ErrorMode::Strict => return Err(err),
                            ErrorMode::Lenient => {
                                warnings.push(err.warning());
                                continue;
                            }
                            ErrorMode::Collect => {
                                collected.push(err);
                                continue;
                            }
                        }
                    }
                };
                if !truthy(&wv) {
                    continue;
                }
            }
            if let Some(row) = eval_record_row(
                rec,
                source,
                root,
                ctx,
                &aug_item,
                index as i64,
                as_name.as_str(),
                codes,
                mode,
                collected,
                warnings,
                Some(index),
                all_paths,
            )? {
                out.push(row);
            }
        }
        return Ok(out);
    }

    if let Some(em) = &rec.emit {
        let ev = match run_program(
            em,
            source,
            root,
            ctx,
            &empty_vars(),
            None,
            None,
            None,
            &[],
            codes,
        ) {
            Ok(v) => v,
            Err(e) => {
                let err = exec_mapping_err_path(&rec.name, "emit", e, "emit", &em.source);
                return match mode {
                    ErrorMode::Strict => Err(err),
                    ErrorMode::Lenient => {
                        warnings.push(err.warning());
                        Ok(vec![])
                    }
                    ErrorMode::Collect => {
                        collected.push(err);
                        Ok(vec![])
                    }
                };
            }
        };
        if !truthy(&ev) {
            return Ok(vec![]);
        }
    }

    let row_paths = filter_paths_by_roots(all_paths, &["item", "member"]);
    let aug_json = augment_loop_element(&JsonValue::Null, &row_paths, "item", MISSING_STR);
    let aug_item = json_to_cel(&aug_json);
    if let Some(row) = eval_record_row(
        rec, source, root, ctx, &aug_item, -1, "item", codes, mode, collected, warnings, None,
        all_paths,
    )? {
        out.push(row);
    }
    Ok(out)
}

fn eval_record_row(
    rec: &CompiledRecord,
    source: &Value,
    root: &Value,
    ctx: &Value,
    item: &Value,
    index: i64,
    as_name: &str,
    codes: &Arc<crate::code_system::CodeSystemRegistry>,
    mode: ErrorMode,
    collected: &mut Vec<MappingError>,
    warnings: &mut Vec<MappingError>,
    row_index: Option<usize>,
    all_paths: &[(String, Vec<String>)],
) -> Result<Option<JsonValue>, MappingError> {
    let vars_paths = filter_paths_by_roots(all_paths, &["vars"]);
    let mut cel_vars_inner: std::collections::HashMap<cel::objects::Key, Value> =
        if vars_paths.is_empty() {
            std::collections::HashMap::new()
        } else {
            let mut wrap = Map::new();
            wrap.insert("vars".into(), JsonValue::Object(Map::new()));
            let mut env = JsonValue::Object(wrap);
            augment_json_with_paths(&mut env, &vars_paths, MISSING_STR);
            let vars_json = env
                .get("vars")
                .cloned()
                .unwrap_or_else(|| JsonValue::Object(Map::new()));
            json_object_to_cel_map(&vars_json)
        };

    for (vn, prog) in &rec.vars {
        let vars_val = Value::Map(cel::objects::Map {
            map: Arc::new(cel_vars_inner.clone()),
        });
        let v = match run_program(
            prog,
            source,
            root,
            ctx,
            &vars_val,
            Some(item),
            Some(index),
            Some(as_name),
            &[],
            codes,
        ) {
            Ok(v) => v,
            Err(e) => {
                let err =
                    exec_mapping_err_path(&rec.name, vn, e, &format!("vars.{vn}"), &prog.source)
                        .with_record(rec.name.clone(), row_index);
                match mode {
                    ErrorMode::Strict => return Err(err),
                    ErrorMode::Lenient => {
                        warnings.push(err.warning());
                    }
                    ErrorMode::Collect => {
                        collected.push(err);
                    }
                }
                Value::Null
            }
        };
        cel_vars_inner.insert(cel::objects::Key::String(Arc::new(vn.clone())), v.clone());
    }

    let vars_val = Value::Map(cel::objects::Map {
        map: Arc::new(cel_vars_inner),
    });

    let mut obj = Map::new();
    for cf in &rec.fields {
        let path = format!("records.{}.fields.{}", rec.name, cf.name);
        let res = run_program(
            &cf.cel,
            source,
            root,
            ctx,
            &vars_val,
            Some(item),
            Some(index),
            Some(as_name),
            &[],
            codes,
        );
        let (required, on_err, defv) = match &cf.yaml {
            FieldYaml::Short(_) => (false, None, None),
            FieldYaml::Long {
                required,
                on_error,
                default,
                ..
            } => (*required, on_error.clone(), default.clone()),
        };
        let output_on_err: Option<&str> = match (&cf.yaml, required, mode) {
            (FieldYaml::Short(_), false, ErrorMode::Lenient) => Some("null"),
            _ => on_err.as_deref(),
        };

        match res {
            Ok(v) => {
                let j = match cel_to_json(&v) {
                    Ok(j) => j,
                    Err(e) => {
                        if matches!(mode, ErrorMode::Strict) {
                            return Err(mapping_err_with_expr(
                                ErrorCode::InternalError,
                                e,
                                Some(path.clone()),
                                &cf.cel.source,
                            ));
                        }
                        let row = mapping_err_with_expr(
                            ErrorCode::InternalError,
                            e.clone(),
                            Some(path.clone()),
                            &cf.cel.source,
                        )
                        .with_record(rec.name.clone(), row_index);
                        match mode {
                            ErrorMode::Lenient if !required => {
                                warnings.push(row.warning());
                            }
                            _ => {
                                collected.push(row);
                            }
                        }
                        JsonValue::Null
                    }
                };
                if j.is_null() && required {
                    if matches!(mode, ErrorMode::Strict) {
                        return Err(mapping_err_with_expr(
                            ErrorCode::MissingRequiredValue,
                            format!("required field {}", cf.name),
                            Some(path),
                            &cf.cel.source,
                        ));
                    }
                    collected.push(
                        mapping_err_with_expr(
                            ErrorCode::MissingRequiredValue,
                            format!("required field {}", cf.name),
                            Some(path),
                            &cf.cel.source,
                        )
                        .with_record(rec.name.clone(), row_index),
                    );
                    return Ok(None);
                }
                apply_field_output(&mut obj, &cf.name, j, output_on_err, defv)?;
            }
            Err(e) => {
                if required {
                    if matches!(mode, ErrorMode::Strict) {
                        return Err(exec_mapping_err_path(
                            &rec.name,
                            &cf.name,
                            e,
                            &cf.name,
                            &cf.cel.source,
                        ));
                    }
                    collected.push(
                        exec_mapping_err_path(
                            &rec.name,
                            &cf.name,
                            e.clone(),
                            &cf.name,
                            &cf.cel.source,
                        )
                        .with_record(rec.name.clone(), row_index),
                    );
                    return Ok(None);
                }
                let policy = field_error_policy(mode, required, on_err.as_deref());
                if matches!(mode, ErrorMode::Strict) && policy.as_str() == "fail" {
                    return Err(exec_mapping_err_path(
                        &rec.name,
                        &cf.name,
                        e,
                        &cf.name,
                        &cf.cel.source,
                    ));
                }
                if matches!(mode, ErrorMode::Collect) && policy.as_str() == "fail" {
                    collected.push(
                        exec_mapping_err_path(
                            &rec.name,
                            &cf.name,
                            e.clone(),
                            &cf.name,
                            &cf.cel.source,
                        )
                        .with_record(rec.name.clone(), row_index),
                    );
                }
                if matches!(mode, ErrorMode::Lenient) {
                    warnings.push(
                        exec_mapping_err_path(
                            &rec.name,
                            &cf.name,
                            e.clone(),
                            &cf.name,
                            &cf.cel.source,
                        )
                        .with_record(rec.name.clone(), row_index)
                        .warning(),
                    );
                }
                // Collect + default `fail`, and lenient + explicit `fail` on optional fields: still emit
                // a partial row (null field) instead of aborting the row via `apply_field_error_output("fail", ...)`.
                let apply_policy = if (matches!(mode, ErrorMode::Collect)
                    || matches!(mode, ErrorMode::Lenient))
                    && policy.as_str() == "fail"
                {
                    "null"
                } else {
                    policy.as_str()
                };
                apply_field_error_output(
                    &mut obj,
                    &cf.name,
                    &path,
                    apply_policy,
                    defv,
                    e,
                    &cf.cel.source,
                )?;
            }
        }
    }

    Ok(Some(JsonValue::Object(obj)))
}

fn apply_field_output(
    obj: &mut Map<String, JsonValue>,
    name: &str,
    j: JsonValue,
    on_err: Option<&str>,
    defv: Option<serde_yaml::Value>,
) -> Result<(), MappingError> {
    let on = on_err.unwrap_or("fail");
    if j.is_null() && on == "omit" {
        return Ok(());
    }
    if j.is_null() && on == "default" {
        if let Some(d) = defv {
            obj.insert(
                name.to_string(),
                serde_json::to_value(d).unwrap_or(JsonValue::Null),
            );
            return Ok(());
        }
    }
    obj.insert(name.to_string(), omit_null_keys(j));
    Ok(())
}

fn apply_field_error_output(
    obj: &mut Map<String, JsonValue>,
    name: &str,
    path: &str,
    on_err: &str,
    defv: Option<serde_yaml::Value>,
    err: ExecutionError,
    expr_source: &str,
) -> Result<(), MappingError> {
    match on_err {
        "fail" => Err(mapping_err_with_expr(
            ErrorCode::EvaluationError,
            err.to_string(),
            Some(path.to_string()),
            expr_source,
        )),
        "omit" => Ok(()),
        "default" => {
            if let Some(d) = defv {
                obj.insert(
                    name.to_string(),
                    serde_json::to_value(d).unwrap_or(JsonValue::Null),
                );
            }
            Ok(())
        }
        _ => {
            obj.insert(name.to_string(), JsonValue::Null);
            Ok(())
        }
    }
}

fn truthy(v: &Value) -> bool {
    if is_missing(v) {
        return false;
    }
    match v {
        Value::Bool(b) => *b,
        Value::Null | Value::UInt(0) | Value::Int(0) => false,
        Value::String(s) => !s.is_empty(),
        Value::List(l) => !l.is_empty(),
        Value::Map(m) => !m.map.is_empty(),
        _ => true,
    }
}

pub(crate) fn json_to_cel(j: &JsonValue) -> Value {
    cel::to_value(j).unwrap_or(Value::Null)
}

pub(crate) fn run_program(
    cel: &CompiledCel,
    source: &Value,
    root: &Value,
    ctx: &Value,
    vars: &Value,
    item: Option<&Value>,
    index: Option<i64>,
    as_name: Option<&str>,
    extra_bindings: &[(&str, Value)],
    codes: &Arc<crate::code_system::CodeSystemRegistry>,
) -> Result<Value, ExecutionError> {
    let mut c = Context::default();
    register_stdlib(&mut c, Arc::clone(codes));
    c.add_variable_from_value("source", source.clone());
    c.add_variable_from_value("root", root.clone());
    c.add_variable_from_value("ctx", ctx.clone());
    c.add_variable_from_value("vars", vars.clone());
    if let Some(ix) = index {
        c.add_variable_from_value("index", ix);
    }
    if let (Some(it), Some(an)) = (item, as_name) {
        c.add_variable_from_value(an, it.clone());
        if an != "item" {
            c.add_variable_from_value("item", it.clone());
        }
    } else {
        c.add_variable_from_value("item", Value::Null);
    }
    for (name, value) in extra_bindings {
        c.add_variable_from_value(*name, value.clone());
    }
    cel.program.execute(&c)
}

/// `path_suffix` is the last segment after `records.{record}.` (e.g. `fields.foo`, `when`, `vars.x`).
fn exec_validation_err(path: &str, expr: &str, message: &str, e: ExecutionError) -> MappingError {
    mapping_err_with_expr(
        ErrorCode::EvaluationError,
        format!("{message}: {}", e),
        Some(path.to_string()),
        expr,
    )
}

fn mapping_err_with_expr(
    code: ErrorCode,
    message: impl Into<String>,
    path: Option<String>,
    expr_source: &str,
) -> MappingError {
    let mut err = MappingError::error(code, message, path, Some(expr_source.to_string()));
    err.source_path = crate::paths::primary_binding_hint(expr_source);
    err
}

fn exec_mapping_err_path(
    record: &str,
    field: &str,
    e: ExecutionError,
    path_suffix: &str,
    expr_source: &str,
) -> MappingError {
    let path =
        if path_suffix.starts_with("vars.") || matches!(path_suffix, "when" | "emit" | "foreach") {
            format!("records.{record}.{path_suffix}")
        } else {
            format!("records.{record}.fields.{field}")
        };
    mapping_err_with_expr(
        ErrorCode::EvaluationError,
        e.to_string(),
        Some(path),
        expr_source,
    )
    .with_record(record.to_string(), None)
}

/// Evaluate one mapping-stdlib CEL expression (no YAML mapping document).
///
/// Bindings match field expressions: `source`, `root`, `ctx`, `vars` (empty object), `item` (null).
/// Dotted paths in the expression are scanned so the binding envelope can inject Missing placeholders
/// under `source` / `root` / `ctx` like full mappings.
pub fn evaluate_cel_expression(
    expr: &str,
    source: JsonValue,
    ctx: JsonValue,
    limits: &crate::security::SecurityLimits,
    codes: Arc<crate::code_system::CodeSystemRegistry>,
) -> Result<JsonValue, StandaloneEvalError> {
    crosswalk_cel::evaluate_cel_expression(expr, source, ctx, limits, codes)
}

/// Evaluate one mapping-stdlib CEL expression against arbitrary root bindings.
///
/// Binding names must be valid CEL-style identifiers. Dotted paths used only in missing-aware
/// helper arguments receive the same Missing sentinel treatment as full mappings.
pub fn evaluate_cel_expression_with_input(
    expr: &str,
    input: StandaloneExpressionInput,
    limits: &crate::security::SecurityLimits,
    codes: Arc<crate::code_system::CodeSystemRegistry>,
) -> Result<JsonValue, StandaloneEvalError> {
    crosswalk_cel::evaluate_cel_expression_with_input(expr, input, limits, codes)
}

/// Compile and evaluate one expression, returning **structured** diagnostics (syntax line/column,
/// evaluation messages) plus an optional JSON value — intended for editors and playgrounds.
pub fn preview_cel_expression(
    expr: &str,
    source: JsonValue,
    ctx: JsonValue,
    limits: &crate::security::SecurityLimits,
    codes: Arc<crate::code_system::CodeSystemRegistry>,
) -> ExpressionPreviewResult {
    crosswalk_cel::preview_cel_expression(expr, source, ctx, limits, codes)
}

/// Compile and evaluate one expression against arbitrary root bindings, returning structured
/// diagnostics instead of `Err`.
pub fn preview_cel_expression_with_input(
    expr: &str,
    input: StandaloneExpressionInput,
    limits: &crate::security::SecurityLimits,
    codes: Arc<crate::code_system::CodeSystemRegistry>,
) -> ExpressionPreviewResult {
    crosswalk_cel::preview_cel_expression_with_input(expr, input, limits, codes)
}

pub fn validate_root_binding_name(name: &str) -> Result<(), String> {
    crosswalk_cel::validate_root_binding_name(name)
}
