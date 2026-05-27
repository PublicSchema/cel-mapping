//! Compatibility YAML parsing for code-system registries.

pub use crosswalk_functions::codes::{CodeEntry, CodeSystemDocument, CodeSystemRegistry};
pub type CodeSystemError = crosswalk_functions::FunctionError;

use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};

pub fn merge_yaml_value(
    registry: &mut CodeSystemRegistry,
    name: &str,
    raw: &serde_yaml::Value,
) -> Result<(), CodeSystemError> {
    registry.merge_document(name, document_from_yaml(raw)?)
}

pub fn merge_from_map(
    registry: &mut CodeSystemRegistry,
    systems: &BTreeMap<String, serde_yaml::Value>,
) -> Result<(), CodeSystemError> {
    for (name, raw) in systems {
        merge_yaml_value(registry, name, raw)?;
    }
    Ok(())
}

pub fn document_from_yaml(raw: &serde_yaml::Value) -> Result<CodeSystemDocument, CodeSystemError> {
    let obj = raw.as_mapping().ok_or_else(|| {
        CodeSystemError::new("CODE_SYSTEM_INVALID", "code_system must be mapping")
    })?;
    let mut entries = BTreeMap::new();
    for (key, value) in obj {
        let source_key = key
            .as_str()
            .ok_or_else(|| CodeSystemError::new("CODE_SYSTEM_INVALID", "non-string key"))?
            .to_string();
        entries.insert(source_key.clone(), entry_from_yaml(value, &source_key)?);
    }
    Ok(CodeSystemDocument { entries })
}

fn entry_from_yaml(
    value: &serde_yaml::Value,
    source_key: &str,
) -> Result<CodeEntry, CodeSystemError> {
    if let Some(id) = value.as_str() {
        return Ok(CodeEntry {
            id: id.to_string(),
            label: BTreeMap::new(),
            aliases: Vec::new(),
            extra: HashMap::new(),
        });
    }

    let mapping = value.as_mapping().ok_or_else(|| {
        CodeSystemError::new("CODE_SYSTEM_INVALID", "entry must be string or map")
    })?;
    let id = mapping
        .get(serde_yaml::Value::String("id".into()))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
        .ok_or_else(|| {
            CodeSystemError::new(
                "CODE_SYSTEM_INVALID",
                format!("missing id for {source_key}"),
            )
        })?;

    let mut label = BTreeMap::new();
    if let Some(labels) = mapping.get(serde_yaml::Value::String("label".into())) {
        if let Some(labels) = labels.as_mapping() {
            for (key, value) in labels {
                if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
                    label.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    let mut aliases = Vec::new();
    if let Some(raw_aliases) = mapping.get(serde_yaml::Value::String("aliases".into())) {
        if let Some(raw_aliases) = raw_aliases.as_sequence() {
            aliases.extend(
                raw_aliases
                    .iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string)),
            );
        }
    }

    let mut extra = HashMap::new();
    for (key, value) in mapping {
        let Some(key) = key.as_str() else {
            continue;
        };
        if matches!(key, "id" | "label" | "aliases") {
            continue;
        }
        if let Ok(value) = serde_json::to_value(value) {
            extra.insert(key.to_string(), value);
        } else {
            extra.insert(key.to_string(), JsonValue::Null);
        }
    }

    Ok(CodeEntry {
        id,
        label,
        aliases,
        extra,
    })
}
