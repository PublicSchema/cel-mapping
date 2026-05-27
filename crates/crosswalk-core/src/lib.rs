//! CEL mapping runtime — spec v0.1 ([`spec.md`](../../../spec.md) at repo root).
//!
//! **Entry points:** [`runtime::MappingRuntime`] (compile YAML mappings, [`runtime::MappingRuntime::evaluate`]),
//! standalone CEL via [`evaluator::evaluate_cel_expression`] and editor diagnostics via
//! [`evaluator::preview_cel_expression`] / [`errors::ExpressionPreviewResult`].
//! See the workspace [`README.md`](../../../README.md) for layout and binding commands.

pub mod code_system;
pub mod compiled;
pub mod compiler;
pub mod errors;
pub mod eval_ctx;
pub mod evaluator;
mod iso_systems;
pub mod mapping;
pub mod publicschema;
pub mod runtime;
pub mod security;

mod budget;

pub use code_system::{CodeEntry, CodeSystemRegistry};
pub use compiled::{CompiledCel, CompiledMapping, ErrorMode};
pub use compiler::compile_mapping_yaml;
pub use crosswalk_cel::{
    evaluate_cel_expression, evaluate_cel_expression_with_input, expr, missing, output, paths,
    preview_cel_expression, preview_cel_expression_with_input, validate_root_binding_name,
    StandaloneExpressionInput,
};
pub use crosswalk_functions_cel::{helper_metadata, HelperArity, HelperMetadata};
pub use errors::{
    CompileError, ErrorCode, ErrorSeverity, ExpressionIssue, ExpressionPhase,
    ExpressionPreviewResult, MappingError, StandaloneEvalError,
};
pub use mapping::MappingDocument;
pub use paths::primary_binding_hint;
pub use publicschema::{
    compile_publicschema_mapping, evaluate_publicschema_mapping,
    preview_publicschema_rule_expression, CompiledPublicSchemaMapping, PrivacyMode,
    PublicSchemaBindingMode, PublicSchemaCompileMeta, PublicSchemaCompileOptions,
    PublicSchemaDirection, PublicSchemaEvaluateOptions, PublicSchemaEvaluationInput,
    PublicSchemaRuleLogEntry, PublicSchemaTransformOutput,
};
pub use runtime::{EvaluationInput, MappingOutput, MappingRuntime, RuntimeOptions};
pub use security::SecurityLimits;
