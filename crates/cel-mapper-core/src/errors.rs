//! Compatibility re-exports for evaluator and mapping diagnostics.

pub use cel_evaluator::{
    truncate_diagnostic_string, CompileError, ErrorCode, ErrorSeverity, ExpressionIssue,
    ExpressionPhase, ExpressionPreviewResult, MappingError, StandaloneEvalError,
};
