//! Per-evaluation host limits (spec §15) — list/string sizes for stdlib hot paths.
//! Installed for the duration of CEL evaluation.
//!
//! **List length:** enforced for variadic host calls, list **outputs**, and list **inputs** to
//! O(n) stdlib transforms (`list_compact`, `list_join`, …). Read-only accessors (`list_length`,
//! `list_first`, `list_contains`, …) do **not** cap source list size so large `source.*` arrays
//! keep working with CEL’s own `.size()` / indexing.

use cel::ExecutionError;
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    static ACTIVE: RefCell<Option<Arc<FunctionSecurityLimits>>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug)]
pub struct FunctionSecurityLimits {
    pub max_list_len: usize,
    pub max_string_bytes: usize,
}

impl Default for FunctionSecurityLimits {
    fn default() -> Self {
        Self {
            max_list_len: 100_000,
            max_string_bytes: 1024 * 1024,
        }
    }
}

/// RAII guard: installs limits for the current thread until dropped.
pub struct BudgetGuard {
    prev: Option<Arc<FunctionSecurityLimits>>,
}

impl BudgetGuard {
    pub fn install(limits: Arc<FunctionSecurityLimits>) -> Self {
        let prev = ACTIVE.with(|a| (*a.borrow_mut()).replace(limits));
        Self { prev }
    }
}

impl Drop for BudgetGuard {
    fn drop(&mut self) {
        ACTIVE.with(|a| {
            *a.borrow_mut() = self.prev.take();
        });
    }
}

fn current() -> Option<Arc<FunctionSecurityLimits>> {
    ACTIVE.with(|a| a.borrow().clone())
}

pub fn enforce_max_list_len(n: usize) -> Result<(), ExecutionError> {
    let Some(lim) = current() else {
        return Ok(());
    };
    if n > lim.max_list_len {
        return Err(ExecutionError::function_error(
            "budget",
            format!("list length {n} exceeds max {}", lim.max_list_len),
        ));
    }
    Ok(())
}

pub fn enforce_max_string_bytes(len: usize) -> Result<(), ExecutionError> {
    let Some(lim) = current() else {
        return Ok(());
    };
    if len > lim.max_string_bytes {
        return Err(ExecutionError::function_error(
            "budget",
            format!(
                "string size {len} bytes exceeds max {}",
                lim.max_string_bytes
            ),
        ));
    }
    Ok(())
}
