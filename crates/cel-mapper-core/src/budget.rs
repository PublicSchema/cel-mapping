//! Compatibility facade for adapter budget limits.

use crate::security::SecurityLimits;
use cel::ExecutionError;
use std::sync::Arc;

pub struct BudgetGuard {
    _inner: mapping_functions_cel::BudgetGuard,
}

impl BudgetGuard {
    pub fn install(limits: Arc<SecurityLimits>) -> Self {
        Self {
            _inner: mapping_functions_cel::BudgetGuard::install(Arc::new(
                mapping_functions_cel::FunctionSecurityLimits {
                    max_list_len: limits.max_list_len,
                    max_string_bytes: limits.max_string_bytes,
                },
            )),
        }
    }
}

#[allow(dead_code)]
pub fn enforce_max_list_len(n: usize) -> Result<(), ExecutionError> {
    let _ = n;
    Ok(())
}

#[allow(dead_code)]
pub fn enforce_max_string_bytes(_len: usize) -> Result<(), ExecutionError> {
    Ok(())
}
