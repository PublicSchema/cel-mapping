//! CEL registration adapter for `crosswalk-functions`.
//!
//! This crate owns CEL helper registration, CEL `Value` conversion, arity
//! validation, helper metadata, missing/null behavior, and request fallback
//! resolution for helper inputs such as country, timezone, and today. Pure
//! deterministic helper semantics stay in `crosswalk-functions`.

mod budget;
mod builtins;
pub mod eval_ctx;
mod helpers;
pub mod missing;
mod output;
mod phone;

pub use budget::{BudgetGuard, FunctionSecurityLimits};
pub use builtins::{helper_metadata, register_crosswalk_functions, register_stdlib};
pub use crosswalk_functions;

#[derive(Clone, Debug)]
pub struct HelperMetadata {
    pub name: &'static str,
    pub arity: HelperArity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HelperArity {
    Exact(usize),
    OneOrTwo,
    Variadic,
}

pub type FunctionRegistry = crosswalk_functions::codes::CodeSystemRegistry;
pub type FunctionRequestContext = eval_ctx::FunctionRequestContext;
