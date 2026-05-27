//! Pure deterministic helper functions for mapping and registry transforms.
//!
//! This crate intentionally has no CEL dependency. Adapter crates own CEL coercion,
//! missing/null handling, and request-context fallback resolution.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionError {
    pub code: &'static str,
    pub message: String,
}

impl FunctionError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FunctionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for FunctionError {}

#[derive(Debug, Error)]
pub enum InfallibleFunctionError {}

#[cfg(feature = "codes")]
pub mod codes;
#[cfg(feature = "date")]
pub mod date;
#[cfg(feature = "email")]
pub mod email;
#[cfg(feature = "ids")]
pub mod ids;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "phone")]
pub mod phone;
#[cfg(feature = "redaction")]
pub mod redaction;
#[cfg(feature = "text")]
pub mod text;
