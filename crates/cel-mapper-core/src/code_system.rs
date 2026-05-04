//! Code system registry (spec §13).

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntry {
    pub id: String,
    #[serde(default)]
    pub label: BTreeMap<String, String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Extra metadata (spec §13.3) preserved as JSON.
    #[serde(flatten)]
    pub extra: HashMap<String, JsonValue>,
}

#[derive(Debug, Error)]
pub enum CodeSystemError {
    #[error("alias collision: {0}")]
    AliasCollision(String),
}

#[derive(Debug, Clone, Default)]
pub struct CodeSystemRegistry {
    systems: HashMap<String, CodeSystemTable>,
}

#[derive(Debug, Clone)]
struct CodeSystemTable {
    /// normalized lookup key -> entry
    by_source: HashMap<String, CodeEntry>,
    /// normalized canonical id -> entry (for label/reverse)
    by_canonical: HashMap<String, CodeEntry>,
    /// canonical id -> representative source key
    reverse: HashMap<String, String>,
}

impl CodeSystemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge_yaml_value(
        &mut self,
        name: &str,
        raw: &serde_yaml::Value,
    ) -> Result<(), CodeSystemError> {
        let table = parse_code_system_yaml(raw)?;
        self.systems.insert(name.to_string(), table);
        Ok(())
    }

    pub fn merge_from_map(
        &mut self,
        systems: &BTreeMap<String, serde_yaml::Value>,
    ) -> Result<(), CodeSystemError> {
        for (k, v) in systems {
            self.merge_yaml_value(k, v)?;
        }
        Ok(())
    }

    pub fn normalize_code(value: &str) -> String {
        value.trim().to_lowercase()
    }

    pub fn map(&self, system: &str, value: &str) -> Option<&CodeEntry> {
        let t = self.systems.get(system)?;
        let k = Self::normalize_code(value);
        t.by_source.get(&k)
    }

    pub fn entry_by_canonical(&self, system: &str, canon: &str) -> Option<&CodeEntry> {
        let t = self.systems.get(system)?;
        let k = Self::normalize_code(canon);
        t.by_canonical.get(&k)
    }

    pub fn exists(&self, system: &str, value: &str) -> bool {
        self.map(system, value).is_some()
    }

    pub fn reverse_map(&self, system: &str, canonical_value: &str) -> Option<String> {
        let t = self.systems.get(system)?;
        let ck = Self::normalize_code(canonical_value);
        t.reverse.get(&ck).cloned()
    }
}

fn parse_code_system_yaml(raw: &serde_yaml::Value) -> Result<CodeSystemTable, CodeSystemError> {
    let obj = raw
        .as_mapping()
        .ok_or_else(|| CodeSystemError::AliasCollision("code_system must be mapping".into()))?;
    let mut by_source: HashMap<String, CodeEntry> = HashMap::new();
    let mut by_canonical: HashMap<String, CodeEntry> = HashMap::new();
    let mut reverse: HashMap<String, String> = HashMap::new();
    let mut alias_owner: HashMap<String, String> = HashMap::new();

    fn claim(
        alias_owner: &mut HashMap<String, String>,
        key: &str,
        owner: &str,
    ) -> Result<(), CodeSystemError> {
        let nk = CodeSystemRegistry::normalize_code(key);
        if let Some(prev) = alias_owner.get(&nk) {
            if prev != owner {
                return Err(CodeSystemError::AliasCollision(format!(
                    "key `{nk}` used by `{prev}` and `{owner}`"
                )));
            }
            return Ok(());
        }
        alias_owner.insert(nk, owner.to_string());
        Ok(())
    }

    for (k, v) in obj {
        let source_key = k
            .as_str()
            .ok_or_else(|| CodeSystemError::AliasCollision("non-string key".into()))?
            .to_string();
        let entry = entry_from_yaml(v, &source_key)?;
        let norm_key = CodeSystemRegistry::normalize_code(&source_key);
        claim(&mut alias_owner, &source_key, &source_key)?;
        by_source.insert(norm_key, entry.clone());

        for a in &entry.aliases {
            claim(&mut alias_owner, a, &source_key)?;
            let an = CodeSystemRegistry::normalize_code(a);
            by_source.insert(an, entry.clone());
        }
        let cid = CodeSystemRegistry::normalize_code(&entry.id);
        claim(&mut alias_owner, &entry.id, &source_key)?;
        by_source.insert(cid.clone(), entry.clone());
        by_canonical.insert(cid.clone(), entry.clone());
        reverse.entry(cid).or_insert_with(|| source_key.clone());
    }

    Ok(CodeSystemTable {
        by_source,
        by_canonical,
        reverse,
    })
}

fn entry_from_yaml(v: &serde_yaml::Value, _key: &str) -> Result<CodeEntry, CodeSystemError> {
    if let Some(s) = v.as_str() {
        return Ok(CodeEntry {
            id: s.to_string(),
            label: BTreeMap::new(),
            aliases: vec![],
            extra: HashMap::new(),
        });
    }
    let m = v
        .as_mapping()
        .ok_or_else(|| CodeSystemError::AliasCollision("entry must be string or map".into()))?;
    let id = m
        .get(serde_yaml::Value::String("id".into()))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CodeSystemError::AliasCollision("missing id".into()))?;

    let mut label = BTreeMap::new();
    if let Some(l) = m.get(serde_yaml::Value::String("label".into())) {
        if let Some(lm) = l.as_mapping() {
            for (lk, lv) in lm {
                if let (Some(a), Some(b)) = (lk.as_str(), lv.as_str()) {
                    label.insert(a.to_string(), b.to_string());
                }
            }
        }
    }
    let mut aliases = Vec::new();
    if let Some(a) = m.get(serde_yaml::Value::String("aliases".into())) {
        if let Some(seq) = a.as_sequence() {
            for it in seq {
                if let Some(s) = it.as_str() {
                    aliases.push(s.to_string());
                }
            }
        }
    }
    let mut extra = HashMap::new();
    for (k, v) in m {
        let ks = match k.as_str() {
            Some(s) => s,
            None => continue,
        };
        if matches!(ks, "id" | "label" | "aliases") {
            continue;
        }
        if let Ok(j) = serde_json::to_value(v) {
            extra.insert(ks.to_string(), j);
        }
    }
    Ok(CodeEntry {
        id,
        label,
        aliases,
        extra,
    })
}
