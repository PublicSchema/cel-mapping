//! Parsed mapping document (spec §5).

use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Default, Deserialize)]
pub struct ErrorsYaml {
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidationYaml {
    pub expr: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FieldYaml {
    Short(String),
    Long {
        expr: String,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        on_error: Option<String>,
        #[serde(default)]
        default: Option<serde_yaml::Value>,
    },
}

impl FieldYaml {
    pub fn expr(&self) -> &str {
        match self {
            FieldYaml::Short(s) => s,
            FieldYaml::Long { expr, .. } => expr,
        }
    }

    pub fn required(&self) -> bool {
        match self {
            FieldYaml::Short(_) => false,
            FieldYaml::Long { required, .. } => *required,
        }
    }

    pub fn on_error(&self) -> Option<&str> {
        match self {
            FieldYaml::Short(_) => None,
            FieldYaml::Long { on_error, .. } => on_error.as_deref(),
        }
    }

    pub fn default_value(&self) -> Option<&serde_yaml::Value> {
        match self {
            FieldYaml::Short(_) => None,
            FieldYaml::Long { default, .. } => default.as_ref(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RecordYaml {
    #[serde(default)]
    pub emit: Option<String>,
    #[serde(default)]
    pub foreach: Option<String>,
    #[serde(default)]
    pub r#as: Option<String>,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub vars: IndexMap<String, String>,
    pub fields: IndexMap<String, FieldYaml>,
}

#[derive(Debug, Deserialize)]
pub struct MappingDocument {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub source_system: Option<String>,
    #[serde(default)]
    pub target_model: Option<String>,
    #[serde(default)]
    pub code_systems: BTreeMap<String, serde_yaml::Value>,
    pub records: IndexMap<String, RecordYaml>,
    #[serde(default)]
    pub validations: Vec<ValidationYaml>,
    #[serde(default)]
    pub errors: ErrorsYaml,
}
