//! Standalone CEL compile, evaluate, preview, and adapter registration boundary.
//!
//! This crate owns expression-oriented runtime behavior shared by the v0.1
//! mapper and the PublicSchema runtime: security limits, diagnostics, missing
//! value handling, root bindings, and registration of `mapping-functions-cel`.
//! It intentionally does not depend on `cel-mapper-core` or
//! `publicschema-mapping`; those crates call this boundary instead.

pub mod ast_paths;
mod cel_scan;
pub mod compiled;
pub mod compiler;
pub mod errors;
pub mod evaluator;
pub mod expr;
pub mod missing;
pub mod output;
pub mod paths;
pub mod security;

pub use cel::Value as CelValue;
pub use compiled::{CompiledCel, ErrorMode};
pub use compiler::compile_expr;
pub use errors::{
    truncate_diagnostic_string, CompileError, ErrorCode, ErrorSeverity, ExpressionIssue,
    ExpressionPhase, ExpressionPreviewResult, MappingError, StandaloneEvalError,
};
pub use evaluator::{
    evaluate_cel_expression, evaluate_cel_expression_with_input, json_to_cel,
    preview_cel_expression, preview_cel_expression_with_input, run_program,
    validate_root_binding_name, StandaloneExpressionInput,
};
pub use mapping_functions;
pub use mapping_functions_cel;
pub use security::SecurityLimits;
