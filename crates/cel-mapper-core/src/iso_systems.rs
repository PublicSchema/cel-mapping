// SPDX-License-Identifier: Apache-2.0
//! Preloaded ISO code systems for `code_normalize(system, value)`.
//!
//! Tables are embedded at compile time from YAML files in `src/data/`.
//! Regenerate the data files via `scripts/gen_iso*.py` when the standards update.

use crate::code_system::CodeSystemRegistry;

const ISO3166_ALPHA3_YAML: &str = include_str!("data/iso3166_alpha3.yaml");
const ISO4217_YAML: &str = include_str!("data/iso4217.yaml");
const ISO639_3_YAML: &str = include_str!("data/iso639_3.yaml");

/// Preload the three ISO systems used by `code_normalize(system, value)` into `registry`.
///
/// Called once at `MappingRuntime::new`; callers can still override individual systems by
/// calling `register_code_system` afterward (later inserts win).
pub fn load_iso_systems(registry: &mut CodeSystemRegistry) {
    for (name, yaml) in [
        ("iso3166-alpha3", ISO3166_ALPHA3_YAML),
        ("iso4217", ISO4217_YAML),
        ("iso639-3", ISO639_3_YAML),
    ] {
        let v: serde_yaml::Value = serde_yaml::from_str(yaml)
            .expect("bundled ISO YAML is well-formed");
        registry
            .merge_yaml_value(name, &v)
            .expect("bundled ISO YAML has no alias collisions");
    }
}
