//! Compatibility facade for adapter budget limits.

use crate::security::SecurityLimits;
use std::sync::Arc;

pub struct BudgetGuard {
    _inner: crosswalk_functions_cel::BudgetGuard,
}

impl BudgetGuard {
    pub fn install(limits: Arc<SecurityLimits>) -> Self {
        Self {
            _inner: crosswalk_functions_cel::BudgetGuard::install(Arc::new(
                crosswalk_functions_cel::FunctionSecurityLimits {
                    max_list_len: limits.max_list_len,
                    max_string_bytes: limits.max_string_bytes,
                },
            )),
        }
    }
}
