//! Runtime limits (spec §15).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityLimits {
    pub max_expression_bytes: usize,
    pub max_output_json_bytes: usize,
    pub max_list_len: usize,
    pub max_string_bytes: usize,
    /// Not enforced by `cel` 0.13: `Program::execute` has no step/cost/fuel API (only parse recursion limits).
    pub max_eval_steps: u64,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_expression_bytes: 256 * 1024,
            max_output_json_bytes: 16 * 1024 * 1024,
            max_list_len: 100_000,
            max_string_bytes: 1024 * 1024,
            max_eval_steps: 1_000_000,
        }
    }
}

impl SecurityLimits {
    pub fn check_expr(&self, src: &str) -> Result<(), String> {
        if src.len() > self.max_expression_bytes {
            return Err(format!(
                "expression exceeds max {} bytes",
                self.max_expression_bytes
            ));
        }
        Ok(())
    }

    /// Serialized JSON size of `records` (approximate output bound, spec §15).
    pub fn check_output_records(
        &self,
        records: &std::collections::BTreeMap<String, Vec<serde_json::Value>>,
    ) -> Result<(), String> {
        let bytes = serde_json::to_string(records)
            .map(|s| s.len())
            .map_err(|e| e.to_string())?;
        if bytes > self.max_output_json_bytes {
            return Err(format!(
                "mapping output exceeds max {} bytes (serialized records are {} bytes)",
                self.max_output_json_bytes, bytes
            ));
        }
        Ok(())
    }
}
