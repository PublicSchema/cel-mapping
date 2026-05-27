//! Compatibility re-exports for evaluator and mapping diagnostics.

pub use crosswalk_cel::{
    truncate_diagnostic_string, CompileError, ErrorCode, ErrorSeverity, ExpressionIssue,
    ExpressionPhase, ExpressionPreviewResult, MappingError, StandaloneEvalError,
};
