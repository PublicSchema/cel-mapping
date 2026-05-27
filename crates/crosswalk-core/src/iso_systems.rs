// SPDX-License-Identifier: Apache-2.0
//! Preloaded ISO code systems for `code_normalize(system, value)`.
//!
//! Tables are embedded at compile time from YAML files in `src/data/`.
//! Regenerate the data files via `scripts/gen_iso*.py` when the standards update.

use crate::code_system::CodeSystemRegistry;

/// Preload the three ISO systems used by `code_normalize(system, value)` into `registry`.
///
/// Called once at `MappingRuntime::new`; callers can still override individual systems by
/// calling `register_code_system` afterward (later inserts win).
pub fn load_iso_systems(registry: &mut CodeSystemRegistry) {
    crosswalk_functions::codes::load_iso_systems(registry);
}
