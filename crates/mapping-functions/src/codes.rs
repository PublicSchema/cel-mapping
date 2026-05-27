use crate::FunctionError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeEntry {
    pub id: String,
    #[serde(default)]
    pub label: BTreeMap<String, String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CodeSystemDocument {
    pub entries: BTreeMap<String, CodeEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct CodeSystemRegistry {
    systems: HashMap<String, CodeSystemTable>,
}

#[derive(Debug, Clone)]
struct CodeSystemTable {
    by_source: HashMap<String, CodeEntry>,
    by_canonical: HashMap<String, CodeEntry>,
    reverse: HashMap<String, String>,
}

impl CodeSystemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn normalize_code(input: &str) -> String {
        input.trim().to_lowercase()
    }

    pub fn merge_document(
        &mut self,
        name: &str,
        document: CodeSystemDocument,
    ) -> Result<(), FunctionError> {
        self.systems
            .insert(name.to_string(), table_from_document(document)?);
        Ok(())
    }

    pub fn merge_documents(
        &mut self,
        documents: impl IntoIterator<Item = (String, CodeSystemDocument)>,
    ) -> Result<(), FunctionError> {
        for (name, document) in documents {
            self.merge_document(&name, document)?;
        }
        Ok(())
    }

    pub fn map(&self, system: &str, source: &str) -> Option<&CodeEntry> {
        let table = self.systems.get(system)?;
        table.by_source.get(&Self::normalize_code(source))
    }

    pub fn entry_by_canonical(&self, system: &str, target: &str) -> Option<&CodeEntry> {
        let table = self.systems.get(system)?;
        table.by_canonical.get(&Self::normalize_code(target))
    }

    pub fn reverse_map(&self, system: &str, target: &str) -> Option<String> {
        let table = self.systems.get(system)?;
        table.reverse.get(&Self::normalize_code(target)).cloned()
    }

    pub fn exists(&self, system: &str, value: &str) -> bool {
        self.map(system, value).is_some()
    }

    pub fn has_system(&self, system: &str) -> bool {
        self.systems.contains_key(system)
    }
}

pub fn load_iso_systems(registry: &mut CodeSystemRegistry) {
    for (name, yaml) in [
        ("iso3166-alpha3", include_str!("data/iso3166_alpha3.yaml")),
        ("iso4217", include_str!("data/iso4217.yaml")),
        ("iso639-3", include_str!("data/iso639_3.yaml")),
    ] {
        let document = parse_simple_iso_yaml(yaml);
        registry
            .merge_document(name, document)
            .expect("bundled ISO code systems have no alias collisions");
    }
}

fn table_from_document(document: CodeSystemDocument) -> Result<CodeSystemTable, FunctionError> {
    let mut by_source = HashMap::new();
    let mut by_canonical = HashMap::new();
    let mut reverse = HashMap::new();
    let mut alias_owner: HashMap<String, String> = HashMap::new();

    for (source_key, entry) in document.entries {
        claim(&mut alias_owner, &source_key, &source_key)?;
        by_source.insert(
            CodeSystemRegistry::normalize_code(&source_key),
            entry.clone(),
        );

        for alias in &entry.aliases {
            claim(&mut alias_owner, alias, &source_key)?;
            by_source.insert(CodeSystemRegistry::normalize_code(alias), entry.clone());
        }

        claim(&mut alias_owner, &entry.id, &source_key)?;
        let canonical = CodeSystemRegistry::normalize_code(&entry.id);
        by_source.insert(canonical.clone(), entry.clone());
        by_canonical.insert(canonical.clone(), entry.clone());
        reverse.entry(canonical).or_insert(source_key);
    }

    Ok(CodeSystemTable {
        by_source,
        by_canonical,
        reverse,
    })
}

fn claim(
    alias_owner: &mut HashMap<String, String>,
    key: &str,
    owner: &str,
) -> Result<(), FunctionError> {
    let normalized = CodeSystemRegistry::normalize_code(key);
    if let Some(previous) = alias_owner.get(&normalized) {
        if previous != owner {
            return Err(FunctionError::new(
                "CODE_ALIAS_COLLISION",
                format!("key `{normalized}` used by `{previous}` and `{owner}`"),
            ));
        }
        return Ok(());
    }
    alias_owner.insert(normalized, owner.to_string());
    Ok(())
}

fn parse_simple_iso_yaml(yaml: &str) -> CodeSystemDocument {
    let mut entries = BTreeMap::new();
    let mut current_key: Option<String> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') && trimmed.ends_with(':') {
            let key = trimmed.trim_end_matches(':').to_string();
            entries.insert(
                key.clone(),
                CodeEntry {
                    id: key.clone(),
                    label: BTreeMap::new(),
                    aliases: Vec::new(),
                    extra: HashMap::new(),
                },
            );
            current_key = Some(key);
            continue;
        }
        let Some(key) = current_key.as_ref() else {
            continue;
        };
        let Some(entry) = entries.get_mut(key) else {
            continue;
        };
        if let Some(id) = trimmed.strip_prefix("id:") {
            entry.id = id.trim().to_string();
        } else if let Some(aliases) = trimmed.strip_prefix("aliases:") {
            let aliases = aliases.trim();
            if aliases.starts_with('[') && aliases.ends_with(']') {
                entry.aliases = aliases
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
        }
    }

    CodeSystemDocument { entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_maps_aliases_and_reverse_values() {
        let mut registry = CodeSystemRegistry::new();
        registry
            .merge_document(
                "demo",
                CodeSystemDocument {
                    entries: BTreeMap::from([(
                        "M".to_string(),
                        CodeEntry {
                            id: "male".to_string(),
                            label: BTreeMap::from([("en".to_string(), "Male".to_string())]),
                            aliases: vec!["man".to_string()],
                            extra: HashMap::new(),
                        },
                    )]),
                },
            )
            .unwrap();

        assert_eq!(registry.map("demo", " man ").unwrap().id, "male");
        assert_eq!(registry.reverse_map("demo", "MALE").as_deref(), Some("M"));
    }

    #[test]
    fn iso_systems_load_without_yaml_dependency() {
        let mut registry = CodeSystemRegistry::new();
        load_iso_systems(&mut registry);
        assert_eq!(registry.map("iso3166-alpha3", "us").unwrap().id, "USA");
        assert_eq!(registry.map("iso4217", "kes").unwrap().id, "KES");
        assert_eq!(registry.map("iso639-3", "sw").unwrap().id, "swa");
    }
}
