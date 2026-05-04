use crate::code_system::CodeSystemRegistry;
use crate::mapping::{FieldYaml, RecordYaml};
use cel::Program;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorMode {
    Strict,
    Collect,
    Lenient,
}

impl ErrorMode {
    pub fn parse(s: Option<&str>) -> Self {
        match s.unwrap_or("strict").to_lowercase().as_str() {
            "collect" => ErrorMode::Collect,
            "lenient" => ErrorMode::Lenient,
            _ => ErrorMode::Strict,
        }
    }
}

/// Compiled CEL program plus the mapping expression as authored (for errors and diagnostics).
#[derive(Debug)]
pub struct CompiledCel {
    pub program: Program,
    /// Expression text from the mapping YAML (before namespaced rewrite).
    pub source: String,
}

#[derive(Debug)]
pub struct CompiledField {
    pub name: String,
    pub cel: CompiledCel,
    pub yaml: FieldYaml,
}

#[derive(Debug)]
pub struct CompiledRecord {
    pub name: String,
    pub emit: Option<CompiledCel>,
    pub foreach: Option<CompiledCel>,
    pub r#as: Option<String>,
    pub when: Option<CompiledCel>,
    pub vars: Vec<(String, CompiledCel)>,
    pub fields: Vec<CompiledField>,
}

#[derive(Debug)]
pub struct CompiledValidation {
    /// Stable path such as `validations[0]`.
    pub path: String,
    pub cel: CompiledCel,
    pub message: String,
}

#[derive(Debug)]
pub struct CompiledMapping {
    pub name: String,
    pub version: String,
    pub error_mode: ErrorMode,
    pub records: Vec<CompiledRecord>,
    pub validations: Vec<CompiledValidation>,
    pub all_expressions: Vec<String>,
    pub code_systems: Arc<CodeSystemRegistry>,
}

impl CompiledMapping {
    pub fn collect_expressions_from_record(r: &RecordYaml) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(e) = &r.emit {
            v.push(e.clone());
        }
        if let Some(e) = &r.foreach {
            v.push(e.clone());
        }
        if let Some(e) = &r.when {
            v.push(e.clone());
        }
        for e in r.vars.values() {
            v.push(e.clone());
        }
        for f in r.fields.values() {
            v.push(f.expr().to_string());
        }
        v
    }
}
